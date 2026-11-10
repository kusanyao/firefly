export interface StatusEntry {
  path: string;
  origPath: string | null;
  indexStatus: string;
  worktreeStatus: string;
  staged: boolean;
  unstaged: boolean;
  untracked: boolean;
  conflicted: boolean;
}

export interface RepoStatus {
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  detached: boolean;
  initial: boolean;
  merging: boolean;
  rebasing: boolean;
  entries: StatusEntry[];
}

export type DiffLineKind = "context" | "add" | "del" | "nonewline";

export interface DiffLine {
  kind: DiffLineKind;
  oldNo: number | null;
  newNo: number | null;
  text: string;
}

export interface LineRef {
  hunk: number;
  line: number;
}

export interface Hunk {
  index: number;
  header: string;
  section: string;
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: DiffLine[];
}

export interface FileDiff {
  path: string;
  staged: boolean;
  binary: boolean;
  untracked: boolean;
  tooLarge: boolean;
  headerLines: string[];
  hunks: Hunk[];
  additions: number;
  deletions: number;
}

export interface RepoInfo {
  root: string;
  status: RepoStatus;
}

export interface CommitResult {
  hash: string;
  subject: string;
}

export interface BranchInfo {
  name: string;
  current: boolean;
  upstream: string | null;
}

export interface RemoteBranch {
  name: string;
  remote: string;
  branch: string;
}

export interface CommitSummary {
  hash: string;
  shortHash: string;
  author: string;
  timestamp: number;
  subject: string;
  refs: string;
}

export interface MergeOutcome {
  conflict: boolean;
  message: string;
}

export interface GraphRow {
  graph: string;
  commit: CommitSummary | null;
}

export interface CommitFile {
  path: string;
  origPath: string | null;
  status: string;
}

export interface AppInit {
  recent: string[];
  lastRepo: string | null;
  version: string;
}

export interface AppError {
  kind: string;
  message: string;
  detail?: string;
}
