import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { DataContext, type DataCtx } from "./context";
import type { ItemsTab, Loc, MapData, PageIssue, Status, SyncProgress, Tab, UserState, WikiData } from "./types";
import Palette from "./components/Palette";
import Tasks from "./pages/Tasks";
import Raid from "./pages/Raid";
import MapView from "./pages/MapView";
import Items from "./pages/Items";
import Settings from "./pages/Settings";

export interface Actions {
  setStatus: (id: string, s: Status) => Promise<void>;
  createTag: (name: string, color: string) => Promise<void>;
  deleteTag: (id: number) => Promise<void>;
  toggleTag: (taskId: string, tagId: number) => Promise<void>;
  finishRaid: (map: string, done: string[]) => Promise<void>;
  openMap: (map: string) => void;
}

const TABS: [Tab, string][] = [["tasks", "タスク"], ["raid", "レイド"], ["map", "マップ"], ["items", "アイテム"], ["settings", "設定"]];
const EMPTY_USER: UserState = { status: {}, tags: [], task_tags: {} };
const START: Loc = { tab: "tasks", taskId: null, itemsTab: "ammo", itemQuery: "", map: "Customs", mapMode: "planned", mapTask: null };
const NO_MAP_DATA: MapData = { maps: [], points: [], stats: null, last_error: null };

export default function App() {
  const [data, setData] = useState<WikiData | null>(null);
  const [user, setUser] = useState<UserState>(EMPTY_USER);
  const [issues, setIssues] = useState<PageIssue[]>([]);
  const [mapData, setMapData] = useState<MapData>(NO_MAP_DATA);
  const [syncing, setSyncing] = useState(false);
  const [progress, setProgress] = useState<SyncProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [palette, setPalette] = useState(false);

  // ---- browser-like history (mouse side buttons, Alt+←/→) ----
  const [nav, setNav] = useState<{ stack: Loc[]; i: number }>({ stack: [START], i: 0 });
  const loc = nav.stack[nav.i];
  const go = useCallback((patch: Partial<Loc>) => {
    setNav((n) => {
      const cur = n.stack[n.i];
      const next = { ...cur, ...patch };
      if (JSON.stringify(cur) === JSON.stringify(next)) return n;
      const stack = [...n.stack.slice(0, n.i + 1), next].slice(-100);
      return { stack, i: stack.length - 1 };
    });
  }, []);
  const back = useCallback(() => setNav((n) => (n.i > 0 ? { ...n, i: n.i - 1 } : n)), []);
  const forward = useCallback(() => setNav((n) => (n.i < n.stack.length - 1 ? { ...n, i: n.i + 1 } : n)), []);

  useEffect(() => {
    const onMouse = (e: MouseEvent) => {
      if (e.button !== 3 && e.button !== 4) return;
      e.preventDefault();
      e.stopPropagation();
      if (e.type === "mouseup") (e.button === 3 ? back : forward)();
    };
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName ?? "";
      const typing = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPalette(true);
      } else if (e.key === "/" && !typing) {
        e.preventDefault();
        setPalette(true);
      } else if ((e.altKey && e.key === "ArrowLeft") || e.key === "BrowserBack") {
        e.preventDefault();
        back();
      } else if ((e.altKey && e.key === "ArrowRight") || e.key === "BrowserForward") {
        e.preventDefault();
        forward();
      }
    };
    const opts = { capture: true } as const;
    for (const t of ["mousedown", "mouseup", "auxclick"] as const) window.addEventListener(t, onMouse, opts);
    window.addEventListener("keydown", onKey, opts);
    return () => {
      for (const t of ["mousedown", "mouseup", "auxclick"] as const) window.removeEventListener(t, onMouse, opts);
      window.removeEventListener("keydown", onKey, opts);
    };
  }, [back, forward]);

  // ---- data ----
  const reloadUser = useCallback(async () => {
    try {
      setUser(await api.userState());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const reload = useCallback(async () => {
    try {
      const [d, u, i, md] = await Promise.all([api.wikiData(), api.userState(), api.issues(), api.mapData()]);
      setData(d);
      setMapData(md);
      setUser(u);
      setIssues(i);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    reload();
    const subs = [
      api.onSyncProgress(setProgress),
      api.onSyncFinished((r) => {
        setSyncing(false);
        if (!r.ok) setError(`同期に失敗しました: ${r.error ?? "unknown"}`);
        reload();
      }),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, [reload]);

  const startSync = useCallback(async () => {
    setError(null);
    setSyncing(true);
    setProgress(null);
    try {
      await api.startSync();
    } catch (e) {
      setSyncing(false);
      setError(String(e));
    }
  }, []);

  const actions: Actions = {
    setStatus: async (id, s) => {
      setUser((u) => {
        const status = { ...u.status };
        if (s === "todo") delete status[id];
        else status[id] = s;
        return { ...u, status };
      });
      try {
        await api.setStatus(id, s);
      } catch (e) {
        setError(String(e));
        reloadUser();
      }
    },
    createTag: async (name, color) => {
      await api.createTag(name, color);
      await reloadUser();
    },
    deleteTag: async (id) => {
      await api.deleteTag(id);
      await reloadUser();
    },
    toggleTag: async (taskId, tagId) => {
      await api.toggleTaskTag(taskId, tagId);
      await reloadUser();
    },
    finishRaid: async (map, done) => {
      await api.finishRaid(map, done);
      await reloadUser();
    },
    openMap: (map) => {
      api.openMap(map).catch((e) => setError(`マップを開けませんでした: ${e}`));
    },
  };

  const ctx = useMemo<DataCtx>(() => {
    const nameIcon: Record<string, string> = { ...(data?.item_icons ?? {}) };
    for (const r of [...(data?.ammo ?? []), ...(data?.provisions ?? []), ...(data?.medical ?? [])]) {
      if (r.icon) nameIcon[r.name] = r.icon;
    }
    return { files: data?.files ?? {}, traders: data?.traders ?? [], nameIcon };
  }, [data]);

  // which quests have map points (and on which maps)
  const pointInfo = useMemo(() => {
    const m: Record<string, { count: number; maps: string[] }> = {};
    for (const p of mapData.points) {
      const e = (m[p.task_id] ??= { count: 0, maps: [] });
      e.count++;
      if (!e.maps.includes(p.map)) e.maps.push(p.map);
    }
    return m;
  }, [mapData.points]);
  const showOnMap = useCallback(
    (id: string) => go({ tab: "map", map: pointInfo[id]?.maps[0] ?? START.map, mapMode: "task", mapTask: id }),
    [go, pointInfo],
  );
  const showMap = useCallback((map: string) => go({ tab: "map", map, mapMode: "planned", mapTask: null }), [go]);

  const empty = data !== null && data.tasks.length === 0;
  const openTask = useCallback((id: string) => go({ tab: "tasks", taskId: id }), [go]);
  const openItem = useCallback((tab: ItemsTab, name: string) => go({ tab: "items", itemsTab: tab, itemQuery: name }), [go]);

  return (
    <DataContext.Provider value={ctx}>
      <div className="app">
        <header className="topbar">
          <h1>Tarkov Helper</h1>
          <div className="histbtns">
            <button title="戻る (マウスの戻るボタン / Alt+←)" disabled={nav.i === 0} onClick={back}>◀</button>
            <button title="進む (マウスの進むボタン / Alt+→)" disabled={nav.i >= nav.stack.length - 1} onClick={forward}>▶</button>
          </div>
          <nav>
            {TABS.map(([k, label]) => (
              <button key={k} className={loc.tab === k ? "active" : ""} onClick={() => go({ tab: k })}>
                {label}
              </button>
            ))}
          </nav>
          <button className="searchbtn" onClick={() => setPalette(true)}>🔍 検索 <kbd>Ctrl+K</kbd></button>
          <div className="spacer" />
          {syncing && (
            <span className="sync-status">
              {progress ? `${progress.phase}: ${progress.message}${progress.total ? ` (${progress.done}/${progress.total})` : ""}` : "同期を開始中…"}
            </span>
          )}
          <button className="primary" disabled={syncing} onClick={startSync}>
            {syncing ? "同期中…" : "Wikiから同期"}
          </button>
        </header>

        {error && (
          <div className="banner error" onClick={() => setError(null)}>
            {error}
          </div>
        )}
        {empty && !syncing && (
          <div className="banner info">データがまだありません。「Wikiから同期」を押すと、タスク・弾薬・食料・医薬品とアイコンを取得します(初回は1〜2分かかります)。</div>
        )}

        <main>
          {!data ? (
            <p className="muted pad">読み込み中…</p>
          ) : (
            <>
              <div className="page" hidden={loc.tab !== "tasks"}>
                <Tasks tasks={data.tasks} user={user} actions={actions} selected={loc.taskId} onSelect={openTask} pointInfo={pointInfo} onShowOnMap={showOnMap} />
              </div>
              <div className="page" hidden={loc.tab !== "raid"}>
                <Raid tasks={data.tasks} user={user} actions={actions} onOpen={openTask} pointInfo={pointInfo} onShowMap={showMap} />
              </div>
              {loc.tab === "map" && (
                <div className="page">
                  <MapView
                    tasks={data.tasks}
                    user={user}
                    mapData={mapData}
                    actions={actions}
                    map={loc.map}
                    mode={loc.mapMode}
                    focus={loc.mapTask}
                    onChange={(patch) => go(patch)}
                    onOpenTask={openTask}
                  />
                </div>
              )}
              <div className="page" hidden={loc.tab !== "items"}>
                <Items data={data} tab={loc.itemsTab} onTab={(t) => go({ itemsTab: t })} seed={loc.itemQuery} />
              </div>
              <div className="page" hidden={loc.tab !== "settings"}>
                <Settings data={data} mapData={mapData} issues={issues} syncing={syncing} progress={progress} onSync={startSync} />
              </div>
            </>
          )}
        </main>

        {palette && data && <Palette data={data} onTask={openTask} onItem={openItem} onClose={() => setPalette(false)} />}
      </div>
    </DataContext.Provider>
  );
}
