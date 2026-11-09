import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppInit,
  BranchInfo,
  CommitFile,
  CommitResult,
  CommitSummary,
  FileDiff,
  GraphRow,
  LineRef,
  MergeOutcome,
  RepoInfo,
  RepoStatus,
} from "../types";

function toAppError(error: unknown): AppError {
  if (typeof error === "string") {
    return { kind: "unknown", message: error };
  }
  if (error && typeof error === "object") {
    const candidate = error as Partial<AppError>;
    if (typeof candidate.message === "string") {
      return {
        kind: candidate.kind ?? "unknown",
        message: candidate.message,
        detail: candidate.detail,
      };
    }
  }
  return { kind: "unknown", message: "发生未知错误" };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

export const ipc = {
  init: () => call<AppInit>("app_init"),

  repoOpen: (path: string) => call<RepoInfo>("repo_open", { path }),
  repoStatus: (repo: string) => call<RepoStatus>("repo_status", { repo }),
  repoDiff: (repo: string, path: string, staged: boolean) =>
    call<FileDiff>("repo_diff", { repo, path, staged }),

  stageFile: (repo: string, path: string) => call<void>("stage_file", { repo, path }),
  unstageFile: (repo: string, path: string) => call<void>("unstage_file", { repo, path }),
  discardFile: (repo: string, path: string) => call<void>("discard_file", { repo, path }),
  stageAll: (repo: string) => call<void>("stage_all", { repo }),
  unstageAll: (repo: string) => call<void>("unstage_all", { repo }),
  toggleHunk: (
    repo: string,
    path: string,
    hunkIndex: number,
    expectedHeader: string,
    staged: boolean,
  ) => call<void>("toggle_hunk", { repo, path, hunkIndex, expectedHeader, staged }),
  toggleLines: (repo: string, path: string, lines: LineRef[], staged: boolean) =>
    call<void>("toggle_lines", { repo, path, lines, staged }),

  commit: (repo: string, message: string, amend: boolean) =>
    call<CommitResult>("commit", { repo, message, amend }),

  repoBranches: (repo: string) => call<BranchInfo[]>("repo_branches", { repo }),
  checkoutBranch: (repo: string, branch: string) =>
    call<void>("checkout_branch", { repo, branch }),
  createBranch: (repo: string, branch: string) =>
    call<void>("create_branch", { repo, branch }),
  mergeBranch: (repo: string, branch: string) =>
    call<MergeOutcome>("merge_branch", { repo, branch }),
  abortMerge: (repo: string) => call<void>("abort_merge", { repo }),
  rebaseBranch: (repo: string, branch: string) =>
    call<MergeOutcome>("rebase_branch", { repo, branch }),
  abortRebase: (repo: string) => call<void>("abort_rebase", { repo }),

  repoLog: (repo: string, limit = 100) => call<CommitSummary[]>("repo_log", { repo, limit }),
  repoGraph: (repo: string, limit = 100) => call<GraphRow[]>("repo_graph", { repo, limit }),
  commitFiles: (repo: string, hash: string) => call<CommitFile[]>("commit_files", { repo, hash }),
  commitDiff: (repo: string, hash: string, path: string) =>
    call<FileDiff>("commit_diff", { repo, hash, path }),
};
