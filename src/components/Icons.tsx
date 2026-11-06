import { useContext, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { DataContext } from "../context";

/**
 * Wiki icon. Served from the local cache (icon:// protocol, works offline);
 * falls back to the wiki's own URL, then to nothing.
 */
function WikiImage({
  file, size, className, alt = "", title, onLoaded,
}: {
  file: string;
  size: number;
  className: string;
  alt?: string;
  title?: string;
  onLoaded: () => void;
}) {
  const { files } = useContext(DataContext);
  const [stage, setStage] = useState(0); // 0 = local cache, 1 = remote URL, 2 = give up
  const remote = files[file];
  if (stage >= 2) return null;
  const src = stage === 0 ? convertFileSrc(file, "icon") : remote;
  if (!src) return null;
  return (
    <img
      key={`${file}:${stage}`}
      className={className}
      src={src}
      width={size}
      height={size}
      alt={alt}
      title={title}
      loading="lazy"
      referrerPolicy="no-referrer"
      ref={(el) => {
        // an image that finished loading before React attached `onLoad` would never report it
        if (el && el.complete && el.naturalWidth > 0) onLoaded();
      }}
      onLoad={onLoaded}
      onError={() => setStage((s) => (s === 0 && remote ? 1 : 2))}
    />
  );
}

/** Item icon; an empty box until (or unless) the image loads. */
export function ItemIcon({ file, name, size = 28 }: { file?: string | null; name?: string; size?: number }) {
  const { files, nameIcon } = useContext(DataContext);
  const [loaded, setLoaded] = useState(false);
  const f = file ?? (name ? nameIcon[name] : undefined);
  const known = !!f && !!files[f];
  return (
    <span className="icon-wrap" style={{ width: size, height: size }}>
      {!loaded && <span className="icon ph" />}
      {known && <WikiImage key={f} file={f!} size={size} className="icon" onLoaded={() => setLoaded(true)} />}
    </span>
  );
}

const hue = (s: string) => [...s].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 360, 7);

/** Trader portrait; initials on a coloured disc until (or unless) the wiki icon loads. */
export function TraderAvatar({ name, size = 26 }: { name: string; size?: number }) {
  const { files, traders } = useContext(DataContext);
  const [loaded, setLoaded] = useState(false);
  const icon = traders.find((t) => t.name === name)?.icon;
  const known = !!icon && !!files[icon];
  return (
    <span className="icon-wrap" style={{ width: size, height: size }}>
      {!loaded && (
        <span className="avatar" title={name} style={{ width: size, height: size, background: `hsl(${hue(name)} 40% 32%)`, fontSize: size * 0.42 }}>
          {name.slice(0, 2)}
        </span>
      )}
      {known && <WikiImage key={icon} file={icon!} size={size} className="avatar" alt={name} title={name} onLoaded={() => setLoaded(true)} />}
    </span>
  );
}
