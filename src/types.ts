// Mirrors the JSON produced by the Rust core (tarkov-core).
export interface Objective {
  text: string;
  kind: string;
  qty: number | null;
  fir: boolean;
  links: string[];
  maps: string[];
}
export interface Rewards {
  exp: number | null;
  roubles: string | null;
  money: string[];
  rep: string[];
  skills: string[];
  unlocks: string[];
  items: { qty: number; name: string }[];
  other: string[];
}
export interface QuestItem {
  item: string;
  amount: number | null;
  requirement: string;
  fir: boolean;
}
export interface Quest {
  id: string;
  title: string;
  trader: string;
  maps: string[];
  loyalty: number | null;
  player_level: number | null;
  previous: string[];
  previous_any: boolean;
  previous_see_requirements: boolean;
  leads_to: string[];
  kappa: string | null;
  req_text: string[];
  req_quests: string[];
  objectives: Objective[];
  rewards: Rewards;
  quest_items: QuestItem[];
}
export interface Ammo {
  caliber: string;
  name: string;
  icon: string | null;
  tracer: boolean;
  subsonic: boolean;
  dmg: number | null;
  pen: number | null;
  armor_dmg: number | null;
  accuracy: number | null;
  recoil: number | null;
  light_bleed: number | null;
  heavy_bleed: number | null;
  dur_burn: number | null;
  heat: number | null;
  speed: number | null;
  projectiles: number | null;
  source: string;
  extra: Record<string, string>;
}
export interface AmmoWeapon {
  caliber: string;
  weapon: string;
  class: string | null;
}
export interface Provision {
  name: string;
  icon: string | null;
  energy: number | null;
  hydration: number | null;
  buffs: string;
  debuffs: string;
  use_time: number | null;
  barter: boolean;
  craft: boolean;
  notes: string[];
}
export interface Medical {
  group: string;
  name: string;
  icon: string | null;
  kind: string;
  buffs: string;
  debuffs: string;
  use_time: number | null;
  uses: number | null;
  hp_pool: number | null;
  max_heal: number | null;
}
export interface Trader {
  name: string;
  icon: string | null;
}
export type Tab = "tasks" | "raid" | "map" | "items" | "settings";
export type MapMode = "planned" | "open" | "all" | "task";
export type ItemsTab = "ammo" | "food" | "medical";
/** A place in the app; the back/forward history is a stack of these. */
export interface Loc {
  tab: Tab;
  taskId: string | null;
  itemsTab: ItemsTab;
  itemQuery: string;
  /** map view: wiki map name, which tasks to show, and the focused task */
  map: string;
  mapMode: MapMode;
  mapTask: string | null;
}
export interface WikiData {
  tasks: Quest[];
  ammo: Ammo[];
  ammo_weapons: AmmoWeapon[];
  provisions: Provision[];
  medical: Medical[];
  traders: Trader[];
  /** item page title -> icon file (reward / required items of quests) */
  item_icons: Record<string, string>;
  /** icon file name -> thumbnail URL ("" = no such file on the wiki) */
  files: Record<string, string>;
  last_sync: number | null;
}
export type Status = "todo" | "planned" | "done";
export interface Tag {
  id: number;
  name: string;
  color: string;
}
export interface UserState {
  status: Record<string, Status>;
  tags: Tag[];
  task_tags: Record<string, number[]>;
}
export interface PageIssue {
  title: string;
  status: string;
  message: string | null;
}
export interface SyncProgress {
  phase: string;
  done: number;
  total: number;
  message: string;
}
export interface SyncResult {
  ok: boolean;
  error?: string;
}
export interface IconStats {
  resolved: number;
  missing: number;
  stored: number;
  last_error: string | null;
  missing_samples: string[];
  items_with_icon: number;
  items_without_icon: number;
  item_missing_samples: string[];
}

export type Bounds = [[number, number], [number, number]];
export interface MapInfo {
  name: string;
  slug: string;
  svg: string | null;
  tiles: string | null;
  bounds: Bounds | null;
  svg_bounds: Bounds | null;
  transform: [number, number, number, number] | null;
  rotation: number;
  min_zoom: number;
  max_zoom: number;
}
export interface TaskPoint {
  task_id: string;
  map: string;
  kind: string;
  objective_id: string;
  label: string;
  x: number;
  y: number;
  z: number;
  outline: [number, number][];
}
export interface PointStats {
  tasks_in_api: number;
  tasks_with_points: number;
  points: number;
  unmatched: string[];
}
export interface MapData {
  maps: MapInfo[];
  points: TaskPoint[];
  stats: PointStats | null;
  last_error: string | null;
}
