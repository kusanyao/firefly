function pad(value: number): string {
  return String(value).padStart(2, "0");
}

/** 列表里用的短时间：今年的省略年份。 */
export function formatWhen(timestamp: number): string {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const ymd = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  const hm = `${pad(date.getHours())}:${pad(date.getMinutes())}`;
  return date.getFullYear() === now.getFullYear() ? `${ymd.slice(5)} ${hm}` : `${ymd} ${hm}`;
}

/** 详情头部用的完整时间。 */
export function formatFull(timestamp: number): string {
  if (!timestamp) return "";
  const date = new Date(timestamp * 1000);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}`;
}
