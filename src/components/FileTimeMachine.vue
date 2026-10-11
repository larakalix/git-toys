<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  nextTick,
  onBeforeUnmount,
  shallowRef,
  watch,
} from "vue";
import { useVirtualList } from "@vueuse/core";
import { History, RotateCcw, LoaderCircle } from "lucide-vue-next";
import AppDialog from "./AppDialog.vue";
import { useRepositoryStore } from "../stores/repository";
import { call, asError } from "../lib/api";
import type {
  FileHistoryEntry,
  FileRevision,
  BlameLine,
  Commit,
} from "../lib/api";
const CodeEditor = defineAsyncComponent(() => import("./CodeEditor.vue"));
const props = defineProps<{ path: string; revision?: string; theme: string }>();
const open = defineModel<boolean>("open", { default: false });
const store = useRepositoryStore();
const entries = shallowRef<FileHistoryEntry[]>([]),
  selected = shallowRef<FileHistoryEntry | null>(null),
  file = shallowRef<FileRevision | null>(null),
  annotations = shallowRef<BlameLine[]>([]),
  mode = shallowRef("diff"),
  loading = shallowRef(false),
  fileLoading = shallowRef(false),
  blameLoading = shallowRef(false),
  error = shallowRef(""),
  hasMore = shallowRef(false),
  restoreOpen = shallowRef(false),
  preview = shallowRef<FileRevision | null>(null),
  previewLoading = shallowRef(false);
let historyVersion = 0,
  fileVersion = 0,
  blameVersion = 0,
  restoreVersion = 0,
  anchor = "",
  repositoryId = "",
  restoreExpected = "";
const rows = useVirtualList(entries, { itemHeight: 76, overscan: 5 });
const blameRows = useVirtualList(annotations, { itemHeight: 30, overscan: 12 });
const canRestore = computed(
  () =>
    !!file.value?.exists &&
    !file.value.unsupported &&
    !fileLoading.value &&
    store.trusted &&
    !store.busy,
);
function invalidate() {
  ++historyVersion;
  ++fileVersion;
  ++blameVersion;
  ++restoreVersion;
}
async function history(more = false) {
  const current = ++historyVersion;
  loading.value = true;
  error.value = "";
  try {
    const result = await call<FileHistoryEntry[]>("file_history", {
      id: repositoryId,
      path: props.path,
      revision: anchor,
      skip: more ? entries.value.length : 0,
      limit: 50,
    });
    if (current !== historyVersion || !open.value) return;
    entries.value = more ? [...entries.value, ...result] : result;
    hasMore.value = result.length === 50;
    if (!more && result[0]) await choose(result[0]);
  } catch (e) {
    if (current === historyVersion) {
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === historyVersion) loading.value = false;
  }
}
async function navigateRevision(event: KeyboardEvent, index: number) {
  const last = entries.value.length - 1;
  const destination =
    event.key === "ArrowDown"
      ? Math.min(last, index + 1)
      : event.key === "ArrowUp"
        ? Math.max(0, index - 1)
        : event.key === "Home"
          ? 0
          : event.key === "End"
            ? last
            : -1;
  if (destination < 0) return;
  event.preventDefault();
  rows.scrollTo(destination);
  await nextTick();
  rows.containerProps.ref.value
    ?.querySelector<HTMLButtonElement>(`[data-revision-index="${destination}"]`)
    ?.focus();
  void choose(entries.value[destination]!);
}
async function choose(entry: FileHistoryEntry) {
  const current = ++fileVersion;
  ++blameVersion;
  ++restoreVersion;
  selected.value = entry;
  file.value = null;
  annotations.value = [];
  preview.value = null;
  restoreOpen.value = false;
  mode.value = "diff";
  error.value = "";
  fileLoading.value = true;
  try {
    const result = await call<FileRevision>("file_revision", {
      id: repositoryId,
      path: entry.path,
      revision: entry.commit.oid,
      previousPath: entry.previousPath,
    });
    if (current === fileVersion && open.value) file.value = result;
  } catch (e) {
    if (current === fileVersion) {
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === fileVersion) fileLoading.value = false;
  }
}
async function blame() {
  mode.value = "blame";
  if (
    !file.value ||
    !file.value.exists ||
    file.value.unsupported ||
    annotations.value.length
  )
    return;
  const current = ++blameVersion;
  blameLoading.value = true;
  try {
    const result = await call<BlameLine[]>("file_blame", {
      id: repositoryId,
      path: file.value.path,
      revision: file.value.revision,
    });
    if (current === blameVersion && open.value) annotations.value = result;
  } catch (e) {
    if (current === blameVersion) {
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === blameVersion) blameLoading.value = false;
  }
}
async function showCommit(oid: string) {
  const current = fileVersion;
  try {
    const result = await call<Commit[]>("history", {
      id: repositoryId,
      query: { revision: oid, skip: 0, limit: 1, search: null, path: null },
    });
    if (
      current !== fileVersion ||
      !result[0] ||
      repositoryId !== store.activeId
    )
      return;
    store.selectedCommit = result[0];
    store.view = "history";
    open.value = false;
  } catch (e) {
    store.fail(e);
  }
}
async function restorePreview() {
  if (!file.value) return;
  const current = ++restoreVersion;
  restoreExpected = store.snapshot?.stateToken ?? "";
  previewLoading.value = true;
  try {
    const result = await call<FileRevision>("file_restore_preview", {
      id: repositoryId,
      target: props.path,
      source: file.value.path,
      revision: file.value.revision,
    });
    if (current === restoreVersion && open.value) {
      preview.value = result;
      restoreOpen.value = true;
    }
  } catch (e) {
    if (current === restoreVersion) {
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === restoreVersion) previewLoading.value = false;
  }
}
function restore() {
  if (!preview.value || repositoryId !== store.activeId) return;
  store.mutate(
    {
      kind: "restoreFileVersion",
      target: props.path,
      source: preview.value.path,
      revision: preview.value.revision,
    },
    restoreExpected,
  );
  restoreOpen.value = false;
}
watch([open, () => props.path, () => props.revision], ([value]) => {
  invalidate();
  entries.value = [];
  file.value = null;
  annotations.value = [];
  selected.value = null;
  restoreOpen.value = false;
  if (value) {
    repositoryId = store.activeId;
    anchor = props.revision || store.snapshot?.head || "HEAD";
    history();
  }
});
watch(
  () => store.activeId,
  () => {
    open.value = false;
  },
);
onBeforeUnmount(invalidate);
const date = (value: string) => new Date(value).toLocaleDateString();
const blameDate = (value: string) =>
  new Date(Number(value) * 1000).toLocaleDateString();
</script>
<template>
  <AppDialog
    v-model:open="open"
    title="File time machine"
    :description="props.path"
    content-class="time-machine-dialog"
  >
    <p class="muted time-machine-note">
      Follow the file through commits and renames. Rename detection is
      heuristic; copies and complex merges may break the trail.
    </p>
    <p v-if="error" class="danger" role="alert">
      {{ error }}<button @click="history()">Retry history</button>
    </p>
    <div class="time-machine-grid">
      <section class="file-timeline" aria-label="File commit history">
        <div
          v-bind="rows.containerProps"
          class="timeline-scroll"
          role="list"
          aria-label="File revisions"
        >
          <div v-bind="rows.wrapperProps">
            <div
              v-for="row in rows.list.value"
              :key="row.data.commit.oid"
              role="listitem"
              class="timeline-row"
            >
              <button
                :class="{
                  active: selected?.commit.oid === row.data.commit.oid,
                }"
                :aria-pressed="selected?.commit.oid === row.data.commit.oid"
                :data-revision-index="row.index"
                @keydown="navigateRevision($event, row.index)"
                @click="choose(row.data)"
              >
                <strong>{{ row.data.commit.subject }}</strong
                ><span
                  >{{ row.data.commit.author }} ·
                  {{ date(row.data.commit.date) }}</span
                ><small
                  >{{ row.data.commit.oid.slice(0, 8) }} ·
                  {{ row.data.deleted ? "Deleted" : row.data.path }}</small
                >
              </button>
            </div>
          </div>
        </div>
        <p v-if="!entries.length && !loading" class="panel-note">
          No history at this revision. New untracked files have no committed
          history.
        </p>
        <button v-if="hasMore" :disabled="loading" @click="history(true)">
          {{ loading ? "Loading…" : "Load older commits" }}
        </button>
        <p v-if="loading" role="status">Reading history…</p>
      </section>
      <section class="file-version">
        <header v-if="selected" class="file-version-header">
          <div>
            <strong>{{ selected.commit.subject }}</strong
            ><small
              >{{ selected.path }} ·
              {{ selected.commit.oid.slice(0, 8) }}</small
            >
          </div>
          <div class="button-row">
            <button :class="{ active: mode === 'diff' }" @click="mode = 'diff'">
              Changes</button
            ><button
              :class="{ active: mode === 'blame' }"
              :disabled="!file?.exists || !!file?.unsupported || fileLoading"
              @click="blame()"
            >
              Blame</button
            ><button
              :disabled="!canRestore || previewLoading"
              @click="restorePreview()"
            >
              <RotateCcw :size="14" />{{
                previewLoading ? "Preparing…" : "Restore this version"
              }}
            </button>
          </div>
        </header>
        <div v-if="fileLoading" class="empty-panel" role="status">
          <LoaderCircle :size="20" class="spin" />Loading file version…
        </div>
        <p v-else-if="file?.unsupported" class="panel-note">
          {{ file.unsupported }}
        </p>
        <template v-else-if="file"
          ><p v-if="!file.exists" class="panel-note">
            File is absent at this revision. Select an earlier version to
            restore it.
          </p>
          <CodeEditor
            v-if="mode === 'diff'"
            :key="file.revision + file.path"
            :original="file.original"
            :modified="file.content"
            :side-by-side="false"
            :theme="props.theme"
          />
          <template v-else
            ><p v-if="blameLoading" role="status">Reading blame…</p>
            <div
              v-bind="blameRows.containerProps"
              class="blame-scroll"
              role="list"
              aria-label="Line authors"
            >
              <div v-bind="blameRows.wrapperProps">
                <div
                  v-for="row in blameRows.list.value"
                  :key="row.data.line"
                  class="blame-row"
                  role="listitem"
                >
                  <button
                    :aria-label="`Open commit ${row.data.oid.slice(0, 8)} by ${row.data.author}`"
                    :title="`${row.data.author} · ${blameDate(row.data.timestamp)}`"
                    @click="showCommit(row.data.oid)"
                  >
                    <History :size="12" /><span>{{ row.data.author }}</span
                    ><small>{{ blameDate(row.data.timestamp) }}</small
                    ><small>{{ row.data.oid.slice(0, 8) }}</small></button
                  ><span class="line-number">{{ row.data.line }}</span
                  ><code>{{ row.data.content }}</code>
                </div>
              </div>
            </div></template
          >
        </template>
      </section>
    </div>
    <AppDialog
      v-model:open="restoreOpen"
      title="Restore this file version?"
      :description="`Write ${props.path} as an unstaged change. The index and commits stay unchanged.`"
      content-class="restore-version-dialog"
    >
      <template v-if="preview"
        ><p class="muted">
          Before: current working file. After:
          {{ preview.revision.slice(0, 8) }} · {{ preview.path }}.
        </p>
        <div class="restore-diff">
          <CodeEditor
            :original="preview.original"
            :modified="preview.content"
            :side-by-side="true"
            :theme="props.theme"
          />
        </div>
        <footer class="dialog-actions">
          <button @click="restoreOpen = false">Cancel</button
          ><button
            class="primary"
            :disabled="store.busy || !store.trusted"
            @click="restore()"
          >
            Restore as unstaged change
          </button>
        </footer></template
      >
    </AppDialog>
  </AppDialog>
</template>
<style scoped>
.time-machine-note {
  margin-bottom: 16px;
}
.time-machine-grid {
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  height: min(60vh, 560px);
  border: 1px solid var(--border);
  border-radius: 7px;
  overflow: hidden;
}
.file-timeline {
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.timeline-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.timeline-row {
  height: 76px;
  padding: 3px;
}
.timeline-row button {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  width: 100%;
  height: 70px;
  text-align: left;
  border: 0;
  padding: 9px;
}
.timeline-row strong,
.timeline-row span,
.timeline-row small {
  white-space: nowrap;
  text-overflow: ellipsis;
  overflow: hidden;
  max-width: 100%;
}
.timeline-row span,
.timeline-row small {
  font-size: 10px;
  color: var(--muted);
}
.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.file-version {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}
.file-version-header {
  padding: 12px;
  border-bottom: 1px solid var(--border);
}
.file-version-header small {
  display: block;
  font-size: 10px;
  color: var(--muted);
  overflow-wrap: anywhere;
  margin: 4px 0 10px;
}
.button-row {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.file-version .code-editor {
  flex: 1;
  min-height: 0;
}
.blame-scroll {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.blame-row {
  height: 30px;
  display: flex;
  align-items: center;
  min-width: max-content;
  font:
    11px "JetBrains Mono",
    monospace;
}
.blame-row button {
  display: flex;
  gap: 6px;
  width: 300px;
  height: 30px;
  border: 0;
  border-radius: 0;
  background: var(--sidebar);
  font:
    10px Inter,
    sans-serif;
  color: var(--muted);
}
.blame-row button span {
  width: 105px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.blame-row code {
  white-space: pre;
  padding-right: 20px;
}
.line-number {
  color: var(--muted);
  width: 45px;
  text-align: right;
  padding-right: 12px;
}
.panel-note {
  padding: 16px;
}
.restore-diff {
  height: 350px;
  margin-top: 16px;
}
.restore-diff .code-editor {
  height: 100%;
}
@media (max-width: 800px) {
  .time-machine-grid {
    grid-template-columns: 200px minmax(0, 1fr);
  }
}
</style>
