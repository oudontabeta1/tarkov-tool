#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tarkov_core::db::{self, IconStats, PageIssue, UserState};
use tarkov_core::parsers::items::{Ammo, AmmoWeapon, Medical, Provision};
use std::collections::HashMap;
use tarkov_core::parsers::quest::{Quest, Trader};
use tarkov_core::positions::{MapInfo, TaskPoint};
use tarkov_core::sync;
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState {
    dir: PathBuf,
    syncing: AtomicBool,
}

impl AppState {
    fn wiki(&self) -> PathBuf {
        self.dir.join("wiki.db")
    }
    fn user(&self) -> PathBuf {
        self.dir.join("user.db")
    }
}

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    format!("{e:#}")
}

#[derive(Serialize)]
struct WikiData {
    tasks: Vec<Quest>,
    ammo: Vec<Ammo>,
    ammo_weapons: Vec<AmmoWeapon>,
    provisions: Vec<Provision>,
    medical: Vec<Medical>,
    traders: Vec<Trader>,
    /// item page title -> icon file (reward / required items of quests)
    item_icons: HashMap<String, String>,
    /// icon file name -> thumbnail URL ("" when the wiki has no such file)
    files: HashMap<String, String>,
    /// unix seconds of the last completed sync
    last_sync: Option<i64>,
}

#[tauri::command]
async fn get_wiki_data(state: State<'_, AppState>) -> CmdResult<WikiData> {
    let c = db::open_wiki(&state.wiki()).map_err(err)?;
    Ok(WikiData {
        tasks: db::all_quests(&c).map_err(err)?,
        ammo: db::all_ammo(&c).map_err(err)?,
        ammo_weapons: db::all_ammo_weapons(&c).map_err(err)?,
        provisions: db::all_provisions(&c).map_err(err)?,
        medical: db::all_medical(&c).map_err(err)?,
        traders: db::all_traders(&c).map_err(err)?,
        item_icons: db::item_icons_map(&c).map_err(err)?,
        files: db::files_map(&c).map_err(err)?,
        last_sync: db::meta_get(&c, "last_sync").map_err(err)?.and_then(|s| s.parse().ok()),
    })
}

#[tauri::command]
async fn get_issues(state: State<'_, AppState>) -> CmdResult<Vec<PageIssue>> {
    db::issues(&db::open_wiki(&state.wiki()).map_err(err)?).map_err(err)
}

#[derive(Serialize)]
struct MapData {
    maps: Vec<MapInfo>,
    points: Vec<TaskPoint>,
    /// last sync's summary (JSON) and error text, if any
    stats: Option<serde_json::Value>,
    last_error: Option<String>,
}

#[tauri::command]
async fn get_map_data(state: State<'_, AppState>) -> CmdResult<MapData> {
    let c = db::open_wiki(&state.wiki()).map_err(err)?;
    Ok(MapData {
        maps: db::all_map_info(&c).map_err(err)?,
        points: db::all_task_points(&c).map_err(err)?,
        stats: db::meta_get(&c, "points_stats").map_err(err)?.and_then(|s| serde_json::from_str(&s).ok()),
        last_error: db::meta_get(&c, "points_last_error").map_err(err)?.filter(|e| !e.is_empty()),
    })
}

#[tauri::command]
async fn get_icon_stats(state: State<'_, AppState>) -> CmdResult<IconStats> {
    db::icon_stats(&db::open_wiki(&state.wiki()).map_err(err)?).map_err(err)
}

#[tauri::command]
async fn get_user_state(state: State<'_, AppState>) -> CmdResult<UserState> {
    db::user_state(&db::open_user(&state.user()).map_err(err)?).map_err(err)
}

#[tauri::command]
async fn set_task_status(state: State<'_, AppState>, task_id: String, status: String) -> CmdResult<()> {
    if !["todo", "planned", "done"].contains(&status.as_str()) {
        return Err("invalid status".into());
    }
    db::set_status(&db::open_user(&state.user()).map_err(err)?, &task_id, &status).map_err(err)
}

#[tauri::command]
async fn create_tag(state: State<'_, AppState>, name: String, color: String) -> CmdResult<()> {
    if name.trim().is_empty() {
        return Ok(());
    }
    db::create_tag(&db::open_user(&state.user()).map_err(err)?, &name, &color).map_err(err)
}

#[tauri::command]
async fn delete_tag(state: State<'_, AppState>, id: i64) -> CmdResult<()> {
    db::delete_tag(&db::open_user(&state.user()).map_err(err)?, id).map_err(err)
}

#[tauri::command]
async fn toggle_task_tag(state: State<'_, AppState>, task_id: String, tag_id: i64) -> CmdResult<()> {
    db::toggle_task_tag(&db::open_user(&state.user()).map_err(err)?, &task_id, tag_id).map_err(err)
}

#[tauri::command]
async fn finish_raid(state: State<'_, AppState>, map: String, done: Vec<String>) -> CmdResult<()> {
    db::finish_raid(&db::open_user(&state.user()).map_err(err)?, &map, &done).map_err(err)
}

#[tauri::command]
async fn get_contact(state: State<'_, AppState>) -> CmdResult<String> {
    let c = db::open_user(&state.user()).map_err(err)?;
    Ok(db::setting_get(&c, "contact").map_err(err)?.unwrap_or_default())
}

#[tauri::command]
async fn set_contact(state: State<'_, AppState>, contact: String) -> CmdResult<()> {
    db::setting_set(&db::open_user(&state.user()).map_err(err)?, "contact", contact.trim()).map_err(err)
}

/// Open the wiki's interactive map for `map` in a separate window (reused if already open).
#[tauri::command]
async fn open_map(app: AppHandle, map: String) -> CmdResult<()> {
    let page = format!("https://escapefromtarkov.fandom.com/wiki/Map:{}", map.trim().replace(' ', "_"));
    let url: tauri::Url = page.parse().map_err(err)?;
    let title = format!("Map: {}", map.trim());
    if let Some(w) = app.get_webview_window("map") {
        let target = serde_json::to_string(&page).map_err(err)?;
        w.eval(&format!("window.location.href = {target}")).map_err(err)?;
        let _ = w.set_title(&title);
        let _ = w.show();
        let _ = w.set_focus();
    } else {
        tauri::WebviewWindowBuilder::new(&app, "map", tauri::WebviewUrl::External(url))
            .title(title)
            .inner_size(1200.0, 860.0)
            .build()
            .map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
async fn start_sync(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    if state.syncing.swap(true, Ordering::SeqCst) {
        return Err("同期はすでに実行中です".into());
    }
    let contact = {
        let c = db::open_user(&state.user()).map_err(err)?;
        db::setting_get(&c, "contact").map_err(err)?.unwrap_or_default()
    };
    let wiki = state.wiki();
    tauri::async_runtime::spawn(async move {
        let emitter = app.clone();
        let res = sync::run(&wiki, &contact, move |p| {
            let _ = emitter.emit("sync-progress", p);
        })
        .await;
        app.state::<AppState>().syncing.store(false, Ordering::SeqCst);
        let payload = match res {
            Ok(()) => serde_json::json!({ "ok": true }),
            Err(e) => serde_json::json!({ "ok": false, "error": format!("{e:#}") }),
        };
        let _ = app.emit("sync-finished", payload);
    });
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // icon://localhost/<file name>  ->  image bytes cached in wiki.db
        .register_uri_scheme_protocol("icon", |ctx, request| {
            let name = percent_encoding::percent_decode_str(request.uri().path().trim_start_matches('/'))
                .decode_utf8_lossy()
                .to_string();
            let found = ctx.app_handle().try_state::<AppState>().and_then(|st| {
                let c = db::open_wiki(&st.wiki()).ok()?;
                db::get_image(&c, &name).ok().flatten()
            });
            match found {
                Some((mime, data)) => tauri::http::Response::builder()
                    .header("Content-Type", mime)
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Cache-Control", "max-age=3600")
                    .body(data)
                    .unwrap(),
                None => tauri::http::Response::builder().status(404).body(Vec::new()).unwrap(),
            }
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(AppState { dir, syncing: AtomicBool::new(false) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_wiki_data,
            get_issues,
            get_icon_stats,
            get_map_data,
            get_user_state,
            set_task_status,
            create_tag,
            delete_tag,
            toggle_task_tag,
            finish_raid,
            get_contact,
            set_contact,
            open_map,
            start_sync
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tarkov Helper");
}
