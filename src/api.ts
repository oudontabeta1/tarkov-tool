import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { IconStats, MapData, PageIssue, Status, SyncProgress, SyncResult, UserState, WikiData } from "./types";

export const api = {
  wikiData: () => invoke<WikiData>("get_wiki_data"),
  mapData: () => invoke<MapData>("get_map_data"),
  iconStats: () => invoke<IconStats>("get_icon_stats"),
  userState: () => invoke<UserState>("get_user_state"),
  issues: () => invoke<PageIssue[]>("get_issues"),
  setStatus: (taskId: string, status: Status) => invoke<void>("set_task_status", { taskId, status }),
  createTag: (name: string, color: string) => invoke<void>("create_tag", { name, color }),
  deleteTag: (id: number) => invoke<void>("delete_tag", { id }),
  toggleTaskTag: (taskId: string, tagId: number) => invoke<void>("toggle_task_tag", { taskId, tagId }),
  finishRaid: (map: string, done: string[]) => invoke<void>("finish_raid", { map, done }),
  contact: () => invoke<string>("get_contact"),
  setContact: (contact: string) => invoke<void>("set_contact", { contact }),
  openMap: (map: string) => invoke<void>("open_map", { map }),
  startSync: () => invoke<void>("start_sync"),
  onSyncProgress: (fn: (p: SyncProgress) => void) => listen<SyncProgress>("sync-progress", (e) => fn(e.payload)),
  onSyncFinished: (fn: (r: SyncResult) => void) => listen<SyncResult>("sync-finished", (e) => fn(e.payload)),
};
