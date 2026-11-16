// Scan-time display: a snapshot from yesterday must not look like a
// plausible "12:03:44" of today — show the date once it's not today.
export function formatScanTime(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const now = new Date();
  const sameDay =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  const time = d.toLocaleTimeString();
  if (sameDay) return time;
  return `${d.toLocaleDateString()} ${time}`;
}
