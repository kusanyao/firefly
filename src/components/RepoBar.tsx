import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Check,
  ChevronDown,
  FolderOpen,
  GitBranch,
  GitMerge,
  GitPullRequestArrow,
  Plus,
  RefreshCw,
} from "lucide-react";
import { useRepo } from "../stores/repo";

export function RepoBar() {
  const repo = useRepo((s) => s.repo);
  const status = useRepo((s) => s.status);
  const recent = useRepo((s) => s.recent);
  const busy = useRepo((s) => s.busy);
  const branches = useRepo((s) => s.branches);
  const branchesLoading = useRepo((s) => s.branchesLoading);

  const [menuOpen, setMenuOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [pendingBranch, setPendingBranch] = useState<string | null>(null);
  const [pendingMerge, setPendingMerge] = useState<string | null>(null);
  const [pendingRebase, setPendingRebase] = useState<string | null>(null);
  const [pendingAbort, setPendingAbort] = useState<null | "merge" | "rebase">(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const closeMenu = () => {
    setMenuOpen(false);
    setCreating(false);
    setNewName("");
  };

  useEffect(() => {
    if (!menuOpen) return;
    const onMouseDown = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        closeMenu();
      }
    };
    document.addEventListener("mousedown", onMouseDown);
    return () => document.removeEventListener("mousedown", onMouseDown);
  }, [menuOpen]);

  const pickRepo = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择 Git 仓库",
      });
      if (typeof selected === "string") {
        void useRepo.getState().openRepo(selected);
      }
    } catch {
      // 用户取消或对话框不可用时不打扰
    }
  };

  const openRecent = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const value = event.target.value;
    event.target.value = "";
    if (value) void useRepo.getState().openRepo(value);
  };

  const toggleMenu = () => {
    if (menuOpen) {
      closeMenu();
      return;
    }
    setMenuOpen(true);
    void useRepo.getState().loadBranches();
  };

  const chooseBranch = (name: string, current: boolean) => {
    closeMenu();
    if (current) return;

    // 有未提交改动时先确认一次，避免把改动意外带到别的分支。
    const dirty = (useRepo.getState().status?.entries.length ?? 0) > 0;
    if (dirty) {
      setPendingBranch(name);
      return;
    }
    void useRepo.getState().switchBranch(name);
  };

  const chooseMerge = (name: string) => {
    closeMenu();
    setPendingMerge(name);
  };

  const chooseRebase = (name: string) => {
    closeMenu();
    setPendingRebase(name);
  };

  const confirmSwitch = () => {
    const name = pendingBranch;
    setPendingBranch(null);
    if (name) void useRepo.getState().switchBranch(name);
  };

  const confirmMerge = () => {
    const name = pendingMerge;
    setPendingMerge(null);
    if (name) void useRepo.getState().mergeBranch(name);
  };

  const confirmRebase = () => {
    const name = pendingRebase;
    setPendingRebase(null);
    if (name) void useRepo.getState().rebaseBranch(name);
  };

  const confirmAbort = () => {
    const action = pendingAbort;
    setPendingAbort(null);
    if (action === "merge") void useRepo.getState().abortMerge();
    if (action === "rebase") void useRepo.getState().abortRebase();
  };

  const branch = status
    ? status.detached
      ? "游离 HEAD"
      : (status.branch ?? "—")
    : "";
  const ahead = status?.ahead ?? 0;
  const behind = status?.behind ?? 0;
  const dirty = (status?.entries.length ?? 0) > 0;
  const merging = status?.merging ?? false;
  const rebasing = status?.rebasing ?? false;

  return (
    <header className="repo-bar">
      <button className="btn" onClick={() => void pickRepo()} disabled={busy}>
        <FolderOpen size={14} />
        打开仓库
      </button>

      {recent.length > 0 && (
        <select className="select recent-select" value="" onChange={openRecent} title="最近打开">
          <option value="">最近…</option>
          {recent.map((path) => (
            <option key={path} value={path}>
              {path}
            </option>
          ))}
        </select>
      )}

      <div className="repo-path" title={repo ?? undefined}>
        {repo ?? "未打开仓库"}
      </div>

      {status && (
        <div className="branch-wrap" ref={menuRef}>
          <button className="branch-btn" onClick={toggleMenu} title="分支">
            <GitBranch size={13} />
            <span className="branch-label">{branch}</span>
            <ChevronDown size={12} />
          </button>

          {(merging || rebasing) && (
            <span className="merge-chip">
              <GitMerge size={11} />
              {merging ? "合并中" : "变基中"}
              <button
                className="merge-abort"
                onClick={() => setPendingAbort(merging ? "merge" : "rebase")}
              >
                中止
              </button>
            </span>
          )}

          {(ahead > 0 || behind > 0) && (
            <span className="branch-ab muted">
              {ahead > 0 ? `↑${ahead}` : ""}
              {behind > 0 ? ` ↓${behind}` : ""}
            </span>
          )}

          {menuOpen && (
            <div className="branch-menu">
              <div className="branch-menu-head">
                切换分支 {branchesLoading && <span className="muted">加载中…</span>}
              </div>
              <div className="branch-menu-list">
                {branches.map((item) => (
                  <div
                    key={item.name}
                    className={`branch-item${item.current ? " current" : ""}`}
                  >
                    <button
                      className="branch-pick"
                      onClick={() => chooseBranch(item.name, item.current)}
                    >
                      <span className="branch-check">
                        {item.current ? <Check size={12} /> : null}
                      </span>
                      <span className="branch-item-name">{item.name}</span>
                      {item.upstream && <span className="branch-upstream">{item.upstream}</span>}
                    </button>
                    {!item.current && !merging && !rebasing && (
                      <>
                        <button
                          className="branch-action"
                          title={`合并到 ${branch}`}
                          onClick={(event) => {
                            event.stopPropagation();
                            chooseMerge(item.name);
                          }}
                        >
                          <GitMerge size={12} />
                        </button>
                        <button
                          className="branch-action"
                          title={`变基到 ${branch}`}
                          onClick={(event) => {
                            event.stopPropagation();
                            chooseRebase(item.name);
                          }}
                        >
                          <GitPullRequestArrow size={12} />
                        </button>
                      </>
                    )}
                  </div>
                ))}
                {!branchesLoading && branches.length === 0 && (
                  <div className="branch-empty">没有本地分支</div>
                )}
              </div>

              {creating ? (
                <div className="branch-create">
                  <input
                    autoFocus
                    className="branch-input"
                    value={newName}
                    placeholder="新分支名，回车创建"
                    onChange={(event) => setNewName(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        void (async () => {
                          const ok = await useRepo.getState().createBranch(newName);
                          if (ok) closeMenu();
                        })();
                      } else if (event.key === "Escape") {
                        event.preventDefault();
                        setCreating(false);
                        setNewName("");
                      }
                    }}
                  />
                </div>
              ) : (
                <button
                  className="branch-create-btn"
                  onClick={() => {
                    setCreating(true);
                    setNewName("");
                  }}
                >
                  <Plus size={12} />
                  新建分支
                </button>
              )}
            </div>
          )}
        </div>
      )}

      <button
        className="icon-btn"
        title="刷新 (F5)"
        onClick={() => void useRepo.getState().refresh()}
        disabled={!repo}
      >
        <RefreshCw size={14} />
      </button>

      {pendingBranch && (
        <div className="modal-backdrop" onMouseDown={() => setPendingBranch(null)}>
          <div className="modal" onMouseDown={(event) => event.stopPropagation()}>
            <div className="modal-title">切换到 {pendingBranch}</div>
            <div className="modal-body">
              当前工作区还有未提交的改动。出现冲突时 git 会拒绝切换；不冲突的话，改动会被带到目标分支。
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setPendingBranch(null)}>
                取消
              </button>
              <button className="btn btn-primary" onClick={confirmSwitch}>
                仍然切换
              </button>
            </div>
          </div>
        </div>
      )}

      {pendingMerge && (
        <div className="modal-backdrop" onMouseDown={() => setPendingMerge(null)}>
          <div className="modal" onMouseDown={(event) => event.stopPropagation()}>
            <div className="modal-title">
              将 {pendingMerge} 合并到 {branch}
            </div>
            <div className="modal-body">
              能快进时会直接快进，否则创建合并提交。
              {dirty ? " 当前工作区有未提交改动，git 可能拒绝合并，或把改动带入合并结果。" : ""}
              {" "}出现冲突时会停在合并状态，之后可以中止。
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setPendingMerge(null)}>
                取消
              </button>
              <button className="btn btn-primary" onClick={confirmMerge}>
                合并
              </button>
            </div>
          </div>
        </div>
      )}

      {pendingRebase && (
        <div className="modal-backdrop" onMouseDown={() => setPendingRebase(null)}>
          <div className="modal" onMouseDown={(event) => event.stopPropagation()}>
            <div className="modal-title">
              将当前分支变基到 {pendingRebase}
            </div>
            <div className="modal-body">
              变基会重写当前分支的提交历史，已经推送过的分支不建议这样做。
              工作区需要是干净的；出现冲突时会停在变基状态，之后可以中止。
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setPendingRebase(null)}>
                取消
              </button>
              <button className="btn btn-primary" onClick={confirmRebase}>
                变基
              </button>
            </div>
          </div>
        </div>
      )}

      {pendingAbort && (
        <div className="modal-backdrop" onMouseDown={() => setPendingAbort(null)}>
          <div className="modal" onMouseDown={(event) => event.stopPropagation()}>
            <div className="modal-title">
              中止{pendingAbort === "merge" ? "合并" : "变基"}？
            </div>
            <div className="modal-body">
              {pendingAbort === "merge"
                ? "合并产生的所有改动都会回退到合并前的状态，已解决的冲突也会丢失。"
                : "变基产生的所有改动都会回退到变基前的状态，已解决的冲突也会丢失。"}
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setPendingAbort(null)}>
                取消
              </button>
              <button className="btn btn-danger" onClick={confirmAbort}>
                中止{pendingAbort === "merge" ? "合并" : "变基"}
              </button>
            </div>
          </div>
        </div>
      )}
    </header>
  );
}
