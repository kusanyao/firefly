use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const MAX_RECENT: usize = 10;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Store {
    pub recent: Vec<String>,
    pub last_repo: Option<String>,
}

impl Store {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("recent.json");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                // 主文件不存在就尝试 .bak，两者都没有则从空状态开始。
                match std::fs::read(data_dir.join("recent.json.bak")) {
                    Ok(bytes) => bytes,
                    Err(_) => return Self::default(),
                }
            }
        };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    pub fn add_recent(&mut self, repo: &str) {
        self.recent.retain(|r| !r.eq_ignore_ascii_case(repo));
        self.recent.insert(0, repo.to_string());
        self.recent.truncate(MAX_RECENT);
    }

    /// 临时文件 + fsync + 备份 + rename。Windows 上杀毒软件可能短时间占用目标文件，
    /// 所以 rename 失败时重试几次。
    pub fn save(&self, data_dir: &Path) -> Result<(), AppError> {
        std::fs::create_dir_all(data_dir)?;
        let path = data_dir.join("recent.json");
        let temp = data_dir.join("recent.json.tmp");
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| AppError::with_detail("internal", "序列化配置失败", e.to_string()))?;

        std::fs::write(&temp, &bytes)?;

        if path.exists() {
            let _ = std::fs::copy(&path, data_dir.join("recent.json.bak"));
        }

        let mut last_err = None;
        for _ in 0..4 {
            match std::fs::rename(&temp, &path) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    last_err = Some(err);
                    std::thread::sleep(Duration::from_millis(60));
                }
            }
        }

        Err(AppError::with_detail(
            "io",
            "保存最近仓库列表失败",
            last_err.map(|e| e.to_string()).unwrap_or_default(),
        ))
    }
}
