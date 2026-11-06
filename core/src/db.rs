//! SQLite storage. `wiki.db` is a rebuildable cache of parsed Wiki data;
//! `user.db` holds progress/tags/settings and must never be wiped by a resync.
use crate::parsers::items::{Ammo, AmmoWeapon, Medical, Provision};
use crate::parsers::quest::{Quest, Trader};
use crate::positions::{MapInfo, TaskPoint};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn open(path: &Path, schema: &str) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(schema)?;
    Ok(conn)
}

const WIKI_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS wiki_pages(title TEXT PRIMARY KEY, revid INTEGER NOT NULL DEFAULT 0,
  fetched_at INTEGER, parse_status TEXT, error_msg TEXT);
CREATE TABLE IF NOT EXISTS tasks(id TEXT PRIMARY KEY, title TEXT NOT NULL, trader TEXT, sort_no INTEGER, data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS ammo(caliber TEXT NOT NULL, name TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(caliber, name));
CREATE TABLE IF NOT EXISTS ammo_weapon(caliber TEXT NOT NULL, weapon TEXT NOT NULL, class TEXT);
CREATE TABLE IF NOT EXISTS provisions(name TEXT PRIMARY KEY, data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS medical(name TEXT PRIMARY KEY, data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE IF NOT EXISTS files(name TEXT PRIMARY KEY, url TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS traders(name TEXT PRIMARY KEY, icon TEXT);
CREATE TABLE IF NOT EXISTS item_icons(name TEXT PRIMARY KEY, icon TEXT);
CREATE TABLE IF NOT EXISTS map_info(name TEXT PRIMARY KEY, data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS task_points(task_id TEXT NOT NULL, map TEXT NOT NULL, data TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS task_points_task ON task_points(task_id);
CREATE TABLE IF NOT EXISTS images(name TEXT PRIMARY KEY, mime TEXT NOT NULL, data BLOB NOT NULL);
";

const USER_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS task_state(task_id TEXT PRIMARY KEY, status TEXT NOT NULL, updated_at INTEGER);
CREATE TABLE IF NOT EXISTS tags(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, color TEXT NOT NULL DEFAULT '#8aa4ff');
CREATE TABLE IF NOT EXISTS task_tags(task_id TEXT NOT NULL, tag_id INTEGER NOT NULL, PRIMARY KEY(task_id, tag_id));
CREATE TABLE IF NOT EXISTS raid_log(id INTEGER PRIMARY KEY AUTOINCREMENT, map TEXT, ended_at INTEGER);
CREATE TABLE IF NOT EXISTS raid_done(raid_id INTEGER NOT NULL, task_id TEXT NOT NULL, PRIMARY KEY(raid_id, task_id));
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT);
";

pub fn open_wiki(path: &Path) -> Result<Connection> {
    open(path, WIKI_SCHEMA)
}
pub fn open_user(path: &Path) -> Result<Connection> {
    open(path, USER_SCHEMA)
}

// ---------- wiki.db ----------

pub fn revids(c: &Connection) -> Result<HashMap<String, i64>> {
    let mut s = c.prepare("SELECT title, revid FROM wiki_pages")?;
    let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn upsert_page(c: &Connection, title: &str, revid: i64, status: &str, err: Option<&str>) -> Result<()> {
    c.execute(
        "INSERT INTO wiki_pages(title, revid, fetched_at, parse_status, error_msg) VALUES(?1,?2,?3,?4,?5)
         ON CONFLICT(title) DO UPDATE SET revid=?2, fetched_at=?3, parse_status=?4, error_msg=?5",
        params![title, revid, now(), status, err],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct PageIssue {
    pub title: String,
    pub status: String,
    pub message: Option<String>,
}

pub fn issues(c: &Connection) -> Result<Vec<PageIssue>> {
    let mut s = c.prepare("SELECT title, parse_status, error_msg FROM wiki_pages WHERE parse_status != 'ok' ORDER BY title")?;
    let rows = s.query_map([], |r| Ok(PageIssue { title: r.get(0)?, status: r.get(1)?, message: r.get(2)? }))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn meta_get(c: &Connection, key: &str) -> Result<Option<String>> {
    Ok(c.query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0)).optional()?)
}
pub fn meta_set(c: &Connection, key: &str, value: &str) -> Result<()> {
    c.execute("INSERT INTO meta(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2", params![key, value])?;
    Ok(())
}

pub fn save_quest(c: &Connection, q: &Quest, sort_no: usize) -> Result<()> {
    c.execute(
        "INSERT INTO tasks(id, title, trader, sort_no, data) VALUES(?1,?2,?3,?4,?5)
         ON CONFLICT(id) DO UPDATE SET title=?2, trader=?3, sort_no=?4, data=?5",
        params![q.id, q.title, q.trader, sort_no as i64, serde_json::to_string(q)?],
    )?;
    Ok(())
}
pub fn set_sort(c: &Connection, id: &str, trader: &str, sort_no: usize) -> Result<()> {
    c.execute("UPDATE tasks SET sort_no=?2, trader=?3 WHERE id=?1", params![id, sort_no as i64, trader])?;
    Ok(())
}
pub fn remove_tasks_except(c: &Connection, keep: &[String]) -> Result<usize> {
    let mut n = 0;
    let ids: Vec<String> = c.prepare("SELECT id FROM tasks")?.query_map([], |r| r.get(0))?.filter_map(|r| r.ok()).collect();
    for id in ids.iter().filter(|i| !keep.contains(i)) {
        n += c.execute("DELETE FROM tasks WHERE id=?1", [id])?;
    }
    Ok(n)
}

fn load<T: serde::de::DeserializeOwned>(c: &Connection, sql: &str) -> Result<Vec<T>> {
    let mut s = c.prepare(sql)?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(|r| r.ok()).filter_map(|j| serde_json::from_str(&j).ok()).collect())
}

pub fn all_quests(c: &Connection) -> Result<Vec<Quest>> {
    load(c, "SELECT data FROM tasks ORDER BY sort_no")
}

pub fn replace_ammo(c: &Connection, caliber: &str, rows: &[Ammo]) -> Result<()> {
    c.execute("DELETE FROM ammo WHERE caliber=?1", [caliber])?;
    for a in rows {
        c.execute(
            "INSERT OR REPLACE INTO ammo(caliber, name, data) VALUES(?1,?2,?3)",
            params![caliber, a.name, serde_json::to_string(a)?],
        )?;
    }
    Ok(())
}
pub fn all_ammo(c: &Connection) -> Result<Vec<Ammo>> {
    load(c, "SELECT data FROM ammo ORDER BY caliber, rowid")
}

pub fn replace_ammo_weapons(c: &Connection, rows: &[AmmoWeapon]) -> Result<()> {
    c.execute("DELETE FROM ammo_weapon", [])?;
    for w in rows {
        c.execute("INSERT INTO ammo_weapon(caliber, weapon, class) VALUES(?1,?2,?3)", params![w.caliber, w.weapon, w.class])?;
    }
    Ok(())
}
pub fn all_ammo_weapons(c: &Connection) -> Result<Vec<AmmoWeapon>> {
    let mut s = c.prepare("SELECT caliber, weapon, class FROM ammo_weapon ORDER BY rowid")?;
    let rows = s.query_map([], |r| Ok(AmmoWeapon { caliber: r.get(0)?, weapon: r.get(1)?, class: r.get(2)? }))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn replace_provisions(c: &Connection, rows: &[Provision]) -> Result<()> {
    c.execute("DELETE FROM provisions", [])?;
    for p in rows {
        c.execute("INSERT OR REPLACE INTO provisions(name, data) VALUES(?1,?2)", params![p.name, serde_json::to_string(p)?])?;
    }
    Ok(())
}
pub fn all_provisions(c: &Connection) -> Result<Vec<Provision>> {
    load(c, "SELECT data FROM provisions ORDER BY rowid")
}

pub fn replace_medical(c: &Connection, rows: &[Medical]) -> Result<()> {
    c.execute("DELETE FROM medical", [])?;
    for m in rows {
        c.execute("INSERT OR REPLACE INTO medical(name, data) VALUES(?1,?2)", params![m.name, serde_json::to_string(m)?])?;
    }
    Ok(())
}
pub fn all_medical(c: &Connection) -> Result<Vec<Medical>> {
    load(c, "SELECT data FROM medical ORDER BY rowid")
}

/// Forget every stored revision so the next sync re-downloads and re-parses all pages.
pub fn clear_pages(c: &Connection) -> Result<()> {
    c.execute("DELETE FROM wiki_pages", [])?;
    c.execute("DELETE FROM files", [])?;
    c.execute("DELETE FROM item_icons", [])?;
    Ok(())
}

/// item page title -> icon file (None = the page has no icon)
pub fn item_icon_names(c: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut s = c.prepare("SELECT name FROM item_icons")?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
pub fn replace_map_info(c: &Connection, rows: &[MapInfo]) -> Result<()> {
    c.execute("DELETE FROM map_info", [])?;
    for m in rows {
        c.execute("INSERT INTO map_info(name, data) VALUES(?1,?2)", params![m.name, serde_json::to_string(m)?])?;
    }
    Ok(())
}
pub fn all_map_info(c: &Connection) -> Result<Vec<MapInfo>> {
    load(c, "SELECT data FROM map_info ORDER BY name")
}

pub fn replace_task_points(c: &Connection, rows: &[TaskPoint]) -> Result<()> {
    c.execute("DELETE FROM task_points", [])?;
    let mut st = c.prepare("INSERT INTO task_points(task_id, map, data) VALUES(?1,?2,?3)")?;
    for p in rows {
        st.execute(params![p.task_id, p.map, serde_json::to_string(p)?])?;
    }
    Ok(())
}
pub fn all_task_points(c: &Connection) -> Result<Vec<TaskPoint>> {
    load(c, "SELECT data FROM task_points ORDER BY rowid")
}

pub fn clear_item_icons(c: &Connection) -> Result<()> {
    c.execute("DELETE FROM item_icons", [])?;
    Ok(())
}
pub fn save_item_icon(c: &Connection, name: &str, icon: Option<&str>) -> Result<()> {
    c.execute(
        "INSERT INTO item_icons(name, icon) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET icon=?2",
        params![name, icon],
    )?;
    Ok(())
}
pub fn item_icons_map(c: &Connection) -> Result<HashMap<String, String>> {
    let mut s = c.prepare("SELECT name, icon FROM item_icons WHERE icon IS NOT NULL")?;
    let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// file name -> thumbnail URL ("" = the wiki has no such file)
pub fn files_map(c: &Connection) -> Result<HashMap<String, String>> {
    let mut s = c.prepare("SELECT name, url FROM files")?;
    let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
pub fn save_files(c: &Connection, rows: &[(String, String)]) -> Result<()> {
    for (n, u) in rows {
        c.execute("INSERT INTO files(name, url) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET url=?2", params![n, u])?;
    }
    Ok(())
}

pub fn image_names(c: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut s = c.prepare("SELECT name FROM images")?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
pub fn save_image(c: &Connection, name: &str, mime: &str, data: &[u8]) -> Result<()> {
    c.execute(
        "INSERT INTO images(name, mime, data) VALUES(?1,?2,?3) ON CONFLICT(name) DO UPDATE SET mime=?2, data=?3",
        params![name, mime, data],
    )?;
    Ok(())
}
pub fn get_image(c: &Connection, name: &str) -> Result<Option<(String, Vec<u8>)>> {
    Ok(c.query_row("SELECT mime, data FROM images WHERE name=?1", [name], |r| Ok((r.get(0)?, r.get(1)?))).optional()?)
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct IconStats {
    /// files the wiki returned a URL for
    pub resolved: i64,
    /// files the wiki has no URL for
    pub missing: i64,
    /// images downloaded into the local cache
    pub stored: i64,
    pub last_error: Option<String>,
    pub missing_samples: Vec<String>,
    /// quest reward / required items whose page gave an icon
    pub items_with_icon: i64,
    /// ...and those without one
    pub items_without_icon: i64,
    pub item_missing_samples: Vec<String>,
}

pub fn icon_stats(c: &Connection) -> Result<IconStats> {
    let count = |sql: &str| -> Result<i64> { Ok(c.query_row(sql, [], |r| r.get(0))?) };
    let samples: Vec<String> = c
        .prepare("SELECT name FROM files WHERE url='' LIMIT 8")?
        .query_map([], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(IconStats {
        resolved: count("SELECT COUNT(*) FROM files WHERE url!=''")?,
        missing: count("SELECT COUNT(*) FROM files WHERE url=''")?,
        stored: count("SELECT COUNT(*) FROM images")?,
        last_error: meta_get(c, "icon_last_error")?.filter(|e| !e.is_empty()),
        missing_samples: samples,
        items_with_icon: count("SELECT COUNT(*) FROM item_icons WHERE icon IS NOT NULL")?,
        items_without_icon: count("SELECT COUNT(*) FROM item_icons WHERE icon IS NULL")?,
        item_missing_samples: c
            .prepare("SELECT name FROM item_icons WHERE icon IS NULL ORDER BY name LIMIT 12")?
            .query_map([], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect(),
    })
}

pub fn save_trader(c: &Connection, t: &Trader) -> Result<()> {
    c.execute(
        "INSERT INTO traders(name, icon) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET icon=?2",
        params![t.name, t.icon],
    )?;
    Ok(())
}
pub fn all_traders(c: &Connection) -> Result<Vec<Trader>> {
    let mut s = c.prepare("SELECT name, icon FROM traders ORDER BY rowid")?;
    let rows = s.query_map([], |r| Ok(Trader { name: r.get(0)?, icon: r.get(1)? }))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ---------- user.db ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UserState {
    /// task id -> "planned" | "done"  (absent = todo)
    pub status: HashMap<String, String>,
    pub tags: Vec<Tag>,
    pub task_tags: HashMap<String, Vec<i64>>,
}

pub fn user_state(c: &Connection) -> Result<UserState> {
    let mut st = UserState::default();
    for r in c.prepare("SELECT task_id, status FROM task_state")?.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (id, s) = r?;
        st.status.insert(id, s);
    }
    for r in c.prepare("SELECT id, name, color FROM tags ORDER BY name")?.query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))? {
        st.tags.push(r?);
    }
    for r in c.prepare("SELECT task_id, tag_id FROM task_tags")?.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (t, g) = r?;
        st.task_tags.entry(t).or_default().push(g);
    }
    Ok(st)
}

pub fn set_status(c: &Connection, task_id: &str, status: &str) -> Result<()> {
    if status == "todo" {
        c.execute("DELETE FROM task_state WHERE task_id=?1", [task_id])?;
    } else {
        c.execute(
            "INSERT INTO task_state(task_id, status, updated_at) VALUES(?1,?2,?3)
             ON CONFLICT(task_id) DO UPDATE SET status=?2, updated_at=?3",
            params![task_id, status, now()],
        )?;
    }
    Ok(())
}

pub fn create_tag(c: &Connection, name: &str, color: &str) -> Result<()> {
    c.execute("INSERT OR IGNORE INTO tags(name, color) VALUES(?1,?2)", params![name.trim(), color])?;
    Ok(())
}
pub fn delete_tag(c: &Connection, id: i64) -> Result<()> {
    c.execute("DELETE FROM task_tags WHERE tag_id=?1", [id])?;
    c.execute("DELETE FROM tags WHERE id=?1", [id])?;
    Ok(())
}
pub fn toggle_task_tag(c: &Connection, task_id: &str, tag_id: i64) -> Result<()> {
    let n = c.execute("DELETE FROM task_tags WHERE task_id=?1 AND tag_id=?2", params![task_id, tag_id])?;
    if n == 0 {
        c.execute("INSERT INTO task_tags(task_id, tag_id) VALUES(?1,?2)", params![task_id, tag_id])?;
    }
    Ok(())
}

/// Record a raid and mark the given tasks as done.
pub fn finish_raid(c: &Connection, map: &str, done: &[String]) -> Result<()> {
    c.execute("INSERT INTO raid_log(map, ended_at) VALUES(?1,?2)", params![map, now()])?;
    let raid = c.last_insert_rowid();
    for t in done {
        c.execute("INSERT OR IGNORE INTO raid_done(raid_id, task_id) VALUES(?1,?2)", params![raid, t])?;
        set_status(c, t, "done")?;
    }
    Ok(())
}

pub fn setting_get(c: &Connection, key: &str) -> Result<Option<String>> {
    Ok(c.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| r.get(0)).optional()?)
}
pub fn setting_set(c: &Connection, key: &str, value: &str) -> Result<()> {
    c.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2", params![key, value])?;
    Ok(())
}
