use super::{decode_bytes, run_git, run_git_bytes, GitResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub timestamp: i64,
    pub subject: String,
    pub refs: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    pub orig_path: Option<String>,
    /// 单字母状态：A/M/D/R/C/T
    pub status: String,
}

/// 分支树的一行：`graph` 是 git 画出来的前缀（含 `*` / `|` / `\` 等），
/// 没有提交的是连接线行。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRow {
    pub graph: String,
    pub commit: Option<CommitSummary>,
}

pub fn read_log(repo: &str, limit: usize) -> GitResult<Vec<CommitSummary>> {
    let limit = limit.clamp(1, 1000);
    let limit_arg = format!("-n{limit}");
    let bytes = run_git_bytes(
        repo,
        &[
            "log",
            limit_arg.as_str(),
            "--date-order",
            "--format=%H%x1f%h%x1f%an%x1f%at%x1f%s%x1f%D%x1e",
        ],
        None,
    )?;
    Ok(parse_log(&decode_bytes(&bytes)))
}

pub fn read_commit_files(repo: &str, hash: &str) -> GitResult<Vec<CommitFile>> {
    // 合并提交默认不输出文件列表，-m 取与第一个父提交的差异。
    let bytes = run_git_bytes(
        repo,
        &[
            "show",
            "--first-parent",
            "-m",
            "--format=",
            "--name-status",
            "-z",
            hash,
        ],
        None,
    )?;
    Ok(parse_name_status(&bytes))
}

pub fn read_graph(repo: &str, limit: usize) -> GitResult<Vec<GraphRow>> {
    let limit = limit.clamp(1, 1000);
    let limit_arg = format!("-n{limit}");
    // %x01 是哨兵：它前面是图形前缀，后面是提交字段。
    // 只取当前 HEAD 的历史，不再使用 --all。
    let out = run_git(
        repo,
        &[
            "log",
            limit_arg.as_str(),
            "--graph",
            "--date-order",
            // 用全名（refs/heads/…、refs/remotes/…）才能准确区分本地/远程/tag。
            "--decorate=full",
            "--format=%x01%H%x1f%h%x1f%an%x1f%at%x1f%s%x1f%D",
        ],
    )?;
    Ok(parse_graph(&out))
}

fn parse_log(text: &str) -> Vec<CommitSummary> {
    text.split('\x1e')
        .filter_map(|record| {
            // 每条记录后面 git 会补一个换行
            let record = record.trim_start_matches(['\r', '\n']);
            if record.is_empty() {
                return None;
            }
            parse_commit_fields(record)
        })
        .collect()
}

fn parse_commit_fields(rest: &str) -> Option<CommitSummary> {
    let mut fields = rest.split('\x1f');
    let hash = fields.next().unwrap_or_default();
    if hash.is_empty() {
        return None;
    }
    Some(CommitSummary {
        hash: hash.to_string(),
        short_hash: fields.next().unwrap_or_default().to_string(),
        author: fields.next().unwrap_or_default().to_string(),
        timestamp: fields.next().and_then(|v| v.parse().ok()).unwrap_or(0),
        subject: fields.next().unwrap_or_default().to_string(),
        refs: fields.next().unwrap_or_default().to_string(),
    })
}

fn parse_graph(text: &str) -> Vec<GraphRow> {
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        match line.split_once('\x01') {
            Some((graph, rest)) => rows.push(GraphRow {
                graph: graph.to_string(),
                commit: parse_commit_fields(rest),
            }),
            // 纯连接线行，例如 "| |"、"|/"
            None => rows.push(GraphRow {
                graph: line.to_string(),
                commit: None,
            }),
        }
    }
    rows
}

fn parse_name_status(bytes: &[u8]) -> Vec<CommitFile> {
    let mut files = Vec::new();
    let mut tokens = bytes.split(|b| *b == 0);

    while let Some(status) = tokens.next() {
        if status.is_empty() {
            continue;
        }
        let status = decode_bytes(status);
        let kind = status.chars().next().unwrap_or('M');

        // 重命名/复制记录：<status>\0<old>\0<new>\0
        if kind == 'R' || kind == 'C' {
            let old = tokens.next().map(decode_bytes);
            let new = tokens.next().map(decode_bytes);
            if let (Some(old), Some(new)) = (old, new) {
                files.push(CommitFile {
                    path: new,
                    orig_path: Some(old),
                    status: kind.to_string(),
                });
            }
            continue;
        }

        if let Some(path) = tokens.next() {
            files.push(CommitFile {
                path: decode_bytes(path),
                orig_path: None,
                status: kind.to_string(),
            });
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_log_records() {
        let raw = "abc123\x1fabc\x1f张三\x1f1730000000\x1f修复登录问题\x1fHEAD -> main\x1e\n\
def456\x1fdef\x1f李四\x1f1729000000\x1f初始化\x1f\x1e\n";
        let commits = parse_log(raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].author, "张三");
        assert_eq!(commits[0].timestamp, 1730000000);
        assert_eq!(commits[0].subject, "修复登录问题");
        assert_eq!(commits[0].refs, "HEAD -> main");
        assert!(commits[1].refs.is_empty());
    }

    #[test]
    fn parses_graph_rows() {
        let raw = "* \x01abc\x1fabc\x1f张三\x1f1730000000\x1ffeature work\x1fHEAD -> feature\n\
| * \x01def\x1fdef\x1f李四\x1f1729000000\x1fmain work\x1fmain\n\
|/\n\
* \x01base\x1fbase\x1f王五\x1f1728000000\x1fbase\x1f\n";

        let rows = parse_graph(raw);
        assert_eq!(rows.len(), 4);

        assert_eq!(rows[0].graph, "* ");
        assert_eq!(rows[0].commit.as_ref().unwrap().subject, "feature work");

        assert_eq!(rows[1].graph, "| * ");
        assert_eq!(rows[1].commit.as_ref().unwrap().subject, "main work");

        assert_eq!(rows[2].graph, "|/");
        assert!(rows[2].commit.is_none());

        assert_eq!(rows[3].commit.as_ref().unwrap().subject, "base");
    }

    #[test]
    fn parses_name_status_with_rename() {
        let raw = b"M\0src/main.rs\0A\0new.txt\0R100\0old.txt\0new.txt\0";
        let files = parse_name_status(raw);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].status, "M");
        assert_eq!(files[1].status, "A");
        assert_eq!(files[2].status, "R");
        assert_eq!(files[2].orig_path.as_deref(), Some("old.txt"));
        assert_eq!(files[2].path, "new.txt");
    }
}
