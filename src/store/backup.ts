import { create } from "zustand";
import { translate, useI18n, type TranslationKey } from "../i18n";
import { getBackend } from "../lib/tauri";
import { useBuffers } from "./buffers";
import { reloadChangedFiles, useHistory } from "./history";
import type {
  Backup,
  BackupPending,
  BackupProgress,
  BackupStatus,
  BackupTools,
  Identity,
  Pulled,
} from "./types";

/**
 * The backup with Git (5.9) of the project on screen: where it stands, what changed since
 * the last backup and the list of backups. Nothing here runs by itself.
 */
type BackupState = {
  projectId: string | null;
  status: BackupStatus | null;
  pending: BackupPending | null;
  backups: Backup[];
  loading: boolean;
  /** What is running (one thing at a time). */
  working: "enable" | "backup" | "connect" | "push" | "pull" | "merge" | null;
  /** `git` and `gh` installed. */
  tools: BackupTools | null;
  /** Progress of the running push or pull. */
  progress: BackupProgress | null;
  /** The last push/pull/connect failure, shown in the GitHub section until the next try. */
  remoteError: unknown;

  /** Reads everything for `projectId`. */
  load: (projectId: string) => Promise<void>;
  /** Reads it again (after files changed, for instance). */
  refresh: () => Promise<void>;
  /** Turns the backup on; `author` only if `git config` has none. */
  enable: (author?: Identity) => Promise<void>;
  /**
   * Saves what is typed and backs up with `message` (null if nothing changed); then pushes
   * if «Subir después de cada respaldo» is on.
   */
  backup: (message: string) => Promise<Backup | null>;
  /** Connects with a new private GitHub repository (`gh`) or an existing URL. */
  connect: (target: { create: string } | { url: string }) => Promise<void>;
  push: () => Promise<void>;
  /**
   * «Traer cambios»: saves what is typed, backs up what is not backed up, and brings GitHub's
   * backups (merging them; on conflicts the merge is left in progress).
   */
  pull: () => Promise<Pulled | null>;
  /** «Terminar fusión»: commits the merge (refused while conflicts are left). */
  finishMerge: () => Promise<void>;
  /** «Cancelar fusión»: everything back as before pulling; the open files are reloaded. */
  abortMerge: () => Promise<void>;
  /** Keeps one side of a binary or deleted-and-changed file in conflict. */
  choose: (path: string, side: "ours" | "theirs") => Promise<void>;
  /** «Desconectar»: the project stops pushing to its repository. */
  disconnect: () => Promise<void>;
  cancel: () => Promise<void>;
  setPushAfterBackup: (value: boolean) => Promise<void>;
};

export const initialBackupState = {
  projectId: null,
  status: null,
  pending: null,
  backups: [],
  loading: false,
  working: null,
  tools: null,
  progress: null,
  remoteError: null,
};

export const useBackup = create<BackupState>()((set, get) => {
  /** Runs `job` as `working`, for the project it started with. */
  const work = async <T>(
    kind: NonNullable<BackupState["working"]>,
    job: (id: string) => Promise<T>,
  ) => {
    const projectId = get().projectId;
    if (!projectId || get().working) return null;
    set({ working: kind });
    try {
      return await job(projectId);
    } finally {
      if (get().projectId === projectId) set({ working: null });
    }
  };

  /** A network operation: its failure stays visible in the GitHub section. */
  const remoteWork = async <T>(
    kind: "connect" | "push" | "pull",
    job: (id: string) => Promise<T>,
  ) => {
    set({ remoteError: null, progress: null });
    try {
      return await work(kind, job);
    } catch (error) {
      set({ remoteError: error });
      throw error;
    } finally {
      set({ progress: null });
    }
  };

  /** Commit messages JuTeX writes, in the interface's language. */
  const message = (key: TranslationKey) => translate(useI18n.getState().locale, key);

  /** The list of backups, if it is on screen. */
  const reloadShownBackups = async (projectId: string) => {
    const shown = useHistory.getState();
    if (shown.projectId === projectId && shown.source === "backup") await shown.load(projectId);
  };

  const read = async (projectId: string) => {
    const backend = getBackend();
    const status = await backend.backupStatus(projectId);
    const [pending, backups] = status.enabled
      ? await Promise.all([backend.backupPending(projectId), backend.backupList(projectId)])
      : [null, []];
    if (get().projectId === projectId) set({ status, pending, backups });
  };

  return {
    ...initialBackupState,

    load: async (projectId) => {
      if (get().projectId !== projectId) set({ ...initialBackupState, projectId });
      set({ loading: true });
      try {
        if (!get().tools) {
          const tools = await getBackend().backupTools();
          if (get().projectId === projectId) set({ tools });
        }
        await read(projectId);
      } finally {
        if (get().projectId === projectId) set({ loading: false });
      }
    },

    refresh: async () => {
      const projectId = get().projectId;
      if (projectId) await read(projectId);
    },

    enable: async (author) => {
      await work("enable", async (projectId) => {
        // The .gitignore and the first list of changes see what is typed.
        await useBuffers.getState().saveAll();
        await getBackend().backupEnable(projectId, author);
        await read(projectId);
      });
    },

    backup: async (message) =>
      await work("backup", async (projectId) => {
        await useBuffers.getState().saveAll();
        const done = await getBackend().backupCreate(projectId, message);
        await read(projectId);
        await reloadShownBackups(projectId);
        return done;
      }).then(async (done) => {
        const { status } = get();
        if (done && status?.pushAfterBackup && status.remote) await get().push();
        return done;
      }),

    connect: async (target) => {
      await remoteWork("connect", async (projectId) => {
        const backend = getBackend();
        const status =
          "create" in target
            ? await backend.backupCreateGithub(projectId, target.create)
            : await backend.backupConnectUrl(projectId, target.url);
        if (get().projectId === projectId) set({ status });
      });
    },

    push: async () => {
      await remoteWork("push", async (projectId) => {
        const status = await getBackend().backupPush(projectId);
        if (get().projectId === projectId) set({ status });
        await read(projectId);
      });
    },

    pull: async () =>
      await remoteWork("pull", async (projectId) => {
        // Unsaved edits are written first, so they are backed up before merging.
        await useBuffers.getState().saveAll();
        const pulled = await getBackend().backupPull(
          projectId,
          message("backup.beforePull"),
          message("backup.mergeMessage"),
        );
        if (pulled.changed.length > 0) await reloadChangedFiles(projectId, pulled.changed);
        await read(projectId);
        await reloadShownBackups(projectId);
        return pulled;
      }),

    finishMerge: async () => {
      await work("merge", async (projectId) => {
        await useBuffers.getState().saveAll();
        const status = await getBackend().backupFinishMerge(
          projectId,
          message("backup.mergeMessage"),
        );
        if (get().projectId === projectId) set({ status });
        await read(projectId);
        await reloadShownBackups(projectId);
      });
      const { status } = get();
      if (status?.pushAfterBackup && status.remote && !status.merge) await get().push();
    },

    abortMerge: async () => {
      await work("merge", async (projectId) => {
        // Saved first, so the open files take what the cancel leaves on disk (hand edits
        // inside the conflicts go away with the merge, as the confirmation says).
        await useBuffers.getState().saveAll();
        const changed = await getBackend().backupAbortMerge(projectId);
        if (changed.length > 0) await reloadChangedFiles(projectId, changed);
        await read(projectId);
      });
    },

    choose: async (path, side) => {
      await work("merge", async (projectId) => {
        await useBuffers.getState().saveAll();
        await getBackend().backupChoose(projectId, path, side);
        await reloadChangedFiles(projectId, [path]);
        await read(projectId);
      });
    },

    disconnect: async () => {
      await work("connect", async (projectId) => {
        const status = await getBackend().backupDisconnect(projectId);
        if (get().projectId === projectId) set({ status, remoteError: null });
      });
    },

    cancel: async () => {
      const projectId = get().projectId;
      if (projectId) await getBackend().backupCancel(projectId);
    },

    setPushAfterBackup: async (value) => {
      const projectId = get().projectId;
      if (!projectId) return;
      const status = await getBackend().backupSetPush(projectId, value);
      if (get().projectId === projectId) set({ status });
    },
  };
});
