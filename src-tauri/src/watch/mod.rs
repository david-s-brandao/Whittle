//! Watches a repository for changes on disk and reports them, debounced.
use notify::{EventKind, RecursiveMode, Watcher as _};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

/// What kind of thing changed.
#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub struct Change {
    /// A tracked-or-untracked, non-ignored file in the working tree changed.
    pub worktree: bool,
    /// Git's own state changed: HEAD, index, refs (commits, branches, tags, stash, fetch...).
    pub git: bool,
}

impl Change {
    fn any(self) -> bool {
        self.worktree || self.git
    }
}

/// Does a path inside `.git` (given relative to it) represent a state change we care about?
fn git_internal_matters(rel: &Path) -> bool {
    let name = rel.to_string_lossy();
    if name.ends_with(".lock") {
        return false;
    }
    let first = rel
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().into_owned());
    match first.as_deref() {
        Some("HEAD" | "index" | "packed-refs" | "MERGE_HEAD" | "ORIG_HEAD" | "FETCH_HEAD") => true,
        Some("refs") => true,
        // rebase / cherry-pick / merge in progress
        Some(d) if d.starts_with("rebase-") || d == "CHERRY_PICK_HEAD" || d == "REVERT_HEAD" => {
            true
        }
        _ => false, // objects, logs, hooks, info, ...
    }
}

/// Reduces a batch of changed paths to what matters to the UI.
pub fn classify(root: &Path, paths: &[PathBuf]) -> Change {
    let repo = git2::Repository::open(root).ok();
    let mut out = Change::default();
    for p in paths {
        let Ok(rel) = p.strip_prefix(root) else {
            continue;
        };
        if let Ok(inner) = rel.strip_prefix(".git") {
            out.git |= git_internal_matters(inner);
        } else if !out.worktree {
            let ignored = repo
                .as_ref()
                .and_then(|r| r.is_path_ignored(rel).ok())
                .unwrap_or(false);
            out.worktree = !ignored;
        }
        if out.worktree && out.git {
            break;
        }
    }
    out
}

struct Inner {
    watcher: notify::RecommendedWatcher,
    /// Directories that currently have a (non-recursive) watch.
    watched: std::collections::HashSet<PathBuf>,
}

impl Inner {
    fn add(&mut self, dir: &Path) {
        if self.watched.insert(dir.to_path_buf())
            && self
                .watcher
                .watch(dir, RecursiveMode::NonRecursive)
                .is_err()
        {
            // e.g. the folder vanished meanwhile, or the system's inotify limit was reached
            self.watched.remove(dir);
        }
    }

    /// Watches `start` and every non-ignored directory below it. Ignored directories
    /// (`node_modules`, `target`, ...) are skipped entirely: they can hold tens of thousands of
    /// folders and nothing in them matters to the UI. Symlinks are not followed.
    fn add_tree(&mut self, root: &Path, start: &Path, repo: Option<&git2::Repository>) {
        let mut stack = vec![start.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let rel = dir.strip_prefix(root).unwrap_or(&dir);
            if rel.starts_with(".git") {
                continue;
            }
            let ignored = !rel.as_os_str().is_empty()
                && repo.is_some_and(|r| r.is_path_ignored(rel).unwrap_or(false));
            if ignored {
                continue;
            }
            self.add(&dir);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    if e.file_type().is_ok_and(|t| t.is_dir()) {
                        stack.push(e.path());
                    }
                }
            }
        }
    }

    /// `.git` itself (HEAD, index, packed-refs, merge/rebase state) and everything under
    /// `.git/refs`. Objects, logs and hooks are deliberately not watched.
    fn add_git_dir(&mut self, root: &Path) {
        let git = root.join(".git");
        if !git.is_dir() {
            return;
        }
        self.add(&git);
        let mut stack = vec![git.join("refs")];
        while let Some(dir) = stack.pop() {
            self.add(&dir);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    if e.file_type().is_ok_and(|t| t.is_dir()) {
                        stack.push(e.path());
                    }
                }
            }
        }
    }
}

pub struct Watcher {
    // Dropping the watcher drops its notify handle and with it the channel sender,
    // which ends the debounce thread.
    inner: std::sync::Arc<std::sync::Mutex<Inner>>,
}

impl Watcher {
    /// Starts watching `root`; `on_change` runs on a background thread once things have been
    /// quiet for `debounce`.
    ///
    /// Read-only "access" events are ignored on purpose: classifying a change reads
    /// `.gitignore` and `.git/config`, which would otherwise trigger events forever.
    pub fn start(
        root: &Path,
        debounce: Duration,
        mut on_change: impl FnMut(Change) + Send + 'static,
    ) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                if !matches!(ev.kind, EventKind::Access(_)) {
                    let _ = tx.send(ev.paths);
                }
            }
        })?;
        let mut inner = Inner {
            watcher,
            watched: Default::default(),
        };
        let repo = git2::Repository::open(root).ok();
        inner.add_tree(root, root, repo.as_ref());
        inner.add_git_dir(root);
        if inner.watched.is_empty() {
            return Err(notify::Error::generic(
                "could not watch any folder of the repository",
            ));
        }
        let inner = std::sync::Arc::new(std::sync::Mutex::new(inner));

        let weak = std::sync::Arc::downgrade(&inner);
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            let mut pending: Vec<PathBuf> = Vec::new();
            loop {
                let next = if pending.is_empty() {
                    rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
                } else {
                    rx.recv_timeout(debounce)
                };
                match next {
                    Ok(paths) => pending.extend(paths),
                    Err(RecvTimeoutError::Timeout) => {
                        // folders created since start need their own watch
                        if let Some(inner) = weak.upgrade() {
                            let repo = git2::Repository::open(&root).ok();
                            if let Ok(mut inner) = inner.lock() {
                                for p in &pending {
                                    if p.starts_with(&root)
                                        && !inner.watched.contains(p)
                                        && p.is_dir()
                                    {
                                        inner.add_tree(&root, p, repo.as_ref());
                                    }
                                }
                            }
                        }
                        let change = classify(&root, &pending);
                        pending.clear();
                        if change.any() {
                            on_change(change);
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        Ok(Self { inner })
    }

    /// How many folders are being watched (for tests and diagnostics).
    pub fn watched_count(&self) -> usize {
        self.inner.lock().map(|i| i.watched.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        dir
    }

    #[test]
    fn classify_filters_noise_and_ignored() {
        let dir = repo();
        let root = dir.path();
        std::fs::write(root.join(".gitignore"), "target/\n*.log\n").unwrap();
        let p = |s: &str| root.join(s);

        let c = classify(
            root,
            &[
                p(".git/objects/ab/cdef"),
                p(".git/index.lock"),
                p(".git/logs/HEAD"),
            ],
        );
        assert!(!c.any(), "git internals are noise: {c:?}");
        let c = classify(root, &[p("target/debug/x"), p("debug.log")]);
        assert!(!c.any(), "ignored paths are noise: {c:?}");
        let c = classify(root, &[p("src/main.rs")]);
        assert_eq!(
            c,
            Change {
                worktree: true,
                git: false
            }
        );
        let c = classify(root, &[p(".git/index")]);
        assert_eq!(
            c,
            Change {
                worktree: false,
                git: true
            }
        );
        let c = classify(root, &[p(".git/refs/heads/main")]);
        assert!(c.git);
        let c = classify(root, &[p(".git/HEAD"), p("a.txt")]);
        assert_eq!(
            c,
            Change {
                worktree: true,
                git: true
            }
        );
    }

    #[test]
    fn detects_real_file_changes_and_ignores_ignored() {
        let dir = repo();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
        std::fs::create_dir(root.join("ignored")).unwrap();
        let (tx, rx) = mpsc::channel();
        let _w = Watcher::start(&root, Duration::from_millis(100), move |c| {
            let _ = tx.send(c);
        })
        .unwrap();

        // the .gitignore write itself may arrive; drain anything pending first
        std::thread::sleep(Duration::from_millis(400));
        while rx.try_recv().is_ok() {}

        std::fs::write(root.join("ignored/junk.txt"), "x").unwrap();
        assert!(
            rx.recv_timeout(Duration::from_millis(800)).is_err(),
            "ignored file must not notify"
        );

        std::fs::write(root.join("real.txt"), "x").unwrap();
        let c = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("worktree change reported");
        assert!(c.worktree);
    }

    #[test]
    fn ignored_trees_are_not_watched_and_startup_is_fast() {
        let dir = repo();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(root.join(".gitignore"), "node_modules/\n").unwrap();
        std::fs::create_dir_all(root.join("src/components")).unwrap();
        // a huge ignored tree: 40 packages x 60 nested folders
        for i in 0..40 {
            for j in 0..60 {
                std::fs::create_dir_all(root.join(format!("node_modules/pkg{i}/lib/sub{j}")))
                    .unwrap();
            }
        }
        let t = std::time::Instant::now();
        let w = Watcher::start(&root, Duration::from_millis(100), |_| {}).unwrap();
        let took = t.elapsed();
        assert!(
            w.watched_count() < 20,
            "watched {} folders",
            w.watched_count()
        );
        assert!(took < Duration::from_millis(500), "startup took {took:?}");
    }

    #[test]
    fn folders_created_later_get_watched() {
        let dir = repo();
        let root = dir.path().canonicalize().unwrap();
        let (tx, rx) = mpsc::channel();
        let _w = Watcher::start(&root, Duration::from_millis(100), move |c| {
            let _ = tx.send(c);
        })
        .unwrap();
        std::thread::sleep(Duration::from_millis(400));
        while rx.try_recv().is_ok() {}

        std::fs::create_dir_all(root.join("fresh/deep")).unwrap();
        std::thread::sleep(Duration::from_millis(700)); // let the new folders be registered
        while rx.try_recv().is_ok() {}
        std::fs::write(root.join("fresh/deep/file.txt"), "x").unwrap();
        let c = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("change inside the new folder is seen");
        assert!(c.worktree);
    }

    #[test]
    fn git_state_changes_are_seen_without_watching_objects() {
        let dir = repo();
        let root = dir.path().canonicalize().unwrap();
        let (tx, rx) = mpsc::channel();
        let w = Watcher::start(&root, Duration::from_millis(100), move |c| {
            let _ = tx.send(c);
        })
        .unwrap();
        std::thread::sleep(Duration::from_millis(400));
        while rx.try_recv().is_ok() {}
        // a ref appearing under .git/refs/heads
        std::fs::write(
            root.join(".git/refs/heads/feature"),
            "0000000000000000000000000000000000000000\n",
        )
        .unwrap();
        let c = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("ref change seen");
        assert!(c.git);
        assert!(w.watched_count() >= 3);
    }
}
