import type { RepoStatus, StatusEntry } from "../types";

export interface ChangeRow {
  entry: StatusEntry;
  staged: boolean;
}

export interface ChangeGroups {
  unstaged: ChangeRow[];
  staged: ChangeRow[];
  all: ChangeRow[];
}

/** 未暂存列表在前、已暂存列表在后；同一文件可能同时出现在两边。 */
export function groupChanges(status: RepoStatus | null): ChangeGroups {
  const entries = status?.entries ?? [];
  const unstaged = entries
    .filter((entry) => entry.unstaged || entry.untracked)
    .map((entry) => ({ entry, staged: false }));
  const staged = entries
    .filter((entry) => entry.staged)
    .map((entry) => ({ entry, staged: true }));
  return { unstaged, staged, all: [...unstaged, ...staged] };
}

export function splitPath(path: string): { dir: string; name: string } {
  const index = path.lastIndexOf("/");
  if (index < 0) return { dir: "", name: path };
  return { dir: path.slice(0, index + 1), name: path.slice(index + 1) };
}

export function statusBadge(row: ChangeRow): string {
  if (row.entry.untracked) return "?";
  if (row.entry.conflicted) return "!";
  const status = row.staged ? row.entry.indexStatus : row.entry.worktreeStatus;
  return status === "." ? " " : status;
}
