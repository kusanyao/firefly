use super::{decode_bytes, run_git, run_git_output, GitError, GitResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub current: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBranch {
    /// 完整短名，例如 `origin/main`
    pub name: String,
    pub remote: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    /// 是否停在冲突状态（合并未完成，需要解决后提交或中止）。
    pub conflict: bool,
    pub message: String,
}

pub fn read_branches(repo: &str) -> GitResult<Vec<BranchInfo>> {
    // for-each-ref 不支持 %x1f 这类转义，只能靠字面分隔符；分支名里不允许出现制表符。
    let out = run_git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)\t%(HEAD)\t%(upstream:short)",
            "refs/heads",
        ],
    )?;
    Ok(parse_branches(&out))
}

pub fn read_remote_branches(repo: &str) -> GitResult<Vec<RemoteBranch>> {
    let out = run_git(
        repo,
        &["for-each-ref", "--format=%(refname:short)", "refs/remotes"],
    )?;
    Ok(parse_remote_branches(&out))
}

fn parse_remote_branches(text: &str) -> Vec<RemoteBranch> {
    let mut list: Vec<RemoteBranch> = text
        .lines()
        .filter_map(|line| {
            let name = line.trim();
            if name.is_empty() || name.ends_with("/HEAD") {
                return None;
            }
            let (remote, branch) = name.split_once('/')?;
            Some(RemoteBranch {
                name: name.to_string(),
                remote: remote.to_string(),
                branch: branch.to_string(),
            })
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}

/// 把远程分支检出为同名的本地跟踪分支。
pub fn checkout_remote_branch(repo: &str, remote: &str, branch: &str) -> GitResult<()> {
    if remote.is_empty()
        || branch.is_empty()
        || remote.starts_with('-')
        || branch.starts_with('-')
    {
        return Err(GitError::new("无效的远程分支"));
    }
    let remote_ref = format!("{remote}/{branch}");
    run_git(repo, &["switch", "-c", branch, "--track", &remote_ref])?;
    Ok(())
}

/// 拉取当前分支的上游更新。冲突不算失败，返回 `conflict = true`。
pub fn pull_current(repo: &str) -> GitResult<MergeOutcome> {
    let output = run_git_output(repo, &["pull", "--no-edit"], None)?;
    let stdout = decode_bytes(&output.stdout);
    let stderr = decode_bytes(&output.stderr);

    if output.status.success() {
        return Ok(MergeOutcome {
            conflict: false,
            message: format!("{stdout}{stderr}").trim().to_string(),
        });
    }

    let combined = format!("{stdout}\n{stderr}");
    let conflict = combined.contains("CONFLICT")
        || combined.contains("Automatic merge failed")
        || combined.contains("Merge conflict");

    if conflict {
        Ok(MergeOutcome {
            conflict: true,
            message: combined.trim().to_string(),
        })
    } else {
        Err(GitError {
            message: if stderr.trim().is_empty() {
                format!("拉取失败：{}", stdout.trim())
            } else {
                format!("拉取失败：{}", stderr.trim())
            },
            stderr,
            code: output.status.code(),
        })
    }
}

pub fn push_current(repo: &str) -> GitResult<String> {
    let output = run_git_output(repo, &["push"], None)?;
    let stdout = decode_bytes(&output.stdout);
    let stderr = decode_bytes(&output.stderr);

    if output.status.success() {
        let message = format!("{stdout}{stderr}").trim().to_string();
        Ok(if message.is_empty() {
            "已推送".to_string()
        } else {
            message
        })
    } else {
        Err(GitError {
            message: if stderr.trim().is_empty() {
                format!("推送失败：{}", stdout.trim())
            } else {
                format!("推送失败：{}", stderr.trim())
            },
            stderr,
            code: output.status.code(),
        })
    }
}

/// 把某个提交遴选到当前分支。冲突不算失败，返回 `conflict = true`。
pub fn cherry_pick(repo: &str, hash: &str) -> GitResult<MergeOutcome> {
    if hash.is_empty() || hash.starts_with('-') {
        return Err(GitError::new("无效的提交"));
    }

    let output = run_git_output(repo, &["cherry-pick", hash], None)?;
    let stdout = decode_bytes(&output.stdout);
    let stderr = decode_bytes(&output.stderr);

    if output.status.success() {
        return Ok(MergeOutcome {
            conflict: false,
            message: format!("{stdout}{stderr}").trim().to_string(),
        });
    }

    let combined = format!("{stdout}\n{stderr}");
    let conflict = combined.contains("CONFLICT")
        || combined.contains("could not apply")
        || combined.contains("After resolving the conflicts");

    if conflict {
        Ok(MergeOutcome {
            conflict: true,
            message: combined.trim().to_string(),
        })
    } else {
        Err(GitError {
            message: if stderr.trim().is_empty() {
                format!("遴选失败：{}", stdout.trim())
            } else {
                format!("遴选失败：{}", stderr.trim())
            },
            stderr,
            code: output.status.code(),
        })
    }
}

pub fn abort_cherry_pick(repo: &str) -> GitResult<()> {
    run_git(repo, &["cherry-pick", "--abort"])?;
    Ok(())
}

/// 抓取所有远程并清理已删除的远程跟踪分支。
pub fn fetch_remotes(repo: &str) -> GitResult<String> {
    let output = run_git_output(repo, &["fetch", "--all", "--prune"], None)?;
    let stdout = decode_bytes(&output.stdout);
    let stderr = decode_bytes(&output.stderr);

    if output.status.success() {
        let message = format!("{stdout}{stderr}").trim().to_string();
        Ok(if message.is_empty() {
            "已是最新".to_string()
        } else {
            message
        })
    } else {
        Err(GitError {
            message: if stderr.trim().is_empty() {
                "抓取远程失败".to_string()
            } else {
                format!("抓取远程失败：{}", stderr.trim())
            },
            stderr,
            code: output.status.code(),
        })
    }
}

pub fn checkout_branch(repo: &str, branch: &str) -> GitResult<()> {
    // 分支名来自 git 自己的输出，这里只做一道最低限度的防注入检查。
    if branch.is_empty() || branch.starts_with('-') {
        return Err(GitError::new("无效的分支名"));
    }
    run_git(repo, &["switch", branch])?;
    Ok(())
}

/// 把指定分支合并进当前分支。冲突不算命令失败，而是返回 `conflict = true`，
/// 让界面进入「合并中」状态而不是单纯报错。
pub fn merge_branch(repo: &str, branch: &str) -> GitResult<MergeOutcome> {
    if branch.is_empty() || branch.starts_with('-') {
        return Err(GitError::new("无效的分支名"));
    }

    let output = run_git_output(repo, &["merge", "--no-edit", branch], None)?;
    let stdout = decode_bytes(&output.stdout);
    let stderr = decode_bytes(&output.stderr);

    if output.status.success() {
        return Ok(MergeOutcome {
            conflict: false,
            message: stdout.trim().to_string(),
        });
    }

    // 冲突信息在不同 git 版本里可能落在 stdout 或 stderr，两边都要看。
    let combined = format!("{stdout}\n{stderr}");
    let conflict = combined.contains("CONFLICT")
        || combined.contains("Automatic merge failed")
        || combined.contains("Merge conflict");

    if conflict {
        Ok(MergeOutcome {
            conflict: true,
            message: combined.trim().to_string(),
        })
    } else {
        Err(GitError {
            message: if stderr.trim().is_empty() {
                "合并失败".to_string()
            } else {
                format!("合并失败：{}", stderr.trim())
            },
            stderr,
            code: output.status.code(),
        })
    }
}

pub fn abort_merge(repo: &str) -> GitResult<()> {
    run_git(repo, &["merge", "--abort"])?;
    Ok(())
}

/// 把当前分支变基到指定分支。冲突同样不算失败，返回 `conflict = true`。
pub fn rebase_branch(repo: &str, branch: &str) -> GitResult<MergeOutcome> {
    if branch.is_empty() || branch.starts_with('-') {
        return Err(GitError::new("无效的分支名"));
    }

    let output = run_git_output(repo, &["rebase", branch], None)?;
    let stdout = decode_bytes(&output.stdout);
    let stderr = decode_bytes(&output.stderr);

    if output.status.success() {
        return Ok(MergeOutcome {
            conflict: false,
            message: stdout.trim().to_string(),
        });
    }

    let combined = format!("{stdout}\n{stderr}");
    let conflict = combined.contains("CONFLICT")
        || combined.contains("could not apply")
        || combined.contains("Resolve all conflicts");

    if conflict {
        Ok(MergeOutcome {
            conflict: true,
            message: combined.trim().to_string(),
        })
    } else {
        Err(GitError {
            message: if stderr.trim().is_empty() {
                "变基失败".to_string()
            } else {
                format!("变基失败：{}", stderr.trim())
            },
            stderr,
            code: output.status.code(),
        })
    }
}

pub fn abort_rebase(repo: &str) -> GitResult<()> {
    run_git(repo, &["rebase", "--abort"])?;
    Ok(())
}

/// 从当前 HEAD 创建并切换到新分支。名字合法性交给 git 校验。
pub fn create_branch(repo: &str, branch: &str) -> GitResult<()> {
    let branch = branch.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err(GitError::new("分支名不能为空，也不能以 - 开头"));
    }
    run_git(repo, &["switch", "-c", branch])?;
    Ok(())
}

fn parse_branches(text: &str) -> Vec<BranchInfo> {
    let mut branches = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let name = fields.next().unwrap_or_default().trim();
        if name.is_empty() {
            continue;
        }
        // 非当前分支时 %(HEAD) 输出的是空格而不是空串
        let head = fields.next().unwrap_or_default().trim();
        let upstream = fields.next().unwrap_or_default().trim();
        branches.push(BranchInfo {
            name: name.to_string(),
            current: head == "*",
            upstream: if upstream.is_empty() {
                None
            } else {
                Some(upstream.to_string())
            },
        });
    }
    branches.sort_by(|a, b| b.current.cmp(&a.current).then_with(|| a.name.cmp(&b.name)));
    branches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_remote_branches() {
        let raw = "origin/main\norigin/feature/x\nupstream/dev\norigin/HEAD\n";
        let list = parse_remote_branches(raw);
        assert_eq!(list.len(), 3);
        // 按名字排序，且过滤掉 */HEAD
        assert_eq!(list[0].name, "origin/feature/x");
        assert_eq!(list[0].remote, "origin");
        assert_eq!(list[0].branch, "feature/x");
        assert_eq!(list[2].name, "upstream/dev");
        assert_eq!(list[2].remote, "upstream");
    }

    #[test]
    fn parses_branches_current_first() {
        let raw = "main\t*\torigin/main\nfeature/login\t \t\n";
        let branches = parse_branches(raw);
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].current);
        assert_eq!(branches[0].upstream.as_deref(), Some("origin/main"));
        assert_eq!(branches[1].name, "feature/login");
        assert!(!branches[1].current);
        assert!(branches[1].upstream.is_none());
    }
}
