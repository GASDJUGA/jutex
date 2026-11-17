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

    #[allow(dead_code)] // read back by `state` (merge in progress)
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

#[cfg(test)]
mod tests {
    use super::*;

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
