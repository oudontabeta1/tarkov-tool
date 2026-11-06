use std::collections::HashSet;
use tarkov_core::positions::{parse_map_configs, parse_task_points, task_id_from_link};

const TASKS: &str = r#"{"data":{"tasks":{
 "t1":{"id":"t1","wikiLink":"https://escapefromtarkov.fandom.com/wiki/Debut","objectives":[
  {"id":"o1","description":"o1","type":"visit","optional":false,"zones":[
   {"id":"Sandbox_1_MedicalArea_exploration","map":"65b8d6f5cdde2479cb2a3125","position":{"x":156.2,"y":25.52,"z":-83.59},
    "outline":[{"x":143.45,"y":25.52,"z":-90.69},{"x":168.95,"y":25.52,"z":-90.69},{"x":168.95,"y":25.52,"z":-76.49},{"x":143.45,"y":25.52,"z":-76.49}],"top":28.52,"bottom":22.52},
   {"id":"Sandbox_1_MedicalArea_exploration","map":"653e6760052c01c1c805532f","position":{"x":156.2,"y":25.52,"z":-83.59},
    "outline":[{"x":143.45,"y":22.52,"z":-90.69}],"top":28.52,"bottom":22.52}]},
  {"id":"o2","description":"o2","type":"findQuestItem","possibleLocations":[
   {"map":"56f40101d2720b2a4d8b45d6","positions":[{"x":310.71,"y":-1.63,"z":-174.95},{"x":489.06,"y":3.83,"z":221.39}]}]},
  {"id":"o3","description":"o3","type":"shoot","count":5,"zones":[]},
  {"id":"o4","description":"o4","type":"visit","zones":[{"id":"z","map":"6a8dd1325e15a5ea2e010820","position":{"x":1,"y":2,"z":3}}]}
 ]},
 "t2":{"id":"t2","wikiLink":"https://escapefromtarkov.fandom.com/wiki/What%E2%80%99s_on_the_Flash_Drive%3F","objectives":[
  {"id":"o5","description":"o5","type":"mark","zones":[{"id":"zz","map":"5704e3c2d2720bac5b8b4567","position":{"x":10,"y":1,"z":20}}]}]},
 "t3":{"id":"t3","wikiLink":"https://escapefromtarkov.fandom.com/wiki/Some_Removed_Quest","objectives":[
  {"id":"o6","description":"o6","type":"visit","zones":[{"id":"zz","map":"5704e3c2d2720bac5b8b4567","position":{"x":1,"y":1,"z":1}}]}]},
 "t4":{"id":"t4","wikiLink":"https://escapefromtarkov.fandom.com/wiki/No_Positions","objectives":[{"id":"o7","description":"o7","type":"giveItem"}]}
}}}"#;
const EN: &str = r#"{"data":{"o1":"Locate the medical area","o2":"","o5":"Mark the spot"}}"#;

#[test]
fn links_become_task_ids() {
    assert_eq!(task_id_from_link("https://escapefromtarkov.fandom.com/wiki/Debut").as_deref(), Some("debut"));
    assert_eq!(task_id_from_link("https://escapefromtarkov.fandom.com/wiki/Gunsmith_-_Part_1#Guide").as_deref(), Some("gunsmith-part-1"));
    assert_eq!(
        task_id_from_link("https://escapefromtarkov.fandom.com/wiki/What%E2%80%99s_on_the_Flash_Drive%3F"),
        task_id_from_link("https://escapefromtarkov.fandom.com/wiki/What’s_on_the_Flash_Drive?"),
    );
    assert_eq!(task_id_from_link("https://example.com/nothing"), None);
}

#[test]
fn task_points_are_extracted_joined_and_deduplicated() {
    let known: HashSet<String> = ["debut", "what-s-on-the-flash-drive"].iter().map(|s| s.to_string()).collect();
    let (pts, st) = parse_task_points(TASKS, Some(EN), Some(&known)).unwrap();
    // Debut: one zone (listed for both Ground Zero variants -> once) + 2 quest-item locations;
    // the infected-factory zone is ignored; the flash-drive task has 1; the unknown quest is dropped.
    assert_eq!(pts.len(), 4, "{pts:#?}");
    assert_eq!(st.tasks_in_api, 4);
    assert_eq!(st.tasks_with_points, 2);
    assert_eq!(st.unmatched, vec!["some-removed-quest"]);

    let zone = pts.iter().find(|p| p.kind == "visit").unwrap();
    assert_eq!((zone.task_id.as_str(), zone.map.as_str()), ("debut", "Ground Zero"));
    assert_eq!((zone.x, zone.z), (156.2, -83.59));
    assert_eq!(zone.outline.len(), 4);
    assert_eq!(zone.label, "Locate the medical area");

    let items: Vec<_> = pts.iter().filter(|p| p.kind == "findQuestItem").collect();
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|p| p.map == "Customs" && p.outline.is_empty()));
    assert!(pts.iter().any(|p| p.task_id == "what-s-on-the-flash-drive" && p.map == "Woods"));
}

#[test]
fn map_configs_from_tarkov_dev() {
    let maps = parse_map_configs(&std::fs::read_to_string(format!("{}/tests/fixtures/tarkov_dev_maps.json", env!("CARGO_MANIFEST_DIR"))).unwrap());
    let names: Vec<&str> = maps.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(maps.len(), 13, "{names:?}");
    for n in ["Customs", "Factory", "Ground Zero", "Icebreaker", "Interchange", "Lighthouse", "Reserve", "Shoreline", "Streets of Tarkov", "Terminal", "The Lab", "The Labyrinth", "Woods"] {
        assert!(names.contains(&n), "missing {n}");
    }
    let c = maps.iter().find(|m| m.name == "Customs").unwrap();
    assert_eq!(c.transform, Some([0.239, 168.65, 0.239, 136.35]));
    assert_eq!(c.rotation, 180.0);
    assert_eq!(c.bounds, Some([[698.0, -307.0], [-372.0, 237.0]]));
    assert!(c.svg.as_deref().unwrap().ends_with("Customs.svg"));
    // maps without an SVG fall back to tiles
    let lab = maps.iter().find(|m| m.name == "The Lab").unwrap();
    assert!(lab.svg.is_none() && lab.tiles.is_some());
}
