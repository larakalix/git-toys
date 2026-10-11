<script setup lang="ts">
import { onMounted, onBeforeUnmount, shallowRef, watch } from "vue";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
const props = defineProps<{
  original: string;
  modified: string;
  sideBySide: boolean;
  theme: string;
  editable?: boolean;
}>();
const emit = defineEmits<{ change: [value: string] }>();
const host = shallowRef<HTMLElement | null>(null);
let editor: monaco.editor.IStandaloneDiffEditor | undefined,
  originalModel: monaco.editor.ITextModel | undefined,
  modifiedModel: monaco.editor.ITextModel | undefined,
  viewRestore: monaco.IDisposable | undefined;
self.MonacoEnvironment = { getWorker: () => new EditorWorker() };
function editorTheme() {
  const dark = props.theme === "dark";
  const tokens = getComputedStyle(document.documentElement);
  const color = (name: string) => tokens.getPropertyValue(name).trim();
  const name = dark ? "git-toys-dark" : "git-toys-light";
  monaco.editor.defineTheme(name, {
    base: dark ? "vs-dark" : "vs",
    inherit: true,
    rules: [],
    colors: {
      "editor.background": color("--surface"),
      "editor.foreground": color("--text"),
      "editorGutter.background": color("--surface"),
      "editorLineNumber.foreground": color("--muted"),
      "editorLineNumber.activeForeground": color("--text"),
      "editorOverviewRuler.border": color("--border"),
    },
  });
  return name;
}

onMounted(() => {
  if (!host.value) return;
  originalModel = monaco.editor.createModel(props.original, "plaintext");
  modifiedModel = monaco.editor.createModel(props.modified, "plaintext");
  editor = monaco.editor.createDiffEditor(host.value, {
    automaticLayout: true,
    renderSideBySide: props.sideBySide,
    readOnly: !props.editable,
    originalEditable: false,
    minimap: { enabled: false },
    fontFamily: "JetBrains Mono",
    fontSize: 12,
    scrollBeyondLastLine: false,
    theme: editorTheme(),
    accessibilitySupport: "on",
    ariaLabel: "File diff",
    renderIndicators: true,
  });
  editor.setModel({ original: originalModel, modified: modifiedModel });
  modifiedModel.onDidChangeContent(() =>
    emit("change", modifiedModel?.getValue() ?? ""),
  );
});
watch([() => props.original, () => props.modified], () => {
  if (
    originalModel?.getValue() === props.original &&
    modifiedModel?.getValue() === props.modified
  )
    return;
  const view = editor?.saveViewState();
  viewRestore?.dispose();
  if (editor && view) {
    viewRestore = editor.onDidUpdateDiff(() => {
      editor?.restoreViewState(view);
      viewRestore?.dispose();
      viewRestore = undefined;
    });
  }
  if (originalModel?.getValue() !== props.original)
    originalModel?.setValue(props.original);
  if (modifiedModel?.getValue() !== props.modified)
    modifiedModel?.setValue(props.modified);
  if (view) editor?.restoreViewState(view);
});
watch(
  () => props.sideBySide,
  (value) => editor?.updateOptions({ renderSideBySide: value }),
);
watch(
  () => props.theme,
  () => monaco.editor.setTheme(editorTheme()),
);
onBeforeUnmount(() => {
  viewRestore?.dispose();
  editor?.dispose();
  originalModel?.dispose();
  modifiedModel?.dispose();
});
</script>
<template><div ref="host" class="code-editor" /></template>
