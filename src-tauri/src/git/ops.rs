use super::diff::{
    build_hunk_patch, build_selected_patch, read_file_diff, DiffLineKind, FileDiff, LineRef,
};
use std::collections::HashSet;
use super::{run_git, run_git_bytes, GitError, GitResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitResult {
    pub hash: String,
    pub subject: String,
}

pub fn stage_file(repo: &str, path: &str) -> GitResult<()> {
    run_git(repo, &["add", "--", path])?;
    Ok(())
}

pub fn stage_all(repo: &str) -> GitResult<()> {
    run_git(repo, &["add", "-A"])?;
    Ok(())
}

pub fn unstage_file(repo: &str, path: &str) -> GitResult<()> {
    match run_git(repo, &["restore", "--staged", "--", path]) {
        Ok(_) => Ok(()),
        Err(err) => {
            // 还没有任何提交时 HEAD 不存在，restore 会失败；退化为从 index 移除。
            if err.stderr.contains("could not resolve HEAD") || err.message.contains("HEAD") {
                run_git(repo, &["rm", "--cached", "--", path])?;
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

pub fn unstage_all(repo: &str) -> GitResult<()> {
    match run_git(repo, &["reset", "-q", "HEAD", "--"]) {
        Ok(_) => Ok(()),
        Err(err) => {
            if err.message.contains("HEAD") {
                run_git(repo, &["rm", "-r", "--cached", "--", "."])?;
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

/// 暂存/取消暂存单个 hunk。
/// - `staged = false`：从工作区 diff 取 hunk，正向应用到 index；
/// - `staged = true`：从 index diff 取 hunk，反向应用（即取消暂存）。
pub fn toggle_hunk(
    repo: &str,
    path: &str,
    hunk_index: usize,
    expected_header: &str,
    staged: bool,
) -> GitResult<()> {
    let diff = read_file_diff(repo, path, staged)?;

    let hunk = diff
        .hunks
        .get(hunk_index)
        .ok_or_else(|| GitError::new("该改动块已不存在，请刷新后重试"))?;

    // 前端看到的内容和现在重新读取的如果对不上，宁可报错也不要暂存错东西。
    if !expected_header.is_empty() && hunk.header != expected_header {
        return Err(GitError::new("文件内容已变化，请刷新后重试"));
    }

    let patch = build_hunk_patch(&diff, hunk_index)
        .ok_or_else(|| GitError::new("无法重建该改动块"))?;

    let mut args: Vec<&str> = vec![
        "apply",
        "--cached",
        "--recount",
        "--whitespace=nowarn",
    ];
    if staged {
        args.push("--reverse");
    }
    args.push("-");

    run_git_bytes(repo, &args, Some(&patch))?;
    Ok(())
}

/// 丢弃工作区里某个文件的未暂存改动。
/// 已跟踪文件恢复为 index 版本；未跟踪文件直接从磁盘删除。
pub fn discard_file(repo: &str, path: &str) -> GitResult<()> {
    if path.trim().is_empty() {
        return Err(GitError::new("文件路径为空"));
    }

    let tracked = run_git(repo, &["ls-files", "--error-unmatch", "--", path]).is_ok();
    if tracked {
        run_git(repo, &["restore", "--worktree", "--", path])?;
    } else {
        run_git(repo, &["clean", "-f", "-d", "--", path])?;
    }
    Ok(())
}

/// 暂存/取消暂存任意选中的行。
/// - `staged = false`：从工作区 diff 取行，正向应用到 index；
/// - `staged = true`：从 index diff 取行，重建一个正向补丁把 index 恢复到目标内容。
pub fn toggle_lines(repo: &str, path: &str, refs: &[LineRef], staged: bool) -> GitResult<()> {
    if refs.is_empty() {
        return Err(GitError::new("没有选中任何行"));
    }

    let diff = read_file_diff(repo, path, staged)?;

    // 新文件的所有行都被取消暂存时，把文件整体移出 index，
    // 否则会在 index 里留下一个空文件。
    if staged && is_new_file(&diff) && selects_all_added_lines(&diff, refs) {
        return unstage_file(repo, path);
    }

    let patch = build_selected_patch(&diff, refs, staged)
        .ok_or_else(|| GitError::new("选中的行里没有可暂存的新增/删除内容"))?;

    run_git_bytes(
        repo,
        &["apply", "--cached", "--recount", "--whitespace=nowarn", "-"],
        Some(&patch),
    )?;
    Ok(())
}

fn is_new_file(diff: &FileDiff) -> bool {
    diff.header_lines
        .iter()
        .any(|line| line.starts_with("new file mode"))
}

fn selects_all_added_lines(diff: &FileDiff, refs: &[LineRef]) -> bool {
    let selected: HashSet<(usize, usize)> = refs.iter().map(|r| (r.hunk, r.line)).collect();
    let mut any = false;
    for hunk in &diff.hunks {
        for (index, line) in hunk.lines.iter().enumerate() {
            match line.kind {
                DiffLineKind::Add => {
                    any = true;
                    if !selected.contains(&(hunk.index, index)) {
                        return false;
                    }
                }
                DiffLineKind::Del => return false,
                _ => {}
            }
        }
    }
    any
}

pub fn commit(repo: &str, message: &str, amend: bool) -> GitResult<CommitResult> {
    let mut args: Vec<&str> = vec!["commit", "-F", "-"];
    if amend {
        args.push("--amend");
    }
    run_git_bytes(repo, &args, Some(message.as_bytes()))?;

    let out = run_git(repo, &["log", "-1", "--format=%h%n%s"])?;
    let mut lines = out.lines();
    Ok(CommitResult {
        hash: lines.next().unwrap_or_default().to_string(),
        subject: lines.next().unwrap_or_default().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
                .args(args)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init", "-q", "-b", "main"]);
    }

    #[test]
    fn stages_only_selected_hunk() {
        let dir = std::env::temp_dir().join(format!("firefly-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init_repo(&dir);

        let file = dir.join("file.txt");
        let original: String = (1..=30).map(|i| format!("line {i}\n")).collect();
        std::fs::write(&file, &original).unwrap();
        let repo = dir.to_str().unwrap();
        stage_file(repo, "file.txt").unwrap();
        // 直接调用 git commit 避免依赖用户全局配置
        std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["-c", "user.name=T", "-c", "user.email=t@e.com", "commit", "-qm", "init"])
            .status()
            .unwrap();

        let modified: String = (1..=30)
            .map(|i| match i {
                3 => "LINE THREE\n".to_string(),
                25 => "LINE TWENTYFIVE\n".to_string(),
                _ => format!("line {i}\n"),
            })
            .collect();
        std::fs::write(&file, modified).unwrap();

        let diff = read_file_diff(repo, "file.txt", false).unwrap();
        assert_eq!(diff.hunks.len(), 2);
        toggle_hunk(repo, "file.txt", 0, &diff.hunks[0].header, false).unwrap();

        let staged = read_file_diff(repo, "file.txt", true).unwrap();
        assert_eq!(staged.hunks.len(), 1);
        assert!(staged.hunks[0].lines.iter().any(|l| l.text == "LINE THREE"));

        let unstaged = read_file_diff(repo, "file.txt", false).unwrap();
        assert_eq!(unstaged.hunks.len(), 1);
        assert!(unstaged.hunks[0].lines.iter().any(|l| l.text == "LINE TWENTYFIVE"));

        // 反向取消暂存
        toggle_hunk(repo, "file.txt", 0, &staged.hunks[0].header, true).unwrap();
        assert!(read_file_diff(repo, "file.txt", true).unwrap().hunks.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stages_and_unstages_only_selected_lines() {
        use crate::git::diff::DiffLineKind;

        let dir = std::env::temp_dir().join(format!("firefly-lines-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init_repo(&dir);

        let file = dir.join("file.txt");
        let original: String = (1..=10).map(|i| format!("line {i}\n")).collect();
        std::fs::write(&file, &original).unwrap();
        let repo = dir.to_str().unwrap();
        stage_file(repo, "file.txt").unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["-c", "user.name=T", "-c", "user.email=t@e.com", "commit", "-qm", "init"])
            .status()
            .unwrap();

        // 3 和 5 两处改动落在同一个 hunk 里
        let modified: String = (1..=10)
            .map(|i| match i {
                3 => "LINE THREE\n".to_string(),
                5 => "LINE FIVE\n".to_string(),
                _ => format!("line {i}\n"),
            })
            .collect();
        std::fs::write(&file, modified).unwrap();

        let diff = read_file_diff(repo, "file.txt", false).unwrap();
        assert_eq!(diff.hunks.len(), 1, "两处改动应在同一个 hunk");

        let mut refs = Vec::new();
        for (index, line) in diff.hunks[0].lines.iter().enumerate() {
            let hit = match line.kind {
                DiffLineKind::Del => line.text == "line 3",
                DiffLineKind::Add => line.text == "LINE THREE",
                _ => false,
            };
            if hit {
                refs.push(LineRef { hunk: 0, line: index });
            }
        }
        assert_eq!(refs.len(), 2);

        toggle_lines(repo, "file.txt", &refs, false).unwrap();

        let staged = read_file_diff(repo, "file.txt", true).unwrap();
        let staged_changes: Vec<&str> = staged
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| matches!(l.kind, DiffLineKind::Add | DiffLineKind::Del))
            .map(|l| l.text.as_str())
            .collect();
        assert!(staged_changes.contains(&"LINE THREE"));
        assert!(!staged_changes.contains(&"LINE FIVE"));

        let unstaged = read_file_diff(repo, "file.txt", false).unwrap();
        let unstaged_changes: Vec<&str> = unstaged
            .hunks
            .iter()
            .flat_map(|h| h.lines.iter())
            .filter(|l| matches!(l.kind, DiffLineKind::Add | DiffLineKind::Del))
            .map(|l| l.text.as_str())
            .collect();
        assert!(unstaged_changes.contains(&"LINE FIVE"));
        assert!(!unstaged_changes.contains(&"LINE THREE"));

        // 反向：把刚暂存的那两行取消暂存
        let mut staged_refs = Vec::new();
        for (hunk_index, hunk) in staged.hunks.iter().enumerate() {
            for (index, line) in hunk.lines.iter().enumerate() {
                let hit = match line.kind {
                    DiffLineKind::Del => line.text == "line 3",
                    DiffLineKind::Add => line.text == "LINE THREE",
                    _ => false,
                };
                if hit {
                    staged_refs.push(LineRef { hunk: hunk_index, line: index });
                }
            }
        }
        assert_eq!(staged_refs.len(), 2);
        toggle_lines(repo, "file.txt", &staged_refs, true).unwrap();

        assert!(read_file_diff(repo, "file.txt", true).unwrap().hunks.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discards_tracked_changes_and_untracked_files() {
        let dir = std::env::temp_dir().join(format!("firefly-discard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init_repo(&dir);

        let repo = dir.to_str().unwrap();
        std::fs::write(dir.join("tracked.txt"), "original\n").unwrap();
        stage_file(repo, "tracked.txt").unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["-c", "user.name=T", "-c", "user.email=t@e.com", "commit", "-qm", "init"])
            .status()
            .unwrap();

        std::fs::write(dir.join("tracked.txt"), "modified\n").unwrap();
        std::fs::write(dir.join("untracked.txt"), "temp\n").unwrap();

        discard_file(repo, "tracked.txt").unwrap();
        // core.autocrlf=true 时检出会变成 CRLF，归一化后比较
        let restored = std::fs::read_to_string(dir.join("tracked.txt"))
            .unwrap()
            .replace("\r\n", "\n");
        assert_eq!(restored, "original\n");

        discard_file(repo, "untracked.txt").unwrap();
        assert!(!dir.join("untracked.txt").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unstaging_all_lines_of_new_file_removes_it_from_index() {
        let dir = std::env::temp_dir().join(format!("firefly-newfile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init_repo(&dir);

        let repo = dir.to_str().unwrap();
        std::fs::write(dir.join("base.txt"), "base\n").unwrap();
        stage_file(repo, "base.txt").unwrap();
        std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["-c", "user.name=T", "-c", "user.email=t@e.com", "commit", "-qm", "init"])
            .status()
            .unwrap();

        std::fs::write(dir.join("newfile.txt"), "a\nb\nc\n").unwrap();
        stage_file(repo, "newfile.txt").unwrap();

        let diff = read_file_diff(repo, "newfile.txt", true).unwrap();
        assert!(diff
            .header_lines
            .iter()
            .any(|line| line.starts_with("new file mode")));

        let refs: Vec<LineRef> = diff
            .hunks
            .iter()
            .flat_map(|hunk| {
                hunk.lines
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| matches!(line.kind, DiffLineKind::Add))
                    .map(move |(index, _)| LineRef {
                        hunk: hunk.index,
                        line: index,
                    })
            })
            .collect();
        assert_eq!(refs.len(), 3);

        toggle_lines(repo, "newfile.txt", &refs, true).unwrap();

        // 应该回到未跟踪状态，而不是 index 里的空文件
        let staged = read_file_diff(repo, "newfile.txt", true).unwrap();
        assert!(staged.hunks.is_empty());
        let worktree = read_file_diff(repo, "newfile.txt", false).unwrap();
        assert!(worktree.untracked);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
