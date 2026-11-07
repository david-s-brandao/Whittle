//! Mutating git operations, run through the user's `git` binary so SSH keys,
//! credential helpers, hooks and signing all apply.
use std::path::Path;
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("failed to run git: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("{0}")]
    Failed(String),
    #[error("{0}")]
    Git(#[from] git2::Error),
}

static GIT_BINARY: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Use this `git` executable instead of the one on PATH (`None` = PATH). Set from the settings.
pub fn set_git_binary(path: Option<String>) {
    if let Ok(mut b) = GIT_BINARY.write() {
        *b = path.filter(|p| !p.trim().is_empty());
    }
}

/// A new `git` process using the configured executable.
fn git_command() -> Command {
    let bin = GIT_BINARY.read().ok().and_then(|b| b.clone());
    Command::new(bin.unwrap_or_else(|| "git".to_string()))
}

/// Runs `<bin> --version`; returns its output ("git version 2.43.0") or why it can't run.
pub fn check_git_binary(bin: &str) -> Result<String, String> {
    let bin = bin.trim();
    let bin = if bin.is_empty() { "git" } else { bin };
    let out = Command::new(bin)
        .arg("--version")
        .output()
        .map_err(|e| format!("Can't run “{bin}”: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() && text.starts_with("git version") {
        Ok(text)
    } else {
        Err(format!("“{bin}” doesn't look like git."))
    }
}

/// Runs `git <args>` in `repo` and returns stdout.
pub fn run(repo: &Path, args: &[&str]) -> Result<String, CliError> {
    let out = git_command()
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if err.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err
        };
        Err(CliError::Failed(msg))
    }
}

/// Runs git and calls `on_line` for every line git prints on stderr (progress uses `\r`, so
/// both `\r` and `\n` end a line). Returns the stderr lines that are not progress noise.
pub fn run_streaming(
    repo: &Path,
    args: &[&str],
    mut on_line: impl FnMut(&str),
) -> Result<Vec<String>, CliError> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = git_command()
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    // drain stdout on a thread so a full pipe can never stall git
    let mut stdout = child.stdout.take().expect("piped");
    let drain = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let mut stderr = child.stderr.take().expect("piped");
    let (mut buf, mut lines) = (Vec::new(), Vec::new());
    let mut chunk = [0u8; 4096];
    let mut flush = |buf: &mut Vec<u8>, lines: &mut Vec<String>| {
        if buf.is_empty() {
            return;
        }
        let line = String::from_utf8_lossy(buf).trim().to_string();
        buf.clear();
        if line.is_empty() {
            return;
        }
        on_line(&line);
        let is_progress = line.contains('%') && !line.contains("done");
        if !is_progress {
            lines.push(line);
        }
    };
    loop {
        let n = stderr.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        for &b in &chunk[..n] {
            if b == b'\r' || b == b'\n' {
                flush(&mut buf, &mut lines);
            } else {
                buf.push(b);
            }
        }
    }
    flush(&mut buf, &mut lines);
    let status = child.wait()?;
    let out = drain.join().unwrap_or_default();
    if status.success() {
        Ok(lines)
    } else {
        let mut all = lines;
        all.extend(out.lines().map(str::to_owned));
        Err(CliError::Failed(all.join("\n")))
    }
}

/// Runs git with `input` on stdin.
fn run_stdin(repo: &Path, args: &[&str], input: &str) -> Result<String, CliError> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = git_command()
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("piped")
        .write_all(input.as_bytes())?;
    let out = child.wait_with_output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(CliError::Failed(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

/// Stages (or, with `unstage`, unstages) the selected hunks/lines of one file.
/// Works on modified tracked files; new/deleted files are staged as a whole instead.
pub fn apply_selection(
    repo: &Path,
    file: &str,
    unstage: bool,
    selections: &[crate::git::Selection],
) -> Result<(), CliError> {
    let diff = crate::git::work_file_diff(repo, file, unstage)?;
    let patch =
        crate::git::build_patch(file, &diff, selections, unstage).map_err(CliError::Failed)?;
    let mut args = vec!["apply", "--cached", "--recount", "--whitespace=nowarn"];
    if unstage {
        args.push("--reverse");
    }
    args.push("-");
    run_stdin(repo, &args, &patch).map(|_| ())
}

/// Runs git and returns (success, stdout, stderr); nothing is thrown away.
fn run_full(repo: &Path, args: &[&str]) -> Result<(bool, String, String), CliError> {
    let out = git_command()
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// Like `run_full` but never opens an editor (git would otherwise wait for one forever).
fn run_noedit(repo: &Path, args: &[&str]) -> Result<(bool, String, String), CliError> {
    let out = git_command()
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true")
        .output()?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// What a merge/rebase step ended with.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct OpResult {
    /// "done", "up_to_date" or "conflicts" (paused: resolve, then continue or abort).
    pub outcome: &'static str,
    pub message: String,
}

fn op_result(outcome: &'static str, message: &str) -> Result<OpResult, CliError> {
    Ok(OpResult {
        outcome,
        message: message.to_string(),
    })
}

/// Turns a failed merge/rebase/continue into either a paused-on-conflicts result or an error.
fn op_failure(what: &str, stdout: &str, stderr: &str) -> Result<OpResult, CliError> {
    let all = format!("{stdout}\n{stderr}");
    let has = |s: &str| all.contains(s);
    if has("CONFLICT") || has("Automatic merge failed") || has("could not apply") {
        return op_result(
            "conflicts",
            &format!("The {what} stopped because of conflicts. Resolve them in the Changes tab, then continue (or abort)."),
        );
    }
    if has("would be overwritten")
        || has("unstaged changes")
        || has("uncommitted changes")
        || has("Please commit or stash")
    {
        return Err(CliError::Failed(format!(
            "You have uncommitted changes that get in the way of this {what}. Commit or stash them first, then try again."
        )));
    }
    if has("unmerged files")
        || has("Committing is not possible")
        || has("you must edit all merge conflicts")
    {
        return Err(CliError::Failed(
            "There are still unresolved conflicts. Resolve and mark every file first.".into(),
        ));
    }
    if has("not something we can merge")
        || has("invalid upstream")
        || has("does not point to a valid")
    {
        return Err(CliError::Failed("That branch doesn't exist.".into()));
    }
    let msg = stderr.trim();
    Err(CliError::Failed(if msg.is_empty() {
        stdout.trim().to_string()
    } else {
        msg.to_string()
    }))
}

/// How to merge. Mirrors the options other Git GUIs offer.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MergeOptions {
    /// "auto" (fast-forward when possible), "never" (always a merge commit), "only"
    /// (refuse unless it can fast-forward).
    pub ff: String,
    /// Combine everything into one set of staged changes; nothing is committed, HEAD stays.
    pub squash: bool,
    /// Do the merge but stop before committing, so the result can be reviewed first.
    pub no_commit: bool,
    /// Message for the merge commit (empty = git's default).
    pub message: Option<String>,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            ff: "auto".into(),
            squash: false,
            no_commit: false,
            message: None,
        }
    }
}

/// Merges `branch` into the current branch. `no_ff` forces a merge commit even when a
/// fast-forward is possible.
pub fn merge(repo: &Path, branch: &str, no_ff: bool) -> Result<OpResult, CliError> {
    let opts = MergeOptions {
        ff: if no_ff { "never" } else { "auto" }.into(),
        ..Default::default()
    };
    merge_with(repo, branch, &opts)
}

pub fn merge_with(repo: &Path, branch: &str, o: &MergeOptions) -> Result<OpResult, CliError> {
    check_name(branch)?;
    let mut args: Vec<String> = vec!["merge".into()];
    let message = o
        .message
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    if o.squash {
        args.push("--squash".into());
    } else {
        match o.ff.as_str() {
            "never" => args.push("--no-ff".into()),
            "only" => args.push("--ff-only".into()),
            _ if o.no_commit => args.push("--no-ff".into()), // otherwise git would just fast-forward
            _ => {}
        }
        if o.no_commit {
            args.push("--no-commit".into());
        } else if let Some(m) = message {
            args.extend(["-m".into(), m.into()]);
        } else {
            args.push("--no-edit".into());
        }
    }
    args.push(branch.into());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (ok, out, err) = run_noedit(repo, &refs)?;
    if !ok {
        let all = format!("{out}\n{err}");
        if all.contains("Not possible to fast-forward") {
            return Err(CliError::Failed(
                "Can't fast-forward: both branches have new commits. Choose “merge commit” instead, or rebase.".into(),
            ));
        }
        return op_failure("merge", &out, &err);
    }
    if out.contains("Already up to date") {
        return op_result("up_to_date", "Already up to date: nothing to merge.");
    }
    if o.squash {
        return op_result(
            "done",
            &format!(
                "Changes from {branch} are staged. Review them in the Changes tab, then commit."
            ),
        );
    }
    if o.no_commit {
        return op_result(
            "done",
            &format!("Merged {branch} into your working tree without committing. Review, then continue the merge."),
        );
    }
    if out.contains("Fast-forward") {
        op_result("done", &format!("Fast-forwarded to {branch}."))
    } else {
        op_result("done", &format!("Merged {branch}."))
    }
}

/// Replays the current branch's commits on top of `onto`.
pub fn rebase(repo: &Path, onto: &str) -> Result<OpResult, CliError> {
    check_name(onto)?;
    let (ok, out, err) = run_noedit(repo, &["rebase", onto])?;
    if !ok {
        return op_failure("rebase", &out, &err);
    }
    let all = format!("{out}{err}");
    if all.contains("up to date") {
        op_result("up_to_date", "Already up to date: nothing to rebase.")
    } else {
        op_result("done", &format!("Rebased onto {onto}."))
    }
}

/// Removes the interactive-rebase helper files when dropped, if no rebase is paused anymore.
struct ScratchCleanup(std::path::PathBuf);

impl Drop for ScratchCleanup {
    fn drop(&mut self) {
        if matches!(crate::git::work_status(&self.0), Ok(s) if s.operation != Some("rebase")) {
            if let Ok(dir) = rebase_scratch(&self.0) {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
    }
}

/// Which operation is paused, if any (merge, rebase, cherry-pick or revert).
fn paused(repo: &Path) -> Result<&'static str, CliError> {
    match crate::git::work_status(repo)?.operation {
        Some(op @ ("merge" | "rebase" | "cherry-pick" | "revert")) => Ok(op),
        _ => Err(CliError::Failed(
            "There is no merge, rebase, cherry-pick or revert in progress.".into(),
        )),
    }
}

/// Finishes a paused operation after the conflicts were resolved. A rebase or a multi-commit
/// cherry-pick may stop again on its next commit, which comes back as `conflicts`.
pub fn operation_continue(repo: &Path) -> Result<OpResult, CliError> {
    let _cleanup = ScratchCleanup(repo.to_path_buf());
    let kind = paused(repo)?;
    let (ok, out, err) = run_noedit(repo, &[kind, "--continue"])?;
    if ok {
        let done = match kind {
            "merge" => "Merge completed.",
            "rebase" => "Rebase completed.",
            "cherry-pick" => "Cherry-pick completed.",
            _ => "Revert completed.",
        };
        op_result("done", done)
    } else {
        op_failure(kind, &out, &err)
    }
}

/// Skips the commit that conflicts (rebase and cherry-pick only).
pub fn operation_skip(repo: &Path) -> Result<OpResult, CliError> {
    let _cleanup = ScratchCleanup(repo.to_path_buf());
    let kind = paused(repo)?;
    if kind != "rebase" && kind != "cherry-pick" {
        return Err(CliError::Failed(
            "Only a rebase or cherry-pick can skip a commit.".into(),
        ));
    }
    let (ok, out, err) = run_noedit(repo, &[kind, "--skip"])?;
    if ok {
        op_result("done", "Done (the conflicting commit was skipped).")
    } else {
        op_failure(kind, &out, &err)
    }
}

fn is_merge_commit(repo: &Path, commit: &str) -> bool {
    git2::Repository::open(repo)
        .ok()
        .and_then(|r| {
            r.revparse_single(commit)
                .ok()
                .and_then(|o| o.peel_to_commit().ok())
                .map(|c| c.parent_count() > 1)
        })
        .unwrap_or(false)
}

/// Copies one commit onto the current branch. Merge commits are applied relative to their
/// first parent.
pub fn cherry_pick(repo: &Path, commit: &str) -> Result<OpResult, CliError> {
    check_name(commit)?;
    let mut args = vec!["cherry-pick"];
    if is_merge_commit(repo, commit) {
        args.extend(["-m", "1"]);
    }
    args.push(commit);
    let (ok, out, err) = run_noedit(repo, &args)?;
    if ok {
        return op_result("done", "Cherry-picked onto the current branch.");
    }
    let all = format!("{out}{err}");
    if all.contains("empty") && all.contains("cherry-pick") {
        // already applied: git stops and asks; leave a clean state
        let _ = run_noedit(repo, &["cherry-pick", "--skip"]);
        return op_result(
            "up_to_date",
            "Nothing to do: those changes are already in this branch.",
        );
    }
    op_failure("cherry-pick", &out, &err)
}

/// Adds a new commit that undoes `commit`.
pub fn revert(repo: &Path, commit: &str) -> Result<OpResult, CliError> {
    check_name(commit)?;
    let mut args = vec!["revert", "--no-edit"];
    if is_merge_commit(repo, commit) {
        args.extend(["-m", "1"]);
    }
    args.push(commit);
    let (ok, out, err) = run_noedit(repo, &args)?;
    if ok {
        op_result("done", "Reverted: a new commit undoes those changes.")
    } else {
        op_failure("revert", &out, &err)
    }
}

/// Moves the current branch to `target`. `mode`: "soft" keeps changes staged, "mixed" keeps
/// them unstaged, "hard" discards them (the UI must confirm). The message names the previous
/// position so the move can be undone.
pub fn reset(repo: &Path, target: &str, mode: &str) -> Result<String, CliError> {
    check_name(target)?;
    let flag = match mode {
        "soft" => "--soft",
        "mixed" => "--mixed",
        "hard" => "--hard",
        _ => return Err(CliError::Failed("Unknown reset mode.".into())),
    };
    let before = run(repo, &["rev-parse", "--short", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string();
    run(repo, &["reset", "-q", flag, target])?;
    let after = run(repo, &["rev-parse", "--short", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok(format!("Branch moved to {after} (it was at {before}). You can undo this by resetting back to {before}."))
}

/// Checks out a commit without a branch (detached HEAD).
pub fn checkout_detached(repo: &Path, commit: &str) -> Result<(), CliError> {
    check_name(commit)?;
    run(repo, &["switch", "--detach", commit])
        .map(|_| ())
        .map_err(friendly)
}

/// Creates a tag at `target` (default HEAD). A non-empty `message` makes an annotated tag.
pub fn create_tag(
    repo: &Path,
    name: &str,
    target: Option<&str>,
    message: Option<&str>,
) -> Result<(), CliError> {
    check_name(name)?;
    run(repo, &["check-ref-format", &format!("refs/tags/{name}")])
        .map_err(|_| CliError::Failed(format!("“{name}” is not a valid tag name.")))?;
    let mut args: Vec<&str> = vec!["tag"];
    let msg = message.map(str::trim).filter(|m| !m.is_empty());
    if let Some(m) = msg {
        args.extend(["-a", "-m", m]);
    }
    args.push(name);
    if let Some(t) = target {
        check_name(t)?;
        args.push(t);
    }
    run(repo, &args).map(|_| ()).map_err(|e| match e {
        CliError::Failed(m) if m.contains("already exists") => {
            CliError::Failed(format!("A tag named “{name}” already exists."))
        }
        other => other,
    })
}

pub fn delete_tag(repo: &Path, name: &str) -> Result<String, CliError> {
    check_name(name)?;
    Ok(run(repo, &["tag", "-d", name])?.trim().to_string())
}

/// Pushes a tag to the default remote, or (with `delete`) removes it there. Streams progress.
pub fn push_tag(
    repo: &Path,
    name: &str,
    delete: bool,
    on_line: impl FnMut(&str),
) -> Result<(), SyncError> {
    check_name(name).map_err(|e| err("other", &e.to_string(), ""))?;
    let Some(remote) = crate::git::default_remote(repo)? else {
        return Err(err(
            "no_remote",
            "This repository has no remote to push the tag to.",
            "",
        ));
    };
    let refspec = format!("refs/tags/{name}");
    let mut args = vec!["push", "--progress", remote.as_str()];
    if delete {
        args.push("--delete");
    }
    args.push(&refspec);
    match run_streaming(repo, &args, on_line) {
        Ok(_) => Ok(()),
        Err(CliError::Failed(raw)) => Err(classify(&raw, has_ssh_agent())),
        Err(e) => Err(err("other", &e.to_string(), "")),
    }
}

/// One line of an interactive rebase plan.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RebaseItem {
    pub id: String,
    /// "pick", "reword", "squash", "fixup" or "drop".
    pub action: String,
    /// New commit message (required for "reword").
    pub message: Option<String>,
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Folder inside `.git` that holds the plan's helper files while a rebase is running.
fn rebase_scratch(repo: &Path) -> Result<std::path::PathBuf, CliError> {
    Ok(git2::Repository::open(repo)?.path().join("whittle-rebase"))
}

/// Interactive rebase of everything from `from` (inclusive) to HEAD, following `items`
/// (the new order and what to do with each commit). No editor ever opens: git is handed a
/// ready-made todo list. May pause on conflicts like any rebase.
pub fn rebase_interactive(
    repo: &Path,
    from: &str,
    items: &[RebaseItem],
) -> Result<OpResult, CliError> {
    let candidates = crate::git::rebase_candidates(repo, from)?;
    if items.is_empty() {
        return Err(CliError::Failed("There is nothing to rebase.".into()));
    }
    // only commits from the range, each at most once
    let mut seen = std::collections::HashSet::new();
    for it in items {
        if !candidates.iter().any(|c| c.id == it.id) || !seen.insert(it.id.clone()) {
            return Err(CliError::Failed(
                "The plan doesn't match the commits in the range.".into(),
            ));
        }
        if !["pick", "reword", "squash", "fixup", "drop"].contains(&it.action.as_str()) {
            return Err(CliError::Failed(format!("Unknown action “{}”.", it.action)));
        }
        if it.action == "reword"
            && it
                .message
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            return Err(CliError::Failed(
                "A reworded commit needs a message.".into(),
            ));
        }
    }
    let first_kept = items.iter().find(|i| i.action != "drop");
    if matches!(
        first_kept.map(|i| i.action.as_str()),
        Some("squash" | "fixup")
    ) {
        return Err(CliError::Failed(
            "The first commit can't be squashed or fixed up: there is nothing before it to combine with.".into(),
        ));
    }
    if first_kept.is_none() {
        return Err(CliError::Failed(
            "Dropping every commit would leave nothing to rebase.".into(),
        ));
    }

    let dir = rebase_scratch(repo)?;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let mut todo = String::new();
    for (n, it) in items.iter().enumerate() {
        match it.action.as_str() {
            "reword" => {
                let file = dir.join(format!("msg-{n}.txt"));
                std::fs::write(&file, it.message.as_deref().unwrap_or("").trim_end())?;
                todo.push_str(&format!(
                    "pick {}\nexec git commit --amend -q -F {}\n",
                    it.id,
                    sh_quote(&file.to_string_lossy())
                ));
            }
            a => todo.push_str(&format!("{a} {}\n", it.id)),
        }
    }
    let todo_file = dir.join("todo.txt");
    std::fs::write(&todo_file, todo)?;
    let editor = dir.join("seq-editor.sh");
    std::fs::write(
        &editor,
        format!(
            "#!/bin/sh\ncat {} > \"$1\"\n",
            sh_quote(&todo_file.to_string_lossy())
        ),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&editor, std::fs::Permissions::from_mode(0o755))?;
    }

    let git_repo = git2::Repository::open(repo)?;
    let first = git_repo.revparse_single(from)?.peel_to_commit()?;
    let mut args: Vec<String> = vec![
        "rebase".into(),
        "--interactive".into(),
        "--no-autosquash".into(),
    ];
    match first.parent_ids().next() {
        Some(parent) => args.push(parent.to_string()),
        None => args.push("--root".into()),
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = git_command()
        .arg("-C")
        .arg(repo)
        .args(&refs)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", &editor)
        .output()?;
    let (ok, so, se) = (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    );
    if ok {
        let _ = std::fs::remove_dir_all(&dir);
        return op_result("done", "Interactive rebase completed.");
    }
    let r = op_failure("rebase", &so, &se);
    // not paused (a real error): nothing will need the helper files again
    if !matches!(&r, Ok(o) if o.outcome == "conflicts") {
        let _ = std::fs::remove_dir_all(&dir);
    }
    r
}

/// Fetches pull request `number` from the default remote (`refs/pull/N/head`) and switches to a
/// local branch `pr-N` at its tip. An existing `pr-N` is only fast-forwarded, never overwritten,
/// so local work on it is safe.
pub fn checkout_pull(repo: &Path, number: u64) -> Result<String, SyncError> {
    let Some(remote) = crate::git::default_remote(repo)? else {
        return Err(err(
            "no_remote",
            "This repository has no remote to fetch the pull request from.",
            "",
        ));
    };
    let hidden = format!("refs/whittle/pr/{number}");
    let refspec = format!("+pull/{number}/head:{hidden}");
    match run_streaming(repo, &["fetch", "--progress", &remote, &refspec], |_| {}) {
        Ok(_) => {}
        Err(CliError::Failed(raw)) => {
            let l = raw.to_lowercase();
            if l.contains("couldn't find remote ref") {
                return Err(err(
                    "other",
                    &format!("Pull request #{number} wasn't found on the remote."),
                    &raw,
                ));
            }
            return Err(classify(&raw, has_ssh_agent()));
        }
        Err(e) => return Err(err("other", &e.to_string(), "")),
    }
    let branch = format!("pr-{number}");
    let exists = run(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .is_ok();
    let fail = |e: CliError| err("other", &e.to_string(), "");
    if exists {
        run(repo, &["switch", &branch]).map_err(|e| fail(friendly(e)))?;
        if let Err(e) = run(repo, &["merge", "--ff-only", &hidden]) {
            return Err(err(
                "diverged",
                &format!("You're on {branch}, but it has local commits that the pull request doesn't, so it wasn't updated."),
                &e.to_string(),
            ));
        }
        Ok(format!(
            "Switched to {branch} (updated to the latest of the pull request)."
        ))
    } else {
        run(repo, &["switch", "-c", &branch, &hidden]).map_err(|e| fail(friendly(e)))?;
        Ok(format!("Checked out pull request #{number} as {branch}."))
    }
}

/// Clones (if needed) and checks out the commit the parent records, for one submodule or all of
/// them (`path = None`), including nested ones. Streams progress.
pub fn submodule_update(
    repo: &Path,
    path: Option<&str>,
    on_line: impl FnMut(&str),
) -> Result<(), SyncError> {
    let mut args = vec!["submodule", "update", "--init", "--recursive", "--progress"];
    if let Some(p) = path {
        if p.starts_with('-') || p.is_empty() {
            return Err(err("other", "That is not a valid submodule path.", ""));
        }
        args.push("--");
        args.push(p);
    }
    match run_streaming(repo, &args, on_line) {
        Ok(_) => Ok(()),
        Err(CliError::Failed(raw)) => Err(classify(&raw, has_ssh_agent())),
        Err(e) => Err(err("other", &e.to_string(), "")),
    }
}

/// Commits that touched `file`, newest first, following renames. At most `limit`.
pub fn file_history(
    repo: &Path,
    file: &str,
    limit: usize,
) -> Result<Vec<crate::git::CommitInfo>, CliError> {
    if file.is_empty() || file.starts_with('-') {
        return Err(CliError::Failed("That is not a valid file path.".into()));
    }
    let n = format!("-n{}", limit.clamp(1, 1000));
    let out = run(repo, &["log", "--follow", "--format=%H", &n, "--", file])?;
    let r = git2::Repository::open(repo)?;
    let mut commits = Vec::new();
    for id in out.lines().filter(|l| !l.is_empty()) {
        let c = r.find_commit(git2::Oid::from_str(id)?)?;
        commits.push(crate::git::commit_info(&r, &c));
    }
    Ok(commits)
}

/// Result of checking a commit's signature.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct SignatureInfo {
    pub signed: bool,
    /// "good", "good_untrusted", "expired", "revoked", "bad", "unverifiable" or "none".
    pub status: &'static str,
    pub signer: String,
    pub key: String,
}

/// Asks git to verify the signature of `commit` (GPG or SSH, whatever it was signed with).
pub fn signature_info(repo: &Path, commit: &str) -> Result<SignatureInfo, CliError> {
    check_name(commit)?;
    let out = run(repo, &["log", "-1", "--format=%G?%x00%GS%x00%GK", commit])?;
    let mut parts = out.trim_end_matches('\n').splitn(3, '\0');
    let code = parts.next().unwrap_or("N");
    let signer = parts.next().unwrap_or("").to_string();
    let key = parts.next().unwrap_or("").to_string();
    let status = match code {
        "G" => "good",
        "U" => "good_untrusted",
        "X" | "Y" => "expired",
        "R" => "revoked",
        "B" => "bad",
        "E" => "unverifiable",
        _ => "none",
    };
    // "E" (can't check) and the rest still mean a signature exists; "N" means none
    Ok(SignatureInfo {
        signed: code != "N",
        status,
        signer,
        key,
    })
}

/// Cancels a paused operation and puts everything back as it was before it started.
pub fn operation_abort(repo: &Path) -> Result<(), CliError> {
    let _cleanup = ScratchCleanup(repo.to_path_buf());
    let kind = paused(repo)?;
    run_noedit(repo, &[kind, "--abort"]).and_then(|(ok, out, err)| {
        if ok {
            Ok(())
        } else {
            Err(CliError::Failed(format!("{out}{err}").trim().to_string()))
        }
    })
}

/// Resolves one conflicted file by taking one side wholesale ("ours" or "theirs") and staging it.
/// If that side deleted the file, the file is removed.
pub fn resolve_side(repo: &Path, file: &str, side: &str) -> Result<(), CliError> {
    let flag = match side {
        "ours" => "--ours",
        "theirs" => "--theirs",
        _ => return Err(CliError::Failed("Unknown side.".into())),
    };
    match run(repo, &["checkout", flag, "--", file]) {
        Ok(_) => run(repo, &["add", "--", file]).map(|_| ()),
        Err(CliError::Failed(m)) if m.contains("does not have") => {
            run(repo, &["rm", "-q", "--", file]).map(|_| ())
        }
        Err(e) => Err(e),
    }
}

/// Marks a conflicted file as resolved (after editing it by hand).
pub fn mark_resolved(repo: &Path, file: &str) -> Result<(), CliError> {
    run(repo, &["add", "--", file]).map(|_| ())
}

fn stash_ref(index: usize) -> String {
    format!("stash@{{{index}}}")
}

/// Stashes local changes (optionally including untracked files). Returns git's confirmation.
pub fn stash_push(repo: &Path, message: &str, include_untracked: bool) -> Result<String, CliError> {
    let message = message.trim();
    let mut args = vec!["stash", "push"];
    if include_untracked {
        args.push("--include-untracked");
    }
    if !message.is_empty() {
        args.extend(["-m", message]);
    }
    let out = run(repo, &args)?;
    if out.contains("No local changes") {
        return Err(CliError::Failed(
            "Nothing to stash: there are no local changes.".into(),
        ));
    }
    Ok(out.trim().to_string())
}

/// Re-applies a stash. With `pop` the stash is removed afterwards, but only if it applied
/// cleanly: on conflicts git keeps it, and so do we.
pub fn stash_apply(repo: &Path, index: usize, pop: bool) -> Result<String, CliError> {
    let r = stash_ref(index);
    let (ok, stdout, stderr) = run_full(repo, &["stash", if pop { "pop" } else { "apply" }, &r])?;
    if ok {
        return Ok(if pop {
            "Stash applied and removed.".into()
        } else {
            "Stash applied.".into()
        });
    }
    let all = format!("{stdout}\n{stderr}");
    if all.contains("CONFLICT") {
        return Err(CliError::Failed(
            "The stash was applied with conflicts. Resolve them in the Changes view. The stash itself was kept.".into(),
        ));
    }
    if all.contains("would be overwritten") || all.contains("Please commit or stash") {
        return Err(CliError::Failed(
            "Your uncommitted changes overlap with this stash. Commit or stash them first, then try again.".into(),
        ));
    }
    let msg = stderr.trim();
    Err(CliError::Failed(if msg.is_empty() {
        stdout.trim().to_string()
    } else {
        msg.to_string()
    }))
}

/// Deletes a stash. Irreversible apart from the hash git prints; the UI confirms first.
pub fn stash_drop(repo: &Path, index: usize) -> Result<String, CliError> {
    Ok(run(repo, &["stash", "drop", &stash_ref(index)])?
        .trim()
        .to_string())
}

/// Runs `git <args> -- <paths...>` (paths can never be mistaken for options).
fn run_paths(repo: &Path, args: &[&str], paths: &[String]) -> Result<(), CliError> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut full: Vec<&str> = args.to_vec();
    full.push("--");
    full.extend(paths.iter().map(String::as_str));
    run(repo, &full).map(|_| ())
}

pub fn stage(repo: &Path, paths: &[String]) -> Result<(), CliError> {
    // -A so deletions are staged too
    run_paths(repo, &["add", "-A"], paths)
}

pub fn unstage(repo: &Path, paths: &[String]) -> Result<(), CliError> {
    if git2::Repository::open(repo)?.head().is_ok() {
        run_paths(repo, &["restore", "--staged"], paths)
    } else {
        // no commits yet: nothing to restore from, just drop from the index
        run_paths(repo, &["rm", "--cached", "-r", "-q"], paths)
    }
}

/// Throws away working-tree changes: tracked files go back to the index version,
/// untracked files are deleted. Irreversible; the UI must confirm first.
pub fn discard(repo: &Path, paths: &[String]) -> Result<(), CliError> {
    let status = crate::git::work_status(repo)?;
    let untracked = |p: &String| {
        status
            .unstaged
            .iter()
            .any(|f| &f.path == p && f.status == "untracked")
    };
    let (new, tracked): (Vec<String>, Vec<String>) = paths.iter().cloned().partition(untracked);
    run_paths(repo, &["restore", "--worktree"], &tracked)?;
    run_paths(repo, &["clean", "-f", "-q"], &new)
}

/// Commits the index. With `amend`, replaces HEAD (an empty message keeps the old one).
pub fn commit(repo: &Path, message: &str, amend: bool) -> Result<(), CliError> {
    let message = message.trim();
    let mut args = vec!["commit", "-q"];
    if amend {
        args.push("--amend");
    }
    if message.is_empty() {
        if !amend {
            return Err(CliError::Failed("Commit message is empty.".into()));
        }
        args.push("--no-edit");
        run(repo, &args).map(|_| ())
    } else {
        args.extend(["-m", message]);
        run(repo, &args).map(|_| ())
    }
}

/// Branch/ref names are passed to git as positional arguments, so they must never look like options.
fn check_name(name: &str) -> Result<(), CliError> {
    if name.is_empty() || name.starts_with('-') {
        return Err(CliError::Failed(format!(
            "“{name}” is not a valid branch name."
        )));
    }
    Ok(())
}

/// Rewrites the most common checkout failure into advice; passes everything else through.
fn friendly(e: CliError) -> CliError {
    match e {
        CliError::Failed(m) if m.contains("would be overwritten") => CliError::Failed(
            "You have uncommitted changes that conflict with that branch. Commit or stash them first, then switch.".into(),
        ),
        other => other,
    }
}

/// Switches to a local branch. For a remote branch like `origin/feature`, creates
/// (or reuses) a local tracking branch and switches to it.
pub fn checkout(repo: &Path, name: &str, remote: bool) -> Result<(), CliError> {
    check_name(name)?;
    let r = if remote {
        run(repo, &["switch", "--track", name])
    } else {
        run(repo, &["switch", name])
    };
    r.map(|_| ()).map_err(friendly)
}

/// Creates a branch from `start` (default HEAD), optionally switching to it.
pub fn create_branch(
    repo: &Path,
    name: &str,
    start: Option<&str>,
    switch: bool,
) -> Result<(), CliError> {
    check_name(name)?;
    run(repo, &["check-ref-format", "--branch", name])
        .map_err(|_| CliError::Failed(format!("“{name}” is not a valid branch name.")))?;
    let mut args = if switch {
        vec!["switch", "-c", name]
    } else {
        vec!["branch", name]
    };
    if let Some(s) = start {
        check_name(s)?;
        args.push(s);
    }
    run(repo, &args).map(|_| ()).map_err(friendly)
}

/// Deletes a local branch. Without `force` git refuses unmerged branches
/// (message contains "not fully merged"). Returns git's "Deleted branch X (was abc1234)".
pub fn delete_branch(repo: &Path, name: &str, force: bool) -> Result<String, CliError> {
    check_name(name)?;
    let out = run(repo, &["branch", if force { "-D" } else { "-d" }, name])?;
    Ok(out.trim().to_string())
}

/// Suggests a folder name from a clone URL: `git@host:org/repo.git` -> `repo`.
pub fn repo_name_from_url(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    let last = u.rsplit(['/', ':', '\\']).next().unwrap_or("");
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

/// Rejects URLs that would be unsafe or leak secrets.
fn validate_clone_url(url: &str) -> Result<(), SyncError> {
    if url.is_empty() {
        return Err(err("other", "Enter a repository URL.", ""));
    }
    if url.starts_with('-') || url.chars().any(char::is_whitespace) {
        return Err(err(
            "other",
            "That doesn't look like a valid repository URL.",
            "",
        ));
    }
    // https://user:token@host/... would be saved in plain text in .git/config
    if let Some((_, rest)) = url.split_once("://") {
        let authority = rest.split('/').next().unwrap_or("");
        if let Some((userinfo, _)) = authority.rsplit_once('@') {
            if userinfo.contains(':') {
                return Err(err(
                    "other",
                    "Don't put a password or token in the URL. Git would save it in plain text inside the repository. Use a credential helper instead (for example `git config --global credential.helper libsecret`).",
                    "",
                ));
            }
        }
    }
    Ok(())
}

/// Clones `url` into `<parent>/<name>`, streaming progress. Returns the new repo's path.
/// Uses the user's git binary, so SSH keys and credential helpers apply.
pub fn clone(
    url: &str,
    parent: &Path,
    name: &str,
    on_line: impl FnMut(&str),
) -> Result<std::path::PathBuf, SyncError> {
    let url = url.trim();
    validate_clone_url(url)?;
    if name.is_empty()
        || name.starts_with('-')
        || name.contains(['/', '\\'])
        || name == ".."
        || name == "."
    {
        return Err(err(
            "other",
            "Choose a valid folder name for the clone.",
            "",
        ));
    }
    if !parent.is_dir() {
        return Err(err("other", "The destination folder does not exist.", ""));
    }
    let dest = parent.join(name);
    if dest.exists()
        && std::fs::read_dir(&dest)
            .map(|mut d| d.next().is_some())
            .unwrap_or(true)
    {
        return Err(err(
            "other",
            &format!(
                "“{}” already exists and is not empty. Pick another folder name.",
                dest.display()
            ),
            "",
        ));
    }
    // `--` stops url/name from ever being read as options; relative name runs inside `parent`
    match run_streaming(parent, &["clone", "--progress", "--", url, name], on_line) {
        Ok(_) => Ok(dest),
        Err(CliError::Failed(raw)) => Err(classify(&raw, has_ssh_agent())),
        Err(e) => Err(err("other", &e.to_string(), "")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOp {
    Fetch,
    Pull,
    Push,
}

/// A failed network operation, in words a person can act on.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct SyncError {
    /// auth_ssh, auth_https, network, rejected, diverged, conflict, no_remote, no_upstream, other
    pub kind: &'static str,
    pub message: String,
    /// Raw git output for the "show details" toggle.
    pub details: String,
}

impl From<git2::Error> for SyncError {
    fn from(e: git2::Error) -> Self {
        SyncError {
            kind: "other",
            message: e.message().to_string(),
            details: String::new(),
        }
    }
}

fn err(kind: &'static str, message: &str, details: &str) -> SyncError {
    SyncError {
        kind,
        message: message.to_string(),
        details: details.to_string(),
    }
}

/// Turns raw git stderr into a categorized, human-readable error.
pub fn classify(raw: &str, has_ssh_agent: bool) -> SyncError {
    let l = raw.to_lowercase();
    let has = |s: &str| l.contains(s);
    if has("permission denied (publickey") || has("no supported authentication methods") {
        let m = if has_ssh_agent {
            "The server rejected your SSH key. Check that your key is loaded (run `ssh-add -l`, or `ssh-add` to load it) and that its public key is added to your account on the server."
        } else {
            "No SSH agent was found, so no SSH key could be offered. Start one (`eval $(ssh-agent)`), load your key with `ssh-add`, then try again."
        };
        return err("auth_ssh", m, raw);
    }
    if has("host key verification failed") {
        return err("auth_ssh", "The server's SSH host key is not trusted yet. Connect once from a terminal (for example `ssh -T git@github.com`) and accept the key, then try again.", raw);
    }
    if has("could not read username")
        || has("could not read password")
        || has("authentication failed")
        || has("terminal prompts disabled")
        || has("invalid username or password")
        || has("returned error: 401")
        || has("returned error: 403")
    {
        return err("auth_https", "Authentication failed. Knife can't ask for a password here, so set up a git credential helper (for example `git config --global credential.helper libsecret`) and sign in once, or use a personal access token.", raw);
    }
    if has("could not resolve host")
        || has("network is unreachable")
        || has("connection timed out")
        || has("connection refused")
        || has("operation timed out")
        || has("unable to connect")
    {
        return err(
            "network",
            "Could not reach the remote. Check your internet connection and the remote URL.",
            raw,
        );
    }
    if has("divergent branches") || has("not possible to fast-forward") {
        return err("diverged", "Your branch and its upstream have both changed (diverged), so it can't simply fast-forward. Use the arrow next to Pull to choose “merge” or “rebase”.", raw);
    }
    if has("stale info") {
        return err("rejected", "The remote branch changed since you last fetched, so the force push was refused. Fetch, then review the new commits.", raw);
    }
    if has("non-fast-forward")
        || has("[rejected]")
        || has("fetch first")
        || has("failed to push some refs")
    {
        return err("rejected", "The remote has commits you don't have yet, so the push was rejected. Pull first, then push again.", raw);
    }
    if has("conflict") || has("would be overwritten") || has("commit your changes or stash") {
        return err("conflict", "The pull stopped because it conflicts with your local changes. Commit or stash your changes first, or resolve the conflicts it reports.", raw);
    }
    if has("repository not found")
        || has("does not appear to be a git repository")
        || has("not found")
    {
        return err("network", "The remote repository was not found. Check the remote URL and that you have access to it.", raw);
    }
    // the line that says what went wrong is usually the last "fatal:"/"error:" one, not the first
    let lines = || raw.lines().map(str::trim).filter(|l| !l.is_empty());
    let headline = lines()
        .rev()
        .find(|l| l.starts_with("fatal:") || l.starts_with("error:"))
        .or_else(|| lines().next())
        .unwrap_or("git failed");
    let headline = headline
        .strip_prefix("fatal: ")
        .or_else(|| headline.strip_prefix("error: "))
        .unwrap_or(headline);
    err("other", headline, raw)
}

fn has_ssh_agent() -> bool {
    std::env::var_os("SSH_AUTH_SOCK").is_some_and(|v| !v.is_empty())
}

/// How a pull integrates the remote changes.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PullOptions {
    /// "default" (respect the user's git config, else merge), "merge", "rebase" or "ff-only".
    pub mode: Option<String>,
    /// Stash local changes first and re-apply them afterwards.
    pub autostash: bool,
}

pub fn sync(
    repo: &Path,
    op: SyncOp,
    force: bool,
    on_line: impl FnMut(&str),
) -> Result<(), SyncError> {
    sync_with(repo, op, force, &PullOptions::default(), on_line)
}

/// Fetch, pull or push the current branch, streaming progress lines to `on_line`.
/// Uses the user's git binary so SSH keys, credential helpers and hooks all apply.
pub fn sync_with(
    repo: &Path,
    op: SyncOp,
    force: bool,
    pull: &PullOptions,
    on_line: impl FnMut(&str),
) -> Result<(), SyncError> {
    let remote = crate::git::default_remote(repo)?;
    let Some(remote) = remote else {
        return Err(err(
            "no_remote",
            "This repository has no remote. Add one with `git remote add origin <url>` first.",
            "",
        ));
    };
    let summary = crate::git::summary(repo)?;
    let args: Vec<String> = match op {
        SyncOp::Fetch => ["fetch", "--all", "--prune", "--progress"]
            .map(String::from)
            .into(),
        SyncOp::Pull => {
            if summary.branch.is_none() || !summary.has_upstream {
                return Err(err(
                    "no_upstream",
                    "This branch isn't tracking a remote branch yet. Push it first to publish it.",
                    "",
                ));
            }
            let cfg = git2::Repository::open(repo)?.config()?;
            let mut a: Vec<String> = ["pull", "--progress"].map(String::from).into();
            match pull.mode.as_deref() {
                Some("merge") => a.push("--no-rebase".into()),
                Some("rebase") => a.push("--rebase".into()),
                Some("ff-only") => a.push("--ff-only".into()),
                _ => {
                    if cfg.get_string("pull.rebase").is_err() && cfg.get_string("pull.ff").is_err()
                    {
                        a.push("--no-rebase".into()); // merge by default, like most Git GUIs
                    }
                }
            }
            if pull.autostash {
                a.push("--autostash".into());
            }
            a
        }
        SyncOp::Push => {
            let mut a: Vec<String> = ["push", "--progress"].map(String::from).into();
            if force {
                a.push("--force-with-lease".into());
            }
            if !summary.has_upstream {
                if summary.branch.is_none() {
                    return Err(err(
                        "other",
                        "Can't push a detached HEAD. Check out a branch first.",
                        "",
                    ));
                }
                a.extend(["-u".into(), remote, "HEAD".into()]);
            }
            a
        }
    };
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match run_streaming(repo, &refs, on_line) {
        Ok(_) => Ok(()),
        Err(CliError::Failed(raw)) => Err(classify(&raw, has_ssh_agent())),
        Err(e) => Err(err("other", &e.to_string(), "")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::work_status;

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), &["init", "-q"]).unwrap();
        run(dir.path(), &["config", "user.name", "Test"]).unwrap();
        run(dir.path(), &["config", "user.email", "t@t"]).unwrap();
        run(dir.path(), &["config", "commit.gpgsign", "false"]).unwrap();
        dir
    }

    fn write(dir: &tempfile::TempDir, name: &str, body: &str) {
        std::fs::write(dir.path().join(name), body).unwrap();
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn rejects_bad_git_command() {
        let dir = repo();
        assert!(run(dir.path(), &["bogus-subcommand"]).is_err());
    }

    #[test]
    fn stage_unstage_before_first_commit() {
        let dir = repo();
        write(&dir, "a.txt", "a");
        assert_eq!(
            work_status(dir.path()).unwrap().unstaged[0].status,
            "untracked"
        );
        stage(dir.path(), &s(&["a.txt"])).unwrap();
        let st = work_status(dir.path()).unwrap();
        assert_eq!(st.staged.len(), 1);
        assert!(st.unstaged.is_empty());
        unstage(dir.path(), &s(&["a.txt"])).unwrap();
        let st = work_status(dir.path()).unwrap();
        assert!(st.staged.is_empty());
        assert_eq!(st.unstaged[0].status, "untracked");
    }

    #[test]
    fn commit_amend_and_empty_message() {
        let dir = repo();
        write(&dir, "a.txt", "a");
        stage(dir.path(), &s(&["a.txt"])).unwrap();
        assert!(commit(dir.path(), "  ", false).is_err());
        commit(dir.path(), "first", false).unwrap();
        assert_eq!(crate::git::head_message(dir.path()).unwrap(), "first");

        write(&dir, "b.txt", "b");
        stage(dir.path(), &s(&["b.txt"])).unwrap();
        commit(dir.path(), "", true).unwrap(); // keep message
        assert_eq!(crate::git::head_message(dir.path()).unwrap(), "first");
        assert_eq!(crate::git::log_ids(dir.path()).unwrap().len(), 1);

        commit(dir.path(), "renamed", true).unwrap();
        assert_eq!(crate::git::head_message(dir.path()).unwrap(), "renamed");
        assert!(work_status(dir.path()).unwrap().staged.is_empty());
    }

    #[test]
    fn discard_tracked_and_untracked_only_named_files() {
        let dir = repo();
        write(&dir, "keep.txt", "k1");
        write(&dir, "a.txt", "one");
        stage(dir.path(), &s(&["keep.txt", "a.txt"])).unwrap();
        commit(dir.path(), "init", false).unwrap();

        write(&dir, "a.txt", "changed");
        write(&dir, "keep.txt", "also changed");
        write(&dir, "new.txt", "untracked");
        discard(dir.path(), &s(&["a.txt", "new.txt"])).unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "one"
        );
        assert!(!dir.path().join("new.txt").exists());
        // not named => untouched
        assert_eq!(
            std::fs::read_to_string(dir.path().join("keep.txt")).unwrap(),
            "also changed"
        );
    }

    #[test]
    fn staged_deletion_and_odd_filenames() {
        let dir = repo();
        write(&dir, "-weird name.txt", "x");
        write(&dir, "gone.txt", "y");
        stage(dir.path(), &s(&["-weird name.txt", "gone.txt"])).unwrap();
        commit(dir.path(), "init", false).unwrap();
        std::fs::remove_file(dir.path().join("gone.txt")).unwrap();
        let st = work_status(dir.path()).unwrap();
        assert_eq!(st.unstaged[0].status, "deleted");
        stage(dir.path(), &s(&["gone.txt"])).unwrap();
        let st = work_status(dir.path()).unwrap();
        assert_eq!(st.staged[0].status, "deleted");
        write(&dir, "-weird name.txt", "z");
        stage(dir.path(), &s(&["-weird name.txt"])).unwrap();
        assert_eq!(work_status(dir.path()).unwrap().staged.len(), 2);
    }

    fn commit_all(dir: &tempfile::TempDir, file: &str, body: &str, msg: &str) {
        write(dir, file, body);
        stage(dir.path(), &[file.to_string()]).unwrap();
        commit(dir.path(), msg, false).unwrap();
    }

    /// bare "server" + two clones, all local.
    fn remote_pair() -> (tempfile::TempDir, tempfile::TempDir) {
        let server = tempfile::tempdir().unwrap();
        run(server.path(), &["init", "-q", "--bare", "-b", "main"]).unwrap();
        let url = server.path().to_string_lossy().into_owned();
        let a = repo();
        run(a.path(), &["checkout", "-q", "-b", "main"]).unwrap();
        run(a.path(), &["remote", "add", "origin", &url]).unwrap();
        commit_all(&a, "f.txt", "1\n", "first");
        (server, a)
    }

    #[test]
    fn push_publishes_then_pull_and_fetch_sync_second_clone() {
        let (server, a) = remote_pair();
        let mut lines = Vec::new();
        // first push has no upstream: must set it automatically
        sync(a.path(), SyncOp::Push, false, |l| lines.push(l.to_string())).unwrap();
        assert!(crate::git::summary(a.path()).unwrap().has_upstream);

        let b = tempfile::tempdir().unwrap();
        let url = server.path().to_string_lossy().into_owned();
        run(b.path(), &["clone", "-q", &url, "."]).unwrap();
        run(b.path(), &["config", "user.name", "T"]).unwrap();
        run(b.path(), &["config", "user.email", "t@t"]).unwrap();

        commit_all(&a, "f.txt", "2\n", "second");
        assert_eq!(crate::git::summary(a.path()).unwrap().ahead, 1);
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        assert_eq!(crate::git::summary(a.path()).unwrap().ahead, 0);

        sync(b.path(), SyncOp::Fetch, false, |_| {}).unwrap();
        assert_eq!(crate::git::summary(b.path()).unwrap().behind, 1);
        sync(b.path(), SyncOp::Pull, false, |_| {}).unwrap();
        let s = crate::git::summary(b.path()).unwrap();
        assert_eq!((s.ahead, s.behind), (0, 0));
        assert_eq!(
            std::fs::read_to_string(b.path().join("f.txt")).unwrap(),
            "2\n"
        );
    }

    #[test]
    fn rejected_push_is_explained_and_force_with_lease_works() {
        let (server, a) = remote_pair();
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        let b = tempfile::tempdir().unwrap();
        let url = server.path().to_string_lossy().into_owned();
        run(b.path(), &["clone", "-q", &url, "."]).unwrap();
        run(b.path(), &["config", "user.name", "T"]).unwrap();
        run(b.path(), &["config", "user.email", "t@t"]).unwrap();
        commit_all(&b, "f.txt", "from b\n", "b commit");
        sync(b.path(), SyncOp::Push, false, |_| {}).unwrap();

        commit_all(&a, "g.txt", "from a\n", "a commit"); // a is now behind+ahead
        let e = sync(a.path(), SyncOp::Push, false, |_| {}).unwrap_err();
        assert_eq!(e.kind, "rejected");
        assert!(e.message.contains("Pull first"));
        assert!(!e.details.is_empty());

        // a has not fetched, so its lease is stale: forced push is still refused
        let e = sync(a.path(), SyncOp::Push, true, |_| {}).unwrap_err();
        assert_eq!(e.kind, "rejected");
    }

    #[test]
    fn missing_remote_and_upstream_have_clear_errors() {
        let dir = repo();
        commit_all(&dir, "a", "a", "init");
        assert_eq!(
            sync(dir.path(), SyncOp::Fetch, false, |_| {})
                .unwrap_err()
                .kind,
            "no_remote"
        );
        run(
            dir.path(),
            &["remote", "add", "origin", "/nonexistent/path"],
        )
        .unwrap();
        assert_eq!(
            sync(dir.path(), SyncOp::Pull, false, |_| {})
                .unwrap_err()
                .kind,
            "no_upstream"
        );
    }

    #[test]
    fn classifies_common_failures() {
        let k = |raw: &str, agent| classify(raw, agent);
        let ssh = "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.";
        assert_eq!(k(ssh, true).kind, "auth_ssh");
        assert!(k(ssh, false).message.contains("No SSH agent"));
        assert!(k(ssh, true).message.contains("rejected your SSH key"));
        assert_eq!(k("Host key verification failed.", true).kind, "auth_ssh");
        assert_eq!(
            k("fatal: could not read Username for 'https://github.com': terminal prompts disabled", true).kind,
            "auth_https"
        );
        assert_eq!(
            k(
                "fatal: unable to access 'x': Could not resolve host: github.com",
                true
            )
            .kind,
            "network"
        );
        assert_eq!(
            k(" ! [rejected] main -> main (non-fast-forward)", true).kind,
            "rejected"
        );
        assert_eq!(
            k(
                "fatal: Need to specify how to reconcile divergent branches.",
                true
            )
            .kind,
            "diverged"
        );
        assert_eq!(
            k(
                "error: Your local changes would be overwritten by merge",
                true
            )
            .kind,
            "conflict"
        );
        let other = k("something weird\nmore", true);
        assert_eq!(
            (other.kind, other.message.as_str()),
            ("other", "something weird")
        );
        assert_eq!(other.details, "something weird\nmore");
        // noisy progress first, the real reason last
        let noisy = k(
            "Cloning into 'x'...\nsome chatter\nfatal: transport 'file' not allowed\nFailed to clone",
            true,
        );
        assert_eq!(noisy.message, "transport 'file' not allowed");
    }

    #[test]
    fn branch_create_checkout_delete() {
        let dir = repo();
        commit_all(&dir, "a", "a", "init");
        let cur = || crate::git::summary(dir.path()).unwrap().branch.unwrap();
        let main = cur();

        create_branch(dir.path(), "feature/x", None, false).unwrap();
        assert_eq!(cur(), main, "create without switch stays put");
        checkout(dir.path(), "feature/x", false).unwrap();
        assert_eq!(cur(), "feature/x");

        // unmerged work on feature/x
        commit_all(&dir, "b", "b", "work");
        checkout(dir.path(), &main, false).unwrap();
        let e = delete_branch(dir.path(), "feature/x", false)
            .unwrap_err()
            .to_string();
        assert!(e.contains("not fully merged"), "{e}");
        assert!(crate::git::refs(dir.path())
            .unwrap()
            .local
            .iter()
            .any(|b| b.name == "feature/x"));
        let msg = delete_branch(dir.path(), "feature/x", true).unwrap();
        assert!(msg.contains("Deleted branch feature/x"), "{msg}");
        assert!(!crate::git::refs(dir.path())
            .unwrap()
            .local
            .iter()
            .any(|b| b.name == "feature/x"));

        // merged branch deletes without force; current branch cannot be deleted
        create_branch(dir.path(), "done", None, true).unwrap();
        checkout(dir.path(), &main, false).unwrap();
        delete_branch(dir.path(), "done", false).unwrap();
        assert!(delete_branch(dir.path(), &main, true).is_err());
    }

    #[test]
    fn branch_names_are_validated() {
        let dir = repo();
        commit_all(&dir, "a", "a", "init");
        for bad in ["", "-x", "--force", "a b", "a..b", "x.lock", "~oops"] {
            assert!(
                create_branch(dir.path(), bad, None, false).is_err(),
                "{bad:?}"
            );
        }
        assert!(checkout(dir.path(), "--detach", false).is_err());
        assert!(delete_branch(dir.path(), "-D", true).is_err());
        assert!(create_branch(dir.path(), "ok", Some("--orphan"), false).is_err());
    }

    #[test]
    fn checkout_conflict_message_and_remote_tracking() {
        let (server, a) = remote_pair();
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        create_branch(a.path(), "topic", None, true).unwrap();
        commit_all(&a, "f.txt", "topic version\n", "topic");
        run(a.path(), &["push", "-q", "origin", "topic"]).unwrap();

        let b = tempfile::tempdir().unwrap();
        let url = server.path().to_string_lossy().into_owned();
        run(b.path(), &["clone", "-q", &url, "."]).unwrap();
        run(b.path(), &["config", "user.name", "T"]).unwrap();
        run(b.path(), &["config", "user.email", "t@t"]).unwrap();
        // dirty file that conflicts with the topic branch's version
        write(&b, "f.txt", "local edit\n");
        let e = checkout(b.path(), "origin/topic", true)
            .unwrap_err()
            .to_string();
        assert!(e.contains("uncommitted changes"), "{e}");
        run(b.path(), &["checkout", "--", "f.txt"]).unwrap();
        checkout(b.path(), "origin/topic", true).unwrap();
        let s = crate::git::summary(b.path()).unwrap();
        assert_eq!(s.branch.as_deref(), Some("topic"));
        assert!(s.has_upstream, "remote checkout must track");
    }

    #[test]
    fn names_from_urls() {
        for (u, n) in [
            ("https://github.com/org/repo.git", "repo"),
            ("https://github.com/org/repo", "repo"),
            ("git@github.com:org/my-repo.git", "my-repo"),
            ("ssh://git@host:2222/srv/x.git/", "x"),
            ("/srv/git/local.git", "local"),
            ("  https://h/o/r.git  ", "r"),
        ] {
            assert_eq!(repo_name_from_url(u), n, "{u}");
        }
    }

    #[test]
    fn clone_local_repo_with_progress_and_guards() {
        let (server, a) = remote_pair();
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        let url = server.path().to_string_lossy().into_owned();
        let parent = tempfile::tempdir().unwrap();

        let mut lines = 0;
        let dest = clone(&url, parent.path(), "copy", |_| lines += 1).unwrap();
        assert_eq!(dest, parent.path().join("copy"));
        assert_eq!(std::fs::read_to_string(dest.join("f.txt")).unwrap(), "1\n");

        // destination exists and is not empty
        let e = clone(&url, parent.path(), "copy", |_| {}).unwrap_err();
        assert!(e.message.contains("already exists"), "{}", e.message);
        // empty existing dir is fine
        std::fs::create_dir(parent.path().join("empty")).unwrap();
        clone(&url, parent.path(), "empty", |_| {}).unwrap();
    }

    #[test]
    fn clone_rejects_unsafe_input() {
        let parent = tempfile::tempdir().unwrap();
        let p = parent.path();
        let bad_url = |u: &str| clone(u, p, "x", |_| {}).unwrap_err();
        assert!(bad_url("").message.contains("Enter"));
        assert!(bad_url("--upload-pack=touch /tmp/pwned")
            .message
            .contains("valid"));
        assert!(bad_url("-c core.sshCommand=evil").message.contains("valid"));
        assert!(bad_url("https://me:ghp_secret@github.com/o/r.git")
            .message
            .contains("password or token"));
        // user without password is fine to *validate* (fails later for other reasons)
        assert!(validate_clone_url("https://me@github.com/o/r.git").is_ok());
        assert!(validate_clone_url("git@github.com:o/r.git").is_ok());
        for name in ["", "-x", "a/b", "..", "."] {
            assert!(
                clone("/nonexistent", p, name, |_| {})
                    .unwrap_err()
                    .message
                    .contains("folder name"),
                "{name}"
            );
        }
        assert!(clone("/nonexistent", &p.join("missing"), "x", |_| {}).is_err());
        // a failing clone is reported, and leaves no half-made folder behind
        assert!(clone("/definitely/not/a/repo", p, "nope", |_| {}).is_err());
        assert!(!p.join("nope").exists());
    }

    // ---- partial staging -------------------------------------------------------------

    use crate::git::{work_file_diff, Selection};

    fn lines(n: usize) -> String {
        (1..=n).map(|i| format!("line {i}\n")).collect()
    }

    /// repo with a committed 30-line file `f.txt`
    fn file_repo() -> tempfile::TempDir {
        let dir = repo();
        commit_all(&dir, "f.txt", &lines(30), "init");
        dir
    }

    fn staged_diff(dir: &tempfile::TempDir) -> String {
        run(dir.path(), &["diff", "--cached", "-U0"]).unwrap()
    }
    fn unstaged_diff(dir: &tempfile::TempDir) -> String {
        run(dir.path(), &["diff", "-U0"]).unwrap()
    }
    fn changed(diff: &str) -> Vec<String> {
        diff.lines()
            .filter(|l| {
                (l.starts_with('+') || l.starts_with('-'))
                    && !l.starts_with("+++")
                    && !l.starts_with("---")
            })
            .map(String::from)
            .collect()
    }

    /// edits line 2 (hunk 0) and line 28 (hunk 1), far enough apart to be separate hunks
    fn two_hunk_edit(dir: &tempfile::TempDir) {
        let edited = lines(30)
            .replace("line 2\n", "line 2 CHANGED\n")
            .replace("line 28\n", "line 28 CHANGED\n");
        write(dir, "f.txt", &edited);
    }

    #[test]
    fn stage_a_single_hunk() {
        let dir = file_repo();
        two_hunk_edit(&dir);
        let d = work_file_diff(dir.path(), "f.txt", false).unwrap();
        assert_eq!(d.hunks.len(), 2);
        apply_selection(
            dir.path(),
            "f.txt",
            false,
            &[Selection {
                hunk_id: d.hunks[1].id.clone(),
                lines: None,
            }],
        )
        .unwrap();
        assert_eq!(
            changed(&staged_diff(&dir)),
            vec!["-line 28", "+line 28 CHANGED"]
        );
        assert_eq!(
            changed(&unstaged_diff(&dir)),
            vec!["-line 2", "+line 2 CHANGED"],
            "other hunk untouched"
        );
    }

    #[test]
    fn stage_selected_lines_only() {
        let dir = file_repo();
        // one hunk: replace line 5, 6 and add a line after 7
        let edited = lines(30)
            .replace("line 5\n", "line 5 NEW\n")
            .replace("line 6\n", "line 6 NEW\n")
            .replace("line 7\n", "line 7\nadded line\n");
        write(&dir, "f.txt", &edited);
        let d = work_file_diff(dir.path(), "f.txt", false).unwrap();
        assert_eq!(d.hunks.len(), 1);
        let h = &d.hunks[0];
        let idx = |kind: char, text: &str| {
            h.lines
                .iter()
                .position(|l| l.kind == kind && l.text == text)
                .unwrap()
        };
        // stage only: "-line 5" / "+line 5 NEW", leaving line 6's change and the added line unstaged
        let pick = vec![idx('-', "line 5"), idx('+', "line 5 NEW")];
        apply_selection(
            dir.path(),
            "f.txt",
            false,
            &[Selection {
                hunk_id: h.id.clone(),
                lines: Some(pick),
            }],
        )
        .unwrap();
        assert_eq!(changed(&staged_diff(&dir)), vec!["-line 5", "+line 5 NEW"]);
        let rest = changed(&unstaged_diff(&dir));
        assert!(
            rest.contains(&"-line 6".to_string()) && rest.contains(&"+added line".to_string()),
            "{rest:?}"
        );
        assert!(!rest.contains(&"+line 5 NEW".to_string()));
        // working tree itself is untouched by staging
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            edited
        );
    }

    #[test]
    fn stage_only_an_added_line_keeps_removals_unstaged() {
        let dir = file_repo();
        write(
            &dir,
            "f.txt",
            &lines(30).replace("line 10\n", "line 10 X\n"),
        );
        let d = work_file_diff(dir.path(), "f.txt", false).unwrap();
        let h = &d.hunks[0];
        let plus = h.lines.iter().position(|l| l.kind == '+').unwrap();
        apply_selection(
            dir.path(),
            "f.txt",
            false,
            &[Selection {
                hunk_id: h.id.clone(),
                lines: Some(vec![plus]),
            }],
        )
        .unwrap();
        // index now has BOTH the old and new line 10 (the removal was not staged)
        let idx = run(dir.path(), &["show", ":f.txt"]).unwrap();
        assert!(idx.contains("line 10\n") && idx.contains("line 10 X\n"));
    }

    #[test]
    fn unstage_a_hunk_and_unstage_selected_lines() {
        let dir = file_repo();
        two_hunk_edit(&dir);
        stage(dir.path(), &["f.txt".to_string()]).unwrap();
        let d = work_file_diff(dir.path(), "f.txt", true).unwrap();
        assert_eq!(d.hunks.len(), 2);
        // unstage hunk 0 entirely
        apply_selection(
            dir.path(),
            "f.txt",
            true,
            &[Selection {
                hunk_id: d.hunks[0].id.clone(),
                lines: None,
            }],
        )
        .unwrap();
        assert_eq!(
            changed(&staged_diff(&dir)),
            vec!["-line 28", "+line 28 CHANGED"]
        );
        // unstage only the "+" line of the remaining hunk: the "-line 28" removal stays staged
        let d = work_file_diff(dir.path(), "f.txt", true).unwrap();
        let h = &d.hunks[0];
        let plus = h.lines.iter().position(|l| l.kind == '+').unwrap();
        apply_selection(
            dir.path(),
            "f.txt",
            true,
            &[Selection {
                hunk_id: h.id.clone(),
                lines: Some(vec![plus]),
            }],
        )
        .unwrap();
        assert_eq!(changed(&staged_diff(&dir)), vec!["-line 28"]);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt"))
                .unwrap()
                .matches("CHANGED")
                .count(),
            2,
            "worktree untouched"
        );
    }

    #[test]
    fn multiple_hunks_in_one_request_get_consistent_offsets() {
        let dir = file_repo();
        // hunk 0 adds 3 lines, hunk 1 edits a later line: positions shift after hunk 0
        let edited = lines(30)
            .replace("line 3\n", "line 3\nA\nB\nC\n")
            .replace("line 25\n", "line 25 EDIT\n");
        write(&dir, "f.txt", &edited);
        let d = work_file_diff(dir.path(), "f.txt", false).unwrap();
        assert_eq!(d.hunks.len(), 2);
        let sels: Vec<Selection> = d
            .hunks
            .iter()
            .map(|h| Selection {
                hunk_id: h.id.clone(),
                lines: None,
            })
            .collect();
        apply_selection(dir.path(), "f.txt", false, &sels).unwrap();
        assert!(unstaged_diff(&dir).is_empty(), "everything staged");
        let idx = run(dir.path(), &["show", ":f.txt"]).unwrap();
        assert_eq!(idx, edited);
    }

    #[test]
    fn stale_selection_is_refused() {
        let dir = file_repo();
        two_hunk_edit(&dir);
        let d = work_file_diff(dir.path(), "f.txt", false).unwrap();
        let id = d.hunks[0].id.clone();
        // the file changes after the user looked at it
        write(
            &dir,
            "f.txt",
            &lines(30).replace("line 2\n", "line 2 DIFFERENT\n"),
        );
        let e = apply_selection(
            dir.path(),
            "f.txt",
            false,
            &[Selection {
                hunk_id: id,
                lines: None,
            }],
        )
        .unwrap_err();
        assert!(e.to_string().contains("changed since"), "{e}");
        assert!(staged_diff(&dir).is_empty(), "nothing was staged");
        assert!(
            apply_selection(dir.path(), "f.txt", false, &[]).is_err(),
            "empty selection"
        );
    }

    #[test]
    fn no_newline_at_eof_whole_hunk_ok_partial_refused() {
        let dir = repo();
        commit_all(&dir, "n.txt", "a\nb\nc", "init"); // no trailing newline
        write(&dir, "n.txt", "a\nb\nc changed");
        let d = work_file_diff(dir.path(), "n.txt", false).unwrap();
        let h = &d.hunks[0];
        assert!(
            h.lines.iter().any(|l| l.kind == '\\'),
            "marker lines are kept"
        );
        let first_change = h.lines.iter().position(|l| l.kind == '-').unwrap();
        let e = apply_selection(
            dir.path(),
            "n.txt",
            false,
            &[Selection {
                hunk_id: h.id.clone(),
                lines: Some(vec![first_change]),
            }],
        )
        .unwrap_err();
        assert!(e.to_string().contains("trailing newline"), "{e}");
        apply_selection(
            dir.path(),
            "n.txt",
            false,
            &[Selection {
                hunk_id: h.id.clone(),
                lines: None,
            }],
        )
        .unwrap();
        assert!(unstaged_diff(&dir).is_empty());
    }

    // ---- stash -----------------------------------------------------------------------

    #[test]
    fn stash_push_apply_pop_drop() {
        let dir = file_repo();
        write(&dir, "f.txt", "edited\n");
        write(&dir, "new.txt", "untracked\n");

        stash_push(dir.path(), "my stash", false).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            lines(30),
            "tracked change stashed away"
        );
        assert!(
            dir.path().join("new.txt").exists(),
            "untracked file left alone without -u"
        );
        let stashes = crate::git::refs(dir.path()).unwrap().stashes;
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].message.contains("my stash"));
        assert_eq!(stashes[0].id.len(), 40);

        // apply keeps the stash, pop removes it
        stash_apply(dir.path(), 0, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "edited\n"
        );
        assert_eq!(crate::git::refs(dir.path()).unwrap().stashes.len(), 1);
        run(dir.path(), &["checkout", "--", "f.txt"]).unwrap();
        stash_apply(dir.path(), 0, true).unwrap();
        assert!(crate::git::refs(dir.path()).unwrap().stashes.is_empty());

        stash_push(dir.path(), "", false).unwrap();
        stash_drop(dir.path(), 0).unwrap();
        assert!(crate::git::refs(dir.path()).unwrap().stashes.is_empty());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            lines(30),
            "drop does not touch files"
        );
    }

    #[test]
    fn stash_untracked_and_nothing_to_stash() {
        let dir = file_repo();
        let e = stash_push(dir.path(), "", false).unwrap_err().to_string();
        assert!(e.contains("Nothing to stash"), "{e}");
        write(&dir, "new.txt", "u\n");
        assert!(
            stash_push(dir.path(), "", false).is_err(),
            "untracked only, without -u"
        );
        stash_push(dir.path(), "with untracked", true).unwrap();
        assert!(!dir.path().join("new.txt").exists());
        stash_apply(dir.path(), 0, true).unwrap();
        assert!(dir.path().join("new.txt").exists());
    }

    #[test]
    fn stash_pop_with_conflict_keeps_the_stash_and_explains() {
        let dir = file_repo();
        write(
            &dir,
            "f.txt",
            &lines(30).replace("line 3\n", "stash side\n"),
        );
        stash_push(dir.path(), "", false).unwrap();
        // conflicting committed change to the same line
        commit_all(
            &dir,
            "f.txt",
            &lines(30).replace("line 3\n", "head side\n"),
            "conflicting",
        );
        let e = stash_apply(dir.path(), 0, true).unwrap_err().to_string();
        assert!(e.contains("conflicts") && e.contains("kept"), "{e}");
        assert_eq!(
            crate::git::refs(dir.path()).unwrap().stashes.len(),
            1,
            "stash must survive a failed pop"
        );
        assert_eq!(
            crate::git::work_status(dir.path()).unwrap().conflicted,
            vec!["f.txt"]
        );
    }

    #[test]
    fn stash_overlap_with_local_changes_is_explained() {
        let dir = file_repo();
        write(&dir, "f.txt", "stashed\n");
        stash_push(dir.path(), "", false).unwrap();
        write(&dir, "f.txt", "local edit\n");
        let e = stash_apply(dir.path(), 0, false).unwrap_err().to_string();
        assert!(e.contains("overlap") || e.contains("conflicts"), "{e}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "local edit\n",
            "local edit untouched"
        );
    }

    // ---- merge & rebase --------------------------------------------------------------

    fn branch_name(dir: &tempfile::TempDir) -> String {
        crate::git::summary(dir.path()).unwrap().branch.unwrap()
    }
    fn parents_of_head(dir: &tempfile::TempDir) -> usize {
        run(dir.path(), &["rev-list", "--parents", "-n", "1", "HEAD"])
            .unwrap()
            .split_whitespace()
            .count()
            - 1
    }
    fn commit_count(dir: &tempfile::TempDir) -> usize {
        crate::git::log_ids(dir.path()).unwrap().len()
    }

    /// base commit; `feature` has one commit on other.txt, main has one on main.txt
    fn diverged() -> (tempfile::TempDir, String) {
        let dir = repo();
        commit_all(&dir, "f.txt", "base\n", "base");
        let main = branch_name(&dir);
        create_branch(dir.path(), "feature", None, true).unwrap();
        commit_all(&dir, "other.txt", "feature work\n", "feature commit");
        checkout(dir.path(), &main, false).unwrap();
        commit_all(&dir, "main.txt", "main work\n", "main commit");
        (dir, main)
    }

    /// both sides edit f.txt line 1 differently
    fn conflicting() -> (tempfile::TempDir, String) {
        let dir = repo();
        commit_all(&dir, "f.txt", "base\n", "base");
        let main = branch_name(&dir);
        create_branch(dir.path(), "feature", None, true).unwrap();
        commit_all(&dir, "f.txt", "feature side\n", "feature edit");
        checkout(dir.path(), &main, false).unwrap();
        commit_all(&dir, "f.txt", "main side\n", "main edit");
        (dir, main)
    }

    #[test]
    fn merge_creates_merge_commit_and_reports_up_to_date() {
        let (dir, _) = diverged();
        let r = merge(dir.path(), "feature", false).unwrap();
        assert_eq!(r.outcome, "done");
        assert_eq!(parents_of_head(&dir), 2);
        assert!(dir.path().join("other.txt").exists() && dir.path().join("main.txt").exists());
        assert_eq!(
            merge(dir.path(), "feature", false).unwrap().outcome,
            "up_to_date"
        );
    }

    #[test]
    fn fast_forward_and_no_ff() {
        let dir = repo();
        commit_all(&dir, "f.txt", "base\n", "base");
        let main = branch_name(&dir);
        create_branch(dir.path(), "ahead", None, true).unwrap();
        commit_all(&dir, "g.txt", "g\n", "ahead commit");
        checkout(dir.path(), &main, false).unwrap();
        let r = merge(dir.path(), "ahead", false).unwrap();
        assert!(r.message.contains("Fast-forwarded"), "{}", r.message);
        assert_eq!(parents_of_head(&dir), 1);

        checkout(dir.path(), &main, false).unwrap();
        run(dir.path(), &["reset", "-q", "--hard", "HEAD~1"]).unwrap();
        merge(dir.path(), "ahead", true).unwrap();
        assert_eq!(parents_of_head(&dir), 2, "--no-ff forces a merge commit");
    }

    #[test]
    fn merge_conflict_resolve_continue() {
        let (dir, _) = conflicting();
        let r = merge(dir.path(), "feature", false).unwrap();
        assert_eq!(r.outcome, "conflicts");
        let st = crate::git::work_status(dir.path()).unwrap();
        assert_eq!(st.conflicted, vec!["f.txt"]);
        assert_eq!(st.operation, Some("merge"));
        // continuing with unresolved conflicts is refused with a clear message
        let e = operation_continue(dir.path()).unwrap_err().to_string();
        assert!(e.contains("unresolved"), "{e}");

        resolve_side(dir.path(), "f.txt", "theirs").unwrap(); // theirs = the merged-in branch
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "feature side\n"
        );
        assert!(crate::git::work_status(dir.path())
            .unwrap()
            .conflicted
            .is_empty());
        let r = operation_continue(dir.path()).unwrap();
        assert_eq!(r.outcome, "done");
        assert_eq!(crate::git::work_status(dir.path()).unwrap().operation, None);
        assert_eq!(parents_of_head(&dir), 2);
    }

    #[test]
    fn merge_abort_restores_everything() {
        let (dir, _) = conflicting();
        let before = run(dir.path(), &["rev-parse", "HEAD"]).unwrap();
        merge(dir.path(), "feature", false).unwrap();
        operation_abort(dir.path()).unwrap();
        assert_eq!(run(dir.path(), &["rev-parse", "HEAD"]).unwrap(), before);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "main side\n"
        );
        assert_eq!(crate::git::work_status(dir.path()).unwrap().operation, None);
        assert!(operation_abort(dir.path()).is_err(), "nothing to abort now");
    }

    #[test]
    fn mark_resolved_after_manual_edit() {
        let (dir, _) = conflicting();
        merge(dir.path(), "feature", false).unwrap();
        write(&dir, "f.txt", "hand merged\n");
        mark_resolved(dir.path(), "f.txt").unwrap();
        operation_continue(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "hand merged\n"
        );
    }

    #[test]
    fn rebase_makes_history_linear() {
        let (dir, main) = diverged();
        checkout(dir.path(), "feature", false).unwrap();
        let r = rebase(dir.path(), &main).unwrap();
        assert_eq!(r.outcome, "done");
        assert_eq!(parents_of_head(&dir), 1, "no merge commit");
        assert_eq!(
            commit_count(&dir),
            3,
            "base, main, and the replayed feature commit; the old one is unreachable"
        );
        assert!(dir.path().join("main.txt").exists() && dir.path().join("other.txt").exists());
        assert_eq!(rebase(dir.path(), &main).unwrap().outcome, "up_to_date");
    }

    #[test]
    fn rebase_conflict_sides_are_swapped_and_continue_works() {
        let (dir, main) = conflicting();
        checkout(dir.path(), "feature", false).unwrap();
        let r = rebase(dir.path(), &main).unwrap();
        assert_eq!(r.outcome, "conflicts");
        assert_eq!(
            crate::git::work_status(dir.path()).unwrap().operation,
            Some("rebase")
        );
        // during a rebase "ours" is the branch being rebased onto, "theirs" is the replayed commit
        resolve_side(dir.path(), "f.txt", "ours").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "main side\n"
        );
        // the replayed commit becomes empty; continuing is fine (git drops it) or needs skip
        let r = operation_continue(dir.path());
        if r.is_err() {
            operation_skip(dir.path()).unwrap();
        }
        assert_eq!(crate::git::work_status(dir.path()).unwrap().operation, None);
    }

    #[test]
    fn rebase_abort_and_skip() {
        let (dir, main) = conflicting();
        checkout(dir.path(), "feature", false).unwrap();
        let tip = run(dir.path(), &["rev-parse", "HEAD"]).unwrap();
        rebase(dir.path(), &main).unwrap();
        operation_abort(dir.path()).unwrap();
        assert_eq!(run(dir.path(), &["rev-parse", "HEAD"]).unwrap(), tip);
        assert_eq!(branch_name(&dir), "feature");

        rebase(dir.path(), &main).unwrap();
        operation_skip(dir.path()).unwrap();
        assert_eq!(crate::git::work_status(dir.path()).unwrap().operation, None);
        assert!(operation_skip(dir.path()).is_err(), "no rebase in progress");
    }

    #[test]
    fn dirty_tree_and_bad_branch_get_clear_errors() {
        let (dir, main) = diverged();
        checkout(dir.path(), "feature", false).unwrap();
        write(&dir, "other.txt", "dirty\n"); // uncommitted change to a tracked file
        let e = rebase(dir.path(), &main).unwrap_err().to_string();
        assert!(e.contains("uncommitted changes"), "{e}");
        assert!(merge(dir.path(), "no-such-branch", false)
            .unwrap_err()
            .to_string()
            .contains("doesn't exist"));
        assert!(
            merge(dir.path(), "--abort", false).is_err(),
            "option-like names are rejected"
        );
        assert!(rebase(dir.path(), "-i").is_err());
    }

    #[test]
    fn delete_modify_conflict_resolves_by_removing() {
        let dir = repo();
        commit_all(&dir, "f.txt", "base\n", "base");
        let main = branch_name(&dir);
        create_branch(dir.path(), "feature", None, true).unwrap();
        run(dir.path(), &["rm", "-q", "f.txt"]).unwrap();
        commit(dir.path(), "delete f", false).unwrap();
        checkout(dir.path(), &main, false).unwrap();
        commit_all(&dir, "f.txt", "modified on main\n", "modify f");
        assert_eq!(
            merge(dir.path(), "feature", false).unwrap().outcome,
            "conflicts"
        );
        resolve_side(dir.path(), "f.txt", "theirs").unwrap(); // their side deleted it
        assert!(!dir.path().join("f.txt").exists());
        operation_continue(dir.path()).unwrap();
    }

    // ---- merge options & pull options -------------------------------------------------

    #[test]
    fn merge_options_ff_only_squash_no_commit_and_message() {
        // ff-only refuses when diverged, with advice
        let (dir, _) = diverged();
        let o = MergeOptions {
            ff: "only".into(),
            ..Default::default()
        };
        let e = merge_with(dir.path(), "feature", &o)
            .unwrap_err()
            .to_string();
        assert!(e.contains("Can't fast-forward"), "{e}");
        assert_eq!(parents_of_head(&dir), 1, "nothing changed");

        // squash: changes staged, no commit, no MERGE_HEAD
        let before = commit_count(&dir);
        let o = MergeOptions {
            squash: true,
            ..Default::default()
        };
        let r = merge_with(dir.path(), "feature", &o).unwrap();
        assert!(r.message.contains("staged"), "{}", r.message);
        assert_eq!(commit_count(&dir), before, "squash does not commit");
        let st = crate::git::work_status(dir.path()).unwrap();
        assert!(st.staged.iter().any(|f| f.path == "other.txt"));
        assert_eq!(st.operation, None);
        run(dir.path(), &["reset", "-q", "--hard", "HEAD"]).unwrap();

        // no-commit: merge prepared (MERGE_HEAD present), continue finishes it
        let o = MergeOptions {
            no_commit: true,
            ..Default::default()
        };
        merge_with(dir.path(), "feature", &o).unwrap();
        assert_eq!(
            crate::git::work_status(dir.path()).unwrap().operation,
            Some("merge")
        );
        assert_eq!(commit_count(&dir), before);
        operation_continue(dir.path()).unwrap();
        assert_eq!(parents_of_head(&dir), 2);

        // custom message on a real merge commit
        let (dir, _) = diverged();
        let o = MergeOptions {
            message: Some("Bring in the feature".into()),
            ..Default::default()
        };
        merge_with(dir.path(), "feature", &o).unwrap();
        assert_eq!(
            crate::git::head_message(dir.path()).unwrap(),
            "Bring in the feature"
        );
    }

    #[test]
    fn no_commit_still_merges_when_fast_forward_would_be_possible() {
        let dir = repo();
        commit_all(&dir, "f.txt", "base\n", "base");
        let main = branch_name(&dir);
        create_branch(dir.path(), "ahead", None, true).unwrap();
        commit_all(&dir, "g.txt", "g\n", "ahead commit");
        checkout(dir.path(), &main, false).unwrap();
        let o = MergeOptions {
            no_commit: true,
            ..Default::default()
        };
        merge_with(dir.path(), "ahead", &o).unwrap();
        // staged and waiting, HEAD untouched: not silently fast-forwarded
        assert_eq!(
            crate::git::work_status(dir.path()).unwrap().operation,
            Some("merge")
        );
        assert!(!crate::git::log_ids(dir.path()).unwrap().is_empty());
        assert_eq!(parents_of_head(&dir), 0, "HEAD is still the base commit");
    }

    #[test]
    fn pull_modes_rebase_ffonly_and_autostash() {
        let (server, a) = remote_pair();
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        let b = tempfile::tempdir().unwrap();
        let url = server.path().to_string_lossy().into_owned();
        run(b.path(), &["clone", "-q", &url, "."]).unwrap();
        run(b.path(), &["config", "user.name", "T"]).unwrap();
        run(b.path(), &["config", "user.email", "t@t"]).unwrap();

        // a pushes a new commit; b has its own local commit => diverged
        commit_all(&a, "from_a.txt", "a\n", "a commit");
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        commit_all(&b, "from_b.txt", "b\n", "b commit");

        let ff = PullOptions {
            mode: Some("ff-only".into()),
            autostash: false,
        };
        let e = sync_with(b.path(), SyncOp::Pull, false, &ff, |_| {}).unwrap_err();
        assert_eq!(e.kind, "diverged");
        assert!(e.message.contains("Pull"), "{}", e.message);

        // rebase keeps history linear; autostash lets it run with a dirty tree
        write(&b, "from_b.txt", "b dirty edit\n");
        let plain = PullOptions {
            mode: Some("rebase".into()),
            autostash: false,
        };
        assert!(
            sync_with(b.path(), SyncOp::Pull, false, &plain, |_| {}).is_err(),
            "dirty tree blocks a plain rebase pull"
        );
        let stash = PullOptions {
            mode: Some("rebase".into()),
            autostash: true,
        };
        sync_with(b.path(), SyncOp::Pull, false, &stash, |_| {}).unwrap();
        assert_eq!(parents_of_head(&b), 1, "rebased, no merge commit");
        assert_eq!(
            std::fs::read_to_string(b.path().join("from_b.txt")).unwrap(),
            "b dirty edit\n",
            "local edit restored"
        );
        assert!(b.path().join("from_a.txt").exists());
    }

    // ---- cherry-pick, revert, reset, detached checkout, tags --------------------------

    fn tip(dir: &tempfile::TempDir, rev: &str) -> String {
        run(dir.path(), &["rev-parse", rev])
            .unwrap()
            .trim()
            .to_string()
    }

    #[test]
    fn cherry_pick_copies_one_commit_and_handles_conflicts() {
        let (dir, main) = diverged();
        let feature_tip = tip(&dir, "feature");
        let before = commit_count(&dir);
        let r = cherry_pick(dir.path(), &feature_tip).unwrap();
        assert_eq!(r.outcome, "done");
        assert!(
            dir.path().join("other.txt").exists(),
            "feature's file arrived"
        );
        assert_eq!(commit_count(&dir), before + 1);
        assert_eq!(branch_name(&dir), main);
        // picking the same changes again: nothing to do, clean state
        let again = cherry_pick(dir.path(), &feature_tip).unwrap();
        assert_eq!(again.outcome, "up_to_date");
        assert_eq!(crate::git::work_status(dir.path()).unwrap().operation, None);

        // conflicting pick pauses; resolve + continue finishes
        let (dir, _) = conflicting();
        let ftip = tip(&dir, "feature");
        let r = cherry_pick(dir.path(), &ftip).unwrap();
        assert_eq!(r.outcome, "conflicts");
        assert_eq!(
            crate::git::work_status(dir.path()).unwrap().operation,
            Some("cherry-pick")
        );
        resolve_side(dir.path(), "f.txt", "theirs").unwrap(); // theirs = the picked commit
        assert_eq!(operation_continue(dir.path()).unwrap().outcome, "done");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "feature side\n"
        );

        // abort restores
        let (dir, _) = conflicting();
        let head = tip(&dir, "HEAD");
        cherry_pick(dir.path(), &tip(&dir, "feature")).unwrap();
        operation_abort(dir.path()).unwrap();
        assert_eq!(tip(&dir, "HEAD"), head);
        assert_eq!(crate::git::work_status(dir.path()).unwrap().operation, None);
    }

    #[test]
    fn revert_adds_an_undo_commit_including_for_merge_commits() {
        let dir = repo();
        commit_all(&dir, "f.txt", "one\n", "first");
        commit_all(&dir, "f.txt", "two\n", "second");
        let second = tip(&dir, "HEAD");
        let r = revert(dir.path(), &second).unwrap();
        assert_eq!(r.outcome, "done");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "one\n"
        );
        assert_eq!(
            commit_count(&dir),
            3,
            "history is kept, a new commit is added"
        );

        // reverting a merge commit works (relative to its first parent)
        let (dir, _) = diverged();
        merge(dir.path(), "feature", false).unwrap();
        assert_eq!(parents_of_head(&dir), 2);
        let merge_commit = tip(&dir, "HEAD");
        revert(dir.path(), &merge_commit).unwrap();
        assert!(
            !dir.path().join("other.txt").exists(),
            "the merged-in work is undone"
        );
        assert!(dir.path().join("main.txt").exists());
    }

    #[test]
    fn reset_modes_soft_mixed_hard() {
        let dir = repo();
        commit_all(&dir, "f.txt", "one\n", "first");
        let first = tip(&dir, "HEAD");
        commit_all(&dir, "f.txt", "two\n", "second");
        let second = tip(&dir, "HEAD");

        // soft: HEAD moves, changes stay staged
        let msg = reset(dir.path(), &first, "soft").unwrap();
        assert!(
            msg.contains(&second[..7]),
            "names the previous position: {msg}"
        );
        let st = crate::git::work_status(dir.path()).unwrap();
        assert_eq!((st.staged.len(), st.unstaged.len()), (1, 0));
        // back to second via the printed hash, then mixed: changes stay but unstaged
        reset(dir.path(), &second, "hard").unwrap();
        reset(dir.path(), &first, "mixed").unwrap();
        let st = crate::git::work_status(dir.path()).unwrap();
        assert_eq!((st.staged.len(), st.unstaged.len()), (0, 1));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "two\n"
        );
        // hard: working tree changes are gone
        reset(dir.path(), &first, "hard").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("f.txt")).unwrap(),
            "one\n"
        );
        assert!(crate::git::work_status(dir.path())
            .unwrap()
            .unstaged
            .is_empty());

        assert!(reset(dir.path(), &first, "nuclear").is_err());
        assert!(
            reset(dir.path(), "--hard", "soft").is_err(),
            "option-like targets are rejected"
        );
    }

    #[test]
    fn detached_checkout_and_head_label() {
        let dir = repo();
        commit_all(&dir, "f.txt", "one\n", "first");
        let first = tip(&dir, "HEAD");
        commit_all(&dir, "f.txt", "two\n", "second");
        checkout_detached(dir.path(), &first).unwrap();
        let refs = crate::git::refs(dir.path()).unwrap();
        assert!(refs
            .labels
            .iter()
            .any(|l| l.kind == "head" && l.id == first));
        assert!(checkout_detached(dir.path(), "--force").is_err());
    }

    #[test]
    fn tags_create_annotated_delete_and_push() {
        let (server, a) = remote_pair();
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        create_tag(a.path(), "v1.0", None, None).unwrap();
        create_tag(a.path(), "v1.1", None, Some("Release 1.1")).unwrap();
        let refs = crate::git::refs(a.path()).unwrap();
        assert_eq!(refs.tags, vec!["v1.0", "v1.1"]);
        assert_eq!(
            run(a.path(), &["cat-file", "-t", "v1.1"]).unwrap().trim(),
            "tag",
            "annotated"
        );
        assert_eq!(
            run(a.path(), &["cat-file", "-t", "v1.0"]).unwrap().trim(),
            "commit",
            "lightweight"
        );
        let labels: Vec<_> = refs.labels.iter().filter(|l| l.kind == "tag").collect();
        assert_eq!(
            labels.len(),
            2,
            "annotated tags resolve through to their commit"
        );

        assert!(create_tag(a.path(), "v1.0", None, None)
            .unwrap_err()
            .to_string()
            .contains("already exists"));
        assert!(create_tag(a.path(), "bad name", None, None).is_err());
        assert!(create_tag(a.path(), "-x", None, None).is_err());

        push_tag(a.path(), "v1.0", false, |_| {}).unwrap();
        let remote_tags = run(server.path(), &["tag"]).unwrap();
        assert!(remote_tags.contains("v1.0") && !remote_tags.contains("v1.1"));
        push_tag(a.path(), "v1.0", true, |_| {}).unwrap();
        assert!(
            !run(server.path(), &["tag"]).unwrap().contains("v1.0"),
            "removed on the remote"
        );

        delete_tag(a.path(), "v1.1").unwrap();
        assert_eq!(crate::git::refs(a.path()).unwrap().tags, vec!["v1.0"]);
        assert!(delete_tag(a.path(), "nope").is_err());
    }

    // ---- signatures --------------------------------------------------------------------

    #[test]
    fn unsigned_and_ssh_signed_commits_are_told_apart() {
        let dir = repo();
        commit_all(&dir, "f.txt", "one\n", "unsigned");
        let unsigned = tip(&dir, "HEAD");
        let info = signature_info(dir.path(), &unsigned).unwrap();
        assert_eq!((info.signed, info.status), (false, "none"));
        let list =
            crate::git::log_page(dir.path(), &crate::git::log_ids(dir.path()).unwrap(), 0, 10)
                .unwrap();
        assert!(!list[0].signed);

        // sign a commit with a throw-away SSH key (skipped where ssh-keygen is unavailable)
        let keys = tempfile::tempdir().unwrap();
        let key = keys.path().join("id_test");
        let made = Command::new("ssh-keygen")
            .args([
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "tester@example.com",
                "-f",
            ])
            .arg(&key)
            .status();
        if !made.map(|s| s.success()).unwrap_or(false) {
            eprintln!("ssh-keygen not available: skipping the SSH-signed half of this test");
            return;
        }
        let pubkey = std::fs::read_to_string(format!("{}.pub", key.display())).unwrap();
        let allowed = keys.path().join("allowed_signers");
        std::fs::write(&allowed, format!("tester@example.com {pubkey}")).unwrap();
        run(dir.path(), &["config", "gpg.format", "ssh"]).unwrap();
        run(
            dir.path(),
            &[
                "config",
                "user.signingkey",
                &format!("{}.pub", key.display()),
            ],
        )
        .unwrap();
        run(
            dir.path(),
            &[
                "config",
                "gpg.ssh.allowedSignersFile",
                allowed.to_str().unwrap(),
            ],
        )
        .unwrap();
        write(&dir, "g.txt", "g\n");
        stage(dir.path(), &["g.txt".to_string()]).unwrap();
        run(dir.path(), &["commit", "-q", "-S", "-m", "signed one"]).unwrap();
        let signed = tip(&dir, "HEAD");

        let info = signature_info(dir.path(), &signed).unwrap();
        assert!(info.signed, "{info:?}");
        assert_eq!(info.status, "good", "{info:?}");
        assert_eq!(info.signer, "tester@example.com");
        let list =
            crate::git::log_page(dir.path(), &crate::git::log_ids(dir.path()).unwrap(), 0, 10)
                .unwrap();
        assert!(
            list[0].signed && !list[1].signed,
            "newest is the signed one"
        );
        assert!(signature_info(dir.path(), "--oops").is_err());
    }

    // ---- blame & file history ----------------------------------------------------------

    #[test]
    fn blame_attributes_lines_and_history_follows_renames() {
        let dir = repo();
        commit_all(&dir, "notes.txt", "alpha\nbeta\ngamma\n", "first version");
        let first = tip(&dir, "HEAD");
        commit_all(
            &dir,
            "notes.txt",
            "alpha\nBETA edited\ngamma\ndelta\n",
            "edit beta, add delta",
        );
        let second = tip(&dir, "HEAD");

        let b = crate::git::blame(dir.path(), "notes.txt", None).unwrap();
        assert_eq!(b.lines, vec!["alpha", "BETA edited", "gamma", "delta"]);
        let owner = |line: u32| {
            b.hunks
                .iter()
                .find(|h| line >= h.start && line < h.start + h.count)
                .unwrap()
                .id
                .clone()
        };
        assert_eq!(owner(1), first);
        assert_eq!(owner(2), second);
        assert_eq!(owner(3), first);
        assert_eq!(owner(4), second);
        assert_eq!(
            b.hunks.iter().map(|h| h.count).sum::<u32>(),
            4,
            "every line is covered once"
        );
        assert_eq!(b.hunks[0].summary, "first version");

        // blame as of an older revision
        let old = crate::git::blame(dir.path(), "notes.txt", Some(&first)).unwrap();
        assert_eq!(old.lines, vec!["alpha", "beta", "gamma"]);
        assert!(old.hunks.iter().all(|h| h.id == first));

        // rename keeps history
        run(dir.path(), &["mv", "notes.txt", "renamed.txt"]).unwrap();
        commit(dir.path(), "rename notes", false).unwrap();
        let h = file_history(dir.path(), "renamed.txt", 50).unwrap();
        let subjects: Vec<_> = h.iter().map(|c| c.summary.as_str()).collect();
        assert_eq!(
            subjects,
            vec!["rename notes", "edit beta, add delta", "first version"]
        );

        assert!(crate::git::blame(dir.path(), "missing.txt", None).is_err());
        assert!(crate::git::blame(dir.path(), "--x", None).is_err());
        assert!(crate::git::blame(dir.path(), "renamed.txt", Some("--evil")).is_err());
        assert!(file_history(dir.path(), "--all", 10).is_err());
        assert_eq!(
            file_history(dir.path(), "renamed.txt", 1).unwrap().len(),
            1,
            "limit honoured"
        );
    }

    // ---- submodules --------------------------------------------------------------------

    #[test]
    fn submodules_list_status_and_update() {
        // git refuses local-path submodules unless told otherwise; opt in for this test only
        std::env::set_var("GIT_ALLOW_PROTOCOL", "file");
        // a library repo, and a project that embeds it
        let lib = repo();
        commit_all(&lib, "lib.txt", "v1\n", "lib v1");
        let lib_url = lib.path().to_string_lossy().into_owned();
        let project = repo();
        commit_all(&project, "app.txt", "app\n", "app");
        run(
            project.path(),
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                &lib_url,
                "libs/lib",
            ],
        )
        .unwrap();
        commit(project.path(), "add lib", false).unwrap();

        let subs = crate::git::submodules(project.path()).unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(
            (subs[0].path.as_str(), subs[0].status),
            ("libs/lib", "clean")
        );
        assert_eq!(subs[0].url.as_deref(), Some(lib_url.as_str()));

        // the submodule moves ahead: parent sees "new_commits"
        let inner = project.path().join("libs/lib");
        std::fs::write(inner.join("lib.txt"), "v2\n").unwrap();
        run(
            &inner,
            &[
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@t",
                "commit",
                "-qam",
                "lib v2",
            ],
        )
        .unwrap();
        assert_eq!(
            crate::git::submodules(project.path()).unwrap()[0].status,
            "new_commits"
        );
        // uncommitted edits inside: "dirty"
        std::fs::write(inner.join("lib.txt"), "v2 plus edit\n").unwrap();
        assert_eq!(
            crate::git::submodules(project.path()).unwrap()[0].status,
            "dirty"
        );
        run(&inner, &["checkout", "-q", "--", "lib.txt"]).unwrap();

        // a fresh clone has the submodule uninitialized until we update it
        let clone_dir = tempfile::tempdir().unwrap();
        let project_url = project.path().to_string_lossy().into_owned();
        run(clone_dir.path(), &["clone", "-q", &project_url, "."]).unwrap();
        run(
            clone_dir.path(),
            &["config", "protocol.file.allow", "always"],
        )
        .unwrap();
        assert_eq!(
            crate::git::submodules(clone_dir.path()).unwrap()[0].status,
            "uninitialized"
        );
        let mut progress = 0;
        submodule_update(clone_dir.path(), None, |_| progress += 1).unwrap();
        assert_eq!(
            crate::git::submodules(clone_dir.path()).unwrap()[0].status,
            "clean"
        );
        assert!(clone_dir.path().join("libs/lib/lib.txt").exists());

        assert!(submodule_update(clone_dir.path(), Some("--evil"), |_| {}).is_err());
    }

    // ---- interactive rebase --------------------------------------------------------------

    /// root + c1..c4, each commit adding its own file; returns their ids oldest first
    fn four_commits() -> (tempfile::TempDir, Vec<String>) {
        let dir = repo();
        commit_all(&dir, "root.txt", "root\n", "root");
        let mut ids = Vec::new();
        for n in 1..=4 {
            commit_all(
                &dir,
                &format!("c{n}.txt"),
                &format!("c{n}\n"),
                &format!("c{n}"),
            );
            ids.push(tip(&dir, "HEAD"));
        }
        (dir, ids)
    }
    fn item(id: &str, action: &str) -> RebaseItem {
        RebaseItem {
            id: id.into(),
            action: action.into(),
            message: None,
        }
    }
    fn subjects(dir: &tempfile::TempDir) -> Vec<String> {
        run(dir.path(), &["log", "--format=%s", "--reverse"])
            .unwrap()
            .lines()
            .map(String::from)
            .collect()
    }

    #[test]
    fn interactive_rebase_reorder_drop_squash_fixup_reword() {
        // reorder: c1 c3 c2 c4
        let (dir, c) = four_commits();
        let plan = vec![
            item(&c[0], "pick"),
            item(&c[2], "pick"),
            item(&c[1], "pick"),
            item(&c[3], "pick"),
        ];
        assert_eq!(
            rebase_interactive(dir.path(), &c[0], &plan)
                .unwrap()
                .outcome,
            "done"
        );
        assert_eq!(subjects(&dir), ["root", "c1", "c3", "c2", "c4"]);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("c4.txt")).unwrap(),
            "c4\n"
        );
        assert!(!git_dir_has_scratch(&dir), "helper files are cleaned up");

        // drop c3
        let (dir, c) = four_commits();
        let plan = vec![
            item(&c[0], "pick"),
            item(&c[1], "pick"),
            item(&c[2], "drop"),
            item(&c[3], "pick"),
        ];
        rebase_interactive(dir.path(), &c[0], &plan).unwrap();
        assert_eq!(subjects(&dir), ["root", "c1", "c2", "c4"]);
        assert!(!dir.path().join("c3.txt").exists());

        // squash c2 into c1: one commit, both messages, both files
        let (dir, c) = four_commits();
        let plan = vec![
            item(&c[0], "pick"),
            item(&c[1], "squash"),
            item(&c[2], "pick"),
            item(&c[3], "pick"),
        ];
        rebase_interactive(dir.path(), &c[0], &plan).unwrap();
        assert_eq!(subjects(&dir), ["root", "c1", "c3", "c4"]);
        let msg = run(dir.path(), &["log", "-1", "--format=%B", "HEAD~2"]).unwrap();
        assert!(
            msg.contains("c1") && msg.contains("c2"),
            "squash keeps both messages: {msg}"
        );
        assert!(dir.path().join("c2.txt").exists());

        // fixup c2 into c1: message of c1 only
        let (dir, c) = four_commits();
        let plan = vec![
            item(&c[0], "pick"),
            item(&c[1], "fixup"),
            item(&c[2], "pick"),
            item(&c[3], "pick"),
        ];
        rebase_interactive(dir.path(), &c[0], &plan).unwrap();
        let msg = run(dir.path(), &["log", "-1", "--format=%B", "HEAD~2"]).unwrap();
        assert_eq!(msg.trim(), "c1");
        assert!(dir.path().join("c2.txt").exists());

        // reword c2 (multi-line, quotes and all), only from c2 on
        let (dir, c) = four_commits();
        let mut plan: Vec<RebaseItem> = c[1..].iter().map(|i| item(i, "pick")).collect();
        plan[0] = RebaseItem {
            id: c[1].clone(),
            action: "reword".into(),
            message: Some(
                "Better subject\n\nBody with 'quotes' and \"doubles\" and $dollars".into(),
            ),
        };
        rebase_interactive(dir.path(), &c[1], &plan).unwrap();
        assert_eq!(subjects(&dir), ["root", "c1", "Better subject", "c3", "c4"]);
        let body = run(dir.path(), &["log", "-1", "--format=%B", "HEAD~2"]).unwrap();
        assert!(
            body.contains("'quotes'") && body.contains("$dollars"),
            "{body}"
        );
        assert!(
            dir.path().join("c2.txt").exists(),
            "contents untouched by a reword"
        );
    }

    fn git_dir_has_scratch(dir: &tempfile::TempDir) -> bool {
        dir.path().join(".git/whittle-rebase").exists()
    }

    #[test]
    fn interactive_rebase_from_the_root_commit() {
        let (dir, c) = four_commits();
        let root = run(dir.path(), &["rev-list", "--max-parents=0", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        let mut plan = vec![item(&root, "pick")];
        plan.extend(c.iter().rev().map(|i| item(i, "pick"))); // reverse everything after root
        rebase_interactive(dir.path(), &root, &plan).unwrap();
        assert_eq!(subjects(&dir), ["root", "c4", "c3", "c2", "c1"]);
    }

    #[test]
    fn interactive_rebase_conflict_pauses_then_abort_restores() {
        let dir = repo();
        commit_all(&dir, "f.txt", "base\n", "base");
        commit_all(&dir, "f.txt", "one\n", "edit one");
        let one = tip(&dir, "HEAD");
        commit_all(&dir, "f.txt", "two\n", "edit two");
        let two = tip(&dir, "HEAD");
        let original = tip(&dir, "HEAD");
        // swap the two edits: they touch the same line, so applying "two" first conflicts
        let r = rebase_interactive(dir.path(), &one, &[item(&two, "pick"), item(&one, "pick")])
            .unwrap();
        assert_eq!(r.outcome, "conflicts");
        assert_eq!(
            crate::git::work_status(dir.path()).unwrap().operation,
            Some("rebase")
        );
        operation_abort(dir.path()).unwrap();
        assert_eq!(
            tip(&dir, "HEAD"),
            original,
            "abort restores the original history"
        );
        assert!(
            !git_dir_has_scratch(&dir),
            "helper files removed after abort"
        );
    }

    #[test]
    fn interactive_rebase_resumes_a_reword_after_resolving_a_conflict() {
        let dir = repo();
        commit_all(&dir, "f.txt", "base\n", "base");
        commit_all(&dir, "f.txt", "one\n", "edit one");
        let one = tip(&dir, "HEAD");
        commit_all(&dir, "g.txt", "g\n", "add g");
        let g = tip(&dir, "HEAD");
        commit_all(&dir, "f.txt", "two\n", "edit two");
        let two = tip(&dir, "HEAD");
        // plan: "edit two" first (conflicts with "edit one"), then reword "add g"
        let reword = RebaseItem {
            id: g.clone(),
            action: "reword".into(),
            message: Some("g reworded".into()),
        };
        let r = rebase_interactive(
            dir.path(),
            &one,
            &[item(&two, "pick"), item(&one, "pick"), reword],
        )
        .unwrap();
        assert_eq!(r.outcome, "conflicts");
        resolve_side(dir.path(), "f.txt", "theirs").unwrap();
        let mut r = operation_continue(dir.path()).unwrap();
        // the second pick may conflict as well; resolve until done
        while r.outcome == "conflicts" {
            resolve_side(dir.path(), "f.txt", "theirs").unwrap();
            r = operation_continue(dir.path()).unwrap();
        }
        assert!(
            subjects(&dir).contains(&"g reworded".to_string()),
            "{:?}",
            subjects(&dir)
        );
        assert!(!git_dir_has_scratch(&dir), "cleaned up once finished");
    }

    #[test]
    fn interactive_rebase_rejects_bad_plans_and_dirty_trees() {
        let (dir, c) = four_commits();
        let p = |v: Vec<RebaseItem>| {
            rebase_interactive(dir.path(), &c[0], &v)
                .unwrap_err()
                .to_string()
        };
        assert!(p(vec![item(&c[0], "squash"), item(&c[1], "pick")]).contains("can't be squashed"));
        assert!(p(vec![item(&c[0], "pick"), item(&c[0], "pick")]).contains("doesn't match"));
        assert!(p(vec![item(
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            "pick"
        )])
        .contains("doesn't match"));
        assert!(p(vec![item(&c[0], "pick; rm -rf /")]).contains("Unknown action"));
        assert!(p(vec![item(&c[0], "reword")]).contains("needs a message"));
        assert!(p(c.iter().map(|i| item(i, "drop")).collect()).contains("nothing to rebase"));
        assert!(p(vec![]).contains("nothing to rebase"));
        assert!(rebase_interactive(dir.path(), "--evil", &[item(&c[0], "pick")]).is_err());
        assert_eq!(
            subjects(&dir),
            ["root", "c1", "c2", "c3", "c4"],
            "nothing changed"
        );

        // dirty tree blocks it with the usual friendly message
        write(&dir, "c1.txt", "uncommitted\n");
        let plan: Vec<_> = c.iter().rev().map(|i| item(i, "pick")).collect();
        let e = rebase_interactive(dir.path(), &c[0], &plan)
            .unwrap_err()
            .to_string();
        assert!(e.contains("uncommitted changes"), "{e}");
        assert!(!git_dir_has_scratch(&dir));
    }

    #[test]
    fn rebase_candidates_refuse_merges_and_foreign_commits() {
        // a range that includes a merge commit is refused
        let (dir, _) = diverged();
        let first = run(dir.path(), &["rev-list", "--max-parents=0", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        merge(dir.path(), "feature", false).unwrap();
        let e = crate::git::rebase_candidates(dir.path(), &first).unwrap_err();
        assert!(e.message().contains("merge commit"), "{}", e.message());

        // a commit that lives only on another branch is not part of this branch's history
        let (dir, _) = diverged();
        let other = tip(&dir, "feature");
        let e = crate::git::rebase_candidates(dir.path(), &other).unwrap_err();
        assert!(
            e.message().contains("not part of the current branch"),
            "{}",
            e.message()
        );

        // a valid range lists the commits oldest first
        let (dir, c) = four_commits();
        let ids: Vec<String> = crate::git::rebase_candidates(dir.path(), &c[1])
            .unwrap()
            .into_iter()
            .map(|x| x.id)
            .collect();
        assert_eq!(ids, c[1..].to_vec());
    }

    // ---- configurable git binary ------------------------------------------------------

    #[test]
    fn git_binary_check_accepts_git_and_rejects_everything_else() {
        assert!(check_git_binary("git").unwrap().starts_with("git version"));
        assert!(
            check_git_binary("").unwrap().starts_with("git version"),
            "empty = PATH"
        );
        assert!(check_git_binary("/no/such/binary")
            .unwrap_err()
            .contains("Can't run"));
        // a real program that is not git must not be accepted as git
        assert!(check_git_binary("echo")
            .unwrap_err()
            .contains("doesn't look like git"));
    }

    // ---- checking out pull requests ----------------------------------------------------

    #[test]
    fn checkout_pull_fetches_the_hidden_ref_and_never_clobbers_local_work() {
        let (server, a) = remote_pair();
        sync(a.path(), SyncOp::Push, false, |_| {}).unwrap();
        // a contributor's branch exists on the server only as refs/pull/7/head (like on GitHub)
        create_branch(a.path(), "contrib", None, true).unwrap();
        commit_all(&a, "pr.txt", "from the PR\n", "pr commit");
        run(a.path(), &["push", "-q", "origin", "HEAD:refs/pull/7/head"]).unwrap();
        checkout(a.path(), "master", false)
            .or_else(|_| checkout(a.path(), "main", false))
            .unwrap();

        let b = tempfile::tempdir().unwrap();
        let url = server.path().to_string_lossy().into_owned();
        run(b.path(), &["clone", "-q", &url, "."]).unwrap();
        run(b.path(), &["config", "user.name", "T"]).unwrap();
        run(b.path(), &["config", "user.email", "t@t"]).unwrap();

        let msg = checkout_pull(b.path(), 7).unwrap();
        assert!(msg.contains("pr-7"), "{msg}");
        assert_eq!(
            crate::git::summary(b.path()).unwrap().branch.as_deref(),
            Some("pr-7")
        );
        assert!(b.path().join("pr.txt").exists());

        // again: updates in place; with a local commit on pr-7 it refuses to overwrite it
        assert!(checkout_pull(b.path(), 7).unwrap().contains("updated"));
        commit_all(&b, "mine.txt", "local\n", "my local work");
        let local = tip(&b, "HEAD");
        run(a.path(), &["checkout", "-q", "contrib"]).unwrap();
        commit_all(&a, "pr2.txt", "more\n", "pr commit 2");
        run(
            a.path(),
            &["push", "-q", "-f", "origin", "HEAD:refs/pull/7/head"],
        )
        .unwrap();
        let e = checkout_pull(b.path(), 7).unwrap_err();
        assert_eq!(e.kind, "diverged", "{e:?}");
        assert_eq!(
            tip(&b, "HEAD"),
            local,
            "local commits on pr-7 are untouched"
        );

        // a number that doesn't exist
        let e = checkout_pull(b.path(), 999).unwrap_err();
        assert!(e.message.contains("wasn't found"), "{}", e.message);
    }
}
