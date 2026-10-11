export type PanelWidth = "sidebarWidth" | "detailWidth";
/** Persist only finite whole pixels accepted by the native u32 contract. */
export function normalizePanelWidth(
  panel: PanelWidth,
  value: number | null | undefined,
): number | null {
  if (value == null || !Number.isFinite(value)) return null;
  const [minimum, maximum] = panel === "sidebarWidth" ? [190, 360] : [340, 750];
  return Math.round(Math.max(minimum!, Math.min(maximum!, value)));
}
