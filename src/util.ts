import type { Quest, Status } from "./types";

export const MAPS = [
  "Customs", "Factory", "Ground Zero", "Icebreaker", "Interchange", "Lighthouse", "Reserve",
  "Shoreline", "Streets of Tarkov", "Terminal", "The Lab", "The Labyrinth", "Woods",
];

export const STATUS_LABEL: Record<Status, string> = { todo: "未着手", planned: "今やる", done: "完了" };

/** Same rule as `wikitext::slug` in the Rust core, so wiki titles map to task ids. */
export const slug = (s: string) =>
  s.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-+|-+$/g, "");

export const KIND_LABEL: Record<string, string> = {
  kill: "討伐", handover: "納品", find: "入手", extract: "脱出", place: "設置/マーク", gunsmith: "改造", other: "その他",
};

export function fmt(n: number | null | undefined, signed = false): string {
  if (n === null || n === undefined) return "–";
  const s = Number.isInteger(n) ? String(n) : n.toFixed(1);
  return signed && n > 0 ? `+${s}` : s;
}

/** A task is locked while its prerequisites are not all (or, for "A or B", none) done. */
export function isLocked(q: Quest, done: (id: string) => boolean): boolean {
  const prev = q.previous.map(slug);
  const prevOk = prev.length === 0 || (q.previous_any ? prev.some(done) : prev.every(done));
  const reqOk = q.req_quests.map(slug).every(done);
  return !(prevOk && reqOk);
}

export function dateLabel(unix: number | null): string {
  return unix ? new Date(unix * 1000).toLocaleString("ja-JP") : "未同期";
}

// ---------- wiki links ----------

export const WIKI_BASE = "https://escapefromtarkov.fandom.com/wiki/";

export const wikiUrl = (title: string) =>
  WIKI_BASE +
  encodeURIComponent(title.replace(/ /g, "_"))
    .replace(/%3A/g, ":")
    .replace(/%2F/g, "/")
    .replace(/%28/g, "(")
    .replace(/%29/g, ")")
    .replace(/%2C/g, ",")
    .replace(/%27/g, "'");

// ---------- search ----------

const fold = (s: string) =>
  s.toLowerCase().normalize("NFKD").replace(/[\u0300-\u036f]/g, "").replace(/[’'`]/g, "");

/**
 * 0 = no match, higher = better. Handles substrings, several word prefixes
 * ("back check"), initials ("bc") and loose subsequences ("bgc" -> Background Check).
 */
export function fuzzyScore(query: string, text: string): number {
  const q = fold(query).replace(/\s+/g, " ").trim();
  if (!q) return 1;
  const t = fold(text);
  const idx = t.indexOf(q);
  if (idx >= 0) {
    const boundary = idx === 0 || /[^a-z0-9]/.test(t[idx - 1]);
    return 1000 + (idx === 0 ? 200 : boundary ? 100 : 0) - Math.min(idx, 50) - Math.min(t.length - q.length, 50) * 0.5;
  }
  const words = t.split(/[^a-z0-9]+/).filter(Boolean);
  const toks = q.split(" ").filter(Boolean);
  if (toks.length > 1 && toks.every((k) => words.some((w) => w.startsWith(k)))) return 700;
  const compact = q.replace(/ /g, "");
  if (compact.length >= 2 && words.map((w) => w[0]).join("").startsWith(compact)) return 600;
  if (compact.length < 3) return 0;
  let pos = 0;
  let first = -1;
  let starts = 0;
  for (const ch of compact) {
    const at = t.indexOf(ch, pos);
    if (at < 0) return 0;
    if (first < 0) first = at;
    if (at === 0 || /[^a-z0-9]/.test(t[at - 1])) starts++;
    pos = at + 1;
  }
  const span = pos - first;
  if (span > compact.length * 5) return 0;
  // coarse buckets: near-equal matches keep the wiki's own order (Background Check before Big Customer)
  return Math.round((300 + starts * 40 - span * 0.2) / 10) * 10;
}

/** Title first, then trader name, then objective / item names. */
export function taskScore(query: string, t: Quest): number {
  if (!query.trim()) return 1;
  const s = Math.max(fuzzyScore(query, t.title), fuzzyScore(query, t.trader) * 0.5);
  if (s > 0) return s;
  const needle = fold(query.trim());
  if (needle.length < 3) return 0;
  const hay = [...t.objectives.map((o) => o.text), ...t.quest_items.map((i) => i.item), ...t.rewards.items.map((i) => i.name)];
  return hay.some((h) => fold(h).includes(needle)) ? 100 : 0;
}
