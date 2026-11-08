import { useEffect, useRef, useState } from "react";
import { useRepo } from "../stores/repo";
import type { Hunk } from "../types";
import { DiffBody } from "./DiffBody";

export function DiffView() {
  const selection = useRepo((s) => s.selection);
  const diff = useRepo((s) => s.diff);
  const loading = useRepo((s) => s.diffLoading);
  const status = useRepo((s) => s.status);

  const [selected, setSelected] = useState<Set<string>>(new Set());
  const anchorRef = useRef<{ hunk: number; line: number } | null>(null);

  // diff 一旦重新加载（行号可能变化），旧的选中下标就不可信了
  useEffect(() => {
    setSelected(new Set());
    anchorRef.current = null;
  }, [diff]);

  const entry = selection
    ? status?.entries.find((item) => item.path === selection.path)
    : undefined;
  const conflicted = Boolean(entry?.conflicted);

  const clearSelection = () => {
    setSelected(new Set());
    anchorRef.current = null;
  };

  const handleLineClick = (
    hunk: Hunk,
    lineIndex: number,
    event: React.MouseEvent<HTMLDivElement>,
  ) => {
    const kind = hunk.lines[lineIndex]?.kind;
    if (kind !== "add" && kind !== "del") return;

    const key = `${hunk.index}:${lineIndex}`;
    setSelected((prev) => {
      const next = new Set(prev);

      if (event.shiftKey && anchorRef.current && anchorRef.current.hunk === hunk.index) {
        const from = Math.min(anchorRef.current.line, lineIndex);
        const to = Math.max(anchorRef.current.line, lineIndex);
        for (let index = from; index <= to; index += 1) {
          const neighbor = hunk.lines[index]?.kind;
          if (neighbor === "add" || neighbor === "del") {
            next.add(`${hunk.index}:${index}`);
          }
        }
        return next;
      }

      if (event.ctrlKey || event.metaKey) {
        if (next.has(key)) next.delete(key);
        else next.add(key);
        anchorRef.current = { hunk: hunk.index, line: lineIndex };
        return next;
      }

      if (next.size === 1 && next.has(key)) {
        next.clear();
        anchorRef.current = null;
        return next;
      }

      next.clear();
      next.add(key);
      anchorRef.current = { hunk: hunk.index, line: lineIndex };
      return next;
    });
  };

  const applySelected = async () => {
    if (!diff || selected.size === 0) return;
    const lines = [...selected].map((key) => {
      const [hunk, line] = key.split(":").map(Number);
      return { hunk, line };
    });
    clearSelection();
    await useRepo.getState().toggleLines(lines, diff.staged);
  };

  if (!selection) {
    return (
      <div className="diff-panel">
        <div className="diff-empty">选择左侧文件查看改动</div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="diff-panel">
        <div className="diff-empty">加载中…</div>
      </div>
    );
  }

  if (!diff) {
    return (
      <div className="diff-panel">
        <div className="diff-empty">没有可显示的改动</div>
      </div>
    );
  }

  return (
    <div className="diff-panel">
      <div className="diff-head">
        <span className="diff-path" title={diff.path}>
          {diff.path}
        </span>
        <span className={`tag ${diff.staged ? "tag-staged" : "tag-unstaged"}`}>
          {diff.staged ? "已暂存" : "未暂存"}
        </span>
        {diff.additions > 0 && <span className="add-count">+{diff.additions}</span>}
        {diff.deletions > 0 && <span className="del-count">-{diff.deletions}</span>}

        <span className="spacer" />

        {selected.size > 0 && (
          <span className="line-actions">
            <span className="muted">已选 {selected.size} 行</span>
            <button className="btn btn-sm" onClick={clearSelection}>
              清除
            </button>
            <button className="btn btn-sm btn-primary" onClick={() => void applySelected()}>
              {diff.staged ? "取消暂存选中行" : "暂存选中行"}
            </button>
          </span>
        )}
      </div>

      {conflicted ? (
        <div className="diff-empty">
          该文件存在合并冲突。当前版本只支持整文件暂存（等同于 git add，标记为已解决），
          请在终端或其他工具中编辑冲突内容。
        </div>
      ) : diff.tooLarge ? (
        <div className="diff-empty">
          文件改动超过 2 MB，未渲染 diff。可以在左侧整文件暂存或取消暂存。
        </div>
      ) : diff.binary ? (
        <div className="diff-empty">二进制文件，无法显示文本 diff。</div>
      ) : diff.hunks.length === 0 ? (
        <div className="diff-empty">没有行级改动（可能是文件权限或模式变化）。</div>
      ) : (
        <DiffBody
          diff={diff}
          selectable={!diff.untracked}
          selectedLines={selected}
          onLineClick={handleLineClick}
          hunkAction={
            diff.untracked
              ? undefined
              : {
                  label: diff.staged ? "取消暂存此块" : "暂存此块",
                  run: (hunk) =>
                    void useRepo.getState().toggleHunk(hunk.index, hunk.header, diff.staged),
                }
          }
        />
      )}
    </div>
  );
}
