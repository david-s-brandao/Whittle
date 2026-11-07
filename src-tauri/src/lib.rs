//! Whittle backend. `git` = reads (libgit2), `cli` = writes (git binary), `store` = projects/settings.
pub mod cli;
pub mod forge;
pub mod git;
pub mod launch;
pub mod platform;
pub mod store;
pub mod watch;

use std::path::Path;
use std::sync::Mutex;
use store::{Project, ProjectStore, Settings};
use tauri::State;

/// Sorted commit ids for the repo currently being paged, rebuilt whenever page 0 is requested.
struct LogCache {
    path: String,
    ids: Vec<git2::Oid>,
}

struct AppState {
    projects: Mutex<ProjectStore>,
    log_cache: Mutex<Option<LogCache>>,
    watcher: Mutex<Option<watch::Watcher>>,
    settings: Mutex<Settings>,
}

type CmdResult<T> = Result<T, String>;

fn lock<'a>(s: &'a State<AppState>) -> CmdResult<std::sync::MutexGuard<'a, ProjectStore>> {
    s.projects
        .lock()
        .map_err(|_| "internal state poisoned".to_string())
}

#[tauri::command]
fn list_projects(state: State<AppState>) -> CmdResult<Vec<Project>> {
    Ok(lock(&state)?.list())
}

/// Validates that `path` is a git repo, then adds it (or bumps last-used).
#[tauri::command]
fn open_project(path: String, state: State<AppState>) -> CmdResult<Project> {
    let root = git::repo_root(Path::new(&path))
        .map_err(|_| format!("“{path}” is not a git repository."))?;
    lock(&state)?.add_or_touch(&root).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_project_starred(path: String, starred: bool, state: State<AppState>) -> CmdResult<()> {
    lock(&state)?
        .set_starred(&path, starred)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_project(path: String, state: State<AppState>) -> CmdResult<()> {
    lock(&state)?.remove(&path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn repo_summary(path: String) -> CmdResult<git::RepoSummary> {
    tauri::async_runtime::spawn_blocking(move || git::summary(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.message().to_string())
}

#[tauri::command]
async fn repo_refs(path: String) -> CmdResult<git::RepoRefs> {
    tauri::async_runtime::spawn_blocking(move || git::refs(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.message().to_string())
}

#[tauri::command]
async fn repo_log(
    path: String,
    skip: usize,
    limit: usize,
    state: State<'_, AppState>,
) -> CmdResult<Vec<git::CommitInfo>> {
    let limit = limit.min(2000);
    let cached = if skip == 0 {
        None
    } else {
        state
            .log_cache
            .lock()
            .map_err(|_| "internal state poisoned".to_string())?
            .as_ref()
            .filter(|c| c.path == path)
            .map(|c| c.ids.clone())
    };
    let (ids, fresh) = match cached {
        Some(ids) => (ids, false),
        None => {
            let p = path.clone();
            let ids = tauri::async_runtime::spawn_blocking(move || git::log_ids(Path::new(&p)))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.message().to_string())?;
            (ids, true)
        }
    };
    let p = path.clone();
    let ids_for_page = ids.clone();
    let page = tauri::async_runtime::spawn_blocking(move || {
        git::log_page(Path::new(&p), &ids_for_page, skip, limit)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.message().to_string())?;
    if fresh {
        *state
            .log_cache
            .lock()
            .map_err(|_| "internal state poisoned".to_string())? = Some(LogCache { path, ids });
    }
    Ok(page)
}

/// Search the history (message, author, email, hash). Reuses the sorted-id cache when it
/// belongs to this repo, otherwise builds it.
#[tauri::command]
async fn search_commits(
    path: String,
    query: String,
    state: State<'_, AppState>,
) -> CmdResult<Vec<git::CommitInfo>> {
    let cached = state
        .log_cache
        .lock()
        .map_err(|_| "internal state poisoned".to_string())?
        .as_ref()
        .filter(|c| c.path == path)
        .map(|c| c.ids.clone());
    let (ids, fresh) = match cached {
        Some(ids) => (ids, false),
        None => {
            let p = path.clone();
            (blocking(move || git::log_ids(Path::new(&p))).await?, true)
        }
    };
    let p = path.clone();
    let ids_for_search = ids.clone();
    let found = blocking(move || git::search(Path::new(&p), &ids_for_search, &query, 300)).await?;
    if fresh {
        *state
            .log_cache
            .lock()
            .map_err(|_| "internal state poisoned".to_string())? = Some(LogCache { path, ids });
    }
    Ok(found)
}

#[tauri::command]
async fn commit_files(path: String, id: String) -> CmdResult<Vec<git::FileChange>> {
    tauri::async_runtime::spawn_blocking(move || git::commit_files(Path::new(&path), &id))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.message().to_string())
}

#[tauri::command]
async fn commit_file_diff(path: String, id: String, file: String) -> CmdResult<git::FileDiff> {
    tauri::async_runtime::spawn_blocking(move || {
        git::commit_file_diff(Path::new(&path), &id, &file)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.message().to_string())
}

/// Runs a blocking git job off the UI thread and flattens errors to strings.
async fn blocking<T, E, F>(f: F) -> CmdResult<T>
where
    T: Send + 'static,
    E: ToString + Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn work_status(path: String) -> CmdResult<git::WorkStatus> {
    blocking(move || git::work_status(Path::new(&path))).await
}

#[tauri::command]
async fn work_file_diff(path: String, file: String, staged: bool) -> CmdResult<git::FileDiff> {
    blocking(move || git::work_file_diff(Path::new(&path), &file, staged)).await
}

#[tauri::command]
async fn stage_files(path: String, files: Vec<String>) -> CmdResult<()> {
    blocking(move || cli::stage(Path::new(&path), &files)).await
}

#[tauri::command]
async fn unstage_files(path: String, files: Vec<String>) -> CmdResult<()> {
    blocking(move || cli::unstage(Path::new(&path), &files)).await
}

#[tauri::command]
async fn discard_files(path: String, files: Vec<String>) -> CmdResult<()> {
    blocking(move || cli::discard(Path::new(&path), &files)).await
}

/// Stage (or unstage) selected hunks/lines of one file.
#[tauri::command]
async fn stage_selection(
    path: String,
    file: String,
    unstage: bool,
    selections: Vec<git::Selection>,
) -> CmdResult<()> {
    blocking(move || cli::apply_selection(Path::new(&path), &file, unstage, &selections)).await
}

#[tauri::command]
async fn stash_push(path: String, message: String, include_untracked: bool) -> CmdResult<String> {
    blocking(move || cli::stash_push(Path::new(&path), &message, include_untracked)).await
}

#[tauri::command]
async fn stash_apply(path: String, index: usize, pop: bool) -> CmdResult<String> {
    blocking(move || cli::stash_apply(Path::new(&path), index, pop)).await
}

#[tauri::command]
async fn stash_drop(path: String, index: usize) -> CmdResult<String> {
    blocking(move || cli::stash_drop(Path::new(&path), index)).await
}

#[tauri::command]
async fn merge_branch(
    path: String,
    branch: String,
    options: cli::MergeOptions,
) -> CmdResult<cli::OpResult> {
    blocking(move || cli::merge_with(Path::new(&path), &branch, &options)).await
}

/// What merging `source` into the current branch would do (nothing is changed).
#[tauri::command]
async fn merge_preview(path: String, source: String) -> CmdResult<git::MergePreview> {
    blocking(move || git::merge_preview(Path::new(&path), &source)).await
}

#[tauri::command]
async fn rebase_onto(path: String, onto: String) -> CmdResult<cli::OpResult> {
    blocking(move || cli::rebase(Path::new(&path), &onto)).await
}

/// action: "continue" | "skip" | "abort" for a paused merge/rebase.
#[tauri::command]
async fn operation_step(path: String, action: String) -> CmdResult<cli::OpResult> {
    blocking(move || {
        let p = Path::new(&path);
        match action.as_str() {
            "continue" => cli::operation_continue(p),
            "skip" => cli::operation_skip(p),
            "abort" => cli::operation_abort(p).map(|()| cli::OpResult {
                outcome: "done",
                message: "Operation aborted. Everything is back as it was.".into(),
            }),
            other => Err(cli::CliError::Failed(format!("unknown action “{other}”"))),
        }
    })
    .await
}

/// side: "ours" | "theirs"
#[tauri::command]
async fn resolve_conflict(path: String, file: String, side: String) -> CmdResult<()> {
    blocking(move || cli::resolve_side(Path::new(&path), &file, &side)).await
}

#[tauri::command]
async fn mark_resolved(path: String, file: String) -> CmdResult<()> {
    blocking(move || cli::mark_resolved(Path::new(&path), &file)).await
}

#[tauri::command]
async fn read_file_text(path: String, file: String) -> CmdResult<String> {
    blocking(move || git::read_text(Path::new(&path), &file)).await
}

/// Opens a file in the user's default editor/application (via `xdg-open`).
#[tauri::command]
fn open_in_editor(path: String, file: String) -> CmdResult<()> {
    let full = git::safe_join(Path::new(&path), &file)?;
    std::process::Command::new("xdg-open")
        .arg(full)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open an editor (xdg-open): {e}"))
}

#[tauri::command]
async fn cherry_pick_commit(path: String, commit: String) -> CmdResult<cli::OpResult> {
    blocking(move || cli::cherry_pick(Path::new(&path), &commit)).await
}

#[tauri::command]
async fn revert_commit(path: String, commit: String) -> CmdResult<cli::OpResult> {
    blocking(move || cli::revert(Path::new(&path), &commit)).await
}

/// mode: "soft" | "mixed" | "hard"
#[tauri::command]
async fn reset_to(path: String, target: String, mode: String) -> CmdResult<String> {
    blocking(move || cli::reset(Path::new(&path), &target, &mode)).await
}

#[tauri::command]
async fn checkout_commit(path: String, commit: String) -> CmdResult<()> {
    blocking(move || cli::checkout_detached(Path::new(&path), &commit)).await
}

#[tauri::command]
async fn create_tag(
    path: String,
    name: String,
    target: Option<String>,
    message: Option<String>,
) -> CmdResult<()> {
    blocking(move || {
        cli::create_tag(
            Path::new(&path),
            &name,
            target.as_deref(),
            message.as_deref(),
        )
    })
    .await
}

#[tauri::command]
async fn delete_tag(path: String, name: String) -> CmdResult<String> {
    blocking(move || cli::delete_tag(Path::new(&path), &name)).await
}

/// Push a tag to the remote (or delete it there). Progress goes to `git-progress` events.
#[tauri::command]
async fn push_tag(
    app: tauri::AppHandle,
    path: String,
    name: String,
    delete: bool,
) -> Result<(), cli::SyncError> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        cli::push_tag(Path::new(&path), &name, delete, |line| {
            let _ = app.emit(
                "git-progress",
                SyncProgress {
                    op: "push",
                    line: line.to_string(),
                },
            );
        })
    })
    .await
    .map_err(|e| cli::SyncError {
        kind: "other",
        message: e.to_string(),
        details: String::new(),
    })?
}

#[tauri::command]
async fn commit_signature(path: String, commit: String) -> CmdResult<cli::SignatureInfo> {
    blocking(move || cli::signature_info(Path::new(&path), &commit)).await
}

#[tauri::command]
async fn file_blame(path: String, file: String, rev: Option<String>) -> CmdResult<git::Blame> {
    blocking(move || git::blame(Path::new(&path), &file, rev.as_deref())).await
}

#[tauri::command]
async fn file_history(path: String, file: String) -> CmdResult<Vec<git::CommitInfo>> {
    blocking(move || cli::file_history(Path::new(&path), &file, 300)).await
}

#[tauri::command]
async fn repo_submodules(path: String) -> CmdResult<Vec<git::SubmoduleInfo>> {
    blocking(move || git::submodules(Path::new(&path))).await
}

/// Initialise / update one submodule (or all when `sub` is None). Progress via `git-progress`.
#[tauri::command]
async fn update_submodules(
    app: tauri::AppHandle,
    path: String,
    sub: Option<String>,
) -> Result<(), cli::SyncError> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        cli::submodule_update(Path::new(&path), sub.as_deref(), |line| {
            let _ = app.emit(
                "git-progress",
                SyncProgress {
                    op: "pull",
                    line: line.to_string(),
                },
            );
        })
    })
    .await
    .map_err(|e| cli::SyncError {
        kind: "other",
        message: e.to_string(),
        details: String::new(),
    })?
}

/// Commits an interactive rebase from `from` could change (oldest first).
#[tauri::command]
async fn rebase_plan(path: String, from: String) -> CmdResult<Vec<git::CommitInfo>> {
    blocking(move || git::rebase_candidates(Path::new(&path), &from)).await
}

#[tauri::command]
async fn rebase_interactive(
    path: String,
    from: String,
    items: Vec<cli::RebaseItem>,
) -> CmdResult<cli::OpResult> {
    blocking(move || cli::rebase_interactive(Path::new(&path), &from, &items)).await
}

#[tauri::command]
async fn commit_changes(path: String, message: String, amend: bool) -> CmdResult<()> {
    blocking(move || cli::commit(Path::new(&path), &message, amend)).await
}

#[tauri::command]
async fn head_message(path: String) -> CmdResult<String> {
    blocking(move || git::head_message(Path::new(&path))).await
}

#[tauri::command]
async fn checkout_branch(path: String, name: String, remote: bool) -> CmdResult<()> {
    blocking(move || cli::checkout(Path::new(&path), &name, remote)).await
}

#[tauri::command]
async fn create_branch(
    path: String,
    name: String,
    start: Option<String>,
    switch: bool,
) -> CmdResult<()> {
    blocking(move || cli::create_branch(Path::new(&path), &name, start.as_deref(), switch)).await
}

#[tauri::command]
async fn delete_branch(path: String, name: String, force: bool) -> CmdResult<String> {
    blocking(move || cli::delete_branch(Path::new(&path), &name, force)).await
}

/// Suggested parent folder for clones: `~/projects` if it exists, else the home folder.
#[tauri::command]
fn default_clone_dir(state: State<AppState>) -> String {
    if let Some(d) = state
        .settings
        .lock()
        .ok()
        .and_then(|s| s.clone_dir.clone())
        .filter(|d| Path::new(d).is_dir())
    {
        return d;
    }
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let projects = home.join("projects");
    if projects.is_dir() { projects } else { home }
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
fn clone_name(url: String) -> String {
    cli::repo_name_from_url(&url)
}

/// Clones into `<parent>/<name>`, streaming `git-progress` events (op = "clone").
#[tauri::command]
async fn clone_repo(
    app: tauri::AppHandle,
    url: String,
    parent: String,
    name: String,
) -> Result<String, cli::SyncError> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        cli::clone(&url, Path::new(&parent), &name, |line| {
            let _ = app.emit(
                "git-progress",
                SyncProgress {
                    op: "clone",
                    line: line.to_string(),
                },
            );
        })
        .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| cli::SyncError {
        kind: "other",
        message: e.to_string(),
        details: String::new(),
    })?
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> CmdResult<Settings> {
    state
        .settings
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "internal state poisoned".to_string())
}

/// Validates, applies (git path takes effect immediately) and saves the settings.
#[tauri::command]
fn save_settings(settings: Settings, state: State<AppState>) -> CmdResult<Settings> {
    let clean = settings.sanitized()?;
    if let Some(g) = &clean.git_path {
        cli::check_git_binary(g)?;
    }
    store::save_settings(&store::config_dir().join("settings.json"), &clean)
        .map_err(|e| format!("Could not save settings: {e}"))?;
    cli::set_git_binary(clean.git_path.clone());
    *state
        .settings
        .lock()
        .map_err(|_| "internal state poisoned".to_string())? = clean.clone();
    Ok(clean)
}

/// For the "Test" button: the version of the git at `path` (or the one on PATH if empty).
#[tauri::command]
fn git_version(path: Option<String>) -> CmdResult<String> {
    cli::check_git_binary(path.as_deref().unwrap_or(""))
}

fn setting(state: &State<AppState>, f: impl Fn(&Settings) -> Option<String>) -> Option<String> {
    state.settings.lock().ok().and_then(|s| f(&s))
}

#[tauri::command]
fn open_terminal(path: String, state: State<AppState>) -> CmdResult<()> {
    launch::open_terminal(
        setting(&state, |s| s.terminal_cmd.clone()).as_deref(),
        Path::new(&path),
    )
}

#[tauri::command]
fn open_editor(path: String, state: State<AppState>) -> CmdResult<()> {
    launch::open_editor(
        setting(&state, |s| s.editor_cmd.clone()).as_deref(),
        Path::new(&path),
    )
}

#[tauri::command]
fn open_folder(path: String) -> CmdResult<()> {
    launch::open_folder(Path::new(&path))
}

// ---- issues & pull requests ---------------------------------------------------------

#[derive(serde::Serialize)]
struct ForgeStatus {
    /// Whether this repository's remote is a hosting service Whittle can talk to.
    supported: bool,
    provider: &'static str,
    host: String,
    owner: String,
    repo: String,
    connected: bool,
    login: Option<String>,
    /// "keyring" (a token pasted into Whittle), "gh" (the GitHub CLI login) or "none".
    source: &'static str,
    message: Option<String>,
}

fn unsupported(message: String) -> ForgeStatus {
    ForgeStatus {
        supported: false,
        provider: "none",
        host: String::new(),
        owner: String::new(),
        repo: String::new(),
        connected: false,
        login: None,
        source: "none",
        message: Some(message),
    }
}

fn forge_status_for(path: &Path) -> ForgeStatus {
    let remote = match forge::repo_remote(path) {
        Ok(r) => r,
        Err(m) => return unsupported(m),
    };
    let provider = forge::provider_of(&remote.host);
    if provider != "github" {
        return unsupported(format!(
            "{} isn't supported yet: Whittle can show issues and pull requests for GitHub repositories.",
            if provider == "gitlab" { "GitLab" } else { "This hosting service" }
        ));
    }
    let auth = forge::resolve_auth(&forge::KeyringStore, &remote.host, forge::gh_cli_token);
    let (mut connected, mut login, mut message) = (auth.token.is_some(), None, None);
    if let Some(token) = auth.token {
        match forge::github::Client::new(&forge::api_base(&remote.host), Some(token)).whoami() {
            Ok(l) => login = Some(l),
            Err(forge::github::ForgeError::Auth) => {
                connected = false;
                message = Some(forge::github::ForgeError::Auth.to_string());
            }
            Err(e) => message = Some(e.to_string()),
        }
    }
    ForgeStatus {
        supported: true,
        provider: "github",
        host: remote.host,
        owner: remote.owner,
        repo: remote.name,
        connected,
        login,
        source: auth.source,
        message,
    }
}

/// The GitHub API for this repository's remote, using whatever login is available.
fn github_for(path: &str) -> Result<forge::github::GitHub, String> {
    let remote = forge::repo_remote(Path::new(path))?;
    if forge::provider_of(&remote.host) != "github" {
        return Err("Only GitHub repositories are supported for issues and pull requests.".into());
    }
    let auth = forge::resolve_auth(&forge::KeyringStore, &remote.host, forge::gh_cli_token);
    let client = forge::github::Client::new(&forge::api_base(&remote.host), auth.token);
    Ok(forge::github::GitHub::new(
        client,
        &remote.owner,
        &remote.name,
    ))
}

#[tauri::command]
async fn forge_status(path: String) -> CmdResult<ForgeStatus> {
    blocking(move || Ok::<_, String>(forge_status_for(Path::new(&path)))).await
}

/// Validates a pasted token, saves it in the system keyring (never in a file), returns the status.
#[tauri::command]
async fn forge_connect(path: String, token: String) -> CmdResult<ForgeStatus> {
    blocking(move || {
        let remote = forge::repo_remote(Path::new(&path))?;
        let token = token.trim().to_string();
        if token.is_empty() {
            return Err("Paste a token first.".to_string());
        }
        forge::github::Client::new(&forge::api_base(&remote.host), Some(token.clone()))
            .whoami()
            .map_err(|e| e.to_string())?;
        forge::TokenStore::set(&forge::KeyringStore, &remote.host, &token)?;
        Ok(forge_status_for(Path::new(&path)))
    })
    .await
}

/// Removes a token saved by Whittle. (The GitHub CLI login, if any, is not touched.)
#[tauri::command]
async fn forge_disconnect(path: String) -> CmdResult<ForgeStatus> {
    blocking(move || {
        let remote = forge::repo_remote(Path::new(&path))?;
        forge::TokenStore::delete(&forge::KeyringStore, &remote.host)?;
        Ok::<_, String>(forge_status_for(Path::new(&path)))
    })
    .await
}

#[tauri::command]
async fn forge_issues(
    path: String,
    state: String,
    query: String,
    page: u32,
) -> CmdResult<forge::github::Page<forge::github::IssueSummary>> {
    blocking(move || {
        github_for(&path)?
            .issues(&state, &query, page)
            .map_err(String::from)
    })
    .await
}

#[tauri::command]
async fn forge_issue(path: String, number: u64) -> CmdResult<forge::github::IssueDetail> {
    blocking(move || github_for(&path)?.issue(number).map_err(String::from)).await
}

#[tauri::command]
async fn forge_pulls(
    path: String,
    state: String,
    query: String,
    page: u32,
) -> CmdResult<forge::github::Page<forge::github::PullSummary>> {
    blocking(move || {
        github_for(&path)?
            .pulls(&state, &query, page)
            .map_err(String::from)
    })
    .await
}

#[tauri::command]
async fn forge_pull(path: String, number: u64) -> CmdResult<forge::github::PullDetail> {
    blocking(move || github_for(&path)?.pull(number).map_err(String::from)).await
}

#[tauri::command]
async fn forge_pull_files(path: String, number: u64) -> CmdResult<Vec<forge::github::PullFile>> {
    blocking(move || github_for(&path)?.pull_files(number).map_err(String::from)).await
}

/// Comment on an issue or on a pull request's conversation.
#[tauri::command]
async fn forge_comment(
    path: String,
    number: u64,
    body: String,
) -> CmdResult<forge::github::Comment> {
    blocking(move || {
        github_for(&path)?
            .add_comment(number, &body)
            .map_err(String::from)
    })
    .await
}

/// state: "open" | "closed"
#[tauri::command]
async fn forge_set_state(path: String, number: u64, state: String) -> CmdResult<()> {
    blocking(move || {
        github_for(&path)?
            .set_state(number, &state)
            .map_err(String::from)
    })
    .await
}

/// event: "APPROVE" | "REQUEST_CHANGES" | "COMMENT"
#[tauri::command]
async fn forge_review(path: String, number: u64, event: String, body: String) -> CmdResult<()> {
    blocking(move || {
        github_for(&path)?
            .review(number, &event, &body)
            .map_err(String::from)
    })
    .await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn forge_review_comment(
    path: String,
    number: u64,
    commit_id: String,
    file: String,
    line: u64,
    side: String,
    body: String,
    in_reply_to: Option<u64>,
) -> CmdResult<forge::github::ReviewComment> {
    blocking(move || {
        github_for(&path)?
            .review_comment(number, &commit_id, &file, line, &side, &body, in_reply_to)
            .map_err(String::from)
    })
    .await
}

#[tauri::command]
async fn checkout_pull(path: String, number: u64) -> Result<String, cli::SyncError> {
    tauri::async_runtime::spawn_blocking(move || cli::checkout_pull(Path::new(&path), number))
        .await
        .map_err(|e| cli::SyncError {
            kind: "other",
            message: e.to_string(),
            details: String::new(),
        })?
}

#[tauri::command]
fn open_url(url: String) -> CmdResult<()> {
    launch::open_url(&url)
}

#[derive(Clone, serde::Serialize)]
struct RepoChanged {
    path: String,
    #[serde(flatten)]
    change: watch::Change,
}

/// Starts (replacing any previous) watching of `path`; emits `repo-changed` events.
/// Runs off the main thread: setting up watches on a big tree must never freeze the window.
#[tauri::command]
async fn watch_repo(
    app: tauri::AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> CmdResult<()> {
    use tauri::Emitter;
    let emit_path = path.clone();
    let w = blocking(move || {
        watch::Watcher::start(
            Path::new(&path),
            std::time::Duration::from_millis(300),
            move |change| {
                let _ = app.emit(
                    "repo-changed",
                    RepoChanged {
                        path: emit_path.clone(),
                        change,
                    },
                );
            },
        )
    })
    .await
    .map_err(|e| format!("Could not watch for file changes: {e}"))?;
    *state
        .watcher
        .lock()
        .map_err(|_| "internal state poisoned".to_string())? = Some(w);
    Ok(())
}

#[tauri::command]
fn unwatch_repo(state: State<AppState>) -> CmdResult<()> {
    *state
        .watcher
        .lock()
        .map_err(|_| "internal state poisoned".to_string())? = None;
    Ok(())
}

#[derive(Clone, serde::Serialize)]
struct SyncProgress {
    op: &'static str,
    line: String,
}

/// op: "fetch" | "pull" | "push". Streams progress as `git-progress` events.
#[tauri::command]
async fn git_sync(
    app: tauri::AppHandle,
    path: String,
    op: String,
    force: bool,
    pull: Option<cli::PullOptions>,
) -> Result<(), cli::SyncError> {
    use tauri::Emitter;
    let (sync_op, name) = match op.as_str() {
        "fetch" => (cli::SyncOp::Fetch, "fetch"),
        "pull" => (cli::SyncOp::Pull, "pull"),
        "push" => (cli::SyncOp::Push, "push"),
        other => {
            return Err(cli::SyncError {
                kind: "other",
                message: format!("unknown operation “{other}”"),
                details: String::new(),
            })
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        let pull = pull.unwrap_or_default();
        cli::sync_with(Path::new(&path), sync_op, force, &pull, |line| {
            let _ = app.emit(
                "git-progress",
                SyncProgress {
                    op: name,
                    line: line.to_string(),
                },
            );
        })
    })
    .await
    .map_err(|e| cli::SyncError {
        kind: "other",
        message: e.to_string(),
        details: String::new(),
    })?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // best effort: a failed migration just means starting with an empty project list
    let _ = store::migrate_legacy_config();
    let settings = store::load_settings(&store::config_dir().join("settings.json"));
    cli::set_git_binary(settings.git_path.clone());
    let store = ProjectStore::load(store::config_dir().join("projects.json"))
        .expect("could not read ~/.config/whittle/projects.json");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            projects: Mutex::new(store),
            log_cache: Mutex::new(None),
            watcher: Mutex::new(None),
            settings: Mutex::new(settings),
        })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            open_project,
            set_project_starred,
            remove_project,
            repo_summary,
            repo_refs,
            repo_log,
            search_commits,
            commit_files,
            commit_file_diff,
            work_status,
            work_file_diff,
            stage_files,
            unstage_files,
            discard_files,
            stage_selection,
            stash_push,
            stash_apply,
            stash_drop,
            cherry_pick_commit,
            revert_commit,
            reset_to,
            checkout_commit,
            create_tag,
            delete_tag,
            push_tag,
            commit_signature,
            file_blame,
            file_history,
            repo_submodules,
            update_submodules,
            rebase_plan,
            rebase_interactive,
            get_settings,
            save_settings,
            git_version,
            open_terminal,
            open_editor,
            open_folder,
            forge_status,
            forge_connect,
            forge_disconnect,
            forge_issues,
            forge_issue,
            forge_pulls,
            forge_pull,
            forge_pull_files,
            forge_comment,
            forge_set_state,
            forge_review,
            forge_review_comment,
            checkout_pull,
            open_url,
            merge_branch,
            merge_preview,
            rebase_onto,
            operation_step,
            resolve_conflict,
            mark_resolved,
            read_file_text,
            open_in_editor,
            commit_changes,
            head_message,
            git_sync,
            checkout_branch,
            create_branch,
            delete_branch,
            watch_repo,
            unwatch_repo,
            default_clone_dir,
            clone_name,
            clone_repo
        ])
        .run(tauri::generate_context!())
        .expect("error while running Whittle");
}
