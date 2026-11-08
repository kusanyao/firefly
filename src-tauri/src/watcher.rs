use crate::error::AppError;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const DEBOUNCE: Duration = Duration::from_millis(300);

/// 监听当前仓库；切换仓库时旧 watcher 被替换掉，对应线程会因为 sender 断开而退出。
pub fn watch_repo(
    app: AppHandle,
    repo: String,
    slot: &Mutex<Option<notify::RecommendedWatcher>>,
) -> Result<(), AppError> {
    let (tx, rx) = mpsc::channel::<Event>();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            if matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
            ) {
                let _ = tx.send(event);
            }
        }
    })
    .map_err(|e| AppError::with_detail("watch", "无法创建文件监视器", e.to_string()))?;

    watcher
        .watch(Path::new(&repo), RecursiveMode::Recursive)
        .map_err(|e| AppError::with_detail("watch", "无法监视该仓库目录", e.to_string()))?;

    *slot.lock().map_err(|_| AppError::internal("监视器状态锁失效"))? = Some(watcher);

    std::thread::spawn(move || {
        // recv() 返回 Err 说明 watcher 已被替换/销毁，线程自然结束。
        while rx.recv().is_ok() {
            while rx.try_recv().is_ok() {}
            std::thread::sleep(DEBOUNCE);
            while rx.try_recv().is_ok() {}
            let _ = app.emit("repo://changed", ());
        }
    });

    Ok(())
}
