pub mod branch;
pub mod diff;
pub mod log;
pub mod ops;
pub mod status;

use serde::Serialize;
use std::io::Write;
use std::process::{Command, Output, Stdio};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitError {
    pub message: String,
    pub stderr: String,
    pub code: Option<i32>,
}

impl GitError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            stderr: String::new(),
            code: None,
        }
    }

    fn from_output(output: &Output, context: &str) -> Self {
        let stderr = decode_bytes(&output.stderr);
        let detail = stderr.trim();
        let message = if detail.is_empty() {
            context.to_string()
        } else {
            format!("{context}：{detail}")
        };
        Self {
            message,
            stderr,
            code: output.status.code(),
        }
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GitError {}

pub type GitResult<T> = Result<T, GitError>;

fn git_command(repo: &str) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        .arg("-c")
        .arg("core.quotepath=false")
        .arg("-c")
        .arg("color.ui=false")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW：避免每次调用 git 都闪一下黑窗
        cmd.creation_flags(0x0800_0000);
    }

    cmd
}

pub fn run_git(repo: &str, args: &[&str]) -> GitResult<String> {
    Ok(decode_bytes(&run_git_bytes(repo, args, None)?))
}

/// 执行 git 并原样返回 Output，不检查退出码。
/// 合并冲突这类「非零退出但属于正常业务状态」的命令需要自己看 stdout/stderr。
pub fn run_git_output(
    repo: &str,
    args: &[&str],
    stdin: Option<&[u8]>,
) -> GitResult<std::process::Output> {
    let mut cmd = git_command(repo);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });

    let mut child = cmd.spawn().map_err(|e| {
        GitError::new(format!(
            "无法启动 git，请确认已安装 Git for Windows 并在 PATH 中：{e}"
        ))
    })?;

    if let Some(data) = stdin {
        if let Some(mut pipe) = child.stdin.take() {
            pipe.write_all(data)
                .map_err(|e| GitError::new(format!("写入 git 标准输入失败：{e}")))?;
        }
    }

    child
        .wait_with_output()
        .map_err(|e| GitError::new(format!("等待 git 进程失败：{e}")))
}

pub fn run_git_bytes(repo: &str, args: &[&str], stdin: Option<&[u8]>) -> GitResult<Vec<u8>> {
    let output = run_git_output(repo, args, stdin)?;

    if !output.status.success() {
        // 开发期诊断：stderr 为空的失败在界面上看不出是哪个命令。
        #[cfg(debug_assertions)]
        eprintln!(
            "git {:?} failed ({:?}): {}",
            args,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(GitError::from_output(&output, "git 命令执行失败"));
    }

    Ok(output.stdout)
}

/// Git for Windows 输出统一按 UTF-8 处理；遇到非法 UTF-8 的行再按 GB18030
/// 兜底解码（老的中文文件常见）。逐行判断可以避免一行 GBK 破坏整份 diff。
pub fn decode_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let mut out = String::with_capacity(bytes.len());
            for line in bytes.split_inclusive(|b| *b == b'\n') {
                match std::str::from_utf8(line) {
                    Ok(s) => out.push_str(s),
                    Err(_) => {
                        let (decoded, _, _) = encoding_rs::GB18030.decode(line);
                        out.push_str(&decoded);
                    }
                }
            }
            out
        }
    }
}

/// 取一条 porcelain 记录里第 `idx` 个空格分隔字段之后的内容（路径可能含空格）。
pub(crate) fn field_after(record: &str, idx: usize) -> Option<&str> {
    record.splitn(idx + 1, ' ').nth(idx)
}
