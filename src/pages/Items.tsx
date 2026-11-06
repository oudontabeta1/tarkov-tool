import { useEffect, useMemo, useState } from "react";
import { ItemIcon } from "../components/Icons";
import SortTable, { type Col } from "../components/SortTable";
import WikiLink from "../components/WikiLink";
import type { Ammo, ItemsTab, Medical, Provision, WikiData } from "../types";
import { fmt, fuzzyScore } from "../util";

type Tab = ItemsTab;

/** Keep a search box in sync with a query pushed in from outside (e.g. the Ctrl+K palette). */
function useSeededQuery(seed: string) {
  const [query, setQuery] = useState(seed);
  useEffect(() => setQuery(seed), [seed]);
  return [query, setQuery] as const;
}

export default function Items({ data, tab, onTab, seed }: { data: WikiData; tab: Tab; onTab: (t: Tab) => void; seed: string }) {
  return (
    <div className="items">
      <div className="subtabs">
        {([["ammo", `弾薬 (${data.ammo.length})`], ["food", `食料・飲料 (${data.provisions.length})`], ["medical", `医薬品 (${data.medical.length})`]] as [Tab, string][]).map(([k, l]) => (
          <button key={k} className={tab === k ? "active" : ""} onClick={() => onTab(k)}>{l}</button>
        ))}
      </div>
      {tab === "ammo" && <AmmoTab data={data} seed={seed} />}
      {tab === "food" && <FoodTab rows={data.provisions} seed={seed} />}
      {tab === "medical" && <MedicalTab rows={data.medical} seed={seed} />}
    </div>
  );
}

/** green = favourable, red = unfavourable */
const tone = (n: number | null, goodWhen: "pos" | "neg") =>
  n === null || n === 0 ? "" : (n > 0) === (goodWhen === "pos") ? "good" : "bad";
const signed = (n: number | null, goodWhen: "pos" | "neg") => <span className={tone(n, goodWhen)}>{fmt(n, true)}</span>;

function AmmoTab({ data, seed }: { data: WikiData; seed: string }) {
  const [caliber, setCaliber] = useState("");
  const [query, setQuery] = useSeededQuery(seed);
  const [weapon, setWeapon] = useState("");

  const calibers = useMemo(() => [...new Set(data.ammo.map((a) => a.caliber))], [data.ammo]);
  const w = weapon.trim().toLowerCase();
  const matchedWeapons = useMemo(
    () => (w ? data.ammo_weapons.filter((x) => x.weapon.toLowerCase().includes(w)) : []),
    [w, data.ammo_weapons],
  );
  const weaponCalibers = useMemo(() => new Set(matchedWeapons.map((x) => x.caliber)), [matchedWeapons]);

  const rows = useMemo(() => {
    return data.ammo.filter(
      (a) => (!caliber || a.caliber === caliber) && fuzzyScore(query, a.name) > 0 && (!w || weaponCalibers.has(a.caliber)),
    );
  }, [data.ammo, caliber, query, w, weaponCalibers]);

  const usedBy = caliber ? data.ammo_weapons.filter((x) => x.caliber === caliber) : [];
  const groups = useMemo(() => {
    const m = new Map<string, string[]>();
    usedBy.forEach((x) => m.set(x.class ?? "対応武器", [...(m.get(x.class ?? "対応武器") ?? []), x.weapon]));
    return [...m.entries()];
  }, [usedBy]);

  const cols: Col<Ammo>[] = [
    { key: "icon", label: "", get: () => null, render: (a) => <ItemIcon file={a.icon} size={48} /> },
    { key: "name", label: "名前", get: (a) => a.name, render: (a) => <>{a.name}<WikiLink title={a.name} />{a.tracer && <span className="chip">曳光</span>}{a.subsonic && <span className="chip">亜音速</span>}</> },
    { key: "caliber", label: "口径", get: (a) => a.caliber },
    { key: "dmg", label: "DMG", get: (a) => a.dmg, num: true },
    { key: "proj", label: "弾数", get: (a) => a.projectiles, num: true },
    { key: "pen", label: "貫通", get: (a) => a.pen, num: true },
    { key: "armor", label: "ｱｰﾏｰDMG%", get: (a) => a.armor_dmg, num: true },
    { key: "acc", label: "精度", get: (a) => a.accuracy, num: true, render: (a) => signed(a.accuracy, "pos") },
    { key: "rec", label: "反動", get: (a) => a.recoil, num: true, render: (a) => signed(a.recoil, "neg") },
    { key: "lb", label: "軽出血", get: (a) => a.light_bleed, num: true, render: (a) => signed(a.light_bleed, "pos") },
    { key: "hb", label: "重出血", get: (a) => a.heavy_bleed, num: true, render: (a) => signed(a.heavy_bleed, "pos") },
    { key: "dur", label: "耐久消費", get: (a) => a.dur_burn, num: true, render: (a) => signed(a.dur_burn, "neg") },
    { key: "speed", label: "弾速", get: (a) => a.speed, num: true },
    { key: "src", label: "入手元", get: (a) => a.source || null },
  ];

  return (
    <div className="pad">
      <div className="row">
        <select value={caliber} onChange={(e) => setCaliber(e.target.value)}>
          <option value="">全口径</option>
          {calibers.map((c) => <option key={c}>{c}</option>)}
        </select>
        <input placeholder="弾薬名で検索(あいまい検索可)" value={query} onChange={(e) => setQuery(e.target.value)} />
        <input placeholder="武器名から使える弾薬を探す(例: AK-74)" value={weapon} onChange={(e) => setWeapon(e.target.value)} />
      </div>
      {matchedWeapons.length > 0 && (
        <p className="muted small">
          一致した武器: {matchedWeapons.slice(0, 12).map((x) => `${x.weapon}(${x.caliber})`).join("、")}
          {matchedWeapons.length > 12 && ` ほか${matchedWeapons.length - 12}件`}
        </p>
      )}
      {groups.length > 0 && (
        <div className="usedby">
          <strong>{caliber} を使う武器</strong>
          {groups.map(([cls, ws]) => <p key={cls}><span className="muted">{cls}:</span> {ws.join("、")}</p>)}
        </div>
      )}
      <SortTable rows={rows} cols={cols} rowKey={(a) => `${a.caliber}/${a.name}`} defaultSort={{ key: "pen", desc: true }} />
    </div>
  );
}

function FoodTab({ rows, seed }: { rows: Provision[]; seed: string }) {
  const [query, setQuery] = useSeededQuery(seed);
  const list = useMemo(() => rows.filter((r) => fuzzyScore(query, r.name) > 0), [rows, query]);
  const cols: Col<Provision>[] = [
    { key: "icon", label: "", get: () => null, render: (r) => <ItemIcon file={r.icon} size={48} /> },
    { key: "name", label: "名前", get: (r) => r.name, render: (r) => <>{r.name}<WikiLink title={r.name} /></> },
    { key: "energy", label: "エネルギー", get: (r) => r.energy, num: true, render: (r) => signed(r.energy, "pos") },
    { key: "hyd", label: "水分", get: (r) => r.hydration, num: true, render: (r) => signed(r.hydration, "pos") },
    { key: "use", label: "使用時間(s)", get: (r) => r.use_time, num: true },
    { key: "buffs", label: "バフ", get: (r) => r.buffs || null, render: (r) => <span className="pre">{r.buffs || "–"}</span> },
    { key: "debuffs", label: "デバフ", get: (r) => r.debuffs || null, render: (r) => <span className="pre">{r.debuffs || "–"}</span> },
    { key: "barter", label: "物々交換", get: (r) => (r.barter ? 1 : 0), render: (r) => (r.barter ? "✔" : "–") },
    { key: "craft", label: "クラフト", get: (r) => (r.craft ? 1 : 0), render: (r) => (r.craft ? "✔" : "–") },
    { key: "notes", label: "タスクでの用途", get: (r) => r.notes.join(" ") || null, render: (r) => <span className="pre">{r.notes.join("\n") || "–"}</span> },
  ];
  return (
    <div className="pad">
      <div className="row"><input placeholder="名前で検索" value={query} onChange={(e) => setQuery(e.target.value)} /></div>
      <SortTable rows={list} cols={cols} rowKey={(r) => r.name} defaultSort={{ key: "energy", desc: true }} />
    </div>
  );
}

function MedicalTab({ rows, seed }: { rows: Medical[]; seed: string }) {
  const [query, setQuery] = useSeededQuery(seed);
  const [group, setGroup] = useState("");
  const groups = useMemo(() => [...new Set(rows.map((r) => r.group))], [rows]);
  const list = useMemo(
    () => rows.filter((r) => (!group || r.group === group) && fuzzyScore(query, r.name) > 0),
    [rows, group, query],
  );
  const cols: Col<Medical>[] = [
    { key: "icon", label: "", get: () => null, render: (r) => <ItemIcon file={r.icon} size={48} /> },
    { key: "name", label: "名前", get: (r) => r.name, render: (r) => <>{r.name}<WikiLink title={r.name} /></> },
    { key: "kind", label: "種類", get: (r) => r.kind || null },
    { key: "use", label: "使用時間(s)", get: (r) => r.use_time, num: true },
    { key: "uses", label: "使用回数", get: (r) => r.uses, num: true },
    { key: "hp", label: "HPプール", get: (r) => r.hp_pool, num: true },
    { key: "heal", label: "最大回復/回", get: (r) => r.max_heal, num: true },
    { key: "buffs", label: "効果", get: (r) => r.buffs || null, render: (r) => <span className="pre">{r.buffs || "–"}</span> },
    { key: "debuffs", label: "副作用", get: (r) => r.debuffs || null, render: (r) => <span className="pre">{r.debuffs || "–"}</span> },
  ];
  return (
    <div className="pad">
      <div className="row">
        <select value={group} onChange={(e) => setGroup(e.target.value)}>
          <option value="">すべて</option>
          {groups.map((g) => <option key={g}>{g}</option>)}
        </select>
        <input placeholder="名前で検索" value={query} onChange={(e) => setQuery(e.target.value)} />
      </div>
      <SortTable rows={list} cols={cols} rowKey={(r) => `${r.group}/${r.name}`} />
    </div>
  );
}
