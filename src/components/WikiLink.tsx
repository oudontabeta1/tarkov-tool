import { openUrl } from "@tauri-apps/plugin-opener";
import { wikiUrl } from "../util";

/** Small ↗ button that opens the item's wiki page in the system browser. */
export default function WikiLink({ title }: { title: string }) {
  return (
    <button className="wikilink" title={`Wikiで開く: ${wikiUrl(title)}`} onClick={() => openUrl(wikiUrl(title)).catch(() => {})}>
      ↗
    </button>
  );
}
