export type RefKind = "head" | "branch" | "remote" | "tag";

export interface RefBadge {
  label: string;
  kind: RefKind;
}

/** 解析 --decorate=full 的 ref 全名，拿到准确的类型和短名字。 */
export function parseRef(raw: string): RefBadge {
  let value = raw.trim();
  let kind: RefKind = "branch";

  if (value.startsWith("HEAD -> ")) {
    kind = "head";
    value = value.slice("HEAD -> ".length);
  }
  if (value.startsWith("tag: ")) {
    kind = "tag";
    value = value.slice(5);
  }

  if (value.startsWith("refs/heads/")) {
    value = value.slice("refs/heads/".length);
  } else if (value.startsWith("refs/remotes/")) {
    if (kind !== "head") kind = "remote";
    value = value.slice("refs/remotes/".length);
  } else if (value.startsWith("refs/tags/")) {
    kind = "tag";
    value = value.slice("refs/tags/".length);
  }

  return { label: value, kind };
}
