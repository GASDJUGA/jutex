/**
 * The backup with Git (5.9) of the in-memory backend: the same rules as src-tauri/src/backup,
 * with commits as snapshots of the project's text files.
 */
import type {
  Backup,
  BackendError,
  BackupPending,
  BackupProgress,
  BackupStatus,
  Conflict,
  FileChange,
  Identity,
  Settings,
} from "../store/types";
import { findConflicts } from "./conflicts";
import type { Backend } from "./tauri";
import type { MockFiles } from "./mockFiles";
import { repoKey, validRemoteUrl, validRepoName } from "./remote";
import { cleanIdentity } from "./settings";

const fail = (code: string, message: string): never => {
  throw { code, message } satisfies BackendError;
};

type Commit = { backup: Backup; contents: Record<string, string> };

type Repo = {
  /** Newest first. */
  commits: Commit[];
  /** `origin` and its branch as last fetched/pushed (`null`: nothing there yet). */
  remote: string | null;
  remoteCommits: Commit[] | null;
  /** `git config` author (none by default, so JuTeX's is asked for). */
  identity: Identity | null;
  pushAfterBackup: boolean;
  /** A merge in progress: the files that had conflicts and GitHub's backups being merged. */
  merge?: { conflicts: Omit<Conflict, "remaining">[]; remote: Commit[]; chosen: string[] } | null;
};

const GITIGNORE = "# JuTeX: datos del editor y archivos que genera LaTeX\n.jutex/\n*.aux\n*.log\n";

/** Lines changed between two snapshots, as `FileChange`s sorted by path. */
export function diffSnapshots(
  before: Record<string, string>,
  after: Record<string, string>,
): FileChange[] {
  return [...new Set([...Object.keys(after), ...Object.keys(before)])]
    .sort()
    .flatMap((path): FileChange[] =>
      !(path in before)
        ? [{ path, kind: "added" }]
        : !(path in after)
          ? [{ path, kind: "deleted" }]
          : after[path] !== before[path]
            ? [{ path, kind: "modified" }]
            : [],
    );
}

type BackupMethods = Pick<
  Backend,
  | "backupStatus"
  | "backupEnable"
  | "githubIdentity"
  | "backupPending"
  | "backupCreate"
  | "backupList"
  | "backupChanges"
  | "backupFile"
  | "backupRestoreFile"
  | "backupRestore"
  | "backupTools"
  | "backupSetPush"
  | "backupConnectUrl"
  | "backupCreateGithub"
  | "backupPush"
  | "backupPull"
  | "backupFinishMerge"
  | "backupAbortMerge"
  | "backupChoose"
  | "backupDisconnect"
  | "backupCancel"
  | "onBackupProgress"
> & {
  /** Projects connected to each remote (`repoKey` → project id). */
  knownRemotes: () => Map<string, string>;
  /** A project just cloned from `url`: its backup is on and in step with the remote. */
  adoptClone: (projectId: string, url: string) => void;
  emitProgress: (progress: BackupProgress) => void;
  /** Simulates a backup pushed from another clone (`null` deletes the file there). */
  remoteBackup: (projectId: string, message: string, files: Record<string, string | null>) => void;
};

export function createMockBackup(
  files: (id: string) => MockFiles,
  settings: () => Settings,
  saveIdentity: (identity: Identity) => void,
): BackupMethods {
  const repos = new Map<string, Repo>();
  const progressListeners = new Set<(p: BackupProgress) => void>();
  const ids = (commits: Commit[]) => new Set(commits.map((c) => c.backup.id));
  /** Local backups the remote branch lacks, and remote ones not here. */
  const aheadBehind = (r: Repo) => {
    if (!r.remoteCommits) return { ahead: r.commits.length, behind: 0 };
    const local = ids(r.commits);
    const remote = ids(r.remoteCommits);
    return {
      ahead: r.commits.filter((c) => !remote.has(c.backup.id)).length,
      behind: r.remoteCommits.filter((c) => !local.has(c.backup.id)).length,
    };
  };
  const requireRemote = (r: Repo) =>
    r.remote ?? fail("forbidden", "el proyecto no está conectado con un repositorio remoto");
  let counter = 0;

  const repo = (id: string) => {
    files(id);
    return repos.get(id) ?? fail("forbidden", "el respaldo con Git no está activado");
  };
  const snapshot = (id: string) => Object.fromEntries(files(id).textFiles());
  const head = (r: Repo) => r.commits[0]?.contents ?? {};
  const author = (r: Repo): Identity => {
    const saved = cleanIdentity({ name: settings().gitName, email: settings().gitEmail });
    return r.identity ?? saved ?? fail("needsIdentity", "falta el autor de los respaldos");
  };
  const ensureGitignore = (id: string) => {
    const fs = files(id);
    const current = snapshot(id)[".gitignore"];
    if (current === undefined) fs.write(".gitignore", GITIGNORE);
    else if (!current.includes(".jutex")) fs.write(".gitignore", `${current}\n${GITIGNORE}`);
  };
  const commit = (id: string, r: Repo, message: string): Backup | null => {
    const contents = snapshot(id);
    if (r.commits.length > 0 && diffSnapshots(head(r), contents).length === 0) return null;
    const who = author(r);
    const backup: Backup = {
      id: `b${++counter}`,
      message: message.split("\n")[0],
      author: who.name,
      time: Date.now(),
      pushed: null,
    };
    r.commits.unshift({ backup, contents });
    return backup;
  };
  const find = (r: Repo, version: string) =>
    r.commits.find((c) => c.backup.id === version) ?? fail("notFound", version);
  const status = (id: string): BackupStatus => {
    const r = repos.get(id);
    files(id);
    const saved = cleanIdentity({ name: settings().gitName, email: settings().gitEmail });
    return {
      enabled: !!r,
      insideOtherRepo: false,
      branch: r ? "main" : null,
      identity: r?.identity ?? saved,
      identitySource: r?.identity ? "git" : saved ? "jutex" : null,
      remote: r?.remote ?? null,
      tracking: !!r?.remoteCommits,
      ...(r ? aheadBehind(r) : { ahead: 0, behind: 0 }),
      pending: r ? diffSnapshots(head(r), snapshot(id)).length : 0,
      pushAfterBackup: r?.pushAfterBackup ?? false,
      merge: r?.merge ? { conflicts: remaining(id, r.merge) } : null,
    };
  };
  /** What is left of each conflict, from the files (like `merge::state` in Rust). */
  const remaining = (id: string, merge: NonNullable<Repo["merge"]>): Conflict[] => {
    const now = snapshot(id);
    return merge.conflicts.map((c) => ({
      ...c,
      remaining:
        c.kind === "text"
          ? findConflicts(now[c.path] ?? "").length
          : Number(!merge.chosen.includes(c.path)),
    }));
  };
  const refuseDuringMerge = (r: Repo) => {
    if (r.merge) fail("mergeInProgress", "termina o cancela la fusión con GitHub");
  };
  /** The merge commit: this project as it is, after GitHub's backups. */
  const commitMerge = (id: string, r: Repo, message: string, remote: Commit[]) => {
    const local = ids(r.commits);
    const brought = remote.filter((c) => !local.has(c.backup.id));
    r.commits = [...r.commits, ...brought];
    r.merge = null;
    const backup: Backup = {
      id: `b${++counter}`,
      message: message.split("\n")[0],
      author: author(r).name,
      time: Date.now(),
      pushed: null,
    };
    r.commits.unshift({ backup, contents: snapshot(id) });
  };

  return {
    backupStatus: async (id) => status(id),
    backupEnable: async (id, identity) => {
      if (identity) {
        const clean = cleanIdentity(identity) ?? fail("invalidName", "el correo no es válido");
        saveIdentity(clean);
      }
      if (!repos.has(id)) {
        repos.set(id, {
          commits: [],
          remote: null,
          remoteCommits: null,
          identity: null,
          pushAfterBackup: false,
        });
      }
      ensureGitignore(id);
      return status(id);
    },
    githubIdentity: async () => ({
      name: "Ana Pérez",
      email: "1234567+ana@users.noreply.github.com",
    }),
    backupPending: async (id): Promise<BackupPending> => ({
      changes: diffSnapshots(head(repo(id)), snapshot(id)),
      tooBig: [],
    }),
    backupCreate: async (id, message) => {
      const r = repo(id);
      refuseDuringMerge(r);
      const text = message.trim();
      if (!text || [...text].length > 1000) fail("invalidName", "mensaje del respaldo");
      ensureGitignore(id);
      return commit(id, r, text);
    },
    backupList: async (id) => {
      const r = repo(id);
      const remote = r.remoteCommits ? ids(r.remoteCommits) : null;
      return r.commits.map((c) => ({
        ...c.backup,
        pushed: remote ? remote.has(c.backup.id) : null,
      }));
    },
    backupChanges: async (id, version) => {
      const r = repo(id);
      const index = r.commits.findIndex((c) => c.backup.id === version);
      if (index < 0) fail("notFound", version);
      return diffSnapshots(r.commits[index + 1]?.contents ?? {}, r.commits[index].contents);
    },
    backupFile: async (id, version, path) => ({
      content: find(repo(id), version).contents[path] ?? null,
      binary: false,
    }),
    backupRestoreFile: async (id, version, path, before) => {
      const r = repo(id);
      refuseDuringMerge(r);
      const content = find(r, version).contents[path];
      commit(id, r, before);
      if (content === undefined) files(id).delete(path);
      else files(id).write(path, content);
    },
    backupTools: async () => ({ git: true, gh: true }),
    backupSetPush: async (id, value) => {
      repo(id).pushAfterBackup = value;
      return status(id);
    },
    backupConnectUrl: async (id, url) => {
      const r = repo(id);
      if (!validRemoteUrl(url))
        fail("invalidName", "la dirección debe empezar con https:// o git@");
      if (r.remote !== url.trim()) r.remoteCommits = null;
      r.remote = url.trim();
      return status(id);
    },
    backupCreateGithub: async (id, name) => {
      const r = repo(id);
      if (!validRepoName(name)) fail("invalidName", "nombre del repositorio");
      // «Conectar otro…»: the new repository replaces the connected one.
      r.remote = `https://github.com/ana/${name.trim()}.git`;
      r.remoteCommits = null;
      return status(id);
    },
    backupPush: async (id) => {
      const r = repo(id);
      refuseDuringMerge(r);
      requireRemote(r);
      if (r.commits.length === 0) fail("forbidden", "todavía no hay respaldos que subir");
      if (aheadBehind(r).behind > 0) fail("diverged", "[rejected] main -> main (fetch first)");
      for (const percent of [50, 100]) {
        progressListeners.forEach((l) =>
          l({ projectId: id, operation: "push", phase: "Writing objects", percent }),
        );
        await new Promise((resolve) => setTimeout(resolve, 10));
      }
      r.remoteCommits = [...r.commits];
      return status(id);
    },
    backupPull: async (id, before, merged) => {
      const r = repo(id);
      refuseDuringMerge(r);
      requireRemote(r);
      if (diffSnapshots(head(r), snapshot(id)).length > 0) commit(id, r, before);
      const { ahead, behind } = aheadBehind(r);
      const nothing = { commits: 0, changed: [], merged: false, conflicts: [] };
      if (!r.remoteCommits || behind === 0) return nothing;
      const target = r.remoteCommits[0].contents;
      const changed = diffSnapshots(head(r), target).map((c) => c.path);
      if (ahead === 0) {
        for (const path of changed) {
          if (path in target) files(id).write(path, target[path]);
          else files(id).delete(path);
        }
        r.commits = [...r.remoteCommits];
        return { commits: behind, changed, merged: false, conflicts: [] };
      }
      // Both sides: per file, whole-file merge against the last backup both have.
      const remoteIds = ids(r.remoteCommits);
      const base = r.commits.find((c) => remoteIds.has(c.backup.id))?.contents ?? {};
      const ours = head(r);
      const conflicts: Omit<Conflict, "remaining">[] = [];
      for (const path of changed) {
        const mine = ours[path];
        const theirs = target[path];
        if (mine === base[path]) {
          if (theirs === undefined) files(id).delete(path);
          else files(id).write(path, theirs);
        } else if (theirs !== base[path] && theirs !== mine) {
          if (mine === undefined || theirs === undefined) {
            conflicts.push({ path, kind: "deleted" });
          } else {
            conflicts.push({ path, kind: "text" });
            files(id).write(path, `<<<<<<< HEAD\n${mine}=======\n${theirs}>>>>>>> origin/main\n`);
          }
        }
      }
      if (conflicts.length > 0) {
        r.merge = { conflicts, remote: [...r.remoteCommits], chosen: [] };
        return { commits: behind, changed, merged: false, conflicts: remaining(id, r.merge) };
      }
      commitMerge(id, r, merged, r.remoteCommits);
      return { commits: behind, changed, merged: true, conflicts: [] };
    },
    backupFinishMerge: async (id, message) => {
      const r = repo(id);
      if (!r.merge) fail("forbidden", "no hay una fusión en curso");
      const left = remaining(id, r.merge!).find((c) => c.remaining > 0);
      if (left) fail("forbidden", `quedan conflictos sin resolver en ${left.path}`);
      commitMerge(id, r, message, r.merge!.remote);
      return status(id);
    },
    backupAbortMerge: async (id) => {
      const r = repo(id);
      if (!r.merge) return [];
      const back = head(r);
      const changed = diffSnapshots(back, snapshot(id)).map((c) => c.path);
      for (const path of changed) {
        if (path in back) files(id).write(path, back[path]);
        else files(id).delete(path);
      }
      r.merge = null;
      return changed;
    },
    backupChoose: async (id, path, side) => {
      const r = repo(id);
      const conflict = r.merge?.conflicts.find((c) => c.path === path);
      if (!conflict || conflict.kind === "text") fail("forbidden", `${path} no se elige así`);
      const kept = side === "ours" ? head(r)[path] : r.merge!.remote[0].contents[path];
      if (kept === undefined) files(id).delete(path);
      else files(id).write(path, kept);
      r.merge!.chosen.push(path);
      return status(id);
    },
    backupDisconnect: async (id) => {
      const r = repo(id);
      r.remote = null;
      r.remoteCommits = null;
      return status(id);
    },
    backupCancel: async () => {},
    onBackupProgress: (callback) => {
      progressListeners.add(callback);
      return () => progressListeners.delete(callback);
    },
    knownRemotes: () =>
      new Map(
        [...repos.entries()].flatMap(([id, r]) => {
          const key = r.remote ? repoKey(r.remote) : null;
          return key ? [[key, id] as [string, string]] : [];
        }),
      ),
    adoptClone: (id, url) => {
      const contents = snapshot(id);
      const backup: Backup = {
        id: `b${++counter}`,
        message: "Primer respaldo",
        author: "Ana",
        time: Date.now(),
        pushed: true,
      };
      const commits = [{ backup, contents }];
      repos.set(id, {
        commits,
        remote: url,
        remoteCommits: [...commits],
        identity: null,
        pushAfterBackup: false,
      });
    },
    emitProgress: (progress) => progressListeners.forEach((l) => l(progress)),
    remoteBackup: (id, message, changes) => {
      const r = repo(id);
      const base = r.remoteCommits?.[0]?.contents ?? {};
      const contents = { ...base };
      for (const [path, content] of Object.entries(changes)) {
        if (content === null) delete contents[path];
        else contents[path] = content;
      }
      const backup: Backup = {
        id: `b${++counter}`,
        message,
        author: "Otra",
        time: Date.now(),
        pushed: true,
      };
      r.remoteCommits = [{ backup, contents }, ...(r.remoteCommits ?? [])];
    },
    backupRestore: async (id, version, before) => {
      const r = repo(id);
      refuseDuringMerge(r);
      const target = find(r, version).contents;
      commit(id, r, before);
      const changed = diffSnapshots(snapshot(id), target).map((c) => c.path);
      for (const path of changed) {
        if (path in target) files(id).write(path, target[path]);
        else files(id).delete(path);
      }
      return changed;
    },
  };
}
