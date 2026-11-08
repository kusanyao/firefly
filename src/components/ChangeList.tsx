import { Minus, Plus } from "lucide-react";
import { groupChanges, splitPath, statusBadge, type ChangeRow } from "../lib/changes";
import { useRepo, type Selection } from "../stores/repo";

export function ChangeList() {
  const status = useRepo((s) => s.status);
  const selection = useRepo((s) => s.selection);
  const { unstaged, staged } = groupChanges(status);

  return (
    <aside className="change-list">
      <Section
        title="未暂存"
        rows={unstaged}
        allLabel="全部暂存"
        selection={selection}
        onAll={() => void useRepo.getState().stageAll()}
      />
      <Section
        title="已暂存"
        rows={staged}
        allLabel="全部取消"
        selection={selection}
        onAll={() => void useRepo.getState().unstageAll()}
      />
    </aside>
  );
}

interface SectionProps {
  title: string;
  rows: ChangeRow[];
  allLabel: string;
  selection: Selection | null;
  onAll: () => void;
}

function Section({ title, rows, allLabel, selection, onAll }: SectionProps) {
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
          <Row key={`${row.entry.path}-${row.staged}`} row={row} selection={selection} />
        ))}
      </div>
    </section>
  );
}

function Row({ row, selection }: { row: ChangeRow; selection: Selection | null }) {
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
    </div>
  );
}
