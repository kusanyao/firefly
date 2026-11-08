use crate::git::GitError;
use serde::Serialize;

/// 与蜂鸟保持一致：命令统一返回 AppError，`kind` 是机器可读标签，
/// `message` 是直接展示给用户的中文，`detail` 是技术细节。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(kind: &str, message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<GitError> for AppError {
    fn from(err: GitError) -> Self {
        let detail = err.stderr.trim();
        Self {
            kind: "git".to_string(),
            message: err.message,
            detail: if detail.is_empty() {
                None
            } else {
                Some(detail.to_string())
            },
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::with_detail("io", "文件读写失败", err.to_string())
    }
}
