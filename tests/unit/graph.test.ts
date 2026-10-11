import { describe, it, expect } from "vitest";
import { layoutGraph, graphColors } from "../../src/lib/graph";
import type { Commit } from "../../src/lib/api";
const commit = (oid: string, parents: string[]): Commit => ({
  oid,
  parents,
  author: "Test",
  email: "test@example.invalid",
  date: "2026-01-01",
  subject: oid,
  message: oid,
  refs: "",
});
describe("history graph", () => {
  it("keeps lanes stable when another history page is appended", () => {
    const first = [
      commit("merge", ["main", "feature"]),
      commit("main", ["base"]),
    ];
    const before = layoutGraph(first);
    const after = layoutGraph([
      ...first,
      commit("feature", ["base"]),
      commit("base", []),
    ]);
    expect(after.slice(0, 2)).toEqual(before);
    expect(after[2]!.lane).toBe(1);
  });
  it("joins both parent lanes at a shared ancestor", () => {
    const rows = layoutGraph([
      commit("merge", ["a", "b"]),
      commit("a", ["root"]),
      commit("b", ["root"]),
      commit("root", []),
    ]);
    expect(rows[2]!.edges).toContainEqual({ from: 1, to: 0, color: 1 });
    expect(rows[3]!.lane).toBe(0);
  });
  it("uses parent colors at a split and never draws an incoming edge for a new head", () => {
    const rows = layoutGraph([
      commit("merge", ["main", "feature"]),
      commit("main", ["base"]),
      commit("feature", ["base"]),
    ]);
    expect(rows[0]!.incoming).toBe(false);
    expect(rows[1]!.incoming).toBe(true);
    expect(rows[0]!.edges).toContainEqual({
      from: 0,
      to: 1,
      color: rows[2]!.color,
    });
  });
  it("allocates all twelve colors before reusing the palette", () => {
    const rows = layoutGraph(
      Array.from({ length: 13 }, (_, index) => commit(`head-${index}`, [])),
    );
    expect(new Set(rows.slice(0, 12).map((row) => row.color)).size).toBe(12);
    expect(rows[12]!.color).toBe(0);
    expect(graphColors).toHaveLength(12);
  });
});
