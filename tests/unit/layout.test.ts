import { describe, expect, it } from "vitest";
import { normalizePanelWidth } from "../../src/lib/layout";
describe("panel preferences", () => {
  it("rounds fractional widths and applies the existing limits", () => {
    expect(normalizePanelWidth("detailWidth", 732.17578125)).toBe(732);
    expect(normalizePanelWidth("sidebarWidth", 240.875)).toBe(241);
    expect(normalizePanelWidth("sidebarWidth", -5)).toBe(190);
    expect(normalizePanelWidth("detailWidth", 9999)).toBe(750);
    expect(normalizePanelWidth("detailWidth", 732 + 10)).toBe(742);
  });
  it("does not send non-finite widths to the native integer fields", () => {
    for (const width of [null, undefined, NaN, Infinity, -Infinity])
      expect(normalizePanelWidth("sidebarWidth", width)).toBeNull();
  });
});
