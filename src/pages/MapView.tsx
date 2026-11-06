import { useEffect, useMemo, useRef, useState } from "react";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import type { Actions } from "../App";
import { TraderAvatar } from "../components/Icons";
import { makeCrs, toLatLng, toLatLngBounds } from "../mapcrs";
import type { MapData, MapMode, Quest, TaskPoint, UserState } from "../types";
import { MAPS } from "../util";

const GLYPH: Record<string, string> = {
  visit: "◎", mark: "✚", plantItem: "▼", plantQuestItem: "▼", findItem: "◆", findQuestItem: "?", shoot: "⌖", buildWeapon: "⚙",
};
const KIND_JA: Record<string, string> = {
  visit: "到達", mark: "マーク", plantItem: "アイテム設置", plantQuestItem: "クエストアイテム設置", findItem: "アイテム入手",
  findQuestItem: "クエストアイテム入手", shoot: "射撃・討伐", extract: "脱出", buildWeapon: "武器改造",
};
const MODES: [MapMode, string][] = [["planned", "今やる"], ["open", "未完了すべて"], ["all", "すべて"], ["task", "選んだ1件"]];

const hueOf = (id: string) => [...id].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 360, 11);
const colorOf = (id: string) => `hsl(${hueOf(id)} 70% 55%)`;

function pointIcon(kind: string, color: string) {
  return L.divIcon({
    className: "pt-icon",
    html: `<span style="background:${color}">${GLYPH[kind] ?? "●"}</span>`,
    iconSize: [28, 28],
    iconAnchor: [14, 14],
  });
}

function popupFor(task: Quest, p: TaskPoint, open: (id: string) => void) {
  const box = document.createElement("div");
  box.className = "pt-popup";
  const title = document.createElement("b");
  title.textContent = task.title;
  const what = document.createElement("div");
  what.textContent = `${KIND_JA[p.kind] ?? p.kind}${p.label ? ` — ${p.label}` : ""}`;
  const btn = document.createElement("button");
  btn.textContent = "タスクの詳細 ▶";
  btn.onclick = () => open(task.id);
  box.append(title, what, btn);
  return box;
}

interface Props {
  tasks: Quest[];
  user: UserState;
  mapData: MapData;
  actions: Actions;
  map: string;
  mode: MapMode;
  focus: string | null;
  onChange: (patch: { map?: string; mapMode?: MapMode; mapTask?: string | null }) => void;
  onOpenTask: (id: string) => void;
}

export default function MapView({ tasks, user, mapData, actions, map, mode, focus, onChange, onOpenTask }: Props) {
  const info = mapData.maps.find((m) => m.name === map) ?? mapData.maps[0];
  const status = (id: string) => user.status[id] ?? "todo";
  const byId = useMemo(() => new Map(tasks.map((t) => [t.id, t])), [tasks]);
  const [hidden, setHidden] = useState<Set<string>>(new Set());

  const countByMap = useMemo(() => {
    const m = new Map<string, Set<string>>();
    for (const p of mapData.points) m.set(p.map, (m.get(p.map) ?? new Set()).add(p.task_id));
    return m;
  }, [mapData.points]);

  const mapPoints = useMemo(() => mapData.points.filter((p) => p.map === info?.name && byId.has(p.task_id)), [mapData.points, info, byId]);

  const listed = useMemo(() => {
    const ids = new Set(mapPoints.map((p) => p.task_id));
    const allowed = (id: string) =>
      mode === "task" ? id === focus : mode === "planned" ? status(id) === "planned" : mode === "open" ? status(id) !== "done" : true;
    return tasks.filter((t) => ids.has(t.id) && allowed(t.id));
    // eslint-disable-next-line
  }, [tasks, mapPoints, mode, focus, user]);

  const visible = useMemo(() => new Set(listed.filter((t) => !hidden.has(t.id)).map((t) => t.id)), [listed, hidden]);
  const visibleKey = [...visible].join(",");

  // ---- Leaflet ----
  const el = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);
  const layerRef = useRef<L.LayerGroup | null>(null);
  const lastFit = useRef("");
  const openRef = useRef(onOpenTask);
  openRef.current = onOpenTask;

  useEffect(() => {
    const base = info?.svg_bounds ?? info?.bounds;
    if (!el.current || !info || !base) return;
    const m = L.map(el.current, { crs: makeCrs(info), minZoom: info.min_zoom, maxZoom: info.max_zoom + 1, zoomSnap: 0.25, zoomDelta: 0.5 });
    const bounds = toLatLngBounds(base);
    if (info.svg) L.imageOverlay(info.svg, bounds).addTo(m);
    else if (info.tiles && info.bounds) {
      L.tileLayer(info.tiles, { minZoom: info.min_zoom, maxZoom: info.max_zoom + 1, maxNativeZoom: info.max_zoom, bounds: toLatLngBounds(info.bounds) }).addTo(m);
    }
    m.attributionControl.addAttribution("地図: tarkov.dev");
    m.fitBounds(bounds);
    layerRef.current = L.layerGroup().addTo(m);
    mapRef.current = m;
    lastFit.current = "";
    return () => {
      m.remove();
      mapRef.current = null;
      layerRef.current = null;
    };
  }, [info]);

  useEffect(() => {
    const m = mapRef.current;
    const g = layerRef.current;
    if (!m || !g || !info) return;
    g.clearLayers();
    const all: L.LatLng[] = [];
    for (const p of mapPoints) {
      const task = byId.get(p.task_id);
      if (!task || !visible.has(p.task_id)) continue;
      const color = colorOf(task.id);
      const ll = toLatLng(p.x, p.z);
      all.push(ll);
      if (p.outline.length >= 3) {
        L.polygon(p.outline.map(([x, z]) => toLatLng(x, z)), { color, weight: 2, fillColor: color, fillOpacity: 0.18 }).addTo(g);
      }
      L.marker(ll, { icon: pointIcon(p.kind, color), keyboard: false })
        .bindTooltip(task.title, { direction: "top", offset: [0, -10] })
        .bindPopup(popupFor(task, p, (id) => openRef.current(id)))
        .addTo(g);
    }
    const token = `${info.name}|${mode}|${focus}`;
    if (all.length > 0 && lastFit.current !== token) {
      lastFit.current = token;
      m.fitBounds(L.latLngBounds(all).pad(0.3), { maxZoom: info.max_zoom - 1 });
    }
    // eslint-disable-next-line
  }, [info, mapPoints, visibleKey, mode, focus]);

  const fitTask = (id: string) => {
    const m = mapRef.current;
    if (!m || !info) return;
    const ll = mapPoints.filter((p) => p.task_id === id).map((p) => toLatLng(p.x, p.z));
    if (ll.length) m.fitBounds(L.latLngBounds(ll).pad(0.4), { maxZoom: info.max_zoom - 1 });
    onChange({ mapTask: id });
  };
  const toggle = (id: string) =>
    setHidden((h) => {
      const n = new Set(h);
      if (!n.delete(id)) n.add(id);
      return n;
    });

  if (mapData.maps.length === 0) {
    return (
      <div className="pad">
        <p>マップの座標データがありません。「Wikiから同期」を実行してください。</p>
        {mapData.last_error && <p className="bad small">前回の取得エラー: {mapData.last_error}</p>}
      </div>
    );
  }
  const order = (n: string) => MAPS.indexOf(n);

  return (
    <div className="mapview">
      <aside className="map-side">
        <div className="map-controls">
          <select value={info?.name} onChange={(e) => onChange({ map: e.target.value })}>
            {[...mapData.maps].sort((a, b) => order(a.name) - order(b.name)).map((m) => (
              <option key={m.name} value={m.name}>
                {m.name}({countByMap.get(m.name)?.size ?? 0})
              </option>
            ))}
          </select>
          <div className="mode-tabs">
            {MODES.map(([k, label]) => (
              <button key={k} className={mode === k ? "active" : ""} onClick={() => onChange({ mapMode: k })}>{label}</button>
            ))}
          </div>
          <div className="row small">
            <button className="small-btn" onClick={() => setHidden(new Set())}>すべて表示</button>
            <button className="small-btn" onClick={() => setHidden(new Set(listed.map((t) => t.id)))}>すべて非表示</button>
            <span className="muted">{listed.length} 件</span>
          </div>
        </div>
        <ul className="map-tasks">
          {listed.length === 0 && (
            <li className="muted pad-s">
              {mode === "planned" ? "このマップに「今やる」タスクの地点はありません。モードを「未完了すべて」にするか、タスクの「+」で追加してください。" : "表示できる地点がありません。"}
            </li>
          )}
          {listed.map((t) => {
            const n = mapPoints.filter((p) => p.task_id === t.id).length;
            const st = status(t.id);
            return (
              <li key={t.id} className={focus === t.id ? "sel" : ""}>
                <input type="checkbox" checked={!hidden.has(t.id)} onChange={() => toggle(t.id)} title="地図に表示" />
                <span className="dot" style={{ background: colorOf(t.id) }} />
                <TraderAvatar name={t.trader} size={32} />
                <button className="link title-link" onClick={() => fitTask(t.id)} title="この地点に移動">
                  {t.title}
                </button>
                <span className="muted small">{n}</span>
                <button className="small-btn" onClick={() => onOpenTask(t.id)}>詳細</button>
                {st !== "done" && (
                  <button className={`plus ${st === "planned" ? "on" : ""}`} title={st === "planned" ? "「今やる」から外す" : "「今やる」に追加"} onClick={() => actions.setStatus(t.id, st === "planned" ? "todo" : "planned")}>
                    {st === "planned" ? "✓" : "+"}
                  </button>
                )}
              </li>
            );
          })}
        </ul>
        <p className="muted small pad-s">
          座標: tarkov.dev のデータ / 地図画像: tarkov.dev(CC BY-NC-SA)。座標を持たないタスク(討伐のみなど)は表示されません。
        </p>
      </aside>
      <div className="map-canvas" ref={el} />
    </div>
  );
}
