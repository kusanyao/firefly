import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { ChangeList } from "./components/ChangeList";
import { CommitDetail } from "./components/CommitDetail";
import { CommitList } from "./components/CommitList";
import { CommitPanel } from "./components/CommitPanel";
import { DiffView } from "./components/DiffView";
import { RepoBar } from "./components/RepoBar";
import { groupChanges } from "./lib/changes";
import { useRepo } from "./stores/repo";

export default function App() {
  const ready = useRepo((s) => s.ready);
  const repo = useRepo((s) => s.repo);
  const view = useRepo((s) => s.view);
  const error = useRepo((s) => s.error);
  const notice = useRepo((s) => s.notice);

  useEffect(() => {
    void useRepo.getState().init();
  }, []);

  useEffect(() => {
    const unlisten = listen("repo://changed", () => {
      void useRepo.getState().refresh();
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const state = useRepo.getState();
      const target = event.target as HTMLElement | null;
      const typing = target?.tagName === "TEXTAREA" || target?.tagName === "INPUT";

      if (event.key === "F5") {
        event.preventDefault();
        if (state.view === "history") {
          void state.loadHistory();
        } else {
          void state.refresh();
        }
        return;
      }

      if (
        !typing &&
        state.view === "changes" &&
        (event.key === "ArrowDown" || event.key === "ArrowUp")
      ) {
        event.preventDefault();
        moveSelection(event.key === "ArrowDown" ? 1 : -1);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  if (!ready) {
    return <div className="splash">正在启动…</div>;
  }

  return (
    <div className="app-shell">
      <RepoBar />

      {repo && (
        <nav className="view-tabs">
          <button
            className={`view-tab${view === "changes" ? " active" : ""}`}
            onClick={() => useRepo.getState().setView("changes")}
          >
            改动
          </button>
          <button
            className={`view-tab${view === "history" ? " active" : ""}`}
            onClick={() => useRepo.getState().setView("history")}
          >
            历史
          </button>
        </nav>
      )}

      {repo ? (
        view === "changes" ? (
          <div className="workspace">
            <ChangeList />
            <main className="main">
              <DiffView />
              <CommitPanel />
            </main>
          </div>
        ) : (
          <div className="workspace">
            <CommitList />
            <CommitDetail />
          </div>
        )
      ) : (
        <div className="empty-state">
          <h2>萤火虫</h2>
          <p>只做「看改动 → 暂存 → 提交」的本地 Git 客户端</p>
          <p className="muted">点击左上角「打开仓库」选择一个 Git 项目</p>
        </div>
      )}

      {notice && <div className="toast">{notice}</div>}

      {error && (
        <div className="error-bar">
          <div className="error-main">
            <strong>{error.message}</strong>
            {error.detail && (
              <details>
                <summary>技术细节</summary>
                <pre>{error.detail}</pre>
              </details>
            )}
          </div>
          <button className="icon-btn" onClick={() => useRepo.getState().dismissError()}>
            ×
          </button>
        </div>
      )}
    </div>
  );
}

function moveSelection(delta: number) {
  const { status, selection, select } = useRepo.getState();
  const rows = groupChanges(status).all;
  if (rows.length === 0) return;

  const current = selection
    ? rows.findIndex(
        (row) => row.entry.path === selection.path && row.staged === selection.staged,
      )
    : -1;
  const nextIndex = Math.min(Math.max(current + delta, 0), rows.length - 1);
  const next = rows[nextIndex];
  if (next) void select(next.entry.path, next.staged);
}
