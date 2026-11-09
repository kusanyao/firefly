import { useState } from "react";
import { Minus, Plus, Undo2 } from "lucide-react";
import { groupChanges, splitPath, statusBadge, type ChangeRow } from "../lib/changes";
import { useRepo, type Selection } from "../stores/repo";

export function ChangeList() {
  const status = useRepo((s) => s.status);
  const selection = useRepo((s) => s.selection);
  const { unstaged, staged } = groupChanges(status);
  const [pendingDiscard, setPendingDiscard] = useState<ChangeRow | null>(null);

  const confirmDiscard = () => {
    const row = pendingDiscard;
    setPendingDiscard(null);
    if (row) void useRepo.getState().discardFile(row.entry.path);
  };

  return (
    <aside className="change-list">
      <Section
        title="未暂存"
        rows={unstaged}
        allLabel="全部暂存"
        selection={selection}
        onAll={() => void useRepo.getState().stageAll()}
        onDiscard={setPendingDiscard}
      />
      <Section
        title="已暂存"
        rows={staged}
        allLabel="全部取消"
        selection={selection}
        onAll={() => void useRepo.getState().unstageAll()}
        onDiscard={setPendingDiscard}
      />

      {pendingDiscard && (
        <div className="modal-backdrop" onMouseDown={() => setPendingDiscard(null)}>
          <div className="modal" onMouseDown={(event) => event.stopPropagation()}>
            <div className="modal-title">
              {pendingDiscard.entry.untracked ? "删除未跟踪文件？" : "丢弃未暂存改动？"}
            </div>
            <div className="modal-body">
              <div className="discard-path">{pendingDiscard.entry.path}</div>
              {pendingDiscard.entry.untracked
                ? "这个文件还没有被 Git 跟踪，删除后无法恢复。"
                : `未暂存的改动会恢复到暂存区版本，无法撤销。${
                    pendingDiscard.entry.staged ? " 已暂存的改动会保留。" : ""
                  }`}
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setPendingDiscard(null)}>
                取消
              </button>
              <button className="btn btn-danger" onClick={confirmDiscard}>
                {pendingDiscard.entry.untracked ? "删除文件" : "丢弃改动"}
              </button>
            </div>
          </div>
        </div>
      )}
    </aside>
  );
}

interface SectionProps {
  title: string;
  rows: ChangeRow[];
  allLabel: string;
  selection: Selection | null;
  onAll: () => void;
  onDiscard: (row: ChangeRow) => void;
}

function Section({ title, rows, allLabel, selection, onAll, onDiscard }: SectionProps) {
  return (
    <section className="section">
      <div className="section-head">
        <span>
          {title} <span className="muted">({rows.length})</span>
        </span>
        {rows.length > 0 && (
          <button className="link-btn" onClick={onAll}>
            {allLabel}
          </button>
        )}
      </div>
      <div className="section-body">
        {rows.length === 0 && <div className="section-empty">没有改动</div>}
        {rows.map((row) => (
          <Row
            key={`${row.entry.path}-${row.staged}`}
            row={row}
            selection={selection}
            onDiscard={onDiscard}
          />
        ))}
      </div>
    </section>
  );
}

interface RowProps {
  row: ChangeRow;
  selection: Selection | null;
  onDiscard: (row: ChangeRow) => void;
}

function Row({ row, selection, onDiscard }: RowProps) {
  const selected =
    selection?.path === row.entry.path && selection.staged === row.staged;
  const badge = statusBadge(row);
  const badgeClass =
    badge === "?" ? "untracked" : badge === "!" ? "conflict" : badge.trim().toLowerCase();
  const { dir, name } = splitPath(row.entry.path);

  const select = () => void useRepo.getState().select(row.entry.path, row.staged);
  const toggle = async () => {
    if (row.staged) {
      await useRepo.getState().unstageFile(row.entry.path);
    } else {
      await useRepo.getState().stageFile(row.entry.path);
    }
  };

  return (
    <div
      className={`change-row${selected ? " selected" : ""}`}
      onClick={select}
      onDoubleClick={() => void toggle()}
      title={row.entry.origPath ? `${row.entry.origPath} → ${row.entry.path}` : row.entry.path}
    >
      <span className={`badge badge-${badgeClass}`}>{badge}</span>
      <span className="row-path">
        <span className="dir">{dir}</span>
        <span className="name">{name}</span>
      </span>
      <button
        className="row-action"
        title={row.staged ? "取消暂存" : "暂存"}
        onClick={(event) => {
          event.stopPropagation();
          void toggle();
        }}
      >
        {row.staged ? <Minus size={13} /> : <Plus size={13} />}
      </button>
      {!row.staged && !row.entry.conflicted && (
        <button
          className="row-action discard"
          title={row.entry.untracked ? "删除未跟踪文件" : "丢弃未暂存改动"}
          onClick={(event) => {
            event.stopPropagation();
            onDiscard(row);
          }}
        >
          <Undo2 size={13} />
        </button>
      )}
    </div>
  );
}
