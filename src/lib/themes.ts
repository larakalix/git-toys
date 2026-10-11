export interface ColorTheme {
  id: string;
  name: string;
  light: string;
  dark: string;
  swatches: readonly string[];
}

// Accent pairs keep text readable in both appearances. Status colors stay semantic.
const palette = (
  id: string,
  name: string,
  light: string,
  dark: string,
  companion: string,
): ColorTheme => ({
  id,
  name,
  light,
  dark,
  swatches: [light, dark, companion, "#f8fafc"],
});
export const colorThemes: readonly ColorTheme[] = [
  palette("teal", "Git Toys · Green", "#147651", "#64dca6", "#c4eadd"),
  palette("blue", "Blue", "#245ac4", "#8ab9ff", "#c7dcff"),
  palette("green", "Green", "#167348", "#69d9a4", "#c0eed5"),
  palette("orange", "Orange", "#a6460d", "#ffb778", "#ffe2c5"),
  palette("mango", "Mango", "#8c5b00", "#f9cd65", "#fff0b9"),
  palette("rose", "Rose", "#b12e55", "#ff9db5", "#ffd7e1"),
  palette("violet", "Violet", "#783cbd", "#c4a3ff", "#e3d4fa"),
  palette("slate", "Slate", "#4d6078", "#adc0da", "#d4dfe9"),
  palette("ocean", "Ocean", "#006f94", "#69d6ee", "#c6edf4"),
  palette("twitch", "Twitch", "#7230c5", "#c8a2ff", "#ddd1ff"),
  palette("cyberpunk", "Cyberpunk", "#4b7000", "#c1f353", "#ed9cff"),
  palette("obsidian", "Obsidian", "#344256", "#aabdd6", "#66768e"),
  palette("amber-minimal", "Amber Minimal", "#925d00", "#f5ca77", "#fff1c8"),
  palette("amethyst-haze", "Amethyst Haze", "#735b98", "#c8b1e5", "#edb1d5"),
  palette("caffeine", "Caffeine", "#795443", "#d9b297", "#ead8bd"),
  palette("graphite", "Graphite", "#535b62", "#b9c4cb", "#dadddf"),
  palette("nature", "Nature", "#306f38", "#9ed6a3", "#cde5cb"),
  palette("retro-arcade", "Retro Arcade", "#a42d79", "#f49dd6", "#39b5ad"),
  palette("solar-dusk", "Solar Dusk", "#a14c19", "#efba88", "#a7bed5"),
  palette("supabase", "Supabase", "#147651", "#64dca6", "#c4eadd"),
  palette("night-kiwi", "Night & Kiwi", "#4a700c", "#b0e86b", "#283323"),
  palette("ghost-persian", "Ghost & Persian", "#392780", "#b9adf5", "#ddd7f4"),
  palette(
    "night-imperial",
    "Night & Imperial",
    "#b32b43",
    "#ff9caa",
    "#243a2e",
  ),
  palette("cyprus-sand", "Cyprus & Sand", "#00635e", "#84d1c7", "#e5d9c7"),
  palette("plum-milk", "Plum & Milk", "#67334f", "#e2afca", "#f4dfcf"),
];
export function getColorTheme(id?: string): ColorTheme {
  return colorThemes.find((theme) => theme.id === id) ?? colorThemes[0]!;
}
const themeFrames = new WeakMap<HTMLElement, number>();

export function applyColorTheme(
  root: HTMLElement,
  id: string | undefined,
  appearance: string,
) {
  const view = root.ownerDocument.defaultView;
  if (root.isConnected && view) {
    const previous = themeFrames.get(root);
    if (previous !== undefined) view.cancelAnimationFrame(previous);
    root.dataset.themeSwitching = "";
  }
  root.dataset.theme = appearance;
  const theme = getColorTheme(id);
  const dark = appearance === "dark";
  root.dataset.colorTheme = theme.id;
  root.style.setProperty("--accent", dark ? theme.dark : theme.light);
  root.style.setProperty(
    "--accent-soft",
    theme.id === "teal"
      ? dark
        ? "#20362c"
        : "#e8f5ee"
      : `color-mix(in srgb, var(--accent) ${dark ? 16 : 10}%, var(--surface))`,
  );
  root.style.setProperty("--graph-0", dark ? theme.dark : theme.light);
  if (root.isConnected && view) {
    // Commit all theme tokens while transitions are suppressed, then restore feedback.
    void root.offsetHeight;
    themeFrames.set(
      root,
      view.requestAnimationFrame(() => {
        delete root.dataset.themeSwitching;
        themeFrames.delete(root);
      }),
    );
  }
}
