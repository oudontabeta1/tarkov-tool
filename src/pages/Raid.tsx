import { useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Actions } from "../App";
import type { Quest, UserState } from "../types";
import { TraderAvatar } from "../components/Icons";
import WikiLink from "../components/WikiLink";
import { KIND_LABEL, MAPS, wikiUrl } from "../util";

interface Props {
  tasks: Quest[];
  user: UserState;
  actions: Actions;
  /** jump to the task's detail view (Tasks tab) */
  onOpen: (id: string) => void;
  pointInfo: Record<string, { count: number; maps: string[] }>;
  /** open the map view for this map with the "今やる" tasks */
  onShowMap: (map: string) => void;
}

export default function Raid({ tasks, user, actions, onOpen, pointInfo, onShowMap }: Props) {
  const [map, setMap] = useState(MAPS[0]);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [opening, setOpening] = useState(false);

  const planned = useMemo(
    () => tasks.filter((t) => user.status[t.id] === "planned" && (t.maps.length === 0 || t.maps.includes(map))),
    [tasks, user, map],
  );
  const candidates = useMemo(
    () => tasks.filter((t) => !user.status[t.id] && t.maps.includes(map)),
    [tasks, user, map],
  );
  const toggle = (id: string) =>
    setChecked((s) => {
      const n = new Set(s);
      if (!n.delete(id)) n.add(id);
      return n;
    });

  const finish = async () => {
    setBusy(true);
    try {
      await actions.finishRaid(map, [...checked].filter((id) => planned.some((t) => t.id === id)));
      setChecked(new Set());
    } finally {
      setBusy(false);
    }
  };
  const chosen = planned.filter((t) => checked.has(t.id)).length;

  /** Open the wiki page of every task shown below in the system browser (one tab each). */
  const openAllInBrowser = async () => {
    if (planned.length === 0) return;
    if (planned.length > 8 && !confirm(`${planned.length} 件のタスクのWikiページをブラウザで開きます。よろしいですか?`)) return;
    setOpening(true);
    try {
      for (const t of planned) {
        await openUrl(wikiUrl(t.title)).catch(() => {});
        await new Promise((r) => setTimeout(r, 250)); // let the browser keep up
      }
    } finally {
      setOpening(false);
    }
  };

  return (
    <div className="raid pad">
      <div className="row">
        <label>
          マップ
          <select value={map} onChange={(e) => { setMap(e.target.value); setChecked(new Set()); }}>
            {MAPS.map((m) => <option key={m}>{m}</option>)}
          </select>
        </label>
        <button onClick={() => onShowMap(map)} title="「今やる」タスクの地点を、地図にまとめて表示します">
          📍 {map} の地点を地図で見る({planned.filter((t) => pointInfo[t.id]?.maps.includes(map)).length}件)
        </button>
        <button onClick={() => actions.openMap(map)}>🗺 Wikiのマップを開く</button>
        <button disabled={planned.length === 0 || opening} onClick={openAllInBrowser} title="表示中のタスクのWikiページを、すべてブラウザで開きます">
          {opening ? "開いています…" : `🌐 表示中の ${planned.length} 件をブラウザで開く`}
        </button>
        <button className="primary" disabled={busy || chosen === 0} onClick={finish}>
          レイド終了:チェックした {chosen} 件を完了にする
        </button>
      </div>

      <h3>「今やる」タスク({planned.length})</h3>
      {planned.length === 0 && <p className="muted">このマップで「今やる」にしたタスクはありません。下の候補から追加するか、タスク画面で設定してください。</p>}
      <div className="cards">
        {planned.map((t) => (
          <label key={t.id} className={`card ${checked.has(t.id) ? "checked" : ""}`}>
            <input type="checkbox" checked={checked.has(t.id)} onChange={() => toggle(t.id)} />
            <div>
              <div className="card-title">
                <TraderAvatar name={t.trader} size={36} />
                <button className="link title-link" title="タスクの詳細を開く" onClick={(e) => { e.preventDefault(); onOpen(t.id); }}>
                  {t.title}
                </button>
                <span className="muted small">{t.trader}</span>
              </div>
              <ul className="plain">
                {t.objectives.map((o, i) => (
                  <li key={i}>
                    <span className={`kind k-${o.kind}`}>{KIND_LABEL[o.kind] ?? o.kind}</span> {o.text}
                    {o.fir && <span className="chip fir">FIR</span>}
                  </li>
                ))}
              </ul>
            </div>
            <div className="card-actions">
              <button className="small-btn" onClick={(e) => { e.preventDefault(); onOpen(t.id); }}>詳細 ▶</button>
              <span onClick={(e) => e.preventDefault()}><WikiLink title={t.title} /></span>
              <button className="link" onClick={(e) => { e.preventDefault(); actions.setStatus(t.id, "todo"); }}>外す</button>
            </div>
          </label>
        ))}
      </div>

      <details>
        <summary>このマップの未着手タスク({candidates.length})から「今やる」に追加</summary>
        <ul className="plain candidates">
          {candidates.map((t) => (
            <li key={t.id}>
              <button className="link" onClick={() => actions.setStatus(t.id, "planned")}>＋ 今やる</button>
              <button className="link title-link" onClick={() => onOpen(t.id)}>{t.title}</button>
              <span className="muted small"> {t.trader}</span>
            </li>
          ))}
        </ul>
      </details>
    </div>
  );
}
