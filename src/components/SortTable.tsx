import { useMemo, useState, type ReactNode } from "react";

export interface Col<T> {
  key: string;
  label: string;
  get: (r: T) => string | number | null;
  render?: (r: T) => ReactNode;
  num?: boolean;
}

export default function SortTable<T>({
  rows,
  cols,
  rowKey,
  defaultSort,
}: {
  rows: T[];
  cols: Col<T>[];
  rowKey: (r: T) => string;
  defaultSort?: { key: string; desc?: boolean };
}) {
  const [sort, setSort] = useState(defaultSort ?? null);

  const sorted = useMemo(() => {
    if (!sort) return rows;
    const col = cols.find((c) => c.key === sort.key);
    if (!col) return rows;
    const dir = sort.desc ? -1 : 1;
    return [...rows].sort((a, b) => {
      const x = col.get(a);
      const y = col.get(b);
      if (x === null && y === null) return 0;
      if (x === null) return 1; // empty values always last
      if (y === null) return -1;
      if (typeof x === "number" && typeof y === "number") return (x - y) * dir;
      return String(x).localeCompare(String(y)) * dir;
    });
  }, [rows, cols, sort]);

  return (
    <div className="table-wrap">
      <table className="grid">
        <thead>
          <tr>
            {cols.map((c) => (
              <th
                key={c.key}
                className={c.num ? "num" : ""}
                onClick={() => setSort(sort?.key === c.key ? { key: c.key, desc: !sort.desc } : { key: c.key, desc: !!c.num })}
              >
                {c.label}
                {sort?.key === c.key ? (sort.desc ? " ▼" : " ▲") : ""}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {sorted.map((r) => (
            <tr key={rowKey(r)}>
              {cols.map((c) => (
                <td key={c.key} className={c.num ? "num" : ""}>
                  {c.render ? c.render(r) : (c.get(r) ?? "–")}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {sorted.length === 0 && <p className="muted pad">該当するデータがありません。</p>}
    </div>
  );
}
