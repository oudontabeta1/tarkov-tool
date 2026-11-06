use tarkov_core::{db, parsers::items, parsers::quest};

fn fx(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{}.txt", env!("CARGO_MANIFEST_DIR"), name)).unwrap()
}

fn tmp(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("tarkov-core-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn wiki_db_roundtrip() {
    let path = tmp("wiki.db");
    let c = db::open_wiki(&path).unwrap();
    let q = quest::parse_quest("Debut", &fx("page_Debut"));
    db::save_quest(&c, &q, 0).unwrap();
    db::save_quest(&c, &quest::parse_quest("Shortage", &fx("page_Shortage")), 1).unwrap();
    db::upsert_page(&c, "Debut", 123, "ok", None).unwrap();
    db::upsert_page(&c, "Shortage", 5, "warn", Some("no objectives found")).unwrap();
    let all = db::all_quests(&c).unwrap();
    assert_eq!(all.iter().map(|q| q.title.as_str()).collect::<Vec<_>>(), vec!["Debut", "Shortage"]);
    assert_eq!(all[0].rewards.exp, Some(3000.0));
    assert_eq!(db::revids(&c).unwrap()["Debut"], 123);
    assert_eq!(db::issues(&c).unwrap().len(), 1);
    // dropping tasks that left the index
    assert_eq!(db::remove_tasks_except(&c, &["debut".to_string()]).unwrap(), 1);
    assert_eq!(db::all_quests(&c).unwrap().len(), 1);

    let rows = items::parse_caliber_page("5.45x39mm", &fx("page_5_45x39mm"));
    db::replace_ammo(&c, "5.45x39mm", &rows).unwrap();
    db::replace_ammo(&c, "5.45x39mm", &rows).unwrap(); // idempotent
    assert_eq!(db::all_ammo(&c).unwrap().len(), 13);
    db::replace_provisions(&c, &items::parse_provisions(&fx("page_Provisions"))).unwrap();
    db::replace_medical(&c, &items::parse_medical(&fx("page_Medical"))).unwrap();
    assert_eq!(db::all_medical(&c).unwrap().len(), 43);
    db::save_files(&c, &[("a.png".to_string(), "https://x/a.png".to_string()), ("gone.png".to_string(), String::new())]).unwrap();
    let files = db::files_map(&c).unwrap();
    assert_eq!(files["a.png"], "https://x/a.png");
    assert_eq!(files["gone.png"], "");
    db::save_trader(&c, &tarkov_core::parsers::quest::Trader { name: "Prapor".into(), icon: Some("P.png".into()) }).unwrap();
    assert_eq!(db::all_traders(&c).unwrap()[0].icon.as_deref(), Some("P.png"));
    db::save_image(&c, "a.png", "image/png", &[1, 2, 3]).unwrap();
    assert_eq!(db::get_image(&c, "a.png").unwrap().unwrap(), ("image/png".to_string(), vec![1, 2, 3]));
    assert!(db::get_image(&c, "nope.png").unwrap().is_none());
    assert!(db::image_names(&c).unwrap().contains("a.png"));
    let st = db::icon_stats(&c).unwrap();
    assert_eq!((st.resolved, st.missing, st.stored), (1, 1, 1));
    assert_eq!(st.missing_samples, vec!["gone.png"]);
    db::save_item_icon(&c, "AK-74N", Some("AK-74N icon.png")).unwrap();
    db::save_item_icon(&c, "No icon item", None).unwrap();
    assert_eq!(db::item_icon_names(&c).unwrap().len(), 2);
    assert_eq!(db::item_icons_map(&c).unwrap().len(), 1);
    db::clear_pages(&c).unwrap();
    assert!(db::item_icon_names(&c).unwrap().is_empty());
    assert!(db::files_map(&c).unwrap().is_empty());
    db::meta_set(&c, "last_sync", "1").unwrap();
    assert_eq!(db::meta_get(&c, "last_sync").unwrap().as_deref(), Some("1"));
}

#[test]
fn user_db_state_tags_and_raid() {
    let path = tmp("user.db");
    let c = db::open_user(&path).unwrap();
    db::set_status(&c, "debut", "planned").unwrap();
    db::create_tag(&c, "キー必要", "#ff8844").unwrap();
    db::create_tag(&c, "キー必要", "#ff8844").unwrap(); // duplicate ignored
    let st = db::user_state(&c).unwrap();
    assert_eq!(st.status["debut"], "planned");
    assert_eq!(st.tags.len(), 1);
    let tag = st.tags[0].id;
    db::toggle_task_tag(&c, "debut", tag).unwrap();
    assert_eq!(db::user_state(&c).unwrap().task_tags["debut"], vec![tag]);
    db::toggle_task_tag(&c, "debut", tag).unwrap();
    assert!(db::user_state(&c).unwrap().task_tags.get("debut").is_none());

    db::finish_raid(&c, "Woods", &["debut".to_string(), "shortage".to_string()]).unwrap();
    let st = db::user_state(&c).unwrap();
    assert_eq!(st.status["debut"], "done");
    assert_eq!(st.status["shortage"], "done");
    db::set_status(&c, "debut", "todo").unwrap();
    assert!(db::user_state(&c).unwrap().status.get("debut").is_none());

    db::setting_set(&c, "contact", "me@example.com").unwrap();
    assert_eq!(db::setting_get(&c, "contact").unwrap().as_deref(), Some("me@example.com"));
    db::delete_tag(&c, tag).unwrap();
    assert!(db::user_state(&c).unwrap().tags.is_empty());
}
