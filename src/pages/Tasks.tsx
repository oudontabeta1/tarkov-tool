import { useEffect, useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Actions } from "../App";
import { ItemIcon, TraderAvatar } from "../components/Icons";
import { MoneyIcon, RepLine, SkillIcon, UnlockIcon, XpIcon } from "../components/RewardIcons";
import type { Quest, Status, Tag, UserState } from "../types";
import { KIND_LABEL, MAPS, STATUS_LABEL, fmt, isLocked, slug, taskScore, wikiUrl } from "../util";

type StatusFilter = "all" | "open" | Status | "locked";
const TAG_COLORS = ["#8aa4ff", "#ffb454", "#7ad6a0", "#ff7a90", "#c792ea", "#5ccfe6"];

interface Props {
  tasks: Quest[];
  user: UserState;
  actions: Actions;
  selected: string | null;
  onSelect: (id: string) => void;
  /** quests that have positions on a map */
  pointInfo: Record<string, { count: number; maps: string[] }>;
  onShowOnMap: (id: string) => void;
}

export default function Tasks({ tasks, user, actions, selected, onSelect, pointInfo, onShowOnMap }: Props) {
  const [query, setQuery] = useState("");
  const [trader, setTrader] = useState("");
  const [map, setMap] = useState("");
  const [status, setStatus] = useState<StatusFilter>("open");
  const [tag, setTag] = useState(0);

  const done = (id: string) => user.status[id] === "done";
  const byId = useMemo(() => new Map(tasks.map((t) => [t.id, t])), [tasks]);
  const traders = useMemo(() => [...new Set(tasks.map((t) => t.trader).filter(Boolean))], [tasks]);

  const list = useMemo(() => {
    const scored: { t: Quest; score: number }[] = [];
    for (const t of tasks) {
      const st = user.status[t.id] ?? "todo";
      if (trader && t.trader !== trader) continue;
      if (map && !t.maps.includes(map)) continue;
      if (tag && !(user.task_tags[t.id] ?? []).includes(tag)) continue;
      const locked = st !== "done" && isLocked(t, done);
      const ok =
        status === "all" ? true : status === "open" ? st !== "done" && !locked : status === "locked" ? locked : st === status;
      if (!ok) continue;
      const score = taskScore(query, t);
      if (score > 0) scored.push({ t, score });
    }
    if (query.trim()) scored.sort((a, b) => b.score - a.score);
    return scored.map((x) => x.t);
  }, [tasks, user, query, trader, map, status, tag]);

  useEffect(() => {
    if (selected) document.getElementById(`task-${selected}`)?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const current = (selected && byId.get(selected)) || null;
  const doneCount = tasks.filter((t) => done(t.id)).length;

  return (
    <div className="split">
      <section className="list-pane">
        <div className="filters">
          <input placeholder="検索(例: bgc → Background Check / ひとつ前の文字だけでもOK)" value={query} onChange={(e) => setQuery(e.target.value)} />
          <div className="trader-chips">
            <button className={`tchip ${trader === "" ? "on" : ""}`} onClick={() => setTrader("")}>全員</button>
            {traders.map((t) => (
              <button key={t} className={`tchip ${trader === t ? "on" : ""}`} title={t} onClick={() => setTrader(trader === t ? "" : t)}>
                <TraderAvatar name={t} size={44} />
              </button>
            ))}
          </div>
          <div className="row">
            <select value={map} onChange={(e) => setMap(e.target.value)}>
              <option value="">全マップ</option>
              {MAPS.map((m) => <option key={m}>{m}</option>)}
            </select>
            <select value={tag} onChange={(e) => setTag(Number(e.target.value))}>
              <option value={0}>全タグ</option>
              {user.tags.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
            </select>
          </div>
          <select value={status} onChange={(e) => setStatus(e.target.value as StatusFilter)}>
            <option value="open">進行可能(未完了・解放済み)</option>
            <option value="planned">今やる</option>
            <option value="todo">未着手</option>
            <option value="done">完了</option>
            <option value="locked">未解放(前提未完了)</option>
            <option value="all">すべて</option>
          </select>
          <div className="muted small">
            {list.length} 件表示 / 全 {tasks.length} 件(完了 {doneCount})
          </div>
        </div>
        <ul className="task-list">
          {list.map((t) => {
            const st = user.status[t.id] ?? "todo";
            const locked = st !== "done" && isLocked(t, done);
            return (
              <li key={t.id} id={`task-${t.id}`} className={`task-item ${selected === t.id ? "sel" : ""} st-${st}`}>
                <button className="task-row" onClick={() => onSelect(t.id)}>
                  <TraderAvatar name={t.trader} size={48} />
                  <span className="main">
                    <span className="title">
                      {locked && "🔒 "}
                      {t.title}
                    </span>
                    <span className="meta">
                      {pointInfo[t.id] && "📍 "}
                      {t.trader}
                      {t.maps.length > 0 && ` · ${t.maps.slice(0, 2).join(", ")}${t.maps.length > 2 ? ` +${t.maps.length - 2}` : ""}`}
                    </span>
                    <span className="chips">
                      {st !== "todo" && <span className={`chip st-${st}`}>{STATUS_LABEL[st]}</span>}
                      {(user.task_tags[t.id] ?? []).map((id) => {
                        const g = user.tags.find((x) => x.id === id);
                        return g ? <span key={id} className="chip" style={{ borderColor: g.color, color: g.color }}>{g.name}</span> : null;
                      })}
                    </span>
                  </span>
                </button>
                {st !== "done" && (
                  <button
                    className={`plus ${st === "planned" ? "on" : ""}`}
                    title={st === "planned" ? "「今やる」から外す" : "「今やる」に追加"}
                    onClick={() => actions.setStatus(t.id, st === "planned" ? "todo" : "planned")}
                  >
                    {st === "planned" ? "✓" : "+"}
                  </button>
                )}
              </li>
            );
          })}
        </ul>
      </section>

      <section className="detail-pane">
        {current ? (
          <Detail q={current} user={user} actions={actions} byId={byId} onOpen={onSelect} done={done} points={pointInfo[current.id]} onShowOnMap={onShowOnMap} />
        ) : (
          <p className="muted pad">左の一覧からタスクを選んでください。</p>
        )}
      </section>
    </div>
  );
}

/** Detail-pane text size (1 = base), remembered between runs. */
function useScale() {
  const [scale, set] = useState(() => {
    const v = Number(localStorage.getItem("detailScale"));
    return v >= 0.8 && v <= 2 ? v : 1.25;
  });
  return [
    scale,
    (v: number) => {
      const n = Math.min(2, Math.max(0.8, Math.round(v * 100) / 100));
      localStorage.setItem("detailScale", String(n));
      set(n);
    },
  ] as const;
}

function Detail({
  q, user, actions, byId, onOpen, done, points, onShowOnMap,
}: {
  points?: { count: number; maps: string[] };
  onShowOnMap: (id: string) => void;
  q: Quest;
  user: UserState;
  actions: Actions;
  byId: Map<string, Quest>;
  onOpen: (id: string) => void;
  done: (id: string) => boolean;
}) {
  const st = user.status[q.id] ?? "todo";
  const [newTag, setNewTag] = useState("");
  const [copied, setCopied] = useState(false);
  const [scale, setScale] = useScale();
  const url = wikiUrl(q.title);

  const link = (title: string) => {
    const t = byId.get(slug(title));
    return (
      <button key={title} className={`link ${t && done(t.id) ? "is-done" : ""}`} disabled={!t} onClick={() => t && onOpen(t.id)}>
        {title}
        {t && done(t.id) ? " ✓" : ""}
      </button>
    );
  };
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable */
    }
  };

  return (
    <div className="detail" style={{ ["--ds" as string]: scale }}>
      <div className="scale-ctl" title="詳細の文字サイズ">
        <button className="small-btn" onClick={() => setScale(scale - 0.1)}>A−</button>
        <button className="small-btn" onClick={() => setScale(scale + 0.1)}>A+</button>
      </div>
      <div className="detail-head">
        <TraderAvatar name={q.trader} size={80} />
        <div>
          <h2>{q.title}</h2>
          <div className="meta-line">
            <span>{q.trader}</span>
            {q.loyalty !== null && <span>忠誠度 LL{q.loyalty}</span>}
            {q.player_level !== null && <span>Lv{q.player_level}~</span>}
            {q.kappa && <span>Kappa: {q.kappa}</span>}
          </div>
        </div>
      </div>

      <div className="wiki-url">
        <span className="muted small">Wiki</span>
        <button className="link url" title="ブラウザで開く" onClick={() => openUrl(url).catch(() => {})}>{url}</button>
        <button className="small-btn" onClick={copy}>{copied ? "コピーしました" : "コピー"}</button>
      </div>

      {points && (
        <div className="map-buttons">
          <span className="muted small">地点</span>
          <button className="map-btn accent" onClick={() => onShowOnMap(q.id)} title="このタスクの地点を地図に表示">
            📍 地図で見る({points.count}件 / {points.maps.join("、")})
          </button>
        </div>
      )}
      {q.maps.length > 0 && (
        <div className="map-buttons">
          <span className="muted small">マップ</span>
          {q.maps.map((m) => (
            <button key={m} className="map-btn" onClick={() => actions.openMap(m)} title="Wikiのインタラクティブマップを別ウィンドウで開く">
              🗺 {m}
            </button>
          ))}
        </div>
      )}

      <div className="status-buttons">
        {(["todo", "planned", "done"] as Status[]).map((s) => (
          <button key={s} className={`st-btn st-${s} ${st === s ? "on" : ""}`} onClick={() => actions.setStatus(q.id, s)}>
            {STATUS_LABEL[s]}
          </button>
        ))}
      </div>

      <div className="tag-box">
        {user.tags.map((g: Tag) => {
          const on = (user.task_tags[q.id] ?? []).includes(g.id);
          return (
            <span key={g.id} className="tag-toggle">
              <button className={`chip tagbtn ${on ? "on" : ""}`} style={{ borderColor: g.color, color: on ? "#111" : g.color, background: on ? g.color : "transparent" }} onClick={() => actions.toggleTag(q.id, g.id)}>
                {g.name}
              </button>
              <button className="x" title="タグを削除" onClick={() => confirm(`タグ「${g.name}」を削除しますか?`) && actions.deleteTag(g.id)}>×</button>
            </span>
          );
        })}
        <input
          className="tag-input"
          placeholder="+ 新しいタグ"
          value={newTag}
          onChange={(e) => setNewTag(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && newTag.trim()) {
              actions.createTag(newTag, TAG_COLORS[user.tags.length % TAG_COLORS.length]);
              setNewTag("");
            }
          }}
        />
      </div>

      {(q.previous.length > 0 || q.previous_see_requirements || q.req_text.length > 0) && (
        <>
          <h3>前提条件</h3>
          {q.previous.length > 0 && (
            <p className="inline-links">
              前提タスク{q.previous_any ? "(いずれか)" : ""}: {q.previous.map(link)}
            </p>
          )}
          {q.req_text.length > 0 && (
            <ul className="plain">
              {q.req_text.map((t, i) => <li key={i}>{t}</li>)}
            </ul>
          )}
          {q.req_quests.length > 0 && <p className="inline-links">必要タスク: {q.req_quests.map(link)}</p>}
        </>
      )}

      <h3>目標</h3>
      <ol className="objectives">
        {q.objectives.length === 0 && <li className="muted">目標を読み取れませんでした(Wikiの書式が想定外の可能性)。</li>}
        {q.objectives.map((o, i) => (
          <li key={i}>
            <span className={`kind k-${o.kind}`}>{KIND_LABEL[o.kind] ?? o.kind}</span> {o.text}
            {o.fir && <span className="chip fir">FIR</span>}
            {o.maps.map((m) => <span key={m} className="chip map">{m}</span>)}
          </li>
        ))}
      </ol>

      {q.quest_items.length > 0 && (
        <>
          <h3>必要アイテム</h3>
          <table className="grid small">
            <thead><tr><th></th><th>アイテム</th><th className="num">数</th><th>種別</th><th>FIR</th></tr></thead>
            <tbody>
              {q.quest_items.map((it) => (
                <tr key={it.item}>
                  <td><ItemIcon name={it.item} size={64} /></td>
                  <td className="item-name">{it.item}</td>
                  <td className="num">{fmt(it.amount)}</td>
                  <td>{it.requirement}</td>
                  <td>{it.fir ? <span className="chip fir">FIR</span> : "–"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}

      <h3>報酬</h3>
      <ul className="rewards">
        {q.rewards.exp !== null && (
          <li><XpIcon size={52} /><span>経験値 <b>+{q.rewards.exp.toLocaleString()}</b> EXP</span></li>
        )}
        {q.rewards.rep.map((r, i) => <li key={`rep${i}`}><RepLine text={r} size={52} /></li>)}
        {q.rewards.money.map((r, i) => <li key={`m${i}`}><MoneyIcon text={r} size={52} /><span>{r}</span></li>)}
        {q.rewards.skills.map((r, i) => <li key={`s${i}`}><SkillIcon size={52} /><span>{r}</span></li>)}
        {q.rewards.items.map((r, i) => (
          <li key={`i${i}`}><ItemIcon name={r.name} size={56} /><span>{r.qty}× {r.name}</span></li>
        ))}
        {q.rewards.unlocks.map((r, i) => <li key={`u${i}`}><UnlockIcon size={52} /><span>{r}</span></li>)}
        {q.rewards.other.map((r, i) => <li key={`o${i}`} className="plain-reward"><span>{r}</span></li>)}
      </ul>

      {q.leads_to.length > 0 && <p className="inline-links">次のタスク: {q.leads_to.map(link)}</p>}
      <p className="muted small">出典: Escape from Tarkov Wiki「{q.title}」(CC BY-SA)</p>
    </div>
  );
}
