//! Quest pages: `{{Infobox quest}}` + Objectives / Rewards / Requirements / Guide.
use crate::wikitext::*;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

macro_rules! re {
    ($p:expr) => {{
        static R: Lazy<Regex> = Lazy::new(|| Regex::new($p).unwrap());
        &*R
    }};
}

pub const MAPS: [&str; 13] = [
    "Customs", "Factory", "Ground Zero", "Icebreaker", "Interchange", "Lighthouse", "Reserve",
    "Shoreline", "Streets of Tarkov", "Terminal", "The Lab", "The Labyrinth", "Woods",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Objective {
    pub text: String,
    /// kill / handover / find / extract / place / gunsmith / other
    pub kind: String,
    pub qty: Option<u32>,
    pub fir: bool,
    pub links: Vec<String>,
    pub maps: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RewardItem {
    pub qty: u32,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rewards {
    pub exp: Option<f64>,
    /// first money line, kept for older data
    pub roubles: Option<String>,
    /// every money line: "80,000 Roubles", "1,000 Euros", ...
    #[serde(default)]
    pub money: Vec<String>,
    pub rep: Vec<String>,
    /// skill level rewards: "+3 Sniper rifles skill levels"
    #[serde(default)]
    pub skills: Vec<String>,
    /// "Unlocks achievement ..." / trader purchase unlocks
    #[serde(default)]
    pub unlocks: Vec<String>,
    pub items: Vec<RewardItem>,
    pub other: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestItem {
    pub item: String,
    pub amount: Option<f64>,
    pub requirement: String,
    pub fir: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub trader: String,
    pub maps: Vec<String>,
    pub loyalty: Option<u32>,
    pub player_level: Option<u32>,
    pub previous: Vec<String>,
    /// true when the page says "A or B or C"
    pub previous_any: bool,
    pub previous_see_requirements: bool,
    pub leads_to: Vec<String>,
    pub kappa: Option<String>,
    pub req_text: Vec<String>,
    pub req_quests: Vec<String>,
    pub objectives: Vec<Objective>,
    pub rewards: Rewards,
    pub quest_items: Vec<QuestItem>,
}

fn kind_of(text: &str) -> &'static str {
    let l = text.to_lowercase();
    let starts = |p: &[&str]| p.iter().any(|x| l.starts_with(x));
    if starts(&["eliminate", "kill", "headshot"]) {
        "kill"
    } else if starts(&["hand over"]) {
        "handover"
    } else if starts(&["find", "obtain", "locate"]) {
        "find"
    } else if starts(&["survive"]) || l.contains("extract") {
        "extract"
    } else if starts(&["mark", "plant", "place", "stash", "put"]) {
        "place"
    } else if starts(&["modify", "build"]) {
        "gunsmith"
    } else {
        "other"
    }
}

/// "+3 Sniper rifles skill levels", "Endurance +0.5", ...
fn is_skill(text: &str) -> bool {
    re!(
        r"(?i)\bskills?\b|\b(endurance|strength|vitality|health|stress resistance|metabolism|immunity|perception|intellect|attention|charisma|memory|covert movement|search|surgery|first aid|pistols?|revolvers?|smgs?|assault rifles?|shotguns?|dmr|sniper rifles?|lmgs?|heavy machine ?guns?|hmgs?|mag drills|recoil control|weapon modding|troubleshooting|throwables?|melee|light vests|heavy vests|lockpicking|shadow connections|sniping)\b"
    )
    .is_match(text)
        && re!(r"[+-]\s*\d").is_match(text)
}

fn top_bullets(body: &str) -> Vec<&str> {
    body.lines().filter(|l| l.starts_with('*') && !l.starts_with("**")).collect()
}

pub fn parse_quest(title: &str, text: &str) -> Quest {
    let ib = infobox(text, "Infobox quest");
    let get = |k: &str| ib.get(k).map(String::as_str).unwrap_or("");
    let secs = sections(text);

    let prev_raw = get("previous");
    let mut q = Quest {
        id: slug(title),
        title: title.to_string(),
        trader: clean(get("given by")),
        loyalty: num(get("LL requirement")).map(|v| v as u32),
        previous: links(prev_raw).into_iter().filter(|l| l != title).collect(),
        previous_any: re!(r"(?i)<br\s*/?>\s*or\s*<br\s*/?>").is_match(prev_raw),
        previous_see_requirements: prev_raw.contains("See requirements"),
        leads_to: links(get("leads to")),
        kappa: Some(clean(get("reqkappa"))).filter(|s| !s.is_empty()),
        ..Default::default()
    };

    // Requirements (player level, other conditions, required quests)
    let req = section(&secs, "Requirements");
    for l in top_bullets(req) {
        let t = clean(l.trim_start_matches(['*', ' ']));
        if !t.to_lowercase().contains("loyalty") && q.player_level.is_none() {
            if let Some(c) = re!(r"(?i)\blevel (\d+)").captures(&t) {
                q.player_level = c[1].parse().ok();
            }
        }
        q.req_text.push(t);
    }
    q.req_quests = req.lines().filter(|l| l.starts_with("**")).flat_map(links).collect();

    // Objectives
    for l in top_bullets(section(&secs, "Objectives")) {
        let raw = l.trim_start_matches(['*', ' ']);
        let text = clean(raw);
        let lk = links(raw);
        let maps: Vec<String> = lk.iter().filter(|x| MAPS.contains(&x.as_str())).cloned().collect();
        let qty = re!(r"\b(\d[\d,]*)\b").captures(&text).and_then(|c| c[1].replace(',', "").parse().ok());
        q.objectives.push(Objective {
            kind: kind_of(&text).to_string(),
            qty,
            fir: re!(r"(?i)in raid").is_match(raw),
            links: lk,
            maps,
            text,
        });
    }

    // Maps: infobox location + maps named in objectives
    let mut maps: Vec<String> = links(get("location")).into_iter().filter(|m| MAPS.contains(&m.as_str())).collect();
    for o in &q.objectives {
        for m in &o.maps {
            if !maps.contains(m) {
                maps.push(m.clone());
            }
        }
    }
    q.maps = maps;

    // Rewards (top-level bullets only; nested lines are Intelligence Center bonuses)
    for l in top_bullets(section(&secs, "Rewards")) {
        let raw = l.trim_start_matches(['*', ' ']);
        let c = clean(raw);
        if re!(r"\bEXP\b").is_match(&c) {
            q.rewards.exp = num(&c);
        } else if re!(r"\bRep\b").is_match(&c) {
            q.rewards.rep.push(c);
        } else if re!(r"Roubles|Dollars|Euros").is_match(&c) && links(raw).is_empty() {
            if q.rewards.roubles.is_none() {
                q.rewards.roubles = Some(c.clone());
            }
            q.rewards.money.push(c);
        } else if re!(r"(?i)^unlocks?\b").is_match(&c) {
            q.rewards.unlocks.push(c);
        } else if is_skill(&c) {
            q.rewards.skills.push(c);
        } else if let Some(m) = re!(r"^(\d[\d,]*)\s*[×x]\s*(.*)$").captures(&c) {
            let name = links(raw).into_iter().next().unwrap_or_else(|| m[2].to_string());
            q.rewards.items.push(RewardItem { qty: m[1].replace(',', "").parse().unwrap_or(1), name });
        } else if !c.is_empty() {
            q.rewards.other.push(c);
        }
    }

    // "Related Quest Items" table in the Guide
    for row in table_rows(section(&secs, "Guide")) {
        if row.len() >= 5 {
            if let (Some(item), Some(amount)) = (links(&row[1]).into_iter().next(), num(&row[2])) {
                q.quest_items.push(QuestItem {
                    item,
                    amount: Some(amount),
                    requirement: clean(&row[3]),
                    fir: clean(&row[4]).to_lowercase().contains("yes"),
                });
            }
        }
    }
    q
}

/// `Template:Navbox quests` -> (trader, quest title) in page order.
pub fn parse_quest_index(navbox: &str) -> Vec<(String, String)> {
    let mut traders: Vec<(String, String)> = Vec::new();
    let mut out: Vec<(String, String)> = Vec::new();
    let key = |l: &str, prefix: &str| l[prefix.len()..].split(|c: char| c == '=' || c.is_whitespace()).next().unwrap_or("").to_string();
    for l in navbox.lines() {
        if l.starts_with("|group") {
            if let Some(t) = links(l).into_iter().next() {
                traders.push((key(l, "|group"), t));
            }
        } else if l.starts_with("|content") {
            let k = key(l, "|content");
            let trader = traders.iter().find(|(kk, _)| *kk == k).map(|(_, t)| t.clone());
            let Some(trader) = trader else { continue };
            for m in re!(r"\[\[([^\]|]+)(?:\|[^\]]*)?\]\]").captures_iter(l) {
                let t = m[1].trim().to_string();
                if !t.contains('#') && !out.iter().any(|(_, x)| *x == t) {
                    out.push((trader.clone(), t));
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Trader {
    pub name: String,
    pub icon: Option<String>,
}

/// Trader page (`{{Infobox character}}`): small `icon`, falling back to the portrait `image`.
pub fn parse_trader(name: &str, text: &str) -> Trader {
    let ib = infobox(text, "Infobox character");
    let pick = |k: &str| ib.get(k).and_then(|v| plain_file(v));
    Trader { name: name.to_string(), icon: pick("icon").or_else(|| pick("image")) }
}
