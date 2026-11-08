use super::{decode_bytes, run_git, run_git_bytes, GitError, GitResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineRef {
    pub hunk: usize,
    pub line: usize,
}

/// 超过这个大小的 diff 不解析、不渲染，只允许整文件暂存。
const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineKind {
    Context,
    Add,
    Del,
    NoNewline,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
    /// 原始字节（含前缀和行尾），重建补丁时原样写回。
    #[serde(skip)]
    pub raw: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub index: usize,
    pub header: String,
    pub section: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
    #[serde(skip)]
    pub raw_header: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub staged: bool,
    pub binary: bool,
    pub untracked: bool,
    pub too_large: bool,
    pub header_lines: Vec<String>,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
    #[serde(skip)]
    pub raw_header: Vec<u8>,
}

impl FileDiff {
    fn empty(path: &str, staged: bool) -> Self {
        Self {
            path: path.to_string(),
            staged,
            binary: false,
            untracked: false,
            too_large: false,
            header_lines: Vec::new(),
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
            raw_header: Vec::new(),
        }
    }
}

pub fn read_file_diff(repo: &str, path: &str, staged: bool) -> GitResult<FileDiff> {
    let tracked = run_git(repo, &["ls-files", "--error-unmatch", "--", path]).is_ok();
    if !tracked && !staged {
        return read_untracked_diff(repo, path);
    }

    let mut args: Vec<&str> = vec![
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "-U3",
    ];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(path);

    let bytes = run_git_bytes(repo, &args, None)?;
    if bytes.len() > MAX_DIFF_BYTES {
        let mut diff = FileDiff::empty(path, staged);
        diff.too_large = true;
        return Ok(diff);
    }
    Ok(parse_file_diff(&bytes, path, staged))
}

/// 查看某个历史提交里单文件的改动（只读，不涉及 hunk 级操作）。
pub fn read_commit_file_diff(repo: &str, hash: &str, path: &str) -> GitResult<FileDiff> {
    let bytes = run_git_bytes(
        repo,
        &[
            "show",
            "--first-parent",
            "-m",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--format=",
            "-U3",
            hash,
            "--",
            path,
        ],
        None,
    )?;
    if bytes.len() > MAX_DIFF_BYTES {
        let mut diff = FileDiff::empty(path, false);
        diff.too_large = true;
        return Ok(diff);
    }
    Ok(parse_file_diff(&bytes, path, false))
}

/// 未跟踪文件 git diff 看不到，直接从磁盘读出来拼成「全是新增」的视图。
/// 这种文件只支持整文件暂存，不做 hunk 级操作。
fn read_untracked_diff(repo: &str, path: &str) -> GitResult<FileDiff> {
    let abs = Path::new(repo).join(path);
    let meta = std::fs::metadata(&abs)
        .map_err(|e| GitError::new(format!("读取文件信息失败（{path}）：{e}")))?;
    if meta.len() > MAX_DIFF_BYTES as u64 {
        let mut diff = FileDiff::empty(path, false);
        diff.untracked = true;
        diff.too_large = true;
        return Ok(diff);
    }

    let bytes = std::fs::read(&abs).map_err(|e| GitError::new(format!("读取文件失败（{path}）：{e}")))?;
    let mut diff = FileDiff::empty(path, false);
    diff.untracked = true;

    if bytes.iter().take(8000).any(|b| *b == 0) {
        diff.binary = true;
        return Ok(diff);
    }

    let header = format!(
        "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n"
    );
    diff.header_lines = header
        .lines()
        .map(str::to_string)
        .collect();
    diff.raw_header = header.into_bytes();

    let mut lines = Vec::new();
    let mut new_no = 1u32;
    for raw_line in bytes.split_inclusive(|b| *b == b'\n') {
        let mut raw = Vec::with_capacity(raw_line.len() + 1);
        raw.push(b'+');
        raw.extend_from_slice(raw_line);
        let text = decode_bytes(raw_line)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        lines.push(DiffLine {
            kind: DiffLineKind::Add,
            old_no: None,
            new_no: Some(new_no),
            text,
            raw,
        });
        new_no += 1;
    }

    if !lines.is_empty() {
        let count = lines.len() as u32;
        let hunk_header = format!("@@ -0,0 +1,{count} @@\n");
        diff.hunks.push(Hunk {
            index: 0,
            header: hunk_header.trim_end().to_string(),
            section: String::new(),
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: count,
            lines,
            raw_header: hunk_header.into_bytes(),
        });
        diff.additions = count;
    }

    Ok(diff)
}

pub fn parse_file_diff(bytes: &[u8], path: &str, staged: bool) -> FileDiff {
    let mut diff = FileDiff::empty(path, staged);
    let mut current: Option<Hunk> = None;
    let mut old_no = 0u32;
    let mut new_no = 0u32;

    for line in bytes.split_inclusive(|b| *b == b'\n') {
        if line.starts_with(b"@@ ") {
            if let Some(hunk) = current.take() {
                diff.hunks.push(hunk);
            }
            let header = decode_bytes(line);
            let header = header.trim_end_matches(['\r', '\n']);
            if let Some((os, ol, ns, nl, section)) = parse_hunk_header(header) {
                old_no = os;
                new_no = ns;
                current = Some(Hunk {
                    index: diff.hunks.len(),
                    header: header.to_string(),
                    section,
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    lines: Vec::new(),
                    raw_header: line.to_vec(),
                });
            }
            continue;
        }

        if let Some(hunk) = current.as_mut() {
            let (kind, old, new) = match line.first().copied() {
                Some(b' ') => {
                    let o = old_no;
                    let n = new_no;
                    old_no += 1;
                    new_no += 1;
                    (DiffLineKind::Context, Some(o), Some(n))
                }
                Some(b'+') => {
                    let n = new_no;
                    new_no += 1;
                    diff.additions += 1;
                    (DiffLineKind::Add, None, Some(n))
                }
                Some(b'-') => {
                    let o = old_no;
                    old_no += 1;
                    diff.deletions += 1;
                    (DiffLineKind::Del, Some(o), None)
                }
                Some(b'\\') => (DiffLineKind::NoNewline, None, None),
                _ => continue,
            };
            let text = decode_bytes(&line[1..])
                .trim_end_matches(['\r', '\n'])
                .to_string();
            hunk.lines.push(DiffLine {
                kind,
                old_no: old,
                new_no: new,
                text,
                raw: line.to_vec(),
            });
            continue;
        }

        // 第一个 hunk 之前的内容都是文件级头部
        diff.raw_header.extend_from_slice(line);
        let text = decode_bytes(line);
        let text = text.trim_end_matches(['\r', '\n']);
        if text.starts_with("Binary files") || text.contains("GIT binary patch") {
            diff.binary = true;
        }
        diff.header_lines.push(text.to_string());
    }

    if let Some(hunk) = current.take() {
        diff.hunks.push(hunk);
    }

    diff
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32, String)> {
    let body = line.strip_prefix("@@ ")?;
    let (ranges, section) = body.split_once(" @@")?;
    let mut parts = ranges.split(' ');
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let (old_start, old_lines) = parse_range(old);
    let (new_start, new_lines) = parse_range(new);
    Some((
        old_start,
        old_lines,
        new_start,
        new_lines,
        section.trim().to_string(),
    ))
}

fn parse_range(value: &str) -> (u32, u32) {
    match value.split_once(',') {
        Some((start, len)) => (start.parse().unwrap_or(0), len.parse().unwrap_or(0)),
        None => (value.parse().unwrap_or(0), 1),
    }
}

/// 由「文件级头部 + 单个 hunk 原文」重建最小补丁，交给 `git apply --cached`。
pub fn build_hunk_patch(diff: &FileDiff, hunk_index: usize) -> Option<Vec<u8>> {
    let hunk = diff.hunks.get(hunk_index)?;
    let mut patch = Vec::with_capacity(diff.raw_header.len() + hunk.raw_header.len());
    patch.extend_from_slice(&diff.raw_header);
    patch.extend_from_slice(&hunk.raw_header);
    for line in &hunk.lines {
        patch.extend_from_slice(&line.raw);
        if !line.raw.ends_with(b"\n") {
            patch.push(b'\n');
        }
    }
    Some(patch)
}

fn with_prefix(raw: &[u8], prefix: u8) -> Vec<u8> {
    let mut out = raw.to_vec();
    if let Some(first) = out.first_mut() {
        *first = prefix;
    }
    out
}

/// 由「选中的行」重建一个能应用到 index 的补丁。
///
/// 关键点：补丁的旧侧永远是 **当前 index 的内容**。
/// - 暂存（`unstage = false`，diff 为 index→工作区）：未选中的删除行变成上下文保留，
///   未选中的新增行直接丢弃。
/// - 取消暂存（`unstage = true`，diff 为 HEAD→index）：旧侧是 diff 的新侧，
///   未选中的新增行变成上下文保留，未选中的删除行直接丢弃。
pub fn build_selected_patch(
    diff: &FileDiff,
    refs: &[LineRef],
    unstage: bool,
) -> Option<Vec<u8>> {
    let selected: HashSet<(usize, usize)> = refs.iter().map(|r| (r.hunk, r.line)).collect();
    if selected.is_empty() {
        return None;
    }

    let mut patch = Vec::new();
    patch.extend_from_slice(&diff.raw_header);
    let mut delta: i64 = 0;
    let mut any = false;

    for hunk in &diff.hunks {
        let has_selected = hunk.lines.iter().enumerate().any(|(index, line)| {
            matches!(line.kind, DiffLineKind::Add | DiffLineKind::Del)
                && selected.contains(&(hunk.index, index))
        });
        if !has_selected {
            continue;
        }

        let mut body: Vec<Vec<u8>> = Vec::new();
        let mut old_count: i64 = 0;
        let mut new_count: i64 = 0;
        let mut keep_no_newline = false;

        for (index, line) in hunk.lines.iter().enumerate() {
            let is_selected = selected.contains(&(hunk.index, index));
            match line.kind {
                DiffLineKind::Context => {
                    body.push(line.raw.clone());
                    old_count += 1;
                    new_count += 1;
                    keep_no_newline = true;
                }
                DiffLineKind::Add => {
                    if unstage {
                        if is_selected {
                            body.push(with_prefix(&line.raw, b'-'));
                            old_count += 1;
                        } else {
                            body.push(with_prefix(&line.raw, b' '));
                            old_count += 1;
                            new_count += 1;
                        }
                        keep_no_newline = true;
                    } else if is_selected {
                        body.push(line.raw.clone());
                        new_count += 1;
                        keep_no_newline = true;
                    } else {
                        keep_no_newline = false;
                    }
                }
                DiffLineKind::Del => {
                    if unstage {
                        if is_selected {
                            body.push(with_prefix(&line.raw, b'+'));
                            new_count += 1;
                            keep_no_newline = true;
                        } else {
                            keep_no_newline = false;
                        }
                    } else if is_selected {
                        body.push(line.raw.clone());
                        old_count += 1;
                        keep_no_newline = true;
                    } else {
                        body.push(with_prefix(&line.raw, b' '));
                        old_count += 1;
                        new_count += 1;
                        keep_no_newline = true;
                    }
                }
                DiffLineKind::NoNewline => {
                    if keep_no_newline {
                        body.push(line.raw.clone());
                    }
                }
            }
        }

        if old_count == 0 && new_count == 0 {
            continue;
        }

        // 旧侧是 index：暂存时取 diff 的旧行号，取消暂存时取 diff 的新行号。
        let old_start = if unstage { hunk.new_start } else { hunk.old_start };
        let old_start = if old_count == 0 { 0 } else { old_start };
        let new_start = if new_count == 0 {
            0
        } else {
            (old_start as i64 + delta).max(0) as u32
        };

        let section = if hunk.section.is_empty() {
            String::new()
        } else {
            format!(" {}", hunk.section)
        };
        let header = format!(
            "@@ -{old_start},{old_count} +{new_start},{new_count} @@{section}\n"
        );
        patch.extend_from_slice(header.as_bytes());
        for line in body {
            patch.extend_from_slice(&line);
        }

        delta += new_count - old_count;
        any = true;
    }

    if any {
        Some(patch)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：这里不能用 "\<换行>" 续行，Rust 会吞掉下一行开头的那一个空格，
    // 而 diff 上下文行恰好以空格开头。
    const SAMPLE: &[u8] = b"diff --git a/file.txt b/file.txt\nindex ac9837c..4e01887 100644\n--- a/file.txt\n+++ b/file.txt\n@@ -1,6 +1,6 @@\n line 1\n line 2\n-line 3\n+LINE THREE\n line 4\n line 5\n line 6\n@@ -22,7 +22,7 @@ line 21\n line 22\n-line 25\n+LINE TWENTYFIVE\n line 26\n";

    #[test]
    fn parses_two_hunks_with_line_numbers() {
        let diff = parse_file_diff(SAMPLE, "file.txt", false);
        assert_eq!(diff.hunks.len(), 2);
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 2);

        let first = &diff.hunks[0];
        assert_eq!(first.old_start, 1);
        assert_eq!(first.new_start, 1);
        assert_eq!(first.lines.len(), 7);

        let deleted = &first.lines[2];
        assert_eq!(deleted.kind, DiffLineKind::Del);
        assert_eq!(deleted.old_no, Some(3));
        assert_eq!(deleted.new_no, None);
        assert_eq!(deleted.text, "line 3");

        let added = &first.lines[3];
        assert_eq!(added.kind, DiffLineKind::Add);
        assert_eq!(added.old_no, None);
        assert_eq!(added.new_no, Some(3));

        let second = &diff.hunks[1];
        assert_eq!(second.index, 1);
        assert_eq!(second.section, "line 21");
        assert_eq!(second.new_start, 22);
    }

    #[test]
    fn builds_patch_with_header_and_single_hunk() {
        let diff = parse_file_diff(SAMPLE, "file.txt", false);
        let patch = build_hunk_patch(&diff, 0).unwrap();
        let text = String::from_utf8(patch).unwrap();

        assert!(text.starts_with("diff --git a/file.txt b/file.txt\n"));
        assert!(text.contains("@@ -1,6 +1,6 @@"));
        assert!(!text.contains("@@ -22,7 +22,7 @@"));
        assert!(text.contains("-line 3\n+LINE THREE\n"));
    }

    #[test]
    fn parses_no_newline_marker() {
        let raw = b"diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n\\ No newline at end of file\n";
        let diff = parse_file_diff(raw, "x", false);
        let hunk = &diff.hunks[0];
        assert_eq!(hunk.lines.len(), 3);
        assert_eq!(hunk.lines[2].kind, DiffLineKind::NoNewline);
        let patch = build_hunk_patch(&diff, 0).unwrap();
        assert!(patch.ends_with(b"\\ No newline at end of file\n"));
    }

    #[test]
    fn detects_binary_diff() {
        let raw = b"diff --git a/img.png b/img.png\nindex aaa..bbb 100644\nBinary files a/img.png and b/img.png differ\n";
        let diff = parse_file_diff(raw, "img.png", false);
        assert!(diff.binary);
        assert!(diff.hunks.is_empty());
    }

    // 一个 hunk 里两组改动：b→B 和 d→D
    const TWO_CHANGES: &[u8] = b"diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,5 +1,5 @@\n a\n-b\n+B\n c\n-d\n+D\n e\n";

    #[test]
    fn stages_selected_lines_only() {
        let diff = parse_file_diff(TWO_CHANGES, "f", false);
        let refs = [LineRef { hunk: 0, line: 1 }, LineRef { hunk: 0, line: 2 }];
        let patch = build_selected_patch(&diff, &refs, false).unwrap();
        let text = String::from_utf8(patch).unwrap();

        // 未选中的 -d 变成上下文，所以两侧都还是 5 行
        assert!(text.contains("@@ -1,5 +1,5 @@"));
        assert!(text.contains("\n a\n-b\n+B\n c\n d\n e\n"));
        // 未选中的新增行 +D 必须被丢弃，未选中的删除行 -d 变成上下文
        assert!(!text.contains("+D"));
        assert!(!text.contains("-d"));
    }

    #[test]
    fn unstages_selected_lines_only() {
        let diff = parse_file_diff(TWO_CHANGES, "f", true);
        let refs = [LineRef { hunk: 0, line: 1 }, LineRef { hunk: 0, line: 2 }];
        let patch = build_selected_patch(&diff, &refs, true).unwrap();
        let text = String::from_utf8(patch).unwrap();

        // 旧侧是 index：B 被移除、b 恢复；未选中的 D 保持上下文，d 丢弃
        assert!(text.contains("@@ -1,5 +1,5 @@"));
        // 按 diff 原顺序处理：先遇到 -b（恢复成 +b），再遇到 +B（移除成 -B）
        assert!(text.contains("\n a\n+b\n-B\n c\n D\n e\n"));
        assert!(!text.contains("+D"));
        assert!(!text.contains("-d"));
    }

    #[test]
    fn parses_hunk_header_variants() {
        assert_eq!(
            parse_hunk_header("@@ -1 +1 @@"),
            Some((1, 1, 1, 1, String::new()))
        );
        assert_eq!(
            parse_hunk_header("@@ -0,0 +1,3 @@ fn main()"),
            Some((0, 0, 1, 3, "fn main()".to_string()))
        );
    }
}
