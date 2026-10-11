export function selectableLines(patch: string) {
  let hunk = false;
  return patch.split("\n").map((text, index) => {
    if (text.startsWith("@@ ")) hunk = true;
    return {
      text,
      index,
      selectable: hunk && (text.startsWith("+") || text.startsWith("-")),
    };
  });
}
