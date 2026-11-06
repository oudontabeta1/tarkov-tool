//! Fandom (MediaWiki) client and incremental sync into `wiki.db`.
//!
//! Strategy: fetch a handful of index pages, then compare each target page's
//! `lastrevid` (cheap `prop=info`, 50 titles/request) with what we stored and
//! download + parse only pages that changed.
use crate::db;
use crate::parsers::{items, quest};
use crate::positions;
use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

const TASKS_URL: &str = "https://json.tarkov.dev/regular/tasks";
const TASKS_EN_URL: &str = "https://json.tarkov.dev/regular/tasks_en";
const MAPS_CONFIG_URL: &str = "https://raw.githubusercontent.com/the-hideout/tarkov-dev/main/src/data/maps.json";
const API: &str = "https://escapefromtarkov.fandom.com/api.php";
const BATCH: usize = 50;
/// Bump when item-icon extraction improves: the lookup is then redone once.
const ITEM_ICONS_VERSION: &str = "2";
/// Bump when parsers change what they extract: the next sync then re-parses every page.
const PARSER_VERSION: &str = "3";
/// Pause after every request, to be polite to the wiki.
const DELAY: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub phase: String,
    pub done: usize,
    pub total: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Fetched {
    pub input: String,
    pub title: String,
    pub revid: i64,
    pub text: Option<String>,
}

pub struct Client {
    http: reqwest::Client,
    api: String,
}

impl Client {
    pub fn new(contact: &str) -> Result<Self> {
        let ua = format!(
            "tarkov-helper/{} (personal desktop tool; contact: {})",
            env!("CARGO_PKG_VERSION"),
            if contact.trim().is_empty() { "not set" } else { contact.trim() }
        );
        let http = reqwest::Client::builder().user_agent(ua).timeout(Duration::from_secs(60)).build()?;
        Ok(Self { http, api: API.to_string() })
    }

    /// One API call plus every `continue` batch (the wiki cuts big results short), merged.
    async fn query(&self, extra: &[(&str, String)]) -> Result<Value> {
        let mut acc = self.query_once(extra).await?;
        let mut cont = acc.get("continue").cloned();
        for _ in 0..40 {
            let Some(obj) = cont.take().and_then(|c| c.as_object().cloned()) else { break };
            let owned: Vec<(String, String)> = obj
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().map(String::from).unwrap_or_else(|| v.to_string())))
                .collect();
            let mut params: Vec<(&str, String)> = extra.to_vec();
            for (k, v) in &owned {
                params.push((k.as_str(), v.clone()));
            }
            let next = self.query_once(&params).await?;
            merge_pages(&mut acc, &next);
            cont = next.get("continue").cloned();
        }
        Ok(acc)
    }

    async fn query_once(&self, extra: &[(&str, String)]) -> Result<Value> {
        let mut last_err = anyhow!("no attempt");
        for attempt in 1..=4u64 {
            let mut params: Vec<(&str, String)> = vec![
                ("action", "query".into()),
                ("format", "json".into()),
                ("formatversion", "2".into()),
                ("maxlag", "5".into()),
                ("redirects", "1".into()),
            ];
            params.extend(extra.iter().map(|(k, v)| (*k, v.clone())));
            let resp = self.http.get(&self.api).query(&params).send().await;
            let wait = match resp {
                Ok(r) if r.status().is_success() => {
                    let v: Value = r.json().await.context("invalid JSON from wiki")?;
                    match v.pointer("/error/code").and_then(Value::as_str) {
                        None => {
                            tokio::time::sleep(DELAY).await;
                            return Ok(v);
                        }
                        Some("maxlag") => {
                            last_err = anyhow!("wiki is lagging (maxlag)");
                            6 * attempt
                        }
                        Some(code) => return Err(anyhow!("wiki API error: {code}")),
                    }
                }
                Ok(r) => {
                    last_err = anyhow!("HTTP {}", r.status());
                    if !(r.status().as_u16() == 429 || r.status().is_server_error()) {
                        return Err(last_err);
                    }
                    8 * attempt
                }
                Err(e) => {
                    last_err = anyhow!(e);
                    4 * attempt
                }
            };
            tokio::time::sleep(Duration::from_secs(wait)).await;
        }
        Err(last_err)
    }

    /// Latest revision id per input title (0 = page missing).
    pub async fn revids(&self, titles: &[String]) -> Result<Vec<(String, i64)>> {
        let v = self.query(&[("prop", "info".into()), ("titles", titles.join("|"))]).await?;
        Ok(resolve(&v, titles).into_iter().map(|(inp, p)| (inp, p.and_then(|p| p["lastrevid"].as_i64()).unwrap_or(0))).collect())
    }

    /// Thumbnail URLs for wiki files (`""` when the file does not exist).
    pub async fn file_urls(&self, names: &[String]) -> Result<Vec<(String, String)>> {
        let titles: Vec<String> = names.iter().map(|n| format!("File:{n}")).collect();
        let v = self
            .query(&[
                ("prop", "imageinfo".into()),
                ("iiprop", "url".into()),
                ("iiurlwidth", "128".into()),
                ("titles", titles.join("|")),
            ])
            .await?;
        Ok(resolve(&v, &titles)
            .into_iter()
            .zip(names)
            .map(|((_, page), name)| {
                let info = page.map(|p| &p["imageinfo"][0]);
                let url = info
                    .and_then(|i| i["thumburl"].as_str().or_else(|| i["url"].as_str()))
                    .unwrap_or("")
                    .to_string();
                (name.clone(), url)
            })
            .collect())
    }

    /// Plain GET of a (large) JSON file from another site.
    pub async fn get_text(&self, url: &str) -> Result<String> {
        let mut last = anyhow!("no attempt");
        for attempt in 1..=3u64 {
            match self.http.get(url).send().await {
                Ok(r) if r.status().is_success() => {
                    let t = r.text().await?;
                    tokio::time::sleep(DELAY).await;
                    return Ok(t);
                }
                Ok(r) => last = anyhow!("{url}: HTTP {}", r.status()),
                Err(e) => last = anyhow!("{url}: {e}"),
            }
            tokio::time::sleep(Duration::from_secs(4 * attempt)).await;
        }
        Err(last)
    }

    /// Download one icon. Tries the thumbnail URL, then the wiki's `Special:FilePath`
    /// redirect, sending the wiki as Referer like a normal page view would.
    pub async fn download(&self, name: &str, url: &str) -> Result<(String, Vec<u8>)> {
        let fallback = format!("https://escapefromtarkov.fandom.com/wiki/Special:FilePath/{}?width=128", encode_path(name));
        let mut last = anyhow!("no URL");
        for u in [url.to_string(), fallback] {
            let resp = self.http.get(&u).header("Referer", "https://escapefromtarkov.fandom.com/").send().await;
            match resp {
                Ok(r) if r.status().is_success() => {
                    let mime = r
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !mime.starts_with("image/") {
                        last = anyhow!("not an image ({mime})");
                        continue;
                    }
                    let bytes = r.bytes().await?.to_vec();
                    return Ok((mime, bytes));
                }
                Ok(r) => last = anyhow!("HTTP {}", r.status()),
                Err(e) => last = anyhow!(e),
            }
        }
        Err(last)
    }

    pub async fn contents(&self, titles: &[String]) -> Result<Vec<Fetched>> {
        let v = self
            .query(&[
                ("prop", "revisions".into()),
                ("rvprop", "ids|content".into()),
                ("rvslots", "main".into()),
                ("titles", titles.join("|")),
            ])
            .await?;
        Ok(resolve(&v, titles)
            .into_iter()
            .map(|(input, p)| match p {
                Some(p) if !p["missing"].as_bool().unwrap_or(false) => {
                    let rev = &p["revisions"][0];
                    Fetched {
                        input,
                        title: p["title"].as_str().unwrap_or("").to_string(),
                        revid: rev["revid"].as_i64().unwrap_or(0),
                        text: rev.pointer("/slots/main/content").and_then(Value::as_str).map(String::from),
                    }
                }
                _ => Fetched { title: input.clone(), input, revid: 0, text: None },
            })
            .collect())
    }
}

fn merge_pages(acc: &mut Value, new: &Value) {
    let Some(new_pages) = new.pointer("/query/pages").and_then(Value::as_array) else { return };
    let Some(acc_pages) = acc.pointer_mut("/query/pages").and_then(Value::as_array_mut) else { return };
    for p in new_pages {
        let title = p["title"].as_str().unwrap_or("");
        if let Some(existing) = acc_pages.iter_mut().find(|e| e["title"].as_str() == Some(title)) {
            if let (Some(eo), Some(po)) = (existing.as_object_mut(), p.as_object()) {
                for (k, v) in po {
                    eo.insert(k.clone(), v.clone());
                }
            }
        } else {
            acc_pages.push(p.clone());
        }
    }
}

fn encode_path(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Map every requested title to its page object, following `normalized` and `redirects`.
fn resolve<'a>(v: &'a Value, inputs: &[String]) -> Vec<(String, Option<&'a Value>)> {
    let map_of = |key: &str| -> HashMap<&str, &str> {
        v["query"][key]
            .as_array()
            .map(|a| a.iter().filter_map(|e| Some((e["from"].as_str()?, e["to"].as_str()?))).collect())
            .unwrap_or_default()
    };
    let (norm, redir) = (map_of("normalized"), map_of("redirects"));
    let pages: HashMap<&str, &Value> = v["query"]["pages"]
        .as_array()
        .map(|a| a.iter().filter_map(|p| Some((p["title"].as_str()?, p))).collect())
        .unwrap_or_default();
    inputs
        .iter()
        .map(|i| {
            let t = norm.get(i.as_str()).copied().unwrap_or(i.as_str());
            let t = redir.get(t).copied().unwrap_or(t);
            (i.clone(), pages.get(t).copied())
        })
        .collect()
}

fn lock(db: &Mutex<Connection>) -> Result<std::sync::MutexGuard<'_, Connection>> {
    db.lock().map_err(|_| anyhow!("database lock poisoned"))
}

/// Quest positions (tarkov.dev JSON) + map display settings. Needs the quests to be in the DB already.
async fn sync_positions(client: &Client, db: &Mutex<Connection>) -> Result<positions::PointStats> {
    let maps_json = client.get_text(MAPS_CONFIG_URL).await?;
    let tasks_json = client.get_text(TASKS_URL).await?;
    let en_json = client.get_text(TASKS_EN_URL).await.ok();
    let infos = positions::parse_map_configs(&maps_json);
    if infos.is_empty() {
        return Err(anyhow!("map settings could not be read (maps.json changed?)"));
    }
    let known: std::collections::HashSet<String> = {
        let conn = lock(db)?;
        db::all_quests(&conn)?.into_iter().map(|q| q.id).collect()
    };
    let (points, stats) = positions::parse_task_points(&tasks_json, en_json.as_deref(), Some(&known))?;
    let conn = lock(db)?;
    db::replace_map_info(&conn, &infos)?;
    db::replace_task_points(&conn, &points)?;
    db::meta_set(&conn, "points_stats", &serde_json::to_string(&stats)?)?;
    Ok(stats)
}

/// Download only the pages whose revision changed since the last sync.
async fn fetch_changed<P: Fn(Progress)>(
    client: &Client,
    db: &Mutex<Connection>,
    titles: &[String],
    phase: &str,
    progress: &P,
) -> Result<Vec<Fetched>> {
    let stored = db::revids(&*lock(db)?)?;
    let mut changed: Vec<String> = Vec::new();
    for (i, chunk) in titles.chunks(BATCH).enumerate() {
        progress(Progress { phase: phase.into(), done: i * BATCH, total: titles.len(), message: "更新の有無を確認中".into() });
        for (title, rev) in client.revids(chunk).await? {
            if rev == 0 || stored.get(&title) != Some(&rev) {
                changed.push(title);
            }
        }
    }
    let mut out = Vec::new();
    for (i, chunk) in changed.chunks(BATCH).enumerate() {
        progress(Progress {
            phase: phase.into(),
            done: i * BATCH,
            total: changed.len(),
            message: format!("{} 件を取得中", changed.len()),
        });
        out.extend(client.contents(chunk).await?);
    }
    Ok(out)
}

pub async fn run<P: Fn(Progress)>(wiki_db: &Path, contact: &str, progress: P) -> Result<()> {
    let client = Client::new(contact)?;
    let db = Mutex::new(db::open_wiki(wiki_db)?);
    {
        let conn = lock(&db)?;
        if db::meta_get(&conn, "parser_version")?.as_deref() != Some(PARSER_VERSION) {
            db::clear_pages(&conn)?;
        }
    }
    let step = |phase: &str, msg: &str| progress(Progress { phase: phase.into(), done: 0, total: 0, message: msg.into() });

    // 1. index pages (always fetched: 4 small pages)
    step("索引", "索引ページを取得中");
    let idx_titles: Vec<String> = ["Template:Navbox quests", "Ammunition", "Provisions", "Medical"].iter().map(|s| s.to_string()).collect();
    let idx = client.contents(&idx_titles).await?;
    let text_of = |name: &str| -> Result<String> {
        idx.iter().find(|f| f.input == name).and_then(|f| f.text.clone()).ok_or_else(|| anyhow!("index page missing: {name}"))
    };
    let navbox = text_of("Template:Navbox quests")?;
    let ammunition = text_of("Ammunition")?;
    {
        let conn = lock(&db)?;
        for f in &idx {
            db::upsert_page(&conn, &f.input, f.revid, if f.text.is_some() { "ok" } else { "missing" }, None)?;
        }
        let prov = items::parse_provisions(&text_of("Provisions")?);
        let med = items::parse_medical(&text_of("Medical")?);
        db::replace_provisions(&conn, &prov)?;
        db::replace_medical(&conn, &med)?;
    }

    // 2. quests
    let index = quest::parse_quest_index(&navbox);
    if index.is_empty() {
        return Err(anyhow!("quest index is empty (wiki layout changed?)"));
    }
    let titles: Vec<String> = index.iter().map(|(_, t)| t.clone()).collect();
    let fetched = fetch_changed(&client, &db, &titles, "タスク", &progress).await?;
    {
        let conn = lock(&db)?;
        for f in &fetched {
            let pos = index.iter().position(|(_, t)| *t == f.input).unwrap_or(0);
            let Some(text) = &f.text else {
                db::upsert_page(&conn, &f.input, 0, "missing", Some("page not found"))?;
                continue;
            };
            let mut q = quest::parse_quest(&f.input, text);
            if q.trader.is_empty() {
                q.trader = index[pos].0.clone();
            }
            let warn = q.objectives.is_empty().then_some("no objectives found");
            db::save_quest(&conn, &q, pos)?;
            db::upsert_page(&conn, &f.input, f.revid, if warn.is_some() { "warn" } else { "ok" }, warn)?;
        }
        // keep order/trader in sync with the index, drop tasks no longer listed
        let keep: Vec<String> = index.iter().map(|(_, t)| crate::wikitext::slug(t)).collect();
        for (pos, (trader, t)) in index.iter().enumerate() {
            db::set_sort(&conn, &crate::wikitext::slug(t), trader, pos)?;
        }
        db::remove_tasks_except(&conn, &keep)?;
    }

    // 3. ammo: caliber -> weapons from the index page, round stats from each caliber page
    let (mut calibers, weapons) = items::parse_ammunition_index(&ammunition);
    calibers.dedup();
    db::replace_ammo_weapons(&*lock(&db)?, &weapons)?;
    let fetched = fetch_changed(&client, &db, &calibers, "弾薬", &progress).await?;
    {
        let conn = lock(&db)?;
        for f in &fetched {
            let Some(text) = &f.text else {
                db::upsert_page(&conn, &f.input, 0, "missing", Some("page not found"))?;
                continue;
            };
            let rows = items::parse_caliber_page(&f.input, text);
            let warn = rows.is_empty().then_some("no ammo table found");
            db::replace_ammo(&conn, &f.input, &rows)?;
            db::upsert_page(&conn, &f.input, f.revid, if warn.is_some() { "warn" } else { "ok" }, warn)?;
        }
    }

    // 4. trader icons
    step("トレーダー", "トレーダー情報を取得中");
    let mut trader_names: Vec<String> = Vec::new();
    for (t, _) in &index {
        if !trader_names.contains(t) {
            trader_names.push(t.clone());
        }
    }
    let pages = client.contents(&trader_names).await?;
    {
        let conn = lock(&db)?;
        for f in &pages {
            let t = match &f.text {
                Some(text) => quest::parse_trader(&f.input, text),
                None => quest::Trader { name: f.input.clone(), icon: None },
            };
            let warn = t.icon.is_none().then_some("no icon/image in infobox");
            db::save_trader(&conn, &t)?;
            db::upsert_page(&conn, &f.input, f.revid, if warn.is_some() { "warn" } else { "ok" }, warn)?;
        }
    }

    {
        let conn = lock(&db)?;
        if db::meta_get(&conn, "item_icons_version")?.as_deref() != Some(ITEM_ICONS_VERSION) {
            db::clear_item_icons(&conn)?;
        }
    }
    // 4a. quest positions (an extra source: a failure here must not abort the wiki sync)
    step("位置情報", "タスクの座標を取得中");
    let pos_error = match sync_positions(&client, &db).await {
        Ok(_) => String::new(),
        Err(e) => format!("{e:#}"),
    };
    db::meta_set(&*lock(&db)?, "points_last_error", &pos_error)?;

    // 4b. icons for quest reward / required items that are not ammo, food or medicine
    let item_names: Vec<String> = {
        let conn = lock(&db)?;
        let known = db::item_icon_names(&conn)?;
        let mut have: std::collections::HashSet<String> = std::collections::HashSet::new();
        db::all_ammo(&conn)?.into_iter().filter(|a| a.icon.is_some()).for_each(|a| { have.insert(a.name); });
        db::all_provisions(&conn)?.into_iter().filter(|a| a.icon.is_some()).for_each(|a| { have.insert(a.name); });
        db::all_medical(&conn)?.into_iter().filter(|a| a.icon.is_some()).for_each(|a| { have.insert(a.name); });
        let mut names: Vec<String> = Vec::new();
        for q in db::all_quests(&conn)? {
            let used = q.rewards.items.iter().map(|i| i.name.clone()).chain(q.quest_items.iter().map(|i| i.item.clone()));
            for n in used {
                if !known.contains(&n) && !have.contains(&n) && !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        names
    };
    for (i, chunk) in item_names.chunks(BATCH).enumerate() {
        progress(Progress { phase: "アイテム".into(), done: i * BATCH, total: item_names.len(), message: "アイテムのアイコン情報を取得中".into() });
        let pages = client.contents(chunk).await?;
        let conn = lock(&db)?;
        for f in &pages {
            let icon = f.text.as_deref().and_then(|t| crate::wikitext::infobox_icon(t, &[&f.title, &f.input]));
            db::save_item_icon(&conn, &f.input, icon.as_deref())?;
        }
    }

    // 5. resolve icon file names to thumbnail URLs (only names we do not know yet)
    let wanted: Vec<String> = {
        let conn = lock(&db)?;
        let known = db::files_map(&conn)?;
        let mut names: Vec<String> = Vec::new();
        let mut add = |n: &Option<String>| {
            if let Some(n) = n {
                if !known.contains_key(n) && !names.contains(n) {
                    names.push(n.clone());
                }
            }
        };
        db::all_ammo(&conn)?.iter().for_each(|a| add(&a.icon));
        db::all_provisions(&conn)?.iter().for_each(|a| add(&a.icon));
        db::all_medical(&conn)?.iter().for_each(|a| add(&a.icon));
        db::all_traders(&conn)?.iter().for_each(|a| add(&a.icon));
        db::item_icons_map(&conn)?.values().for_each(|n| add(&Some(n.clone())));
        names
    };
    for (i, chunk) in wanted.chunks(BATCH).enumerate() {
        progress(Progress { phase: "アイコン".into(), done: i * BATCH, total: wanted.len(), message: "アイコンのURLを取得中".into() });
        let rows = client.file_urls(chunk).await?;
        db::save_files(&*lock(&db)?, &rows)?;
    }

    // 6. keep a local copy of every icon (works offline, no hotlinking from the app window)
    let todo: Vec<(String, String)> = {
        let conn = lock(&db)?;
        let have = db::image_names(&conn)?;
        let mut v: Vec<(String, String)> = db::files_map(&conn)?
            .into_iter()
            .filter(|(n, u)| !u.is_empty() && !have.contains(n))
            .collect();
        v.sort();
        v
    };
    let (mut failed, mut last_error) = (0usize, String::new());
    for (i, (name, url)) in todo.iter().enumerate() {
        if i % 10 == 0 {
            progress(Progress { phase: "アイコン保存".into(), done: i, total: todo.len(), message: "画像をダウンロード中".into() });
        }
        match client.download(name, url).await {
            Ok((mime, bytes)) => db::save_image(&*lock(&db)?, name, &mime, &bytes)?,
            Err(e) => {
                failed += 1;
                last_error = format!("{name}: {e}");
            }
        }
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    {
        let conn = lock(&db)?;
        db::meta_set(&conn, "icon_last_error", &if failed > 0 { format!("{failed}件失敗 (最後: {last_error})") } else { String::new() })?;
        db::meta_set(&conn, "item_icons_version", ITEM_ICONS_VERSION)?;
        db::meta_set(&conn, "parser_version", PARSER_VERSION)?;
        db::meta_set(&conn, "last_sync", &db::now().to_string())?;
    }
    progress(Progress { phase: "完了".into(), done: 1, total: 1, message: "同期が完了しました".into() });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_pages_combines_continued_results() {
        let mut a: Value = serde_json::json!({"query": {"pages": [
            {"title": "A", "revisions": [{"revid": 1}]},
            {"title": "B"}
        ]}});
        let b: Value = serde_json::json!({"query": {"pages": [
            {"title": "B", "revisions": [{"revid": 2}]},
            {"title": "C", "revisions": [{"revid": 3}]}
        ]}});
        merge_pages(&mut a, &b);
        let pages = a["query"]["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[1]["revisions"][0]["revid"], 2);
        assert_eq!(pages[0]["revisions"][0]["revid"], 1);
    }

    #[test]
    fn resolve_follows_normalization_and_redirects() {
        let v: Value = serde_json::json!({
            "query": {
                "normalized": [{"from": "debut", "to": "Debut"}],
                "redirects": [{"from": "Old name", "to": "New name"}],
                "pages": [
                    {"title": "Debut", "lastrevid": 11},
                    {"title": "New name", "lastrevid": 22},
                    {"title": "Gone", "missing": true}
                ]
            }
        });
        let inputs: Vec<String> = ["debut", "Old name", "Gone", "Nope"].iter().map(|s| s.to_string()).collect();
        let r = resolve(&v, &inputs);
        assert_eq!(r[0].1.unwrap()["lastrevid"], 11);
        assert_eq!(r[1].1.unwrap()["lastrevid"], 22);
        assert!(r[2].1.unwrap()["missing"].as_bool().unwrap());
        assert!(r[3].1.is_none());
    }
}
