//! Merging what GitHub has with what is here (5.9, «Traer cambios»). Git does the merge
//! (`git merge --no-commit`) and writes its conflict marks into the files; this module reads
//! that state back, so the editor can show each conflict and the panel can finish or cancel
//! the merge. The state lives in the project's `.git` (`MERGE_HEAD`, the index, and
//! `JUTEX_MERGE` with the files that had conflicts), so it survives closing JuTeX.

use crate::error::{AppError, AppResult};
use crate::gitops::git_error;
use crate::util::fs::atomic_write;
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// How a file is in conflict, which decides how it is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictKind {
    /// Text with Git's marks, resolved in the editor.
    Text,
    /// An image, a PDF…: one side or the other.
    Binary,
    /// One side deleted it and the other changed it: one side or the other.
    Deleted,
}

impl ConflictKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Binary => "binary",
            Self::Deleted => "deleted",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "text" => Some(Self::Text),
            "binary" => Some(Self::Binary),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

/// A file that had a conflict in the merge in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    /// Relative to the project, with `/`.
    pub path: String,
    pub kind: ConflictKind,
    /// Conflicts left in it (0: resolved; it stays listed until the merge is finished).
    pub remaining: usize,
}

/// Where JuTeX keeps the list of files that had conflicts, next to Git's `MERGE_HEAD`.
fn list_file(repo: &Repository) -> PathBuf {
    repo.path().join("JUTEX_MERGE")
}

/// A merge is in progress (`git merge` left its `MERGE_HEAD`).
pub fn in_progress(repo: &Repository) -> bool {
    repo.path().join("MERGE_HEAD").exists()
}

/// Fails with [`AppError::MergeInProgress`] while a merge is in progress.
pub fn refuse_during(repo: &Repository) -> AppResult<()> {
    if in_progress(repo) {
        return Err(AppError::MergeInProgress(
            "termina o cancela la fusión con GitHub".into(),
        ));
    }
    Ok(())
}

fn is_line(line: &str, mark: &str) -> bool {
    line.strip_suffix('\r').unwrap_or(line) == mark
}

/// Complete conflict blocks in `text`: a line starting with `<<<<<<< `, then a line that is
/// exactly `=======`, then one starting with `>>>>>>> ` (lines may end in `\r`).
pub fn count_blocks(text: &str) -> usize {
    #[derive(PartialEq)]
    enum Seen {
        Nothing,
        Ours,
        Theirs,
    }
    let mut seen = Seen::Nothing;
    let mut blocks = 0;
    for line in text.split('\n') {
        if line.starts_with("<<<<<<< ") {
            seen = Seen::Ours;
        } else if seen == Seen::Ours && is_line(line, "=======") {
            seen = Seen::Theirs;
        } else if seen == Seen::Theirs && line.starts_with(">>>>>>> ") {
            blocks += 1;
            seen = Seen::Nothing;
        }
    }
    blocks
}

/// What Git left in the index for each path with a conflict: whether ours and theirs are
/// there, and whether every version present is text.
fn index_conflicts(repo: &Repository) -> AppResult<BTreeMap<String, ConflictKind>> {
    let index = repo.index().map_err(git_error)?;
    let mut out = BTreeMap::new();
    for conflict in index.conflicts().map_err(git_error)? {
        let conflict = conflict.map_err(git_error)?;
        let entries = [&conflict.ancestor, &conflict.our, &conflict.their];
        let Some(path) = entries.iter().find_map(|e| {
            e.as_ref()
                .map(|e| String::from_utf8_lossy(&e.path).into_owned())
        }) else {
            continue;
        };
        let kind = if conflict.our.is_none() || conflict.their.is_none() {
            ConflictKind::Deleted
        } else {
            let text = entries.iter().filter_map(|e| e.as_ref()).all(|e| {
                repo.find_blob(e.id)
                    .is_ok_and(|b| !b.is_binary() && std::str::from_utf8(b.content()).is_ok())
            });
            if text {
                ConflictKind::Text
            } else {
                ConflictKind::Binary
            }
        };
        out.insert(path, kind);
    }
    Ok(out)
}

/// Conflicts left in `path`: its blocks of marks for text, otherwise whether the index
/// still has its conflict.
fn remaining(root: &Path, path: &str, kind: ConflictKind, unresolved: bool) -> usize {
    match kind {
        ConflictKind::Text => fs::read_to_string(root.join(path))
            .map(|t| count_blocks(&t))
            .unwrap_or(0),
        ConflictKind::Binary | ConflictKind::Deleted => usize::from(unresolved),
    }
}

/// Right after `git merge` stopped with conflicts: lists them and remembers them in
/// `JUTEX_MERGE`.
pub(crate) fn start(repo: &Repository, root: &Path) -> AppResult<Vec<Conflict>> {
    let conflicts = index_conflicts(repo)?;
    let list: String = conflicts
        .iter()
        .map(|(path, kind)| format!("{}\t{path}\n", kind.as_str()))
        .collect();
    atomic_write(&list_file(repo), list.as_bytes())?;
    Ok(conflicts
        .into_iter()
        .map(|(path, kind)| Conflict {
            remaining: remaining(root, &path, kind, true),
            path,
            kind,
        })
        .collect())
}

/// The files of `JUTEX_MERGE`, in order.
fn remembered(repo: &Repository) -> Vec<(String, ConflictKind)> {
    fs::read_to_string(list_file(repo))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (kind, path) = line.split_once('\t')?;
            Some((path.to_owned(), ConflictKind::parse(kind)?))
        })
        .collect()
}

/// A merge in progress: the files that had conflicts and what is left in each.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeState {
    pub conflicts: Vec<Conflict>,
}

pub(crate) fn state_of(repo: &Repository, root: &Path) -> AppResult<Option<MergeState>> {
    if !in_progress(repo) {
        return Ok(None);
    }
    let unresolved = index_conflicts(repo)?;
    let mut files = remembered(repo);
    // A merge started outside JuTeX (`git pull` in a terminal), or JuTeX closed before it
    // wrote its list: the index still knows the conflicts.
    for (path, kind) in &unresolved {
        if !files.iter().any(|(p, _)| p == path) {
            files.push((path.clone(), *kind));
        }
    }
    let conflicts = files
        .into_iter()
        .map(|(path, kind)| Conflict {
            remaining: remaining(root, &path, kind, unresolved.contains_key(&path)),
            path,
            kind,
        })
        .collect();
    Ok(Some(MergeState { conflicts }))
}

/// The merge in progress, read from the project's `.git` (`None`: there is none).
pub fn state(root: &Path) -> AppResult<Option<MergeState>> {
    match super::open(root)? {
        Some(repo) => state_of(&repo, root),
        None => Ok(None),
    }
}

/// «Terminar fusión»: refused while a file still has conflicts; otherwise commits the
/// project as it is on disk as the merge, with `message`.
pub fn finish(root: &Path, message: &str, fallback: Option<&super::Identity>) -> AppResult<()> {
    let repo = super::require(root)?;
    let Some(state) = state_of(&repo, root)? else {
        return Err(AppError::Forbidden("no hay una fusión en curso".into()));
    };
    if let Some(left) = state.conflicts.iter().find(|c| c.remaining > 0) {
        return Err(AppError::Forbidden(format!(
            "quedan conflictos sin resolver en {}",
            left.path
        )));
    }
    super::commit_merge(&repo, root, message, fallback)?;
    Ok(())
}

/// «Cancelar fusión»: `git merge --abort`. Returns the paths that changed on disk.
pub fn abort(root: &Path) -> AppResult<Vec<String>> {
    let repo = super::require(root)?;
    if !in_progress(&repo) {
        return Ok(Vec::new());
    }
    // What differs from the last backup now (what the merge brought or the user edited)
    // is what goes back.
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let mut changed: Vec<String> = repo
        .statuses(Some(&mut options))
        .map_err(git_error)?
        .iter()
        .filter_map(|e| e.path().ok().map(str::to_owned))
        .collect();
    changed.sort();
    changed.dedup();
    let out = super::remote::run_git(root, &["merge", "--abort"])?;
    if !out.success {
        // `--abort` refuses when a file the merge brought was edited afterwards. Everything
        // was backed up before pulling and `HEAD` is still that backup: back to it.
        let reset = super::remote::run_git(root, &["reset", "-q", "--hard", "HEAD"])?;
        if !reset.success {
            return Err(AppError::Other(format!(
                "git merge --abort: {}\ngit reset --hard: {}",
                out.text(),
                reset.text()
            )));
        }
    }
    let _ = fs::remove_file(list_file(&repo));
    Ok(changed)
}

/// Which side of a conflict to keep, for a file that cannot have marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// What is here.
    Ours,
    /// What came from GitHub.
    Theirs,
}

/// Keeps one side of a binary or deleted-and-changed file in conflict.
pub fn choose(root: &Path, path: &str, side: Side) -> AppResult<()> {
    let target = crate::util::paths::resolve_new(root, path)?;
    let root = dunce::canonicalize(root).map_err(|e| AppError::io(root, e))?;
    let rel = crate::util::paths::to_relative(&root, &target)?;
    let repo = super::require(&root)?;
    let kind = remembered(&repo)
        .into_iter()
        .find(|(p, _)| *p == rel)
        .map(|(_, kind)| kind)
        .ok_or_else(|| AppError::Forbidden(format!("{rel} no está en conflicto")))?;
    if kind == ConflictKind::Text {
        return Err(AppError::Forbidden(format!(
            "{rel} se resuelve en el editor"
        )));
    }
    let index = repo.index().map_err(git_error)?;
    let stage = match side {
        Side::Ours => 2,
        Side::Theirs => 3,
    };
    let kept = index.get_path(Path::new(&rel), stage).is_some();
    let flag = match side {
        Side::Ours => "--ours",
        Side::Theirs => "--theirs",
    };
    // `:(literal)`: git reads the name as it is, never as a pattern (`fig*.png` would also
    // take `fig1.png`).
    let spec = format!(":(literal){rel}");
    let steps: Vec<Vec<&str>> = if kept {
        vec![
            vec!["checkout", flag, "--", &spec],
            vec!["add", "--", &spec],
        ]
    } else {
        vec![vec!["rm", "-q", "--", &spec]]
    };
    for args in steps {
        let out = super::remote::run_git(&root, &args)?;
        if !out.success {
            return Err(AppError::Other(format!("git {}: {}", args[0], out.text())));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::backup::tests::{git_in, other_clone, push_from_other, read, with_origin};
    use crate::backup::{backup, list, open, pending, remote, restore_project};
    use crate::error::AppError;
    use crate::projects::trash::fake::FakeTrash;
    use tempfile::TempDir;

    /// A project and its `origin`, with the same line changed on both sides and the merge
    /// left in progress.
    fn in_conflict() -> (TempDir, TempDir) {
        let (dir, origin) = with_origin();
        let root = dir.path();
        remote::push(root, |_| {}, |_| {}).unwrap();
        push_from_other(&origin, "main.tex", "remoto\n");
        fs::write(root.join("main.tex"), "local\n").unwrap();
        backup(root, "local", None).unwrap();
        remote::fetch(root, |_| {}, |_| {}).unwrap();
        let pulled = remote::bring_in(root, "Fusión con GitHub", None).unwrap();
        assert_eq!(pulled.conflicts.len(), 1);
        (dir, origin)
    }

    #[test]
    fn el_estado_de_fusion_sobrevive_a_reabrir() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, _origin) = in_conflict();
        let root = dir.path();
        // A `.tex` with marks that Git did not list is not a conflict.
        fs::write(
            root.join("ejemplo.tex"),
            "<<<<<<< a\nx\n=======\ny\n>>>>>>> b\n",
        )
        .unwrap();
        let s = state(root).unwrap().unwrap();
        assert_eq!(
            s.conflicts,
            [Conflict {
                path: "main.tex".into(),
                kind: ConflictKind::Text,
                remaining: 1
            }]
        );
        fs::write(root.join("main.tex"), "los dos\n").unwrap();
        let s = state(root).unwrap().unwrap();
        assert_eq!(s.conflicts[0].remaining, 0);
    }

    #[test]
    fn terminar_exige_sin_marcas_y_hace_el_commit() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, _origin) = in_conflict();
        let root = dir.path();
        assert!(matches!(
            finish(root, "Fusión con GitHub", None),
            Err(AppError::Forbidden(_))
        ));
        fs::write(root.join("main.tex"), "los dos\n").unwrap();
        finish(root, "Fusión con GitHub", None).unwrap();
        assert_eq!(state(root).unwrap(), None);
        assert!(!root.join(".git/JUTEX_MERGE").exists());
        assert_eq!(list(root).unwrap()[0].message, "Fusión con GitHub");
        let repo = open(root).unwrap().unwrap();
        assert_eq!(
            repo.head()
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .parent_count(),
            2
        );
        assert!(pending(root).unwrap().changes.is_empty());
        assert_eq!(read(root, "main.tex"), "los dos\n");
    }

    #[test]
    fn cancelar_vuelve_a_como_estaba() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, _origin) = in_conflict();
        let root = dir.path();
        fs::write(root.join("main.tex"), "editado a mano\n").unwrap();
        let changed = abort(root).unwrap();
        assert!(changed.contains(&"main.tex".to_string()), "{changed:?}");
        assert_eq!(read(root, "main.tex"), "local\n");
        assert_eq!(state(root).unwrap(), None);
        assert!(!root.join(".git/JUTEX_MERGE").exists());
    }

    #[test]
    fn cancelar_aunque_se_haya_editado_un_archivo_unido_solo() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, origin) = with_origin();
        let root = dir.path();
        fs::write(root.join("cap.tex"), "cap\n").unwrap();
        backup(root, "cap", None).unwrap();
        remote::push(root, |_| {}, |_| {}).unwrap();
        // GitHub changes both files; here only main.tex changes: cap.tex merges by itself.
        let (_other, clone) = other_clone(&origin);
        fs::write(clone.join("main.tex"), "remoto\n").unwrap();
        fs::write(clone.join("cap.tex"), "cap de GitHub\n").unwrap();
        git_in(&clone, &["commit", "-qam", "remoto"]);
        git_in(&clone, &["push", "-q", "origin", "main"]);
        fs::write(root.join("main.tex"), "local\n").unwrap();
        backup(root, "local", None).unwrap();
        remote::fetch(root, |_| {}, |_| {}).unwrap();
        remote::bring_in(root, "Fusión con GitHub", None).unwrap();
        // The user edits the file that merged by itself (autosave writes it).
        fs::write(root.join("cap.tex"), "editado durante la fusión\n").unwrap();
        abort(root).unwrap();
        assert_eq!(state(root).unwrap(), None);
        assert_eq!(read(root, "cap.tex"), "cap\n");
        assert_eq!(read(root, "main.tex"), "local\n");
    }

    #[test]
    fn una_fusion_empezada_fuera_de_jutex_no_se_termina_con_marcas() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, origin) = with_origin();
        let root = dir.path();
        remote::push(root, |_| {}, |_| {}).unwrap();
        push_from_other(&origin, "main.tex", "remoto\n");
        fs::write(root.join("main.tex"), "local\n").unwrap();
        backup(root, "local", None).unwrap();
        // `git pull` in a terminal: no JUTEX_MERGE.
        git_in(root, &["fetch", "-q", "origin"]);
        let out = crate::platform::process::command(remote::tool("git").unwrap())
            .args(["-c", "user.name=Ana", "-c", "user.email=ana@x.cl"])
            .args(["merge", "--no-edit", "origin/main"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(!out.status.success());
        let s = state(root).unwrap().unwrap();
        assert_eq!(
            s.conflicts,
            [Conflict {
                path: "main.tex".into(),
                kind: ConflictKind::Text,
                remaining: 1
            }]
        );
        assert!(matches!(
            finish(root, "Fusión con GitHub", None),
            Err(AppError::Forbidden(_))
        ));
    }

    #[test]
    fn elegir_un_lado_de_un_binario() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, origin) = with_origin();
        let root = dir.path();
        let name = "-figura 1.png";
        fs::write(root.join(name), [0u8, 1, 2, 0]).unwrap();
        backup(root, "figura", None).unwrap();
        remote::push(root, |_| {}, |_| {}).unwrap();
        let (_other, clone) = other_clone(&origin);
        fs::write(clone.join(name), [0u8, 9, 9, 0]).unwrap();
        git_in(&clone, &["commit", "-qam", "remoto"]);
        git_in(&clone, &["push", "-q", "origin", "main"]);
        fs::write(root.join(name), [0u8, 5, 5, 0]).unwrap();
        backup(root, "local", None).unwrap();
        remote::fetch(root, |_| {}, |_| {}).unwrap();
        let pulled = remote::bring_in(root, "Fusión con GitHub", None).unwrap();
        assert_eq!(
            pulled.conflicts,
            [Conflict {
                path: name.into(),
                kind: ConflictKind::Binary,
                remaining: 1
            }]
        );
        choose(root, name, Side::Theirs).unwrap();
        assert_eq!(fs::read(root.join(name)).unwrap(), [0u8, 9, 9, 0]);
        assert_eq!(state(root).unwrap().unwrap().conflicts[0].remaining, 0);
        finish(root, "Fusión con GitHub", None).unwrap();
    }

    #[test]
    fn el_nombre_se_lee_literal_y_no_como_patron() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, origin) = with_origin();
        let root = dir.path();
        // `fig*.png` in conflict, and `fig1.png`, which a pattern would also take.
        fs::write(root.join("fig*.png"), [0u8, 1, 0]).unwrap();
        fs::write(root.join("fig1.png"), [0u8, 7, 0]).unwrap();
        backup(root, "figuras", None).unwrap();
        remote::push(root, |_| {}, |_| {}).unwrap();
        let (_other, clone) = other_clone(&origin);
        git_in(&clone, &["rm", "-q", "--", ":(literal)fig*.png"]);
        git_in(&clone, &["commit", "-qm", "borrada"]);
        git_in(&clone, &["push", "-q", "origin", "main"]);
        fs::write(root.join("fig*.png"), [0u8, 5, 0]).unwrap();
        backup(root, "local", None).unwrap();
        remote::fetch(root, |_| {}, |_| {}).unwrap();
        let pulled = remote::bring_in(root, "Fusión con GitHub", None).unwrap();
        assert_eq!(pulled.conflicts[0].kind, ConflictKind::Deleted);
        // GitHub's side: deleted. Only that file goes, not what the name would match.
        choose(root, "fig*.png", Side::Theirs).unwrap();
        assert!(!root.join("fig*.png").exists());
        assert_eq!(fs::read(root.join("fig1.png")).unwrap(), [0u8, 7, 0]);
    }

    #[test]
    fn borrado_contra_cambio() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, origin) = with_origin();
        let root = dir.path();
        fs::write(root.join("cap.tex"), "uno\n").unwrap();
        backup(root, "cap", None).unwrap();
        remote::push(root, |_| {}, |_| {}).unwrap();
        let (_other, clone) = other_clone(&origin);
        git_in(&clone, &["rm", "-q", "cap.tex"]);
        git_in(&clone, &["commit", "-qm", "borrado"]);
        git_in(&clone, &["push", "-q", "origin", "main"]);
        fs::write(root.join("cap.tex"), "dos\n").unwrap();
        backup(root, "local", None).unwrap();
        remote::fetch(root, |_| {}, |_| {}).unwrap();
        let pulled = remote::bring_in(root, "Fusión con GitHub", None).unwrap();
        assert_eq!(pulled.conflicts[0].kind, ConflictKind::Deleted);
        choose(root, "cap.tex", Side::Theirs).unwrap();
        assert!(!root.join("cap.tex").exists());
        assert_eq!(state(root).unwrap().unwrap().conflicts[0].remaining, 0);
        finish(root, "Fusión con GitHub", None).unwrap();
        assert!(!root.join("cap.tex").exists());
    }

    #[test]
    fn nada_se_respalda_ni_sube_durante_la_fusion() {
        if remote::tool("git").is_none() {
            return;
        }
        let (dir, _origin) = in_conflict();
        let root = dir.path();
        let first = list(root).unwrap().last().unwrap().id.clone();
        assert!(matches!(
            backup(root, "x", None),
            Err(AppError::MergeInProgress(_))
        ));
        assert!(matches!(
            remote::push(root, |_| {}, |_| {}),
            Err(AppError::MergeInProgress(_))
        ));
        assert!(matches!(
            restore_project(
                root,
                &first,
                "antes",
                None,
                &FakeTrash::new(TempDir::new().unwrap().path().join("papelera"))
            ),
            Err(AppError::MergeInProgress(_))
        ));
        assert!(choose(root, "../fuera.tex", Side::Ours).is_err());
    }

    #[test]
    fn cuenta_bloques() {
        let one = "a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> origin/main\nb\n";
        assert_eq!(count_blocks(one), 1);
        assert_eq!(count_blocks(&format!("{one}{one}")), 2);
        assert_eq!(count_blocks("a\n<<<<<<< HEAD\nx\n=======\ny\nb\n"), 0);
        assert_eq!(
            count_blocks("<<<<<<< HEAD\nx\n======= x\ny\n>>>>>>> origin/main\n"),
            0
        );
        assert_eq!(count_blocks(&one.replace('\n', "\r\n")), 1);
        assert_eq!(count_blocks(""), 0);
    }
}
