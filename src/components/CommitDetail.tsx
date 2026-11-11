import { useState } from "react";
import { Cherry } from "lucide-react";
import { splitPath } from "../lib/changes";
import { formatFull } from "../lib/format";
import { parseRef } from "../lib/refs";
import { useRepo } from "../stores/repo";
import { DiffBody } from "./DiffBody";

export function CommitDetail() {
  const commit = useRepo((s) => s.selectedCommit);
  const files = useRepo((s) => s.commitFiles);
  const filesLoading = useRepo((s) => s.commitFilesLoading);
  const selectedFile = useRepo((s) => s.selectedCommitFile);
  const diff = useRepo((s) => s.commitDiff);
  const diffLoading = useRepo((s) => s.commitDiffLoading);
  const status = useRepo((s) => s.status);
  const [pendingCherry, setPendingCherry] = useState(false);

  const operationBusy = Boolean(
    status?.merging || status?.rebasing || status?.cherryPicking,
  );

  if (!commit) {
    return (
      <main className="main">
        <div className="diff-empty">选择一条提交查看内容</div>
      </main>
    );
  }

  return (
    <main className="main commit-detail">
      <div className="commit-head">
        <div className="commit-head-main">
          <div className="commit-title">{commit.subject || "(无提交信息)"}</div>
          <div className="commit-head-meta">
            <span className="commit-hash">{commit.shortHash}</span>
            <span>{commit.author}</span>
            <span>{formatFull(commit.timestamp)}</span>
            {commit.refs && (
              <span className="commit-head-refs">
                {commit.refs.split(",").map((raw) => {
                  const badge = parseRef(raw);
                  return (
                    <span className={`ref ref-${badge.kind}`} key={raw}>
                      {badge.label}
                    </span>
                  );
                })}
              </span>
            )}
          </div>
        </div>
        <button
          className="btn btn-sm"
          disabled={operationBusy}
          onClick={() => setPendingCherry(true)}
          title="把该提交遴选到当前分支"
        >
          <Cherry size={13} />
          遴选
        </button>
      </div>

      {pendingCherry && (
        <div className="modal-backdrop" onMouseDown={() => setPendingCherry(false)}>
          <div className="modal" onMouseDown={(event) => event.stopPropagation()}>
            <div className="modal-title">遴选 {commit.shortHash} 到当前分支？</div>
            <div className="modal-body">
              会在当前分支创建一个内容相同的新提交。工作区有未提交改动时 git 可能拒绝；
              出现冲突时会停在遴选状态，可中止。
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setPendingCherry(false)}>
                取消
              </button>
              <button
                className="btn btn-primary"
                onClick={() => {
                  const hash = commit.hash;
                  setPendingCherry(false);
                  void useRepo.getState().cherryPick(hash);
                }}
              >
                遴选
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="commit-files">
        {filesLoading && <div className="section-empty">加载中…</div>}
        {!filesLoading && files.length === 0 && (
          <div className="section-empty">这次提交没有文件改动（可能是合并提交）</div>
        )}
        {files.map((file) => {
          const { dir, name } = splitPath(file.path);
          return (
            <div
              key={`${file.status}-${file.path}`}
              className={`commit-file-row${selectedFile === file.path ? " selected" : ""}`}
              onClick={() => void useRepo.getState().selectCommitFile(file.path)}
              title={file.origPath ? `${file.origPath} → ${file.path}` : file.path}
            >
              <span className={`badge badge-${file.status.toLowerCase()}`}>{file.status}</span>
              <span className="row-path">
                <span className="dir">{dir}</span>
                <span className="name">{name}</span>
              </span>
            </div>
          );
        })}
      </div>

      <div className="commit-diff-panel">
        {diffLoading ? (
          <div className="diff-empty">加载中…</div>
        ) : !diff ? (
          <div className="diff-empty">选择一个文件查看改动</div>
        ) : diff.tooLarge ? (
          <div className="diff-empty">文件改动超过 2 MB，未渲染。</div>
        ) : diff.binary ? (
          <div className="diff-empty">二进制文件，无法显示文本 diff。</div>
        ) : diff.hunks.length === 0 ? (
          <div className="diff-empty">没有行级改动（可能是合并提交或仅模式变化）。</div>
        ) : (
          <DiffBody diff={diff} />
        )}
      </div>
    </main>
  );
}
