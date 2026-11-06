//! Quest positions from `json.tarkov.dev` and map display settings from tarkov-dev's `maps.json`.
//!
//! The wiki has no per-quest coordinates, so positions come from tarkov.dev's data; quests are
//! joined to the wiki's by the quest's `wikiLink`.
use crate::wikitext::slug;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// (game map id, wiki map name, tarkov-dev slug). Event / tutorial maps are left out.
pub const GAME_MAPS: [(&str, &str, &str); 16] = [
    ("55f2d3fd4bdc2d5f408b4567", "Factory", "factory"),
    ("59fc81d786f774390775787e", "Factory", "factory"),
    ("56f40101d2720b2a4d8b45d6", "Customs", "customs"),
    ("5704e3c2d2720bac5b8b4567", "Woods", "woods"),
    ("5704e4dad2720bb55b8b4567", "Lighthouse", "lighthouse"),
    ("5704e554d2720bac5b8b456e", "Shoreline", "shoreline"),
    ("5704e5fad2720bc05b8b4567", "Reserve", "reserve"),
    ("5714dbc024597771384a510d", "Interchange", "interchange"),
    ("5714dc692459777137212e12", "Streets of Tarkov", "streets-of-tarkov"),
    ("5b0fc42d86f7744a585f9105", "The Lab", "the-lab"),
    ("6a294a5b5eb5f9a1700417b7", "The Lab", "the-lab"),
    ("653e6760052c01c1c805532f", "Ground Zero", "ground-zero"),
    ("65b8d6f5cdde2479cb2a3125", "Ground Zero", "ground-zero"),
    ("65cc8f81a9aac3e77d0cfd3e", "Terminal", "terminal"),
    ("6733700029c367a3d40b02af", "The Labyrinth", "the-labyrinth"),
    ("69af492a4819ea4ba10a69c5", "Icebreaker", "icebreaker"),
];

/// How to draw one map (values copied from tarkov-dev's `maps.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapInfo {
    /// wiki map name ("Customs")
    pub name: String,
    pub slug: String,
    pub svg: Option<String>,
    pub tiles: Option<String>,
    /// [[x, z], [x, z]] corners in game coordinates
    pub bounds: Option<[[f64; 2]; 2]>,
    pub svg_bounds: Option<[[f64; 2]; 2]>,
    /// [scaleX, marginX, scaleY, marginY]
    pub transform: Option<[f64; 4]>,
    pub rotation: f64,
    pub min_zoom: f64,
    pub max_zoom: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskPoint {
    pub task_id: String,
    /// wiki map name
    pub map: String,
    /// objective type: visit / mark / plantItem / findItem / findQuestItem / ...
    pub kind: String,
    pub objective_id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// zone outline as [x, z] pairs (empty for single positions)
    #[serde(default)]
    pub outline: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PointStats {
    pub tasks_in_api: usize,
    pub tasks_with_points: usize,
    pub points: usize,
    /// quests that have positions but no matching wiki quest (first few)
    pub unmatched: Vec<String>,
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `https://.../wiki/What%E2%80%99s_on_the_Flash_Drive%3F` -> task id (`what-s-on-the-flash-drive`)
pub fn task_id_from_link(link: &str) -> Option<String> {
    let title = link.split("/wiki/").nth(1)?.split(['#', '?']).next()?;
    let id = slug(&pct_decode(title).replace('_', " "));
    (!id.is_empty()).then_some(id)
}

fn pair(v: &Value) -> Option<[f64; 2]> {
    Some([v.get(0)?.as_f64()?, v.get(1)?.as_f64()?])
}
fn bounds_of(v: Option<&Value>) -> Option<[[f64; 2]; 2]> {
    let v = v?;
    Some([pair(v.get(0)?)?, pair(v.get(1)?)?])
}

/// Parse tarkov-dev's `src/data/maps.json` into one [`MapInfo`] per wiki map.
pub fn parse_map_configs(json: &str) -> Vec<MapInfo> {
    let Ok(v) = serde_json::from_str::<Value>(json) else { return vec![] };
    let Some(list) = v.as_array() else { return vec![] };
    let mut out: Vec<MapInfo> = Vec::new();
    for (_, wiki, slug_) in GAME_MAPS {
        if out.iter().any(|m| m.name == wiki) {
            continue;
        }
        let Some(entry) = list.iter().find(|m| m["normalizedName"].as_str() == Some(slug_)) else { continue };
        let Some(view) = entry["maps"].as_array().and_then(|a| a.iter().find(|x| x["projection"].as_str() == Some("interactive"))) else {
            continue;
        };
        out.push(MapInfo {
            name: wiki.to_string(),
            slug: slug_.to_string(),
            svg: view["svgPath"].as_str().map(String::from),
            tiles: view["tilePath"].as_str().map(String::from),
            bounds: bounds_of(view.get("bounds")),
            svg_bounds: bounds_of(view.get("svgBounds")),
            transform: view.get("transform").and_then(|t| {
                let a = t.as_array()?;
                Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2)?.as_f64()?, a.get(3)?.as_f64()?])
            }),
            rotation: view["coordinateRotation"].as_f64().unwrap_or(0.0),
            min_zoom: view["minZoom"].as_f64().unwrap_or(1.0),
            max_zoom: view["maxZoom"].as_f64().unwrap_or(6.0),
        });
    }
    out
}

fn map_name(id: &str) -> Option<&'static str> {
    GAME_MAPS.iter().find(|(i, _, _)| *i == id).map(|(_, n, _)| *n)
}

fn vec3(v: &Value) -> Option<(f64, f64, f64)> {
    Some((v["x"].as_f64()?, v["y"].as_f64().unwrap_or(0.0), v["z"].as_f64()?))
}

/// Every position of every quest objective, joined to wiki quest ids.
/// `known`: wiki quest ids; positions of other quests are dropped (and counted in `unmatched`).
pub fn parse_task_points(
    tasks_json: &str,
    en_json: Option<&str>,
    known: Option<&HashSet<String>>,
) -> Result<(Vec<TaskPoint>, PointStats)> {
    let v: Value = serde_json::from_str(tasks_json)?;
    let tasks = v.pointer("/data/tasks").and_then(Value::as_object).ok_or_else(|| anyhow!("unexpected tasks JSON (no data.tasks)"))?;
    let en: Option<Value> = en_json.and_then(|j| serde_json::from_str(j).ok());
    let text = |key: &str| -> String {
        en.as_ref().and_then(|e| e.pointer("/data")?.get(key)?.as_str()).unwrap_or("").to_string()
    };

    let mut points: Vec<TaskPoint> = Vec::new();
    let mut seen: HashSet<(String, String, String, String, i64, i64)> = HashSet::new();
    let mut stats = PointStats { tasks_in_api: tasks.len(), ..Default::default() };
    let mut with_points: HashSet<String> = HashSet::new();

    for t in tasks.values() {
        let Some(id) = t["wikiLink"].as_str().and_then(task_id_from_link) else { continue };
        let mut mine: Vec<TaskPoint> = Vec::new();
        for o in t["objectives"].as_array().into_iter().flatten() {
            let kind = o["type"].as_str().unwrap_or("").to_string();
            let oid = o["id"].as_str().unwrap_or("").to_string();
            let desc = text(o["description"].as_str().unwrap_or(""));

            for z in o["zones"].as_array().into_iter().flatten() {
                let Some(map) = z["map"].as_str().and_then(map_name) else { continue };
                let Some((x, y, zz)) = vec3(&z["position"]) else { continue };
                let zid = z["id"].as_str().unwrap_or("").to_string();
                let key = (id.clone(), oid.clone(), zid.clone(), map.to_string(), (x * 10.0) as i64, (zz * 10.0) as i64);
                if !seen.insert(key) {
                    continue; // same zone listed for several variants of one map (e.g. Ground Zero 21+)
                }
                let outline = z["outline"].as_array().into_iter().flatten().filter_map(|p| Some([p["x"].as_f64()?, p["z"].as_f64()?])).collect();
                mine.push(TaskPoint {
                    task_id: id.clone(),
                    map: map.to_string(),
                    kind: kind.clone(),
                    objective_id: oid.clone(),
                    label: if desc.is_empty() { zid } else { desc.clone() },
                    x,
                    y,
                    z: zz,
                    outline,
                });
            }
            for loc in o["possibleLocations"].as_array().into_iter().flatten() {
                let Some(map) = loc["map"].as_str().and_then(map_name) else { continue };
                for p in loc["positions"].as_array().into_iter().flatten() {
                    let Some((x, y, zz)) = vec3(p) else { continue };
                    let key = (id.clone(), oid.clone(), String::new(), map.to_string(), (x * 10.0) as i64, (zz * 10.0) as i64);
                    if !seen.insert(key) {
                        continue;
                    }
                    mine.push(TaskPoint {
                        task_id: id.clone(),
                        map: map.to_string(),
                        kind: kind.clone(),
                        objective_id: oid.clone(),
                        label: desc.clone(),
                        x,
                        y,
                        z: zz,
                        outline: vec![],
                    });
                }
            }
        }
        if mine.is_empty() {
            continue;
        }
        if known.map_or(true, |k| k.contains(&id)) {
            with_points.insert(id);
            points.extend(mine);
        } else if stats.unmatched.len() < 15 {
            stats.unmatched.push(id);
        }
    }
    stats.tasks_with_points = with_points.len();
    stats.points = points.len();
    Ok((points, stats))
}
