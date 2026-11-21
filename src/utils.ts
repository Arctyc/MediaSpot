export function initials(title: string): string {
  const parts = title.split(/\s+/).filter(Boolean).slice(0, 2);
  return parts.map((w) => w.charAt(0).toUpperCase()).join("") || "?";
}

export function parseDate(s?: string | null): Date | null {
  if (!s) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
  if (m) return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  const d = new Date(s);
  return isNaN(d.getTime()) ? null : d;
}

export function fmtDate(s?: string | null): string {
  const d = parseDate(s);
  return d ? d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) : "TBA";
}

export function fmtDateTime(s?: string | null): string {
  const d = parseDate(s);
  return d ? d.toLocaleString() : "";
}

export function relativeDays(s?: string | null): string {
  const d = parseDate(s);
  if (!d) return "";
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const day = new Date(d);
  day.setHours(0, 0, 0, 0);
  const n = Math.round((day.getTime() - today.getTime()) / 86400000);
  if (n === 0) return "today";
  if (n === 1) return "tomorrow";
  if (n === -1) return "yesterday";
  return n > 0 ? `in ${n} days` : `${-n} days ago`;
}

const pad2 = (n: number) => n.toString().padStart(2, "0");

export function epCode(season: number, number: number | null): string {
  return `S${pad2(season)}E${number == null ? "??" : pad2(number)}`;
}

export function epDate(ep: { airstamp: string | null; airdate: string | null }): string | null {
  return ep.airstamp ?? ep.airdate;
}
