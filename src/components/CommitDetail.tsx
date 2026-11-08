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
