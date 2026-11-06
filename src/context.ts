import { createContext } from "react";
import type { Trader } from "./types";

export interface DataCtx {
  files: Record<string, string>;
  traders: Trader[];
  /** item name -> icon file name (ammo, provisions, medical) */
  nameIcon: Record<string, string>;
}

export const DataContext = createContext<DataCtx>({ files: {}, traders: [], nameIcon: {} });
