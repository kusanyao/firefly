use crate::error::AppError;
use crate::git::branch::{
    abort_cherry_pick as abort_cherry_pick_impl, abort_merge as abort_merge_impl,
    abort_rebase as abort_rebase_impl, checkout_branch as switch_branch,
    checkout_remote_branch as checkout_remote_impl, cherry_pick as cherry_pick_impl,
    create_branch as create_branch_impl, fetch_remotes as fetch_remotes_impl,
    merge_branch as merge_branch_impl, pull_current as pull_current_impl,
    push_current as push_current_impl, read_branches, read_remote_branches,
    rebase_branch as rebase_branch_impl, BranchInfo, MergeOutcome, RemoteBranch,
};
use crate::git::diff::{read_commit_file_diff, read_file_diff, FileDiff, LineRef};
use crate::git::log::{
    read_all_commits, read_commit_files, read_graph, read_log, CommitFile, CommitSummary, GraphRow,
};
use crate::git::ops::{self, CommitResult};
use crate::git::status::{read_repo_root, read_status, RepoStatus};
use crate::state::AppState;
use crate::watcher;
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInit {
    pub recent: Vec<String>,
    pub last_repo: Option<String>,
    pub version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub root: String,
    pub status: RepoStatus,
}

#[tauri::command]
pub fn app_init(state: State<'_, AppState>) -> Result<AppInit, AppError> {
    let store = state
        .store
        .lock()
        .map_err(|_| AppError::internal("配置状态锁失效"))?;
    Ok(AppInit {
        recent: store.recent.clone(),
        last_repo: store.last_repo.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command(async)]
pub fn repo_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<RepoInfo, AppError> {
    let root = read_repo_root(&path)?;
    let status = read_status(&root)?;

    {
        let mut store = state
            .store
            .lock()
            .map_err(|_| AppError::internal("配置状态锁失效"))?;
        store.add_recent(&root);
        store.last_repo = Some(root.clone());
        store.save(&state.data_dir)?;
    }

    watcher::watch_repo(app, root.clone(), &state.watcher)?;

    Ok(RepoInfo { root, status })
}

#[tauri::command(async)]
pub fn repo_status(repo: String) -> Result<RepoStatus, AppError> {
    Ok(read_status(&repo)?)
}

#[tauri::command(async)]
pub fn repo_diff(repo: String, path: String, staged: bool) -> Result<FileDiff, AppError> {
    Ok(read_file_diff(&repo, &path, staged)?)
}

#[tauri::command(async)]
pub fn stage_file(repo: String, path: String) -> Result<(), AppError> {
    Ok(ops::stage_file(&repo, &path)?)
}

#[tauri::command(async)]
pub fn unstage_file(repo: String, path: String) -> Result<(), AppError> {
    Ok(ops::unstage_file(&repo, &path)?)
}

#[tauri::command(async)]
pub fn discard_file(repo: String, path: String) -> Result<(), AppError> {
    Ok(ops::discard_file(&repo, &path)?)
}

#[tauri::command(async)]
pub fn stage_all(repo: String) -> Result<(), AppError> {
    Ok(ops::stage_all(&repo)?)
}

#[tauri::command(async)]
pub fn unstage_all(repo: String) -> Result<(), AppError> {
    Ok(ops::unstage_all(&repo)?)
}

#[tauri::command(async)]
pub fn toggle_hunk(
    repo: String,
    path: String,
    hunk_index: usize,
    expected_header: String,
    staged: bool,
) -> Result<(), AppError> {
    Ok(ops::toggle_hunk(
        &repo,
        &path,
        hunk_index,
        &expected_header,
        staged,
    )?)
}

#[tauri::command(async)]
pub fn repo_branches(repo: String) -> Result<Vec<BranchInfo>, AppError> {
    Ok(read_branches(&repo)?)
}

#[tauri::command(async)]
pub fn repo_remote_branches(repo: String) -> Result<Vec<RemoteBranch>, AppError> {
    Ok(read_remote_branches(&repo)?)
}

#[tauri::command(async)]
pub fn fetch_remotes(repo: String) -> Result<String, AppError> {
    Ok(fetch_remotes_impl(&repo)?)
}

#[tauri::command(async)]
pub fn pull_current(repo: String) -> Result<MergeOutcome, AppError> {
    Ok(pull_current_impl(&repo)?)
}

#[tauri::command(async)]
pub fn push_current(repo: String) -> Result<String, AppError> {
    Ok(push_current_impl(&repo)?)
}

#[tauri::command(async)]
pub fn checkout_remote_branch(
    repo: String,
    remote: String,
    branch: String,
) -> Result<(), AppError> {
    Ok(checkout_remote_impl(&repo, &remote, &branch)?)
}

#[tauri::command(async)]
pub fn checkout_branch(repo: String, branch: String) -> Result<(), AppError> {
    Ok(switch_branch(&repo, &branch)?)
}

#[tauri::command(async)]
pub fn create_branch(repo: String, branch: String) -> Result<(), AppError> {
    Ok(create_branch_impl(&repo, &branch)?)
}

#[tauri::command(async)]
pub fn merge_branch(repo: String, branch: String) -> Result<MergeOutcome, AppError> {
    Ok(merge_branch_impl(&repo, &branch)?)
}

#[tauri::command(async)]
pub fn abort_merge(repo: String) -> Result<(), AppError> {
    Ok(abort_merge_impl(&repo)?)
}

#[tauri::command(async)]
pub fn rebase_branch(repo: String, branch: String) -> Result<MergeOutcome, AppError> {
    Ok(rebase_branch_impl(&repo, &branch)?)
}

#[tauri::command(async)]
pub fn abort_rebase(repo: String) -> Result<(), AppError> {
    Ok(abort_rebase_impl(&repo)?)
}

#[tauri::command(async)]
pub fn cherry_pick(repo: String, hash: String) -> Result<MergeOutcome, AppError> {
    Ok(cherry_pick_impl(&repo, &hash)?)
}

#[tauri::command(async)]
pub fn abort_cherry_pick(repo: String) -> Result<(), AppError> {
    Ok(abort_cherry_pick_impl(&repo)?)
}

#[tauri::command(async)]
pub fn repo_log(repo: String, limit: Option<usize>) -> Result<Vec<CommitSummary>, AppError> {
    Ok(read_log(&repo, limit.unwrap_or(100))?)
}

#[tauri::command(async)]
pub fn all_commits(repo: String, limit: Option<usize>) -> Result<Vec<CommitSummary>, AppError> {
    Ok(read_all_commits(&repo, limit.unwrap_or(100))?)
}

#[tauri::command(async)]
pub fn repo_graph(repo: String, limit: Option<usize>) -> Result<Vec<GraphRow>, AppError> {
    Ok(read_graph(&repo, limit.unwrap_or(100))?)
}

#[tauri::command(async)]
pub fn commit_files(repo: String, hash: String) -> Result<Vec<CommitFile>, AppError> {
    Ok(read_commit_files(&repo, &hash)?)
}

#[tauri::command(async)]
pub fn commit_diff(repo: String, hash: String, path: String) -> Result<FileDiff, AppError> {
    Ok(read_commit_file_diff(&repo, &hash, &path)?)
}

#[tauri::command(async)]
pub fn toggle_lines(
    repo: String,
    path: String,
    lines: Vec<LineRef>,
    staged: bool,
) -> Result<(), AppError> {
    Ok(ops::toggle_lines(&repo, &path, &lines, staged)?)
}

#[tauri::command(async)]
pub fn commit(repo: String, message: String, amend: bool) -> Result<CommitResult, AppError> {
    if message.trim().is_empty() {
        return Err(AppError::new("invalid_input", "提交信息不能为空"));
    }
    Ok(ops::commit(&repo, &message, amend)?)
}
