import { useEffect, useMemo, useRef, useState } from "react";
import type { ItemsTab, WikiData } from "../types";
import { fuzzyScore, taskScore } from "../util";
import { ItemIcon, TraderAvatar } from "./Icons";

interface Hit {
  key: string;
  score: number;
  label: string;
  sub: string;
  trader?: string;
  icon?: string | null;
  pick: () => void;
}

const KIND_LABEL = { ammo: "弾薬", food: "食料", medical: "医薬品" } as const;

export default function Palette({
  data, onTask, onItem, onClose,
}: {
  data: WikiData;
  onTask: (id: string) => void;
  onItem: (tab: ItemsTab, name: string) => void;
  onClose: () => void;
}) {
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);

  const hits = useMemo(() => {
    if (!q.trim()) return [] as Hit[];
    const out: Hit[] = [];
    for (const t of data.tasks) {
      const score = taskScore(q, t);
      if (score > 0) out.push({ key: `t:${t.id}`, score, label: t.title, sub: `タスク · ${t.trader}`, trader: t.trader, pick: () => onTask(t.id) });
    }
    const items: [ItemsTab, { name: string; icon: string | null }[]][] = [
      ["ammo", data.ammo],
      ["food", data.provisions],
      ["medical", data.medical],
    ];
    for (const [tab, rows] of items) {
      for (const r of rows) {
        const score = fuzzyScore(q, r.name);
        if (score > 0) out.push({ key: `${tab}:${r.name}`, score: score * 0.95, label: r.name, sub: KIND_LABEL[tab], icon: r.icon, pick: () => onItem(tab, r.name) });
      }
    }
    return out.sort((a, b) => b.score - a.score).slice(0, 14);
  }, [q, data, onTask, onItem]);

  const choose = (h: Hit | undefined) => {
    if (!h) return;
    h.pick();
    onClose();
  };

  return (
    <div className="palette-backdrop" onMouseDown={onClose}>
      <div className="palette" onMouseDown={(e) => e.stopPropagation()}>
        <input
          ref={input}
          value={q}
          placeholder="タスク・アイテムを検索(例: bgc → Background Check / salewa / prapor)"
          onChange={(e) => { setQ(e.target.value); setSel(0); }}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
            else if (e.key === "ArrowDown") { e.preventDefault(); setSel((s) => Math.min(s + 1, hits.length - 1)); }
            else if (e.key === "ArrowUp") { e.preventDefault(); setSel((s) => Math.max(s - 1, 0)); }
            else if (e.key === "Enter") choose(hits[sel]);
          }}
        />
        <ul>
          {hits.map((h, i) => (
            <li key={h.key} className={i === sel ? "sel" : ""} onMouseEnter={() => setSel(i)} onClick={() => choose(h)}>
              {h.trader ? <TraderAvatar name={h.trader} size={36} /> : <ItemIcon file={h.icon} size={36} />}
              <span className="label">{h.label}</span>
              <span className="muted small">{h.sub}</span>
            </li>
          ))}
          {q.trim() && hits.length === 0 && <li className="muted">一致するものがありません</li>}
        </ul>
        <div className="muted small hint">↑↓ で選択 / Enter で開く / Esc で閉じる</div>
      </div>
    </div>
  );
}
