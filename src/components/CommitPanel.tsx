import { useState } from "react";
import { groupChanges } from "../lib/changes";
import { useRepo } from "../stores/repo";

export function CommitPanel() {
  const [message, setMessage] = useState("");
  const [amend, setAmend] = useState(false);
  const status = useRepo((s) => s.status);
  const busy = useRepo((s) => s.busy);

  const stagedCount = groupChanges(status).staged.length;
  const canCommit = message.trim().length > 0 && stagedCount > 0 && !busy;

  const doCommit = async () => {
    if (!canCommit) return;
    const ok = await useRepo.getState().commit(message, amend);
    if (ok) {
      setMessage("");
      setAmend(false);
    }
  };

  return (
    <div className="commit-panel">
      <textarea
        className="input commit-message"
        placeholder="提交信息（Ctrl+Enter 提交）"
        value={message}
        onChange={(event) => setMessage(event.target.value)}
        onKeyDown={(event) => {
          if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
            event.preventDefault();
            void doCommit();
          }
        }}
        spellCheck={false}
      />
      <div className="commit-actions">
        <label className={`checkbox${status?.initial ? " disabled" : ""}`}>
          <input
            type="checkbox"
            checked={amend}
            disabled={Boolean(status?.initial)}
            onChange={(event) => setAmend(event.target.checked)}
          />
          修正上次提交
        </label>
        <span className="muted">已暂存 {stagedCount} 个文件</span>
        <div className="spacer" />
        <button className="btn btn-primary" disabled={!canCommit} onClick={() => void doCommit()}>
          {busy ? "提交中…" : "提交"}
        </button>
      </div>
    </div>
  );
}
