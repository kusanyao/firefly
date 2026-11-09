import { useMemo } from "react";
import { formatWhen } from "../lib/format";
import { parseRef } from "../lib/refs";
import { useRepo } from "../stores/repo";

const LANE_COLORS = [
  "#3ddc97",
  "#60a5fa",
  "#fbbf24",
  "#c084fc",
  "#f87171",
  "#22d3ee",
  "#a3e635",
  "#fb923c",
];

function laneColor(lane: number): string {
  return LANE_COLORS[lane % LANE_COLORS.length];
}

/** 这一行里有哪些 lane 继续向下延伸（`|` 或提交节点 `*`）。 */
function activeLanes(text: string): number[] {
  const lanes = new Set<number>();
  text.split("").forEach((char, index) => {
    if (char === "|" || char === "*") {
      lanes.add(Math.floor(index / 2));
    }
  });
  return [...lanes];
}

/** 斜线：从一条 lane 的中心连到相邻 lane 的中心。 */
function diagonals(text: string): Array<{ lane: number; kind: "\\" | "/" }> {
  const out: Array<{ lane: number; kind: "\\" | "/" }> = [];
  text.split("").forEach((char, index) => {
    if (char === "\\" || char === "/") {
      out.push({ lane: Math.floor(index / 2), kind: char });
    }
  });
  return out;
}

/**
 * 竖向 lane 线用 CSS 画，贯穿整行高度——否则两行的提交行里，
 * git 的 `|` 字符只覆盖第一行文字，连接线会断开。
 */
function GraphColumn({ text, width, topPad }: { text: string; width: number; topPad: number }) {
  const lanes = useMemo(() => activeLanes(text), [text]);
  const diags = useMemo(() => diagonals(text), [text]);

  return (
    <div className="graph-col" style={{ width: `${width}ch` }}>
      {lanes.map((lane) => (
        <span
          key={`v${lane}`}
          className="lane-line"
          style={{
            left: `calc(${lane * 2}ch + 0.5ch)`,
            background: laneColor(lane),
          }}
        />
      ))}
      {diags.map((diag, index) => {
        const color = laneColor(diag.lane + 1);
        const stops = `transparent calc(50% - 0.9px), ${color} calc(50% - 0.9px), ${color} calc(50% + 0.9px), transparent calc(50% + 0.9px)`;
        // 注意：渐变轴与色带垂直——`to bottom left` 画出来的是 `\`，`to bottom right` 才是 `/`。
        const axis = diag.kind === "\\" ? "left" : "right";
        return (
          <span
            key={`d${index}`}
            className="lane-diag"
            style={{
              left: `calc(${diag.lane * 2}ch + 0.5ch)`,
              background: `linear-gradient(to bottom ${axis}, ${stops})`,
            }}
          />
        );
      })}
      <span className="graph-text" style={{ paddingTop: topPad }}>
        <GraphText text={text} />
      </span>
    </div>
  );
}

/** 只渲染提交节点；竖线和斜线都由 CSS 绘制。 */
function GraphText({ text }: { text: string }) {
  return (
    <>
      {text.split("").map((char, index) => {
        if (char !== "*") {
          return <span key={index}> </span>;
        }
        const color = laneColor(Math.floor(index / 2));
        return (
          <span key={index} className="graph-node" style={{ color }}>
            ●
          </span>
        );
      })}
    </>
  );
}


export function CommitList() {
  const graph = useRepo((s) => s.graph);
  const commits = useRepo((s) => s.commits);
  const selected = useRepo((s) => s.selectedCommit);
  const loading = useRepo((s) => s.historyLoading);

  const graphWidth = useMemo(() => {
    const longest = graph.reduce((max, row) => Math.max(max, row.graph.length), 0);
    return Math.min(60, Math.max(2, longest));
  }, [graph]);

  return (
    <aside className="commit-list">
      <div className="section-head">
        <span>
          提交树 <span className="muted">({commits.length})</span>
        </span>
        {loading && <span className="muted">加载中…</span>}
      </div>

      <div className="commit-body">
        {!loading && graph.length === 0 && <div className="section-empty">还没有提交</div>}

        {graph.map((row, index) => {
          if (!row.commit) {
            return (
              <div className="graph-line" key={`graph-${index}`}>
                <GraphColumn text={row.graph} width={graphWidth} topPad={0} />
              </div>
            );
          }

          const commit = row.commit;
          const refs = commit.refs
            ? commit.refs
                .split(",")
                .map((ref) => ref.trim())
                .filter(Boolean)
            : [];

          return (
            <div
              key={commit.hash}
              className={`commit-row${selected?.hash === commit.hash ? " selected" : ""}`}
              onClick={() => void useRepo.getState().selectCommit(commit)}
              title={`${commit.subject}\n${commit.author}`}
            >
              <GraphColumn text={row.graph} width={graphWidth} topPad={5} />

              <div className="commit-main">
                <div className="commit-subject">{commit.subject || "(无提交信息)"}</div>
                <div className="commit-meta">
                  {refs.map((raw) => {
                    const badge = parseRef(raw);
                    return (
                      <span className={`ref ref-${badge.kind}`} key={raw}>
                        {badge.label}
                      </span>
                    );
                  })}
                  <span className="commit-hash">{commit.shortHash}</span>
                  <span className="commit-author">{commit.author}</span>
                  <span className="commit-when">{formatWhen(commit.timestamp)}</span>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </aside>
  );
}
