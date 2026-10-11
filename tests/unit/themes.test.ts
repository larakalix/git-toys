import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ColorThemePicker from "../../src/components/ColorThemePicker.vue";
import {
  applyColorTheme,
  colorThemes,
  getColorTheme,
} from "../../src/lib/themes";
function luminance(hex: string) {
  const rgb = hex
    .slice(1)
    .match(/../g)!
    .map((channel) => {
      const value = parseInt(channel, 16) / 255;
      return value <= 0.04045
        ? value / 12.92
        : ((value + 0.055) / 1.055) ** 2.4;
    });
  return rgb[0]! * 0.2126 + rgb[1]! * 0.7152 + rgb[2]! * 0.0722;
}
function contrast(a: string, b: string) {
  const values = [luminance(a), luminance(b)].sort((a, b) => b - a);
  return (values[0]! + 0.05) / (values[1]! + 0.05);
}
describe("color themes", () => {
  it("falls back safely for missing or unrecognized saved palettes", () => {
    expect(getColorTheme().id).toBe("teal");
    expect(getColorTheme("removed-palette").id).toBe("teal");
    expect(new Set(colorThemes.map((theme) => theme.id)).size).toBe(
      colorThemes.length,
    );
  });
  it("keeps primary labels and accent text readable in both appearances", () => {
    for (const theme of colorThemes) {
      expect(
        contrast(theme.light, "#ffffff"),
        theme.name,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        contrast(theme.dark, "#141414"),
        theme.name,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        contrast(theme.dark, "#1c1c1c"),
        theme.name,
      ).toBeGreaterThanOrEqual(4.5);
    }
  });
  it("updates and resets palette tokens without touching status colors", () => {
    const root = document.createElement("div");
    root.style.setProperty("--danger", "red");
    applyColorTheme(root, "violet", "dark");
    expect(root.dataset.colorTheme).toBe("violet");
    expect(root.style.getPropertyValue("--accent")).toBe(
      getColorTheme("violet").dark,
    );
    expect(root.style.getPropertyValue("--danger")).toBe("red");
    applyColorTheme(root, undefined, "light");
    expect(root.dataset.colorTheme).toBe("teal");
    expect(root.style.getPropertyValue("--accent-soft")).toBe("#e8f5ee");
  });
  it("exposes one labeled native radio per palette and emits selection", async () => {
    const picker = mount(ColorThemePicker, { props: { modelValue: "teal" } });
    expect(picker.findAll('input[type="radio"]')).toHaveLength(
      colorThemes.length,
    );
    await picker.get('input[value="violet"]').setValue();
    expect(picker.emitted("update:modelValue")).toEqual([["violet"]]);
    await picker.setProps({ modelValue: "violet" });
    expect(picker.get(".selected input").attributes("value")).toBe("violet");
    picker.unmount();
  });
});
