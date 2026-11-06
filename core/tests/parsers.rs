use tarkov_core::parsers::{items, quest};
use tarkov_core::wikitext;

fn fx(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{}.txt", env!("CARGO_MANIFEST_DIR"), name)).unwrap()
}

#[test]
fn quest_debut() {
    let q = quest::parse_quest("Debut", &fx("page_Debut"));
    assert_eq!(q.trader, "Prapor");
    assert_eq!(q.loyalty, Some(1));
    assert_eq!(q.maps, vec!["Customs", "Ground Zero", "Interchange", "Woods"]);
    assert_eq!(q.objectives.len(), 1);
    assert_eq!(q.objectives[0].kind, "kill");
    assert_eq!(q.objectives[0].qty, Some(5));
    assert_eq!(q.rewards.exp, Some(3000.0));
    assert!(q.rewards.roubles.as_deref().unwrap().contains("80,000"));
    assert_eq!(q.rewards.rep.len(), 1);
}

#[test]
fn quest_shortage_items_and_prereqs() {
    let q = quest::parse_quest("Shortage", &fx("page_Shortage"));
    assert_eq!(q.previous, vec!["First in Line", "Operation Aquarius", "Supply Plans"]);
    assert!(q.previous_any);
    assert_eq!(q.objectives.len(), 2);
    assert!(q.objectives[0].fir && q.objectives[0].kind == "find");
    assert_eq!(q.objectives[1].kind, "handover");
    assert_eq!(q.quest_items.len(), 1);
    assert_eq!(q.quest_items[0].item, "Salewa first aid kit");
    assert_eq!(q.quest_items[0].amount, Some(3.0));
    assert!(q.quest_items[0].fir);
}

#[test]
fn quest_collector_requirements() {
    let q = quest::parse_quest("Collector", &fx("page_Collector"));
    assert_eq!(q.player_level, Some(40), "loyalty level must not be read as player level");
    assert!(q.previous_see_requirements);
    assert!(q.previous.is_empty());
    assert!(q.req_quests.contains(&"Chemical - Part 3".to_string()));
    assert!(q.objectives.len() > 20);
    assert!(q.objectives.iter().all(|o| o.fir && o.kind == "handover"));
}

#[test]
fn quest_gunsmith_and_stabilize() {
    let g = quest::parse_quest("Gunsmith - AKM", &fx("page_Gunsmith_-_AKM"));
    assert_eq!(g.trader, "Mechanic");
    assert_eq!(g.loyalty, Some(3));
    assert_eq!(g.kappa.as_deref(), Some("Yes"));
    assert_eq!(g.objectives[0].kind, "gunsmith");
    assert!(g.quest_items.is_empty(), "shopping-list tables must not become quest items");
    let s = quest::parse_quest("Stabilize Business", &fx("page_Stabilize_Business"));
    assert_eq!(s.maps, vec!["Ground Zero"]);
    assert_eq!(s.previous, vec!["Ask for Directions"]);
    assert_eq!(s.leads_to, vec!["Battery Change"]);
    assert_eq!(s.rewards.items.len(), 1);
    assert_eq!(s.rewards.items[0].qty, 1);
}

#[test]
fn quest_index() {
    let idx = quest::parse_quest_index(&fx("page_Template_Navbox_quests"));
    assert!(idx.len() > 500, "got {}", idx.len());
    assert!(idx.iter().any(|(t, q)| t == "Prapor" && q == "Debut"));
    assert!(idx.iter().any(|(t, q)| t == "BTR Driver" && q == "Ask for Directions"));
    assert!(!idx.iter().any(|(_, q)| q.contains('#')), "Operational Tasks link must be skipped");
}

#[test]
fn caliber_table() {
    let rows = items::parse_caliber_page("5.45x39mm", &fx("page_5_45x39mm"));
    assert_eq!(rows.len(), 13);
    let bs = rows.iter().find(|a| a.name == "5.45x39mm BS gs").unwrap();
    assert_eq!((bs.dmg, bs.pen, bs.armor_dmg, bs.speed), (Some(45.0), Some(54.0), Some(57.0), Some(830.0)));
    assert_eq!(bs.accuracy, Some(-4.0));
    assert_eq!(bs.source, "Loot only");
    assert_eq!(bs.icon.as_deref(), Some("74BSICON.png"));
    assert!(rows.iter().find(|a| a.name == "5.45x39mm BT gs").unwrap().tracer);
    assert!(rows.iter().find(|a| a.name == "5.45x39mm US gs").unwrap().subsonic);
}

#[test]
fn ammunition_index() {
    let (cal, weapons) = items::parse_ammunition_index(&fx("page_Ammunition"));
    assert!(cal.len() >= 30, "got {}", cal.len());
    assert!(cal.contains(&"5.45x39mm".to_string()) && cal.contains(&"12/70".to_string()));
    let ak = weapons.iter().find(|w| w.caliber == "5.45x39mm" && w.weapon == "AK-74N").unwrap();
    assert_eq!(ak.class.as_deref(), Some("Assault rifles"));
    assert!(weapons.iter().any(|w| w.caliber == "12/70" && w.weapon == "M870" && w.class.is_none()));
}

#[test]
fn provisions_and_medical() {
    let p = items::parse_provisions(&fx("page_Provisions"));
    assert!(p.len() >= 40, "got {}", p.len());
    let beef = p.iter().find(|x| x.name == "Can of GigaBeef meat").unwrap();
    assert_eq!((beef.energy, beef.hydration, beef.use_time), (Some(100.0), Some(-50.0), Some(5.0)));
    assert!(!beef.barter && !beef.craft);
    assert_eq!(beef.icon.as_deref(), Some("Can of GigaBeef meat icon.png"));
    assert!(beef.notes.iter().any(|n| n.contains("Collector")));
    let m = items::parse_medical(&fx("page_Medical"));
    assert_eq!(m.len(), 43);
    let salewa = m.iter().find(|x| x.name == "Salewa first aid kit").unwrap();
    assert_eq!((salewa.use_time, salewa.hp_pool, salewa.max_heal), (Some(3.0), Some(400.0), Some(85.0)));
    assert!(m.iter().any(|x| x.group == "Injectors and stimulants"));
    assert!(m.iter().filter(|x| x.icon.is_some()).count() > 35, "most medical rows have an icon");
}

#[test]
fn helpers() {
    assert_eq!(wikitext::slug("Gunsmith - Part 1"), "gunsmith-part-1");
    assert_eq!(wikitext::clean("[[File:Pain.png|46px|Pain]] [[A|b]] <font color=\"red\">x</font>"), "[Pain] b x");
    assert_eq!(wikitext::num("<font color=\"green\">+50</font>"), Some(50.0));
}

#[test]
fn trader_icon() {
    let t = quest::parse_trader("Prapor", "{{Infobox character\n|image =Prapor.png\n|icon =Prapor icon.png\n|faction =x\n}}\ntext");
    assert_eq!(t.icon.as_deref(), Some("Prapor icon.png"));
    let t = quest::parse_trader("Ref", "{{Infobox character\n|image =[[File:Ref.png|200px]]\n|icon =\n}}");
    assert_eq!(t.icon.as_deref(), Some("Ref.png"));
    assert!(quest::parse_trader("X", "no infobox").icon.is_none());
    assert_eq!(wikitext::file_name("![[File:74BSICON.png|link=A]]").as_deref(), Some("74BSICON.png"));
}

#[test]
fn reward_kinds() {
    let page = r#"{{Infobox quest
|given by =[[Mechanic]]
}}
==Objectives==
* Do it
==Rewards==
* +8,200 [[EXP]]
* [[Mechanic]] Rep <font color="green">'''+0.02'''</font>
* 1,000 Euros
** 1,050 Euros with [[Hideout#Modules|Intelligence Center]] Level 1
* 40,000 Roubles
* +3 [[Sniper rifles]] skill levels
* [[Endurance]] +0.5
* 2× [[Pack of nails]]
* Unlocks purchase of [[Sniper rifle X]] at Jaeger
* Unlocks achievement [[Achievements#Achievements|Dawn]]
==Guide==
x"#;
    let q = quest::parse_quest("T", page);
    let r = &q.rewards;
    assert_eq!(r.exp, Some(8200.0));
    assert_eq!(r.rep.len(), 1);
    assert_eq!(r.money, vec!["1,000 Euros", "40,000 Roubles"]);
    assert_eq!(r.roubles.as_deref(), Some("1,000 Euros"));
    assert_eq!(r.skills.len(), 2, "{:?}", r.skills);
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.unlocks.len(), 2);
    assert!(r.other.is_empty(), "{:?}", r.other);
}

#[test]
fn item_page_icons() {
    let ak = "Kalashnikov AK-74N 5.45x39 assault rifle";
    // weapon pages hide the icon in an odd line
    assert_eq!(wikitext::infobox_icon(&fx("page_AK-74N"), &[ak]).as_deref(), Some("AK-74N icon.png"));
    // ammo pages have a normal `icon` parameter
    assert_eq!(wikitext::infobox_icon(&fx("page_5_45x39mm_BS_gs"), &["x"]).as_deref(), Some("74BSICON.png"));
    assert_eq!(wikitext::infobox_icon(&fx("page_12_70_flechette"), &["x"]).as_deref(), Some("12-70 Flechette icon.png"));
    // `image` is only the last resort; template placeholders are ignored
    assert_eq!(wikitext::infobox_icon("{{Infobox item\n|image =Big.png\n|icon =\n}}", &["x"]).as_deref(), Some("Big.png"));
    assert_eq!(wikitext::infobox_icon("{{Infobox item\n|icon ={{{icon|}}}\n}}", &["x"]), None);
    assert_eq!(wikitext::infobox_icon("no infobox here", &["x"]), None);
    // lower-case template name and a closing `}}` on the last parameter line
    assert_eq!(wikitext::infobox_icon("{{infobox weapon\n|type =x\n|icon =TKPD icon.png}}\ntext", &["x"]).as_deref(), Some("TKPD icon.png"));
    // no usable infobox: use the image that links back to this page (crafting table)
    let page = "Intro\n{|class=\"wikitable\"\n![[File:Other.png|frameless|link=Pliers]] x1\n![[File:TKPD icon.png|frameless|link=TKPD 9.3x64 carbine]] x1\n|}";
    assert_eq!(wikitext::infobox_icon(page, &["tkpd 9.3x64 carbine"]).as_deref(), Some("TKPD icon.png"));
}
