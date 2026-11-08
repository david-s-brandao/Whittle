//! Read-only git access through libgit2.
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RepoSummary {
    /// Current branch name, or short hash when HEAD is detached; `None` for an unborn branch.
    pub branch: Option<String>,
    pub dirty: bool,
    /// Commits ahead of / behind the upstream branch (0 when there is no upstream).
    pub ahead: usize,
    pub behind: usize,
    pub has_upstream: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    /// Upstream shorthand (local branches only), e.g. `origin/main`.
    pub upstream: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
    /// The stash is a commit; with this id its contents can be shown like any commit.
    pub id: String,
    /// When it was created, unix seconds.
    pub time: i64,
}

/// A branch/tag name attached to a commit, for drawing next to it in the history.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct RefLabel {
    /// The commit it points at (tags are resolved through to the commit).
    pub id: String,
    pub name: String,
    /// "local", "remote", "tag" or "head" (only for a detached HEAD).
    pub kind: &'static str,
    /// This is the branch that is checked out (or the detached HEAD itself).
    pub current: bool,
    /// This is the remote branch the checked-out branch tracks (e.g. `origin/main`).
    pub tracked_by_current: bool,
}

#[derive(Debug, Serialize, Default, PartialEq, Eq)]
pub struct RepoRefs {
    pub local: Vec<BranchInfo>,
    pub remote: Vec<BranchInfo>,
    pub tags: Vec<String>,
    pub stashes: Vec<StashInfo>,
    /// Commit HEAD points at, if any.
    pub head_id: Option<String>,
    /// Every branch / tag with the commit it points at.
    pub labels: Vec<RefLabel>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub summary: String,
    pub author: String,
    pub email: String,
    /// Author time, unix seconds.
    pub time: i64,
    pub parents: Vec<String>,
    /// The commit carries a GPG or SSH signature (not verified here; see `signature_info`).
    pub signed: bool,
}

/// Builds the list entry for a commit.
pub fn commit_info(repo: &git2::Repository, c: &git2::Commit) -> CommitInfo {
    let a = c.author();
    let id = c.id().to_string();
    CommitInfo {
        short_id: id[..7].to_owned(),
        summary: c.summary().unwrap_or("").to_owned(),
        author: a.name().unwrap_or("").to_owned(),
        email: a.email().unwrap_or("").to_owned(),
        time: a.when().seconds(),
        parents: c.parent_ids().map(|p| p.to_string()).collect(),
        signed: repo.extract_signature(&c.id(), None).is_ok(),
        id,
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub old_path: Option<String>,
    /// One of: added, modified, deleted, renamed, copied, typechange.
    pub status: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DiffLine {
    /// '+', '-', ' ', or '\\' for the "No newline at end of file" marker.
    pub kind: char,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Hunk {
    /// Fingerprint of the hunk's content. Partial-staging requests echo it back so a stale
    /// view (file edited since it was shown) is detected instead of staging the wrong lines.
    pub id: String,
    pub header: String,
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize, Default, PartialEq, Eq)]
pub struct FileDiff {
    pub binary: bool,
    pub hunks: Vec<Hunk>,
}

/// Returns true if `path` is the root (or inside) a git repository.
pub fn is_repo(path: &Path) -> bool {
    git2::Repository::discover(path).is_ok()
}

/// Name of the remote to publish to: `origin` if present, else the first one.
pub fn default_remote(path: &Path) -> Result<Option<String>, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let remotes = repo.remotes()?;
    let names: Vec<&str> = remotes.iter().flatten().collect();
    Ok(names
        .iter()
        .find(|n| **n == "origin")
        .or(names.first())
        .map(|n| n.to_string()))
}

/// Resolves the working-tree root of the repo containing `path`.
pub fn repo_root(path: &Path) -> Result<std::path::PathBuf, git2::Error> {
    let repo = git2::Repository::discover(path)?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| git2::Error::from_str("bare repositories are not supported"))
}

pub fn summary(path: &Path) -> Result<RepoSummary, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let branch = match repo.head() {
        Ok(h) if h.is_branch() => h.shorthand().map(str::to_owned),
        Ok(h) => h.target().map(|o| o.to_string()[..7].to_owned()),
        Err(_) => None,
    };
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).include_ignored(false);
    let dirty = !repo.statuses(Some(&mut opts))?.is_empty();
    let (mut ahead, mut behind, mut has_upstream) = (0, 0, false);
    if let Ok(head) = repo.head() {
        if head.is_branch() {
            let local = git2::Branch::wrap(head);
            if let Ok(up) = local.upstream() {
                has_upstream = true;
                if let (Some(l), Some(u)) = (local.get().target(), up.get().target()) {
                    (ahead, behind) = repo.graph_ahead_behind(l, u)?;
                }
            }
        }
    }
    Ok(RepoSummary {
        branch,
        dirty,
        ahead,
        behind,
        has_upstream,
    })
}

pub fn refs(path: &Path) -> Result<RepoRefs, git2::Error> {
    let mut repo = git2::Repository::open(path)?;
    let mut out = RepoRefs::default();

    for b in repo.branches(None)? {
        let (b, kind) = b?;
        let Some(name) = b.name()?.map(str::to_owned) else {
            continue;
        };
        match kind {
            git2::BranchType::Local => {
                let upstream = b
                    .upstream()
                    .ok()
                    .and_then(|u| u.name().ok().flatten().map(str::to_owned));
                out.local.push(BranchInfo {
                    name,
                    is_head: b.is_head(),
                    upstream,
                });
            }
            git2::BranchType::Remote => {
                if name.ends_with("/HEAD") {
                    continue;
                }
                out.remote.push(BranchInfo {
                    name,
                    is_head: false,
                    upstream: None,
                });
            }
        }
    }
    out.local.sort_by(|a, b| a.name.cmp(&b.name));
    out.remote.sort_by(|a, b| a.name.cmp(&b.name));

    out.tags = repo
        .tag_names(None)?
        .iter()
        .flatten()
        .map(str::to_owned)
        .collect();
    out.tags.sort();

    // labels: where each ref points, and which of them relate to the checked-out branch
    out.head_id = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| c.id().to_string());
    let current_upstream = out
        .local
        .iter()
        .find(|b| b.is_head)
        .and_then(|b| b.upstream.clone());
    let tip = |full: &str| {
        repo.find_reference(full)
            .ok()
            .and_then(|r| r.peel_to_commit().ok())
            .map(|c| c.id().to_string())
    };
    for b in &out.local {
        if let Some(id) = tip(&format!("refs/heads/{}", b.name)) {
            out.labels.push(RefLabel {
                id,
                name: b.name.clone(),
                kind: "local",
                current: b.is_head,
                tracked_by_current: false,
            });
        }
    }
    for b in &out.remote {
        if let Some(id) = tip(&format!("refs/remotes/{}", b.name)) {
            out.labels.push(RefLabel {
                id,
                name: b.name.clone(),
                kind: "remote",
                current: false,
                tracked_by_current: current_upstream.as_deref() == Some(b.name.as_str()),
            });
        }
    }
    for t in &out.tags {
        if let Some(id) = tip(&format!("refs/tags/{t}")) {
            out.labels.push(RefLabel {
                id,
                name: t.clone(),
                kind: "tag",
                current: false,
                tracked_by_current: false,
            });
        }
    }
    if repo.head_detached().unwrap_or(false) {
        if let Some(id) = &out.head_id {
            out.labels.push(RefLabel {
                id: id.clone(),
                name: "HEAD".into(),
                kind: "head",
                current: true,
                tracked_by_current: false,
            });
        }
    }

    let mut raw: Vec<(usize, String, git2::Oid)> = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        raw.push((index, message.to_owned(), *oid));
        true
    })?;
    for (index, message, oid) in raw {
        let time = repo
            .find_commit(oid)
            .map(|c| c.time().seconds())
            .unwrap_or(0);
        out.stashes.push(StashInfo {
            index,
            message,
            id: oid.to_string(),
            time,
        });
    }
    Ok(out)
}

/// All commit ids reachable from branches, tags and HEAD, newest first
/// (children always before parents). This is the expensive full sort; callers
/// cache the result and page through it with [`log_page`].
pub fn log_ids(path: &Path) -> Result<Vec<git2::Oid>, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    walk.push_glob("refs/heads/*")?;
    walk.push_glob("refs/remotes/*")?;
    walk.push_glob("refs/tags/*")?;
    let _ = walk.push_head(); // unborn/detached HEAD is fine
    walk.collect()
}

/// Commits whose message, author name/email or hash contain every whitespace-separated term of
/// `query` (case-insensitive), newest first, at most `limit`. Scans `ids` in order, so
/// results keep the history order.
pub fn search(
    path: &Path,
    ids: &[git2::Oid],
    query: &str,
    limit: usize,
) -> Result<Vec<CommitInfo>, git2::Error> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let repo = git2::Repository::open(path)?;
    let mut out = Vec::new();
    for oid in ids {
        let c = repo.find_commit(*oid)?;
        let hash = oid.to_string();
        let a = c.author();
        let haystack = format!(
            "{}\n{}\n{}\n{}",
            c.message().unwrap_or(""),
            a.name().unwrap_or(""),
            a.email().unwrap_or(""),
            hash
        )
        .to_lowercase();
        if terms.iter().all(|t| haystack.contains(t.as_str())) {
            out.push(commit_info(&repo, &c));
            if out.len() >= limit {
                break;
            }
        }
    }
    Ok(out)
}

/// Loads commit details for `ids[skip..skip + limit]`.
pub fn log_page(
    path: &Path,
    ids: &[git2::Oid],
    skip: usize,
    limit: usize,
) -> Result<Vec<CommitInfo>, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let end = skip.saturating_add(limit).min(ids.len());
    let mut out = Vec::with_capacity(end.saturating_sub(skip));
    for oid in ids.iter().take(end).skip(skip) {
        let c = repo.find_commit(*oid)?;
        out.push(commit_info(&repo, &c));
    }
    Ok(out)
}

fn commit_diff_for<'r>(
    repo: &'r git2::Repository,
    id: &str,
    opts: Option<&mut git2::DiffOptions>,
) -> Result<git2::Diff<'r>, git2::Error> {
    let commit = repo.find_commit(git2::Oid::from_str(id)?)?;
    let new_tree = commit.tree()?;
    // Diff against the first parent (for merges this shows what the merge changed vs. mainline).
    let old_tree = commit.parent(0).ok().map(|p| p.tree()).transpose()?;
    let mut diff = repo.diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), opts)?;
    diff.find_similar(None)?;
    Ok(diff)
}

pub fn commit_files(path: &Path, id: &str) -> Result<Vec<FileChange>, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let diff = commit_diff_for(&repo, id, None)?;
    let mut out = Vec::new();
    for d in diff.deltas() {
        let status = match d.status() {
            git2::Delta::Added => "added",
            git2::Delta::Deleted => "deleted",
            git2::Delta::Renamed => "renamed",
            git2::Delta::Copied => "copied",
            git2::Delta::Typechange => "typechange",
            _ => "modified",
        };
        let new = d
            .new_file()
            .path()
            .map(|p| p.to_string_lossy().into_owned());
        let old = d
            .old_file()
            .path()
            .map(|p| p.to_string_lossy().into_owned());
        let path = new.clone().or_else(|| old.clone()).unwrap_or_default();
        let old_path = (status == "renamed" || status == "copied")
            .then_some(old)
            .flatten();
        out.push(FileChange {
            path,
            old_path,
            status,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn collect_diff(diff: &git2::Diff) -> Result<FileDiff, git2::Error> {
    let out = std::cell::RefCell::new(FileDiff::default());
    diff.foreach(
        &mut |d, _| {
            out.borrow_mut().binary |= d.flags().is_binary();
            true
        },
        None,
        Some(&mut |_, h| {
            out.borrow_mut().hunks.push(Hunk {
                id: String::new(),
                header: String::from_utf8_lossy(h.header()).trim_end().to_owned(),
                old_start: h.old_start(),
                new_start: h.new_start(),
                lines: Vec::new(),
            });
            true
        }),
        Some(&mut |_, _, l| {
            let (kind, text) = match l.origin() {
                '+' | '-' | ' ' => (
                    l.origin(),
                    String::from_utf8_lossy(l.content())
                        .trim_end_matches(['\n', '\r'])
                        .to_owned(),
                ),
                // libgit2 reports "\ No newline at end of file" as '=', '>' or '<'
                '=' | '>' | '<' => ('\\', "No newline at end of file".to_owned()),
                _ => return true, // file / hunk headers
            };
            if let Some(h) = out.borrow_mut().hunks.last_mut() {
                h.lines.push(DiffLine {
                    kind,
                    old_no: l.old_lineno(),
                    new_no: l.new_lineno(),
                    text,
                });
            }
            true
        }),
    )?;
    let mut diff = out.into_inner();
    for h in &mut diff.hunks {
        h.id = hunk_fingerprint(h);
    }
    Ok(diff)
}

fn hunk_fingerprint(h: &Hunk) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    h.header.hash(&mut hasher);
    for l in &h.lines {
        (l.kind, &l.text).hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Diff of a single file in a commit, structured into hunks and lines.
pub fn commit_file_diff(path: &Path, id: &str, file: &str) -> Result<FileDiff, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file).context_lines(3);
    let diff = commit_diff_for(&repo, id, Some(&mut opts))?;
    collect_diff(&diff)
}

/// A request to stage/unstage part of a file's diff.
#[derive(Debug, serde::Deserialize, Clone)]
pub struct Selection {
    /// `Hunk::id` from the diff the user was looking at.
    pub hunk_id: String,
    /// Indices into `Hunk::lines` of the +/- lines to include; `None` = the whole hunk.
    pub lines: Option<Vec<usize>>,
}

/// Builds a unified-diff patch containing only the selected parts of `diff`.
///
/// Forward patches (staging: index -> workdir changes applied to the index): a `-` line that is
/// not selected stays in the file, so it becomes context; an unselected `+` line is dropped.
/// Reverse patches (unstaging: applied with `git apply --reverse` to the index): an unselected
/// `+` line is still in the index, so it becomes context; an unselected `-` line is dropped.
pub fn build_patch(
    file: &str,
    diff: &FileDiff,
    selections: &[Selection],
    reverse: bool,
) -> Result<String, String> {
    if diff.binary {
        return Err("Binary files can only be staged as a whole.".into());
    }
    let mut out = format!("diff --git a/{file} b/{file}\n--- a/{file}\n+++ b/{file}\n");
    let mut delta: i64 = 0;
    let mut any = false;
    for sel in selections {
        if !diff.hunks.iter().any(|h| h.id == sel.hunk_id) {
            return Err(
                "This file changed since it was shown. Reloaded the diff, please select again."
                    .into(),
            );
        }
    }
    for hunk in &diff.hunks {
        let Some(sel) = selections.iter().find(|s| s.hunk_id == hunk.id) else {
            continue;
        };
        let has_marker = hunk.lines.iter().any(|l| l.kind == '\\');
        let partial = sel.lines.is_some();
        if partial && has_marker {
            return Err("This hunk touches a line without a trailing newline. Stage the whole hunk instead.".into());
        }
        let (mut body, mut old, mut new, mut changed) = (String::new(), 0i64, 0i64, false);
        let mut last_emitted = false;
        for (i, l) in hunk.lines.iter().enumerate() {
            match l.kind {
                '+' | '-' => {
                    let chosen = sel.lines.as_ref().is_none_or(|v| v.contains(&i));
                    if chosen {
                        body.push_str(&format!("{}{}\n", l.kind, l.text));
                        if l.kind == '+' {
                            new += 1
                        } else {
                            old += 1
                        }
                        changed = true;
                        last_emitted = true;
                    } else if (l.kind == '-') != reverse {
                        // stays as-is on both sides
                        body.push_str(&format!(" {}\n", l.text));
                        old += 1;
                        new += 1;
                        last_emitted = true;
                    } else {
                        last_emitted = false;
                    }
                }
                '\\' => {
                    if last_emitted {
                        body.push_str("\\ No newline at end of file\n");
                    }
                }
                _ => {
                    body.push_str(&format!(" {}\n", l.text));
                    old += 1;
                    new += 1;
                    last_emitted = true;
                }
            }
        }
        if !changed {
            continue;
        }
        let (old_start, new_start) = if reverse {
            (hunk.old_start as i64, hunk.new_start as i64)
        } else {
            (hunk.old_start as i64, hunk.old_start as i64 + delta)
        };
        out.push_str(&format!(
            "@@ -{old_start},{old} +{new_start},{new} @@\n{body}"
        ));
        delta += new - old;
        any = true;
    }
    if any {
        Ok(out)
    } else {
        Err("Nothing selected.".into())
    }
}

#[derive(Debug, Serialize, Default, PartialEq, Eq)]
pub struct WorkStatus {
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub conflicted: Vec<String>,
    /// A multi-step operation that is paused and waiting for the user: "merge", "rebase",
    /// "cherry-pick" or "revert".
    pub operation: Option<&'static str>,
}

/// Working-tree status split into staged (HEAD -> index) and unstaged (index -> workdir).
/// Untracked files appear in `unstaged` with status "untracked".
pub fn work_status(path: &Path) -> Result<WorkStatus, git2::Error> {
    use git2::Status as S;
    let repo = git2::Repository::open(path)?;
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut out = WorkStatus::default();
    for e in statuses.iter() {
        let st = e.status();
        let Some(p) = e.path() else { continue };
        if st.is_conflicted() {
            out.conflicted.push(p.to_owned());
            continue;
        }
        let change = |status: &'static str, old: Option<String>| FileChange {
            path: p.to_owned(),
            old_path: old,
            status,
        };
        if st.intersects(
            S::INDEX_NEW
                | S::INDEX_MODIFIED
                | S::INDEX_DELETED
                | S::INDEX_RENAMED
                | S::INDEX_TYPECHANGE,
        ) {
            let (status, old) = if st.contains(S::INDEX_NEW) {
                ("added", None)
            } else if st.contains(S::INDEX_DELETED) {
                ("deleted", None)
            } else if st.contains(S::INDEX_RENAMED) {
                let old = e.head_to_index().and_then(|d| {
                    d.old_file()
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                });
                ("renamed", old)
            } else if st.contains(S::INDEX_TYPECHANGE) {
                ("typechange", None)
            } else {
                ("modified", None)
            };
            out.staged.push(change(status, old));
        }
        if st.intersects(
            S::WT_NEW | S::WT_MODIFIED | S::WT_DELETED | S::WT_TYPECHANGE | S::WT_RENAMED,
        ) {
            let status = if st.contains(S::WT_NEW) {
                "untracked"
            } else if st.contains(S::WT_DELETED) {
                "deleted"
            } else if st.contains(S::WT_TYPECHANGE) {
                "typechange"
            } else {
                "modified"
            };
            out.unstaged.push(change(status, None));
        }
    }
    out.staged.sort_by(|a, b| a.path.cmp(&b.path));
    out.unstaged.sort_by(|a, b| a.path.cmp(&b.path));
    out.conflicted.sort();
    out.operation = match repo.state() {
        git2::RepositoryState::Merge => Some("merge"),
        git2::RepositoryState::Rebase
        | git2::RepositoryState::RebaseInteractive
        | git2::RepositoryState::RebaseMerge => Some("rebase"),
        git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
            Some("cherry-pick")
        }
        git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => Some("revert"),
        _ => None,
    };
    Ok(out)
}

/// Diff of one file: HEAD -> index when `staged`, otherwise index -> workdir
/// (untracked files are shown as entirely added).
pub fn work_file_diff(path: &Path, file: &str, staged: bool) -> Result<FileDiff, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file).context_lines(3);
    let diff = if staged {
        let head = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(head.as_ref(), None, Some(&mut opts))?
    } else {
        opts.include_untracked(true)
            .show_untracked_content(true)
            .recurse_untracked_dirs(true);
        repo.diff_index_to_workdir(None, Some(&mut opts))?
    };
    collect_diff(&diff)
}

/// Commits an interactive rebase starting at `from` could rewrite: `from` itself up to HEAD,
/// oldest first. Fails when `from` is not on the current branch or the range has merge commits
/// (those can't be reordered safely).
pub fn rebase_candidates(path: &Path, from: &str) -> Result<Vec<CommitInfo>, git2::Error> {
    if from.is_empty() || from.starts_with('-') {
        return Err(git2::Error::from_str("That is not a valid commit."));
    }
    let repo = git2::Repository::open(path)?;
    let start = repo.revparse_single(from)?.peel_to_commit()?;
    let head = repo.head()?.peel_to_commit()?;
    if start.id() != head.id() && !repo.graph_descendant_of(head.id(), start.id())? {
        return Err(git2::Error::from_str(
            "That commit is not part of the current branch's history.",
        ));
    }
    let mut walk = repo.revwalk()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?;
    walk.push(head.id())?;
    for p in start.parent_ids() {
        walk.hide(p)?;
    }
    let mut out = Vec::new();
    for oid in walk {
        let c = repo.find_commit(oid?)?;
        if c.parent_count() > 1 {
            return Err(git2::Error::from_str(
                "This range contains a merge commit, which interactive rebase can't reorder. Pick a more recent commit.",
            ));
        }
        out.push(commit_info(&repo, &c));
        if out.len() > 300 {
            return Err(git2::Error::from_str(
                "That is too many commits to rebase interactively.",
            ));
        }
    }
    Ok(out)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SubmoduleInfo {
    pub name: String,
    pub path: String,
    pub url: Option<String>,
    /// "uninitialized" (not cloned yet), "clean", "new_commits" (checked out at a different
    /// commit than the parent records) or "dirty" (uncommitted changes inside).
    pub status: &'static str,
    /// Commit the parent repository expects.
    pub expected_id: Option<String>,
    /// Commit actually checked out inside the submodule.
    pub current_id: Option<String>,
}

pub fn submodules(path: &Path) -> Result<Vec<SubmoduleInfo>, git2::Error> {
    use git2::SubmoduleStatus as S;
    let repo = git2::Repository::open(path)?;
    let mut out = Vec::new();
    for sm in repo.submodules()? {
        let name = sm.name().unwrap_or("").to_owned();
        let st = repo
            .submodule_status(&name, git2::SubmoduleIgnore::None)
            .unwrap_or(S::empty());
        let status = if st.contains(S::WD_UNINITIALIZED) || !st.contains(S::IN_WD) {
            "uninitialized"
        } else if st.intersects(S::WD_WD_MODIFIED | S::WD_UNTRACKED | S::WD_INDEX_MODIFIED) {
            "dirty"
        } else if st.contains(S::WD_MODIFIED) {
            "new_commits"
        } else {
            "clean"
        };
        out.push(SubmoduleInfo {
            name,
            path: sm.path().to_string_lossy().into_owned(),
            url: sm.url().map(str::to_owned),
            status,
            expected_id: sm.index_id().or(sm.head_id()).map(|o| o.to_string()),
            current_id: sm.workdir_id().map(|o| o.to_string()),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// One run of consecutive lines last changed by the same commit.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct BlameHunk {
    /// First line (1-based) and number of lines.
    pub start: u32,
    pub count: u32,
    pub id: String,
    pub short_id: String,
    pub author: String,
    /// Author time, unix seconds.
    pub time: i64,
    pub summary: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Blame {
    pub hunks: Vec<BlameHunk>,
    pub lines: Vec<String>,
}

/// Who last changed each line of `file` as of `rev` (default HEAD).
pub fn blame(path: &Path, file: &str, rev: Option<&str>) -> Result<Blame, git2::Error> {
    if file.is_empty() || file.starts_with('-') {
        return Err(git2::Error::from_str("That is not a valid file path."));
    }
    let repo = git2::Repository::open(path)?;
    let commit = match rev {
        Some(r) if !r.starts_with('-') => repo.revparse_single(r)?.peel_to_commit()?,
        Some(_) => return Err(git2::Error::from_str("That is not a valid revision.")),
        None => repo.head()?.peel_to_commit()?,
    };
    let entry = commit
        .tree()?
        .get_path(Path::new(file))
        .map_err(|_| git2::Error::from_str("That file does not exist at this commit."))?;
    let blob = repo.find_blob(entry.id())?;
    if blob.is_binary() {
        return Err(git2::Error::from_str("Binary file: no blame available."));
    }
    if blob.size() > 1024 * 1024 {
        return Err(git2::Error::from_str("This file is too large for blame."));
    }
    let lines: Vec<String> = String::from_utf8_lossy(blob.content())
        .lines()
        .map(str::to_owned)
        .collect();

    let mut opts = git2::BlameOptions::new();
    opts.newest_commit(commit.id());
    let blame = repo.blame_file(Path::new(file), Some(&mut opts))?;
    let mut hunks = Vec::new();
    for h in blame.iter() {
        let id = h.final_commit_id();
        let c = repo.find_commit(id)?;
        let sig = h.final_signature();
        let hex = id.to_string();
        hunks.push(BlameHunk {
            start: h.final_start_line() as u32,
            count: h.lines_in_hunk() as u32,
            short_id: hex[..7].to_owned(),
            id: hex,
            author: sig.name().unwrap_or("").to_owned(),
            time: sig.when().seconds(),
            summary: c.summary().unwrap_or("").to_owned(),
        });
    }
    Ok(Blame { hunks, lines })
}

/// Joins `rel` onto the repo root and makes sure the result stays inside it (no `..` tricks,
/// no symlinks pointing out). The file must exist.
pub fn safe_join(root: &Path, rel: &str) -> Result<std::path::PathBuf, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let full = root
        .join(rel)
        .canonicalize()
        .map_err(|_| format!("“{rel}” does not exist."))?;
    if full.starts_with(&root) {
        Ok(full)
    } else {
        Err("That path is outside the repository.".into())
    }
}

/// Text of a working-tree file (for previewing conflicted files). Capped at 2 MB.
pub fn read_text(root: &Path, rel: &str) -> Result<String, String> {
    let full = safe_join(root, rel)?;
    let meta = std::fs::metadata(&full).map_err(|e| e.to_string())?;
    if meta.len() > 2 * 1024 * 1024 {
        return Err("This file is too large to preview.".into());
    }
    let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
    if bytes.contains(&0) {
        return Err("Binary file: no preview.".into());
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Overwrites a working-tree file with `content` (the merged result from the conflict editor).
/// The file must already exist inside the repo.
pub fn write_text(root: &Path, rel: &str, content: &str) -> Result<(), String> {
    let full = safe_join(root, rel)?;
    std::fs::write(&full, content).map_err(|e| e.to_string())
}

/// What merging `source` into the current branch would do, computed without touching any file.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct MergePreview {
    pub source: String,
    /// Name of the checked-out branch (or "HEAD" when detached).
    pub target: String,
    /// Commits the source has that the current branch lacks (what would come in).
    pub incoming: usize,
    /// Commits only the current branch has (diverged when both are non-zero).
    pub only_here: usize,
    pub up_to_date: bool,
    pub can_fast_forward: bool,
    /// The incoming commits, newest first (at most 50).
    pub commits: Vec<CommitInfo>,
    /// Files the incoming commits change.
    pub files: usize,
    /// Files that would conflict, predicted by an in-memory merge. `None` if it couldn't be tried.
    pub conflicts: Option<Vec<String>>,
}

pub fn merge_preview(path: &Path, source: &str) -> Result<MergePreview, git2::Error> {
    if source.is_empty() || source.starts_with('-') {
        return Err(git2::Error::from_str("That is not a valid branch name."));
    }
    let repo = git2::Repository::open(path)?;
    let src = repo
        .revparse_single(source)
        .and_then(|o| o.peel_to_commit())
        .map_err(|_| git2::Error::from_str("That branch doesn't exist."))?;
    let head = repo.head()?.peel_to_commit()?;
    let target = repo
        .head()?
        .shorthand()
        .map(str::to_owned)
        .unwrap_or_else(|| "HEAD".into());

    let (incoming, only_here) = repo.graph_ahead_behind(src.id(), head.id())?;

    let mut commits = Vec::new();
    if incoming > 0 {
        let mut walk = repo.revwalk()?;
        walk.set_sorting(git2::Sort::TIME)?;
        walk.push(src.id())?;
        walk.hide(head.id())?;
        for oid in walk.take(50) {
            let c = repo.find_commit(oid?)?;
            commits.push(commit_info(&repo, &c));
        }
    }

    // files touched by the incoming side, measured from the common ancestor
    let base_tree = repo
        .merge_base(head.id(), src.id())
        .ok()
        .and_then(|b| repo.find_commit(b).ok())
        .and_then(|c| c.tree().ok());
    let files = repo
        .diff_tree_to_tree(base_tree.as_ref(), Some(&src.tree()?), None)
        .map(|d| d.deltas().len())
        .unwrap_or(0);

    let can_fast_forward = incoming > 0 && only_here == 0;
    let conflicts = if incoming == 0 || can_fast_forward {
        Some(Vec::new())
    } else {
        repo.merge_commits(&head, &src, None).ok().map(|idx| {
            let mut paths: Vec<String> = idx
                .conflicts()
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|c| {
                    c.our
                        .or(c.their)
                        .or(c.ancestor)
                        .map(|e| String::from_utf8_lossy(&e.path).into_owned())
                })
                .collect();
            paths.sort();
            paths.dedup();
            paths
        })
    };

    Ok(MergePreview {
        source: source.to_owned(),
        target,
        incoming,
        only_here,
        up_to_date: incoming == 0,
        can_fast_forward,
        commits,
        files,
        conflicts,
    })
}

/// Message of HEAD, used to pre-fill the box when amending.
pub fn head_message(path: &Path) -> Result<String, git2::Error> {
    let repo = git2::Repository::open(path)?;
    let c = repo.head()?.peel_to_commit()?;
    Ok(c.message().unwrap_or("").trim_end().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_file(repo: &git2::Repository, name: &str) -> git2::Oid {
        let dir = repo.workdir().unwrap();
        std::fs::write(dir.join(name), name).unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new(name)).unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .into_iter()
            .collect();
        let refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, name, &tree, &refs)
            .unwrap()
    }

    #[test]
    fn write_text_overwrites_inside_the_repo_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "old").unwrap();
        write_text(dir.path(), "a.txt", "new\n").unwrap();
        assert_eq!(read_text(dir.path(), "a.txt").unwrap(), "new\n");
        assert!(write_text(dir.path(), "../escape.txt", "x").is_err());
        assert!(write_text(dir.path(), "missing.txt", "x").is_err());
    }

    #[test]
    fn lists_branches_tags_stashes() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = git2::Repository::init(dir.path()).unwrap();
        let oid = commit_file(&repo, "a.txt");
        {
            let commit = repo.find_commit(oid).unwrap();
            repo.branch("feature", &commit, false).unwrap();
            repo.tag_lightweight("v1", commit.as_object(), false)
                .unwrap();
        }
        repo.reference("refs/remotes/origin/main", oid, false, "t")
            .unwrap();
        repo.reference("refs/remotes/origin/HEAD", oid, false, "t")
            .unwrap();

        std::fs::write(dir.path().join("a.txt"), "changed").unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        repo.stash_save(&sig, "wip", None).unwrap();

        let r = refs(dir.path()).unwrap();
        assert_eq!(r.local.len(), 2);
        assert_eq!(r.local.iter().filter(|b| b.is_head).count(), 1);
        assert!(r.local.iter().any(|b| b.name == "feature"));
        assert_eq!(r.remote.len(), 1, "origin/HEAD is hidden");
        assert_eq!(r.remote[0].name, "origin/main");
        assert_eq!(r.tags, vec!["v1"]);
        assert_eq!(r.stashes.len(), 1);
        assert!(r.stashes[0].message.contains("wip"));

        // labels: every ref with its commit, current branch flagged, HEAD id present
        assert_eq!(r.head_id.as_deref(), Some(oid.to_string().as_str()));
        let label = |name: &str| {
            r.labels
                .iter()
                .find(|l| l.name == name)
                .unwrap_or_else(|| panic!("no label {name}"))
        };
        assert!(label("feature").kind == "local" && !label("feature").current);
        assert_eq!(label("feature").id, oid.to_string());
        assert_eq!(label("v1").kind, "tag");
        assert_eq!(label("origin/main").kind, "remote");
        assert_eq!(r.labels.iter().filter(|l| l.current).count(), 1);
        assert!(
            r.labels.iter().all(|l| l.kind != "head"),
            "attached HEAD gets no separate label"
        );
    }

    #[test]
    fn labels_show_upstream_ahead_of_current_branch_and_detached_head() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let first = commit_file(&repo, "a.txt");
        let second = commit_file(&repo, "b.txt");
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();
        // pretend a fetch brought origin/<branch> one commit ahead of the local branch
        repo.remote("origin", "/nonexistent/remote.git").unwrap();
        repo.reference(&format!("refs/remotes/origin/{branch}"), second, false, "t")
            .unwrap();
        repo.find_branch(&branch, git2::BranchType::Local)
            .unwrap()
            .set_upstream(Some(&format!("origin/{branch}")))
            .unwrap();
        repo.reference(&format!("refs/heads/{branch}"), first, true, "rewind")
            .unwrap();

        let r = refs(dir.path()).unwrap();
        let cur = r.labels.iter().find(|l| l.current).unwrap();
        assert_eq!(
            (cur.name.as_str(), cur.id.as_str()),
            (branch.as_str(), first.to_string().as_str())
        );
        let up = r.labels.iter().find(|l| l.tracked_by_current).unwrap();
        assert_eq!(
            up.id,
            second.to_string(),
            "the upstream label sits on the newer commit"
        );
        assert_eq!(r.head_id.as_deref(), Some(first.to_string().as_str()));

        repo.set_head_detached(second).unwrap();
        let r = refs(dir.path()).unwrap();
        let h = r.labels.iter().find(|l| l.kind == "head").unwrap();
        assert_eq!(h.id, second.to_string());
        assert!(r
            .labels
            .iter()
            .filter(|l| l.kind == "local")
            .all(|l| !l.current));
    }

    #[test]
    fn detects_repo() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_repo(dir.path()));
        git2::Repository::init(dir.path()).unwrap();
        assert!(is_repo(dir.path()));
    }

    #[test]
    fn root_from_subdir() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let sub = dir.path().join("a/b");
        std::fs::create_dir_all(&sub).unwrap();
        assert_eq!(
            repo_root(&sub).unwrap().canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn summary_reports_branch_and_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let s = summary(dir.path()).unwrap();
        assert!(s.branch.is_none() && !s.dirty);

        std::fs::write(dir.path().join("f.txt"), "hi").unwrap();
        assert!(summary(dir.path()).unwrap().dirty);

        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("f.txt")).unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
        let s = summary(dir.path()).unwrap();
        assert!(s.branch.is_some() && !s.dirty);
    }

    fn page(path: &Path, skip: usize, limit: usize) -> Vec<CommitInfo> {
        let ids = log_ids(path).unwrap();
        log_page(path, &ids, skip, limit).unwrap()
    }

    #[test]
    fn log_pages_across_branches() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let first = commit_file(&repo, "a");
        commit_file(&repo, "b");
        commit_file(&repo, "c");
        // a commit only reachable from another branch
        {
            let c = repo.find_commit(first).unwrap();
            repo.branch("side", &c, false).unwrap();
        }
        let all = page(dir.path(), 0, 100);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].summary, "c");
        assert_eq!(all[2].parents.len(), 0);
        assert_eq!(all[0].short_id.len(), 7);

        let second = page(dir.path(), 1, 1);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].summary, "b");
        assert!(page(dir.path(), 5, 10).is_empty());
    }

    #[test]
    fn log_empty_repo() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        assert!(page(dir.path(), 0, 10).is_empty());
    }

    #[test]
    fn commit_files_and_diff() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let first = commit_file(&repo, "a.txt");
        let root_files = commit_files(dir.path(), &first.to_string()).unwrap();
        assert_eq!(root_files.len(), 1);
        assert_eq!(root_files[0].status, "added");

        // modify a.txt and add b.txt in one commit
        std::fs::write(dir.path().join("a.txt"), "a.txt\nsecond line\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("a.txt")).unwrap();
        idx.add_path(Path::new("b.txt")).unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        let parent = repo.find_commit(first).unwrap();
        let second = repo
            .commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
            .unwrap()
            .to_string();

        let files = commit_files(dir.path(), &second).unwrap();
        let names: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.status)).collect();
        assert_eq!(names, vec![("a.txt", "modified"), ("b.txt", "added")]);

        let d = commit_file_diff(dir.path(), &second, "a.txt").unwrap();
        assert!(!d.binary);
        assert_eq!(d.hunks.len(), 1);
        let added: Vec<_> = d.hunks[0].lines.iter().filter(|l| l.kind == '+').collect();
        // the old last line had no trailing newline, so it is rewritten too
        assert_eq!(added.len(), 2);
        assert_eq!(added[1].text, "second line");
        assert_eq!(added[1].new_no, Some(2));
    }

    #[test]
    fn safe_join_blocks_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/a.txt"), "hi").unwrap();
        std::fs::write(dir.path().join("secret.txt"), "nope").unwrap();
        assert!(safe_join(&root, "sub/a.txt").is_ok());
        assert!(safe_join(&root, "../secret.txt")
            .unwrap_err()
            .contains("outside"));
        assert!(safe_join(&root, "sub/../../secret.txt").is_err());
        assert!(safe_join(&root, "missing.txt")
            .unwrap_err()
            .contains("does not exist"));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path().join("secret.txt"), root.join("link.txt"))
                .unwrap();
            assert!(
                safe_join(&root, "link.txt").is_err(),
                "symlink out of the repo is refused"
            );
        }
        assert_eq!(read_text(&root, "sub/a.txt").unwrap(), "hi");
        std::fs::write(root.join("bin"), [0u8, 1, 2]).unwrap();
        assert!(read_text(&root, "bin").unwrap_err().contains("Binary"));
    }

    #[test]
    fn search_matches_message_author_hash_with_and_semantics() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = |n: &str, e: &str| git2::Signature::now(n, e).unwrap();
        let mut parent: Option<git2::Oid> = None;
        for (i, (msg, name, mail)) in [
            ("Add login page", "Ana Ferreira", "ana@x.org"),
            ("Fix login redirect bug", "Miguel Santos", "miguel@y.org"),
            (
                "Refactor store\n\nLong body mentions Kubernetes",
                "Ana Ferreira",
                "ana@x.org",
            ),
        ]
        .iter()
        .enumerate()
        {
            std::fs::write(dir.path().join("f.txt"), i.to_string()).unwrap();
            let mut idx = repo.index().unwrap();
            idx.add_path(Path::new("f.txt")).unwrap();
            idx.write().unwrap();
            let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
            let parents: Vec<git2::Commit> = parent
                .iter()
                .map(|p| repo.find_commit(*p).unwrap())
                .collect();
            let refs: Vec<&git2::Commit> = parents.iter().collect();
            let s = sig(name, mail);
            parent = Some(
                repo.commit(Some("HEAD"), &s, &s, msg, &tree, &refs)
                    .unwrap(),
            );
        }
        let ids = log_ids(dir.path()).unwrap();
        let q = |s: &str| search(dir.path(), &ids, s, 50).unwrap();

        assert_eq!(q("login").len(), 2);
        assert_eq!(
            q("LOGIN redirect").len(),
            1,
            "terms are ANDed, case-insensitive"
        );
        assert_eq!(q("ana").len(), 2, "author name / email");
        assert_eq!(q("miguel@y.org")[0].summary, "Fix login redirect bug");
        assert_eq!(q("kubernetes").len(), 1, "body text is searched too");
        assert_eq!(q("ana login")[0].summary, "Add login page");
        let hash = &ids[0].to_string()[..8];
        assert_eq!(q(hash)[0].id, ids[0].to_string(), "hash prefix");
        assert!(q("nothing matches this").is_empty());
        assert!(q("   ").is_empty(), "blank query returns nothing");
        assert_eq!(
            search(dir.path(), &ids, "login", 1).unwrap().len(),
            1,
            "limit is honoured"
        );
        assert_eq!(
            q("login")[0].summary,
            "Fix login redirect bug",
            "newest first"
        );
    }

    fn two_branches(dir: &Path) -> git2::Repository {
        let repo = git2::Repository::init(dir).unwrap();
        commit_file(&repo, "base.txt");
        repo
    }

    #[test]
    fn merge_preview_counts_fast_forward_diverged_and_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let repo = two_branches(dir.path());
        let main = repo.head().unwrap().shorthand().unwrap().to_string();
        let base = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("ahead", &base, false).unwrap();

        // main moves on: touches shared.txt
        std::fs::write(dir.path().join("shared.txt"), "main\n").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("shared.txt")).unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "main edits shared",
            &tree,
            &[&base],
        )
        .unwrap();

        // `ahead` branch (from base) edits shared.txt differently => conflict; plus one clean commit
        repo.set_head("refs/heads/ahead").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        std::fs::write(dir.path().join("shared.txt"), "ahead\n").unwrap();
        let mut idx = repo.index().unwrap();
        idx.add_path(Path::new("shared.txt")).unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let ahead_parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "ahead edits shared",
            &tree,
            &[&ahead_parent],
        )
        .unwrap();
        commit_file(&repo, "new_only_on_ahead.txt");

        repo.set_head(&format!("refs/heads/{main}")).unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();

        let p = merge_preview(dir.path(), "ahead").unwrap();
        assert_eq!((p.incoming, p.only_here), (2, 1));
        assert!(!p.up_to_date && !p.can_fast_forward);
        assert_eq!(p.target, main);
        assert_eq!(p.commits.len(), 2);
        assert_eq!(
            p.commits[0].summary, "new_only_on_ahead.txt",
            "newest first"
        );
        assert!(p.files >= 2);
        assert_eq!(p.conflicts, Some(vec!["shared.txt".to_string()]));

        // up to date: merging the branch you are on / an ancestor
        let u = merge_preview(dir.path(), &main).unwrap();
        assert!(u.up_to_date && u.incoming == 0 && u.commits.is_empty());

        // bad input
        assert!(merge_preview(dir.path(), "--evil").is_err());
        assert!(merge_preview(dir.path(), "no-such-branch")
            .unwrap_err()
            .message()
            .contains("doesn't exist"));
    }

    #[test]
    fn merge_preview_fast_forward_has_no_conflicts_and_resolves_remote_names() {
        let dir = tempfile::tempdir().unwrap();
        let repo = two_branches(dir.path());
        let base = repo.head().unwrap().peel_to_commit().unwrap();
        let tip = commit_file(&repo, "x.txt");
        // current branch goes back to base; origin/feature points at the newer commit
        repo.reference("refs/remotes/origin/feature", tip, false, "t")
            .unwrap();
        repo.reference(
            repo.head().unwrap().name().unwrap(),
            base.id(),
            true,
            "rewind",
        )
        .unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();

        let p = merge_preview(dir.path(), "origin/feature").unwrap();
        assert!(p.can_fast_forward && p.incoming == 1 && p.only_here == 0);
        assert_eq!(p.conflicts, Some(vec![]));
        assert_eq!(p.files, 1);
    }
}
