import type { Commit } from "./api";
export interface GraphRow {
  lane: number;
  incoming: boolean;
  color: number;
  edges: { from: number; to: number; color: number }[];
  lanes: number;
}
/** Carry unresolved parent lanes across pages by laying out the complete loaded prefix. */
export function layoutGraph(commits: Commit[]): GraphRow[] {
  const lanes: { oid: string; color: number }[] = [];
  let nextColor = 0;
  return commits.map((commit) => {
    let lane = lanes.findIndex((v) => v.oid === commit.oid);
    const incoming = lane >= 0;
    if (lane < 0) {
      lane = lanes.length;
      lanes.push({ oid: commit.oid, color: nextColor++ % graphColors.length });
    }
    const before = [...lanes];
    const color = before[lane]!.color;
    lanes.splice(lane, 1);
    commit.parents.forEach((oid, index) => {
      if (!lanes.some((v) => v.oid === oid))
        lanes.splice(Math.min(lane + index, lanes.length), 0, {
          oid,
          color: index === 0 ? color : nextColor++ % graphColors.length,
        });
    });
    const edges: GraphRow["edges"] = [];
    before.forEach((v, from) => {
      if (from === lane) return;
      const to = lanes.findIndex((p) => p.oid === v.oid);
      if (to >= 0) edges.push({ from, to, color: v.color });
    });
    commit.parents.forEach((oid, index) => {
      const to = lanes.findIndex((v) => v.oid === oid);
      if (to >= 0)
        edges.push({
          from: lane,
          to,
          color: index === 0 ? color : lanes[to]!.color,
        });
    });
    return {
      lane,
      incoming,
      color,
      edges,
      lanes: Math.max(before.length, lanes.length),
    };
  });
}
/** Theme-aware tokens keep graph contrast independent of surface colors. */
export const graphColors = Array.from(
  { length: 12 },
  (_, index) => `var(--graph-${index})`,
);
export function graphEdgePath(
  edge: GraphRow["edges"][number],
  lane: number,
): string {
  const from = edge.from * 22 + 16,
    to = edge.to * 22 + 16;
  const start = edge.from === lane ? 29 : 0;
  if (from === to) return `M ${from} ${start} V 58`;
  return `M ${from} ${start} C ${from} ${start + (58 - start) * 0.65}, ${to} ${start + (58 - start) * 0.35}, ${to} 58`;
}
