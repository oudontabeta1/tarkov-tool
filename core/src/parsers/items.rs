//! Ammunition, provisions and medical pages.
use crate::wikitext::*;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! re {
    ($p:expr) => {{
        static R: Lazy<Regex> = Lazy::new(|| Regex::new($p).unwrap());
        &*R
    }};
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ammo {
    pub caliber: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub tracer: bool,
    pub subsonic: bool,
    pub dmg: Option<f64>,
    pub pen: Option<f64>,
    pub armor_dmg: Option<f64>,
    pub accuracy: Option<f64>,
    pub recoil: Option<f64>,
    pub light_bleed: Option<f64>,
    pub heavy_bleed: Option<f64>,
    pub dur_burn: Option<f64>,
    pub heat: Option<f64>,
    pub speed: Option<f64>,
    pub projectiles: Option<f64>,
    pub source: String,
    /// columns that are not mapped above (header -> text)
    pub extra: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoWeapon {
    pub caliber: String,
    pub weapon: String,
    pub class: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Provision {
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub energy: Option<f64>,
    pub hydration: Option<f64>,
    pub buffs: String,
    pub debuffs: String,
    pub use_time: Option<f64>,
    pub barter: bool,
    pub craft: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Medical {
    pub group: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub kind: String,
    pub buffs: String,
    pub debuffs: String,
    pub use_time: Option<f64>,
    pub uses: Option<f64>,
    pub hp_pool: Option<f64>,
    pub max_heal: Option<f64>,
}

fn header_key(h: &str) -> String {
    let l = clean(h).to_lowercase();
    let has = |s: &str| l.contains(s);
    let k = if l == "name" || l == "icon" {
        ""
    } else if has("speed") {
        "speed"
    } else if has("projectile") {
        "projectiles"
    } else if has("penetration") {
        "pen"
    } else if has("armor") {
        "armor_dmg"
    } else if has("accuracy") {
        "accuracy"
    } else if has("recoil") {
        "recoil"
    } else if has("light") {
        "light_bleed"
    } else if has("heavy") {
        "heavy_bleed"
    } else if has("durab") {
        "dur_burn"
    } else if has("heat") {
        "heat"
    } else if has("source") {
        "source"
    } else if has("dmg") || has("damage") {
        "dmg"
    } else {
        ""
    };
    k.to_string()
}

/// A caliber page (e.g. `5.45x39mm`): `==Types==` table with every round's stats.
pub fn parse_caliber_page(caliber: &str, text: &str) -> Vec<Ammo> {
    let secs = sections(text);
    let rows = table_rows(section(&secs, "Types"));
    let Some(hi) = rows.iter().position(|r| r.iter().any(|c| clean(c) == "Name")) else { return vec![] };
    let headers = rows[hi].clone();
    let mut out = Vec::new();
    for row in rows.iter().skip(hi + 1) {
        if row.len() < headers.len() {
            continue;
        }
        let Some(name) = links(&row[1]).into_iter().next() else { continue };
        let mut a = Ammo {
            caliber: caliber.to_string(),
            name,
            icon: file_name(&row[0]),
            tracer: row[1].contains("<sup>T</sup>"),
            subsonic: row[1].contains("<sup>S</sup>"),
            ..Default::default()
        };
        for (h, cell) in headers.iter().zip(row.iter()).skip(2) {
            let v = num(cell);
            match header_key(h).as_str() {
                "dmg" => a.dmg = v,
                "pen" => a.pen = v,
                "armor_dmg" => a.armor_dmg = v,
                "accuracy" => a.accuracy = v,
                "recoil" => a.recoil = v,
                "light_bleed" => a.light_bleed = v,
                "heavy_bleed" => a.heavy_bleed = v,
                "dur_burn" => a.dur_burn = v,
                "heat" => a.heat = v,
                "speed" => a.speed = v,
                "projectiles" => a.projectiles = v,
                "source" => a.source = clean(cell),
                _ => {
                    let c = clean(cell);
                    if !c.is_empty() {
                        a.extra.insert(clean(h), c);
                    }
                }
            }
        }
        out.push(a);
    }
    out
}

/// `Ammunition` page: caliber names in page order, and caliber -> weapons.
pub fn parse_ammunition_index(text: &str) -> (Vec<String>, Vec<AmmoWeapon>) {
    let mut calibers = Vec::new();
    let mut weapons = Vec::new();
    for (_, body) in sections(text) {
        for row in table_rows(&body) {
            if row.len() != 3 {
                continue;
            }
            let Some(cal) = links(&row[1]).into_iter().next() else { continue };
            calibers.push(cal.clone());
            for part in row[2].split("<br/>") {
                let class = re!(r"^([A-Za-z\- ]+):").captures(part.trim()).map(|c| c[1].trim().to_string());
                for w in links(part) {
                    weapons.push(AmmoWeapon { caliber: cal.clone(), weapon: w, class: class.clone() });
                }
            }
        }
    }
    (calibers, weapons)
}

pub fn parse_provisions(text: &str) -> Vec<Provision> {
    let mut out = Vec::new();
    for row in table_rows(text) {
        if row.len() < 9 {
            continue;
        }
        let Some(name) = links(&row[1]).into_iter().next() else { continue };
        out.push(Provision {
            name,
            icon: file_name(&row[0]),
            energy: num(&row[2]),
            hydration: num(&row[3]),
            buffs: clean(&row[4]),
            debuffs: clean(&row[5]),
            use_time: num(&row[6]),
            barter: row[7].contains('✔'),
            craft: row[8].contains('✔'),
            notes: row
                .get(9)
                .map(|n| n.lines().filter(|l| l.starts_with('*')).map(|l| clean(l.trim_start_matches('*'))).collect())
                .unwrap_or_default(),
        });
    }
    out
}

pub fn parse_medical(text: &str) -> Vec<Medical> {
    let mut out = Vec::new();
    for (name, body) in sections(text) {
        if name != "Medical items" && name != "Injectors and stimulants" {
            continue;
        }
        for row in table_rows(&body) {
            if row.len() < 6 {
                continue;
            }
            let Some(item) = links(&row[1]).into_iter().next() else { continue };
            out.push(Medical {
                group: name.clone(),
                name: item,
                icon: file_name(&row[0]),
                kind: clean(&row[2]),
                buffs: clean(&row[3]),
                debuffs: clean(&row[4]),
                use_time: num(&row[5]),
                uses: row.get(6).and_then(|c| num(c)),
                hp_pool: row.get(7).and_then(|c| num(c)),
                max_heal: row.get(8).and_then(|c| num(c)),
            });
        }
    }
    out
}
