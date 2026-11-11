import { useEffect, useMemo, useState } from "react";
import { Cherry } from "lucide-react";
import { ipc } from "../lib/ipc";
import { formatWhen } from "../lib/format";
import { parseRef } from "../lib/refs";
import { useRepo } from "../stores/repo";
import type { CommitSummary } from "../types";

interface CherryPickerProps {
  open: boolean;
  onClose: () => void;
}

/** 从所有分支的最近提交里挑一个，遴选到当前分支。 */
export function CherryPicker({ open, onClose }: CherryPickerProps) {
  const repo = useRepo((s) => s.repo);
  const branch = useRepo((s) => s.status?.branch ?? null);

  const [commits, setCommits] = useState<CommitSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !repo) return;
    setLoading(true);
    setFailed(false);
    setQuery("");
    setSelected(null);

    ipc
      .allCommits(repo, 100)
      .then((list) => setCommits(list))
      .catch(() => {
        setCommits([]);
        setFailed(true);
      })
      .finally(() => setLoading(false));
  }, [open, repo]);

  const filtered = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    if (!keyword) return commits;
    return commits.filter(
      (commit) =>
        commit.subject.toLowerCase().includes(keyword) ||
        commit.shortHash.toLowerCase().includes(keyword) ||
        commit.author.toLowerCase().includes(keyword),
    );
  }, [commits, query]);

  if (!open) return null;

  const submit = () => {
    if (!selected) return;
    void useRepo.getState().cherryPick(selected);
    onClose();
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal cherry-modal" onMouseDown={(event) => event.stopPropagation()}>
        <div className="modal-title">遴选提交到 {branch ?? "当前分支"}</div>

        <div className="cherry-search">
          <input
            autoFocus
            className="branch-input"
            value={query}
            placeholder="搜索标题 / 哈希 / 作者…"
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                onClose();
              }
            }}
          />
        </div>

        <div className="cherry-list">
          {loading && <div className="section-empty">加载中…</div>}
          {!loading && failed && <div className="section-empty">加载提交列表失败</div>}
          {!loading && !failed && filtered.length === 0 && (
            <div className="section-empty">没有匹配的提交</div>
          )}

          {filtered.map((commit) => (
            <button
              key={commit.hash}
              className={`cherry-row${selected === commit.hash ? " selected" : ""}`}
              onClick={() => setSelected(commit.hash)}
              title={commit.subject}
            >
              <span className="cherry-subject">{commit.subject || "(无提交信息)"}</span>
              <span className="cherry-meta">
                <span className="commit-hash">{commit.shortHash}</span>
                <span>{commit.author}</span>
                <span>{formatWhen(commit.timestamp)}</span>
                {commit.refs &&
                  commit.refs.split(",").map((raw) => {
                    const badge = parseRef(raw);
                    return (
                      <span className={`ref ref-${badge.kind}`} key={raw}>
                        {badge.label}
                      </span>
                    );
                  })}
              </span>
            </button>
          ))}
        </div>

        <div className="modal-actions">
          <button className="btn" onClick={onClose}>
            取消
          </button>
          <button className="btn btn-primary" disabled={!selected} onClick={submit}>
            <Cherry size={13} />
            遴选
          </button>
        </div>
      </div>
    </div>
  );
}
