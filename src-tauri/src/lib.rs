mod commands;
mod error;
mod git;
mod state;
mod storage;
mod watcher;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store = storage::Store::load(&data_dir);
            app.manage(state::AppState::new(store, data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_init,
            commands::repo_open,
            commands::repo_status,
            commands::repo_diff,
            commands::stage_file,
            commands::unstage_file,
            commands::discard_file,
            commands::stage_all,
            commands::unstage_all,
            commands::toggle_hunk,
            commands::toggle_lines,
            commands::commit,
            commands::repo_log,
            commands::repo_graph,
            commands::commit_files,
            commands::commit_diff,
            commands::repo_branches,
            commands::repo_remote_branches,
            commands::fetch_remotes,
            commands::pull_current,
            commands::push_current,
            commands::checkout_remote_branch,
            commands::checkout_branch,
            commands::create_branch,
            commands::merge_branch,
            commands::abort_merge,
            commands::rebase_branch,
            commands::abort_rebase,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
