import { useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { DiffLine, FileDiff, Hunk } from "../types";

type Row =
  | { key: string; kind: "hunk"; hunk: Hunk }
  | { key: string; kind: "line"; line: DiffLine; hunk: Hunk; lineIndex: number };

interface DiffBodyProps {
  diff: FileDiff;
  hunkAction?: { label: string; run: (hunk: Hunk) => void };
  selectedLines?: ReadonlySet<string>;
  selectable?: boolean;
  onLineClick?: (hunk: Hunk, lineIndex: number, event: React.MouseEvent<HTMLDivElement>) => void;
}

/** 只负责把 diff 渲染成虚拟滚动的行；空态/二进制等判断由调用方处理。 */
export function DiffBody({
  diff,
  hunkAction,
  selectedLines,
  selectable,
  onLineClick,
}: DiffBodyProps) {
  const rows = useMemo<Row[]>(() => {
    const out: Row[] = [];
    diff.hunks.forEach((hunk) => {
      out.push({ key: `h${hunk.index}`, kind: "hunk", hunk });
      hunk.lines.forEach((line, lineIndex) => {
        out.push({
          key: `h${hunk.index}l${lineIndex}`,
          kind: "line",
          line,
          hunk,
          lineIndex,
        });
      });
    });
    return out;
  }, [diff]);

  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => (rows[index]?.kind === "hunk" ? 30 : 19),
    overscan: 40,
  });

  return (
    <div ref={parentRef} className="diff-scroll">
      <div className="diff-inner" style={{ height: `${virtualizer.getTotalSize()}px` }}>
        {virtualizer.getVirtualItems().map((item) => {
          const row = rows[item.index];
          return (
            <div
              key={row.key}
              className="diff-row"
              style={{
                height: `${item.size}px`,
                transform: `translateY(${item.start}px)`,
              }}
            >
              {row.kind === "hunk" ? (
                <div className="hunk-head">
                  {hunkAction && (
                    <button className="btn btn-sm" onClick={() => hunkAction.run(row.hunk)}>
                      {hunkAction.label}
                    </button>
                  )}
                  <span className="hunk-title">{row.hunk.header}</span>
                </div>
              ) : (
                <Line
                  line={row.line}
                  selected={selectedLines?.has(`${row.hunk.index}:${row.lineIndex}`) ?? false}
                  selectable={
                    Boolean(selectable) &&
                    (row.line.kind === "add" || row.line.kind === "del")
                  }
                  onClick={
                    onLineClick && (row.line.kind === "add" || row.line.kind === "del")
                      ? (event) => onLineClick(row.hunk, row.lineIndex, event)
                      : undefined
                  }
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

interface LineProps {
  line: DiffLine;
  selected: boolean;
  selectable: boolean;
  onClick?: (event: React.MouseEvent<HTMLDivElement>) => void;
}

function Line({ line, selected, selectable, onClick }: LineProps) {
  const sign =
    line.kind === "add" ? "+" : line.kind === "del" ? "-" : line.kind === "nonewline" ? "\\" : " ";

  return (
    <div
      className={`diff-line ${line.kind}${selectable ? " selectable" : ""}${
        selected ? " selected" : ""
      }`}
      onClick={onClick}
    >
      <span className="ln">{line.oldNo ?? ""}</span>
      <span className="ln">{line.newNo ?? ""}</span>
      <span className="sign">{sign}</span>
      <span className="text">{line.text}</span>
    </div>
  );
}
