import { create } from "zustand";
import { ipc } from "../lib/ipc";
import type {
  AppError,
  BranchInfo,
  CommitFile,
  CommitSummary,
  FileDiff,
  GraphRow,
  LineRef,
  RepoStatus,
} from "../types";

export type ViewMode = "changes" | "history";

export interface Selection {
  path: string;
  staged: boolean;
}

interface RepoStore {
  ready: boolean;
  recent: string[];
  lastRepo: string | null;
  repo: string | null;
  status: RepoStatus | null;
  selection: Selection | null;
  diff: FileDiff | null;
  diffLoading: boolean;
  busy: boolean;
  error: AppError | null;
  notice: string | null;

  branches: BranchInfo[];
  branchesLoading: boolean;

  view: ViewMode;
  graph: GraphRow[];
  showAllBranches: boolean;
  commits: CommitSummary[];
  historyLoading: boolean;
  selectedCommit: CommitSummary | null;
  commitFiles: CommitFile[];
  commitFilesLoading: boolean;
  selectedCommitFile: string | null;
  commitDiff: FileDiff | null;
  commitDiffLoading: boolean;

  init: () => Promise<void>;
  openRepo: (path: string, silent?: boolean) => Promise<void>;
  refresh: () => Promise<void>;
  select: (path: string, staged: boolean) => Promise<void>;
  stageFile: (path: string) => Promise<void>;
  unstageFile: (path: string) => Promise<void>;
  stageAll: () => Promise<void>;
  unstageAll: () => Promise<void>;
  toggleHunk: (hunkIndex: number, header: string, staged: boolean) => Promise<void>;
  toggleLines: (lines: LineRef[], staged: boolean) => Promise<void>;
  commit: (message: string, amend: boolean) => Promise<boolean>;
  loadBranches: () => Promise<void>;
  switchBranch: (branch: string) => Promise<void>;
  createBranch: (branch: string) => Promise<boolean>;
  mergeBranch: (branch: string) => Promise<boolean>;
  abortMerge: () => Promise<void>;
  rebaseBranch: (branch: string) => Promise<boolean>;
  abortRebase: () => Promise<void>;
  setView: (view: ViewMode) => void;
  loadHistory: () => Promise<void>;
  setShowAllBranches: (value: boolean) => void;
  selectCommit: (commit: CommitSummary) => Promise<void>;
  selectCommitFile: (path: string) => Promise<void>;
  dismissError: () => void;
  dismissNotice: () => void;
}

let selectionSeq = 0;
let noticeTimer: ReturnType<typeof setTimeout> | null = null;

export const useRepo = create<RepoStore>((set, get) => {
  const showNotice = (text: string) => {
    if (noticeTimer) clearTimeout(noticeTimer);
    set({ notice: text });
    noticeTimer = setTimeout(() => set({ notice: null }), 3200);
  };

  const resetHistory = {
    graph: [] as GraphRow[],
    commits: [] as CommitSummary[],
    selectedCommit: null,
    commitFiles: [] as CommitFile[],
    selectedCommitFile: null,
    commitDiff: null,
  };

  return {
    ready: false,
    recent: [],
    lastRepo: null,
    repo: null,
    status: null,
    selection: null,
    diff: null,
    diffLoading: false,
    busy: false,
    error: null,
    notice: null,

    branches: [],
    branchesLoading: false,

    view: "changes",
    graph: [],
    showAllBranches: true,
    commits: [],
    historyLoading: false,
    selectedCommit: null,
    commitFiles: [],
    commitFilesLoading: false,
    selectedCommitFile: null,
    commitDiff: null,
    commitDiffLoading: false,

    init: async () => {
      try {
        const data = await ipc.init();
        set({ ready: true, recent: data.recent, lastRepo: data.lastRepo });
        if (data.lastRepo) {
          await get().openRepo(data.lastRepo, true);
        }
      } catch (error) {
        set({ ready: true, error: error as AppError });
      }
    },

    openRepo: async (path, silent = false) => {
      try {
        set({ busy: true, error: null });
        const info = await ipc.repoOpen(path);
        const data = await ipc.init();
        set({
          busy: false,
          repo: info.root,
          status: info.status,
          recent: data.recent,
          lastRepo: info.root,
          selection: null,
          diff: null,
          branches: [],
          ...resetHistory,
        });

        void get().loadBranches();

        if (get().view === "history") {
          await get().loadHistory();
          return;
        }

        // 打开仓库后自动选中第一个改动文件，和 SmartGit 的习惯一致。
        const first =
          info.status.entries.find((entry) => entry.unstaged || entry.untracked) ??
          info.status.entries.find((entry) => entry.staged);
        if (first) {
          await get().select(first.path, !first.unstaged && first.staged);
        }
      } catch (error) {
        set({ busy: false });
        if (!silent) set({ error: error as AppError });
      }
    },

    refresh: async () => {
      const { repo, selection } = get();
      if (!repo) return;
      try {
        const status = await ipc.repoStatus(repo);
        set({ status });
        if (!selection) return;

        const entry = status.entries.find((item) => item.path === selection.path);
        const stillVisible =
          entry && (selection.staged ? entry.staged : entry.unstaged || entry.untracked);
        if (!stillVisible) {
          set({ selection: null, diff: null });
          return;
        }

        // 静默重载 diff：不清空旧内容，避免文件监视器触发时滚动位置跳回顶部。
        const seq = ++selectionSeq;
        const diff = await ipc.repoDiff(repo, selection.path, selection.staged);
        if (seq === selectionSeq) {
          // diffLoading: false 同时兜住「刷新抢在 select 之前完成」的情况
          set({ diff, diffLoading: false });
        }
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    select: async (path, staged) => {
      const { repo } = get();
      if (!repo) return;
      const seq = ++selectionSeq;
      set({ selection: { path, staged }, diff: null, diffLoading: true });
      try {
        const diff = await ipc.repoDiff(repo, path, staged);
        if (seq !== selectionSeq) return;
        set({ diff, diffLoading: false });
      } catch (error) {
        if (seq !== selectionSeq) return;
        set({ diff: null, diffLoading: false, error: error as AppError });
      }
    },

    stageFile: async (path) => {
      const repo = get().repo;
      if (!repo) return;
      try {
        await ipc.stageFile(repo, path);
        await get().refresh();
        const entry = get().status?.entries.find((item) => item.path === path);
        if (entry?.staged) await get().select(path, true);
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    unstageFile: async (path) => {
      const repo = get().repo;
      if (!repo) return;
      try {
        await ipc.unstageFile(repo, path);
        await get().refresh();
        const entry = get().status?.entries.find((item) => item.path === path);
        if (entry) await get().select(path, false);
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    stageAll: async () => {
      const repo = get().repo;
      if (!repo) return;
      const selection = get().selection;
      try {
        await ipc.stageAll(repo);
        await get().refresh();
        if (selection) await get().select(selection.path, true);
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    unstageAll: async () => {
      const repo = get().repo;
      if (!repo) return;
      const selection = get().selection;
      try {
        await ipc.unstageAll(repo);
        await get().refresh();
        if (selection) await get().select(selection.path, false);
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    toggleHunk: async (hunkIndex, header, staged) => {
      const { repo, selection } = get();
      if (!repo || !selection) return;
      try {
        await ipc.toggleHunk(repo, selection.path, hunkIndex, header, staged);
        await get().refresh();
      } catch (error) {
        set({ error: error as AppError });
        await get().refresh();
      }
    },

    toggleLines: async (lines, staged) => {
      const { repo, selection } = get();
      if (!repo || !selection || lines.length === 0) return;
      try {
        await ipc.toggleLines(repo, selection.path, lines, staged);
        await get().refresh();
      } catch (error) {
        set({ error: error as AppError });
        await get().refresh();
      }
    },

    commit: async (message, amend) => {
      const repo = get().repo;
      if (!repo) return false;
      try {
        set({ busy: true, error: null });
        const result = await ipc.commit(repo, message, amend);
        set({ busy: false, selection: null, diff: null });
        showNotice(`已提交 ${result.hash} · ${result.subject}`);
        await get().refresh();
        if (get().commits.length > 0) await get().loadHistory();
        return true;
      } catch (error) {
        set({ busy: false, error: error as AppError });
        return false;
      }
    },

    loadBranches: async () => {
      const { repo } = get();
      if (!repo) return;
      try {
        set({ branchesLoading: true });
        const branches = await ipc.repoBranches(repo);
        set({ branches, branchesLoading: false });
      } catch (error) {
        set({ branchesLoading: false, error: error as AppError });
      }
    },

    switchBranch: async (branch) => {
      const { repo } = get();
      if (!repo) return;
      try {
        set({ error: null });
        await ipc.checkoutBranch(repo, branch);
        // 历史列表属于旧分支，清掉；如果当前正在看历史会自动重新加载。
        set({ selection: null, diff: null, ...resetHistory });
        showNotice(`已切换到 ${branch}`);
        await get().refresh();
        await get().loadBranches();
        if (get().view === "history") {
          await get().loadHistory();
        }
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    createBranch: async (branch) => {
      const { repo } = get();
      if (!repo) return false;
      const name = branch.trim();
      if (!name) return false;

      try {
        set({ error: null });
        await ipc.createBranch(repo, name);
        set({ selection: null, diff: null, ...resetHistory });
        showNotice(`已创建并切换到 ${name}`);
        await get().refresh();
        await get().loadBranches();
        if (get().view === "history") {
          await get().loadHistory();
        }
        return true;
      } catch (error) {
        set({ error: error as AppError });
        return false;
      }
    },

    mergeBranch: async (branch) => {
      const { repo } = get();
      if (!repo) return false;
      try {
        set({ error: null });
        const outcome = await ipc.mergeBranch(repo, branch);
        set({ selection: null, diff: null, ...resetHistory });
        showNotice(
          outcome.conflict
            ? `合并 ${branch} 出现冲突，请解决或中止合并`
            : `已合并 ${branch}`,
        );
        await get().refresh();
        await get().loadBranches();
        if (get().view === "history") {
          await get().loadHistory();
        }
        return !outcome.conflict;
      } catch (error) {
        set({ error: error as AppError });
        await get().refresh();
        return false;
      }
    },

    abortMerge: async () => {
      const { repo } = get();
      if (!repo) return;
      try {
        await ipc.abortMerge(repo);
        set({ selection: null, diff: null, ...resetHistory });
        showNotice("已中止合并");
        await get().refresh();
        if (get().view === "history") {
          await get().loadHistory();
        }
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    rebaseBranch: async (branch) => {
      const { repo } = get();
      if (!repo) return false;
      try {
        set({ error: null });
        const outcome = await ipc.rebaseBranch(repo, branch);
        set({ selection: null, diff: null, ...resetHistory });
        showNotice(
          outcome.conflict
            ? `变基到 ${branch} 出现冲突，请解决或中止变基`
            : `已变基到 ${branch}`,
        );
        await get().refresh();
        await get().loadBranches();
        if (get().view === "history") {
          await get().loadHistory();
        }
        return !outcome.conflict;
      } catch (error) {
        set({ error: error as AppError });
        await get().refresh();
        return false;
      }
    },

    abortRebase: async () => {
      const { repo } = get();
      if (!repo) return;
      try {
        await ipc.abortRebase(repo);
        set({ selection: null, diff: null, ...resetHistory });
        showNotice("已中止变基");
        await get().refresh();
        if (get().view === "history") {
          await get().loadHistory();
        }
      } catch (error) {
        set({ error: error as AppError });
      }
    },

    setView: (view) => {
      set({ view });
      if (view === "history" && get().commits.length === 0) {
        void get().loadHistory();
      }
    },

    loadHistory: async () => {
      const { repo } = get();
      if (!repo) return;
      set({ historyLoading: true });
      try {
        const rows = await ipc.repoGraph(repo, 100, get().showAllBranches);
        const commits = rows.flatMap((row) => (row.commit ? [row.commit] : []));
        set({ graph: rows, commits, historyLoading: false });

        const current = get().selectedCommit;
        const keep = current ? commits.find((item) => item.hash === current.hash) : undefined;
        if (keep) {
          set({ selectedCommit: keep });
        } else if (commits.length > 0) {
          await get().selectCommit(commits[0]);
        } else {
          set({ ...resetHistory });
        }
      } catch (error) {
        set({ historyLoading: false, error: error as AppError });
      }
    },

    setShowAllBranches: (value) => {
      set({ showAllBranches: value });
      void get().loadHistory();
    },

    selectCommit: async (commit) => {
      const { repo } = get();
      if (!repo) return;
      set({
        selectedCommit: commit,
        commitFiles: [],
        commitFilesLoading: true,
        selectedCommitFile: null,
        commitDiff: null,
      });
      try {
        const files = await ipc.commitFiles(repo, commit.hash);
        if (get().selectedCommit?.hash !== commit.hash) return;
        set({ commitFiles: files, commitFilesLoading: false });
        if (files.length > 0) {
          await get().selectCommitFile(files[0].path);
        }
      } catch (error) {
        set({ commitFilesLoading: false, error: error as AppError });
      }
    },

    selectCommitFile: async (path) => {
      const { repo, selectedCommit } = get();
      if (!repo || !selectedCommit) return;
      const hash = selectedCommit.hash;
      set({ selectedCommitFile: path, commitDiff: null, commitDiffLoading: true });
      try {
        const diff = await ipc.commitDiff(repo, hash, path);
        if (get().selectedCommit?.hash !== hash || get().selectedCommitFile !== path) return;
        set({ commitDiff: diff, commitDiffLoading: false });
      } catch (error) {
        set({ commitDiffLoading: false, error: error as AppError });
      }
    },

    dismissError: () => set({ error: null }),
    dismissNotice: () => set({ notice: null }),
  };
});
