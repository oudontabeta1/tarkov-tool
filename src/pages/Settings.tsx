import { useEffect, useState } from "react";
import { api } from "../api";
import type { IconStats, MapData, PageIssue, SyncProgress, WikiData } from "../types";
import { dateLabel } from "../util";

export default function Settings({
  data, mapData, issues, syncing, progress, onSync,
}: {
  data: WikiData;
  mapData: MapData;
  issues: PageIssue[];
  syncing: boolean;
  progress: SyncProgress | null;
  onSync: () => void;
}) {
  const [contact, setContact] = useState("");
  const [saved, setSaved] = useState(false);
  const [icons, setIcons] = useState<IconStats | null>(null);

  useEffect(() => {
    api.contact().then(setContact).catch(() => {});
  }, []);
  useEffect(() => {
    if (!syncing) api.iconStats().then(setIcons).catch(() => {});
  }, [syncing, data.last_sync]);

  const save = async () => {
    await api.setContact(contact);
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };
  const pct = progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : null;

  return (
    <div className="settings pad">
      <h3>Wikiとの同期</h3>
      <p>
        最終同期: {dateLabel(data.last_sync)} / タスク {data.tasks.length}件・弾薬 {data.ammo.length}件・食料 {data.provisions.length}件・医薬品 {data.medical.length}件
      </p>
      <p className="muted small">
        2回目以降は、更新されたページだけを取得します。Wikiへの負荷を抑えるため、リクエストは1秒間隔で送ります。
      </p>
      <button className="primary" disabled={syncing} onClick={onSync}>{syncing ? "同期中…" : "今すぐ同期"}</button>
      {syncing && progress && (
        <div className="progress">
          <div className="bar"><div style={{ width: `${pct ?? 5}%` }} /></div>
          <span className="muted small">{progress.phase}: {progress.message}</span>
        </div>
      )}

      <h3>マップの座標データ(tarkov.dev)</h3>
      {mapData.stats ? (
        <>
          <p>
            座標を持つタスク {mapData.stats.tasks_with_points}件 / 地点 {mapData.stats.points}件 / 取得できたマップ設定 {mapData.maps.length}件
            (APIのタスク {mapData.stats.tasks_in_api}件)
          </p>
          {mapData.stats.unmatched.length > 0 && (
            <p className="muted small">Wikiのタスクと一致しなかった例: {mapData.stats.unmatched.join("、")}</p>
          )}
        </>
      ) : (
        <p className="muted small">まだ取得していません。「Wikiから同期」を実行してください。</p>
      )}
      {mapData.last_error && <p className="bad small">前回の取得エラー: {mapData.last_error}</p>}

      <h3>アイコンの状態</h3>
      {icons ? (
        <>
          <p>
            Wikiで見つかった画像 {icons.resolved}件 / 見つからなかった画像 {icons.missing}件 / このPCに保存済み {icons.stored}件
          </p>
          <p>
            タスクの報酬・必要アイテム: アイコンあり {icons.items_with_icon}件 / アイコンなし {icons.items_without_icon}件
          </p>
          {icons.item_missing_samples.length > 0 && (
            <p className="muted small">アイコンが見つからなかったアイテムの例: {icons.item_missing_samples.join("、")}</p>
          )}
          {icons.last_error && <p className="bad small">保存に失敗した画像があります: {icons.last_error}</p>}
          {icons.missing_samples.length > 0 && <p className="muted small">見つからなかった例: {icons.missing_samples.join("、")}</p>}
          {icons.resolved === 0 && data.last_sync && <p className="bad small">アイコンのURLを1件も取得できていません。この画面のスクリーンショットを共有してください。</p>}
        </>
      ) : (
        <p className="muted small">確認中…</p>
      )}

      <h3>連絡先(任意)</h3>
      <p className="muted small">
        Wikiへのアクセス時に User-Agent へ含める、メールアドレスやGitHubのURLなどです。サーバー管理者が問題に気づいた際の連絡用で、他には使いません。
      </p>
      <div className="row">
        <input value={contact} placeholder="you@example.com" onChange={(e) => setContact(e.target.value)} />
        <button onClick={save}>{saved ? "保存しました" : "保存"}</button>
      </div>

      <h3>取得に問題があったページ({issues.length})</h3>
      {issues.length === 0 ? (
        <p className="muted small">問題はありません。</p>
      ) : (
        <table className="grid small">
          <thead><tr><th>ページ</th><th>状態</th><th>内容</th></tr></thead>
          <tbody>
            {issues.map((i) => (
              <tr key={i.title}><td>{i.title}</td><td>{i.status}</td><td>{i.message ?? ""}</td></tr>
            ))}
          </tbody>
        </table>
      )}

      <h3>ライセンス・出典</h3>
      <p className="small">
        タスク・アイテムのデータは「Escape from Tarkov Wiki」(escapefromtarkov.fandom.com)の内容を元にしており、
        Creative Commons Attribution-ShareAlike (CC BY-SA) のもとで提供されています。非公式のファンツールで、Battlestate Games とは関係ありません。
      </p>
    </div>
  );
}
