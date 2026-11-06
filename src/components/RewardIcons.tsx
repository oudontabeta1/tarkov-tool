import type { ReactNode } from "react";
import { TraderAvatar } from "./Icons";

const Disc = ({ size, bg, children, title }: { size: number; bg: string; children: ReactNode; title: string }) => (
  <span className="reward-disc" title={title} style={{ width: size, height: size, background: bg, fontSize: size * 0.4 }}>
    {children}
  </span>
);

export const XpIcon = ({ size = 44 }: { size?: number }) => (
  <Disc size={size} bg="linear-gradient(135deg,#d8b13a,#8a6a12)" title="経験値 (EXP)">XP</Disc>
);

const SYMBOL: Record<string, [string, string]> = {
  roubles: ["₽", "#5a7d3a"],
  dollars: ["$", "#2f7a52"],
  euros: ["€", "#3a5f9a"],
};
export const currencyOf = (text: string) => (/dollars/i.test(text) ? "dollars" : /euros/i.test(text) ? "euros" : "roubles");

export const MoneyIcon = ({ text, size = 44 }: { text: string; size?: number }) => {
  const [sym, bg] = SYMBOL[currencyOf(text)];
  return <Disc size={size} bg={bg} title={currencyOf(text)}>{sym}</Disc>;
};

export const SkillIcon = ({ size = 44 }: { size?: number }) => (
  <span className="reward-disc" title="スキルレベル" style={{ width: size, height: size, background: "linear-gradient(135deg,#8a63d2,#4a3585)" }}>
    <svg width={size * 0.6} height={size * 0.6} viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 18 L10 11 L14 15 L20 6" />
      <path d="M15 6 H20 V11" />
    </svg>
  </span>
);

export const UnlockIcon = ({ size = 44 }: { size?: number }) => (
  <span className="reward-disc" title="解放" style={{ width: size, height: size, background: "#3a4a5a" }}>
    <svg width={size * 0.55} height={size * 0.55} viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11 V8 a4 4 0 0 1 7.5 -2" />
    </svg>
  </span>
);

/** "Prapor Rep +0.1" -> trader portrait + the change, green for gains and red for losses. */
export function RepLine({ text, size = 44 }: { text: string; size?: number }) {
  const m = text.match(/^(.+?)\s+Rep\b\s*(.*)$/i);
  const trader = m?.[1] ?? "";
  const delta = m?.[2] ?? text;
  const neg = /-\s*\d/.test(delta);
  return (
    <>
      {trader ? <TraderAvatar name={trader} size={size} /> : <Disc size={size} bg="#555" title="評価">Rep</Disc>}
      <span>
        {trader} 評価 <b className={neg ? "bad" : "good"}>{delta}</b>
      </span>
    </>
  );
}
