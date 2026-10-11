<script setup lang="ts">
import {
  defineAsyncComponent,
  shallowRef,
  watch,
  computed,
  onBeforeUnmount,
} from "vue";
import {
  Columns2,
  AlignLeft,
  FileCode,
  Plus,
  Minus,
  Trash2,
  RefreshCw,
} from "lucide-vue-next";
import { call } from "../lib/api";
import type { Diff } from "../lib/api";
import { useRepositoryStore } from "../stores/repository";
const CodeEditor = defineAsyncComponent(() => import("./CodeEditor.vue"));
const props = defineProps<{
  path: string;
  staged?: boolean;
  commit?: string;
  base?: string;
  theme: string;
}>();
const emit = defineEmits<{
  action: [kind: string, defaults?: Record<string, unknown>];
}>();
const store = useRepositoryStore(),
  diff = shallowRef<Diff | null>(null),
  sideBySide = shallowRef(false),
  loading = shallowRef(false),
  failed = shallowRef(false),
  loadedToken = shallowRef<string | undefined>();
let version = 0;
async function refreshDiff(clear = false) {
  const current = ++version;
  if (clear) {
    diff.value = null;
    loadedToken.value = undefined;
  }
  failed.value = false;
  if (!props.path) {
    loading.value = false;
    return;
  }
  const token = store.snapshot?.stateToken;
  loading.value = true;
  try {
    const result = await call<Diff>("diff", {
      id: store.activeId,
      request: {
        path: props.path,
        staged: props.staged ?? false,
        commit: props.commit ?? null,
        base: props.base ?? null,
      },
    });
    if (current === version) {
      diff.value = result;
      loadedToken.value = token;
    }
  } catch (e) {
    if (current === version) {
      failed.value = true;
      store.fail(e);
    }
  } finally {
    if (current === version) loading.value = false;
  }
}
watch(
  [
    () => props.path,
    () => props.staged ?? false,
    () => props.commit ?? null,
    () => props.base ?? null,
    () => store.activeId,
    () => store.snapshot?.stateToken,
  ],
  (next, previous) =>
    refreshDiff(
      next.slice(0, 5).some((value, index) => value !== previous[index]),
    ),
  { immediate: true },
);
onBeforeUnmount(() => {
  ++version;
});
const actionsDisabled = computed(
  () =>
    !store.trusted ||
    store.busy ||
    loading.value ||
    failed.value ||
    !diff.value ||
    loadedToken.value !== store.snapshot?.stateToken,
);
const hunkHeaders = computed(
  () =>
    diff.value?.hunks.map((h, i) => ({
      index: i,
      label: h.split("\n").find((l) => l.startsWith("@@")) ?? `Hunk ${i + 1}`,
    })) ?? [],
);
</script>
<template>
  <section class="diff-panel">
    <header class="diff-heading">
      <div>
        <FileCode :size="16" /><strong>{{ path || "File diff" }}</strong
        ><span v-if="path" class="label-badge">{{
          commit ? "Commit" : staged ? "Staged" : "Working tree"
        }}</span>
      </div>
      <div>
        <span v-if="loading && diff" class="diff-refresh-state" role="status"
          >Refreshing…</span
        >
        <span v-if="failed" class="danger-text" role="status"
          >Diff unavailable</span
        >
        <button
          v-if="failed"
          class="icon-button"
          aria-label="Retry diff"
          :disabled="loading"
          @click="refreshDiff()"
        >
          <RefreshCw :size="15" />
        </button>
        <button
          class="icon-button"
          :class="{ active: !sideBySide }"
          aria-label="Inline diff"
          @click="sideBySide = false"
        >
          <AlignLeft :size="16" /></button
        ><button
          class="icon-button"
          :class="{ active: sideBySide }"
          aria-label="Side-by-side diff"
          @click="sideBySide = true"
        >
          <Columns2 :size="16" />
        </button>
      </div>
    </header>
    <div v-if="!path" class="empty-panel">
      <FileCode :size="32" />
      <h2>A closer look</h2>
      <p>Select a changed file to review its diff.</p>
    </div>
    <div v-else-if="loading && !diff" class="empty-panel" role="status">
      Loading diff…
    </div>
    <div v-else-if="diff?.binary || diff?.truncated" class="empty-panel">
      <h2>{{ diff.binary ? "Binary file" : "Large file" }}</h2>
      <p>
        {{
          diff.binary
            ? "A text diff is not available."
            : "Content exceeds the 2 MB editor limit. Review it in your external editor."
        }}
      </p>
    </div>
    <CodeEditor
      v-else-if="diff"
      :original="diff.original"
      :modified="diff.modified"
      :side-by-side="sideBySide"
      :theme="theme"
    />
    <div v-else-if="failed" class="empty-panel">
      <h2>Unable to load this diff</h2>
      <p>Retry using the refresh button above.</p>
    </div>
    <footer v-if="path && !commit" class="diff-actions">
      <button
        :disabled="actionsDisabled"
        @click="
          store.mutate({ kind: staged ? 'unstage' : 'stage', paths: [path] })
        "
      >
        <Minus v-if="staged" :size="15" /><Plus v-else :size="15" />{{
          staged ? "Unstage file" : "Stage file"
        }}</button
      ><button
        v-if="!staged"
        class="text-button danger-text"
        :disabled="actionsDisabled"
        @click="emit('action', 'discard', { paths: [path] })"
      >
        <Trash2 :size="14" />Discard…
      </button>
      <details v-if="hunkHeaders.length" class="hunk-picker">
        <summary>{{ hunkHeaders.length }} hunks</summary>
        <button
          v-for="h in hunkHeaders"
          :key="h.index"
          :disabled="actionsDisabled"
          @click="
            store.mutate({
              kind: 'stageHunk',
              path,
              hunk: h.index,
              reverse: staged ?? false,
            })
          "
        >
          {{ staged ? "Unstage" : "Stage" }} {{ h.label }}
        </button>
      </details>
    </footer>
  </section>
</template>
