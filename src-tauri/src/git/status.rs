use super::{field_after, run_git, GitResult};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusEntry {
    pub path: String,
    pub orig_path: Option<String>,
    pub index_status: char,
    pub worktree_status: char,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub conflicted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub detached: bool,
    pub initial: bool,
    /// 是否有进行中的合并（.git/MERGE_HEAD 存在）。
    pub merging: bool,
    /// 是否有进行中的变基（.git/rebase-merge 或 rebase-apply 存在）。
    pub rebasing: bool,
    /// 是否有进行中的遴选（.git/CHERRY_PICK_HEAD 存在）。
    pub cherry_picking: bool,
    pub entries: Vec<StatusEntry>,
}

pub fn read_status(repo: &str) -> GitResult<RepoStatus> {
    let output = run_git(repo, &["status", "--porcelain=v2", "--branch", "-z"])?;
    let mut status = parse_status(&output);

    // 一次 rev-parse 拿到 git 目录，再直接看状态文件，避免每次刷新多跑两个进程。
    if let Ok(git_dir) = run_git(repo, &["rev-parse", "--git-dir"]) {
        let dir = PathBuf::from(git_dir.trim());
        let base = if dir.is_absolute() {
            dir
        } else {
            Path::new(repo).join(dir)
        };
        status.merging = base.join("MERGE_HEAD").exists();
        status.rebasing =
            base.join("rebase-merge").exists() || base.join("rebase-apply").exists();
        status.cherry_picking = base.join("CHERRY_PICK_HEAD").exists();
    }

    Ok(status)
}

pub fn read_repo_root(repo: &str) -> GitResult<String> {
    let out = run_git(repo, &["rev-parse", "--show-toplevel"])?;
    Ok(out.trim_end_matches(['\r', '\n']).to_string())
}

pub fn parse_status(output: &str) -> RepoStatus {
    let mut status = RepoStatus {
        branch: None,
        upstream: None,
        ahead: 0,
        behind: 0,
        detached: false,
        initial: false,
        merging: false,
        rebasing: false,
        cherry_picking: false,
        entries: Vec::new(),
    };

    let mut tokens = output.split('\0');
    while let Some(record) = tokens.next() {
        if record.is_empty() {
            continue;
        }

        match record.as_bytes()[0] {
            b'#' => {
                let Some(rest) = record.strip_prefix("# ") else {
                    continue;
                };
                let (key, value) = rest.split_once(' ').unwrap_or((rest, ""));
                match key {
                    "branch.oid" => status.initial = value == "(initial)",
                    "branch.head" => {
                        if value == "(detached)" {
                            status.detached = true;
                        } else {
                            status.branch = Some(value.to_string());
                        }
                    }
                    "branch.upstream" => status.upstream = Some(value.to_string()),
                    "branch.ab" => {
                        for part in value.split_whitespace() {
                            if let Some(n) = part.strip_prefix('+') {
                                status.ahead = n.parse().unwrap_or(0);
                            } else if let Some(n) = part.strip_prefix('-') {
                                status.behind = n.parse().unwrap_or(0);
                            }
                        }
                    }
                    _ => {}
                }
            }
            b'1' => {
                if let Some(path) = field_after(record, 8) {
                    let (x, y) = xy(record);
                    status.entries.push(make_entry(path, None, x, y, false, false));
                }
            }
            b'2' => {
                let path = field_after(record, 9).map(str::to_string);
                let orig = tokens.next().map(str::to_string);
                if let Some(path) = path {
                    let (x, y) = xy(record);
                    status
                        .entries
                        .push(make_entry(&path, orig, x, y, false, false));
                }
            }
            b'u' => {
                if let Some(path) = field_after(record, 10) {
                    let (x, y) = xy(record);
                    status
                        .entries
                        .push(make_entry(path, None, x, y, false, true));
                }
            }
            b'?' => {
                if let Some(path) = record.get(2..) {
                    status.entries.push(make_entry(path, None, '?', '?', true, false));
                }
            }
            b'!' => {}
            _ => {}
        }
    }

    status
}

fn xy(record: &str) -> (char, char) {
    let bytes = record.as_bytes();
    let x = bytes.get(2).copied().unwrap_or(b'.') as char;
    let y = bytes.get(3).copied().unwrap_or(b'.') as char;
    (x, y)
}

fn make_entry(
    path: &str,
    orig_path: Option<String>,
    index_status: char,
    worktree_status: char,
    untracked: bool,
    conflicted: bool,
) -> StatusEntry {
    StatusEntry {
        path: path.to_string(),
        orig_path,
        index_status,
        worktree_status,
        staged: !untracked && !conflicted && index_status != '.',
        // 冲突文件列在「未暂存」里，界面上单独标记；整文件 add 即标记为已解决。
        unstaged: conflicted || untracked || worktree_status != '.',
        untracked,
        conflicted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_and_entries() {
        let raw = "# branch.oid abc123\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 aaa bbb src/main.rs\0\
1 M. N... 100644 100644 100644 ccc ddd 中文 文件.md\0\
? new file.txt\0";
        let status = parse_status(raw);
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 1);
        assert_eq!(status.entries.len(), 3);

        let unstaged = &status.entries[0];
        assert_eq!(unstaged.path, "src/main.rs");
        assert!(unstaged.unstaged && !unstaged.staged);

        let staged = &status.entries[1];
        assert_eq!(staged.path, "中文 文件.md");
        assert!(staged.staged && !staged.unstaged);

        let untracked = &status.entries[2];
        assert!(untracked.untracked && untracked.unstaged);
    }

    #[test]
    fn parses_rename_with_orig_path() {
        let raw = "2 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\0old name.txt\0";
        let status = parse_status(raw);
        assert_eq!(status.entries.len(), 1);
        assert_eq!(status.entries[0].path, "new name.txt");
        assert_eq!(status.entries[0].orig_path.as_deref(), Some("old name.txt"));
        assert!(status.entries[0].staged);
    }
}
