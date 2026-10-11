<script setup lang="ts">
import { computed, shallowRef, watch, ref, nextTick } from "vue";
import { useVirtualList } from "@vueuse/core";
import {
  Plus,
  Minus,
  File,
  GitCommitHorizontal,
  Check,
  Archive,
  Sparkles,
  History,
} from "lucide-vue-next";
import { useRepositoryStore } from "../stores/repository";
import type { FileChange } from "../lib/api";
import CommitOrganizer from "./organizer/CommitOrganizer.vue";
const organizerOpen = shallowRef(false),
  composer = ref<HTMLTextAreaElement>();
async function groupStaged(name: string) {
  if (!message.value.trim()) message.value = name;
  await nextTick();
  composer.value?.focus();
}
import AiCommitDialog from "./AiCommitDialog.vue";
const aiOpen = shallowRef(false);
const store = useRepositoryStore(),
  message = shallowRef("");
const emit = defineEmits<{
  select: [path: string, staged: boolean];
  action: [kind: string, defaults?: Record<string, unknown>];
  conflict: [path: string];
  fileHistory: [path: string];
}>();
const items = computed(() => {
  const files = store.snapshot?.files ?? [];
  return [
    ...files
      .filter((f) => f.index !== "." && !f.untracked && !f.conflict)
      .map((file) => ({ file, staged: true })),
    ...files
      .filter((f) => f.worktree !== "." || f.untracked || f.conflict)
      .map((file) => ({ file, staged: false })),
  ];
});
const staged = computed(() => items.value.filter((v) => v.staged).length);
const { list, containerProps, wrapperProps } = useVirtualList(items, {
  itemHeight: 44,
  overscan: 12,
});
function toggle(file: FileChange, staged: boolean) {
  store.mutate({ kind: staged ? "unstage" : "stage", paths: [file.path] });
}
watch(
  () => store.activeId,
  () => (message.value = ""),
);
watch(
  () => store.operations,
  (ops) => {
    const last = ops.at(-1);
    if (last?.kind === "commit" && last.status === "completed")
      message.value = "";
  },
);
</script>
<template>
  <CommitOrganizer v-model:open="organizerOpen" @staged="groupStaged" />
  <section class="changes-panel">
    <header class="panel-heading">
      <div>
        <h1>Working changes</h1>
        <p>Make every commit intentional.</p>
      </div>
      <button
        class="icon-button"
        aria-label="Stash changes"
        :disabled="!store.trusted || store.busy"
        @click="emit('action', 'stash')"
      >
        <Archive :size="18" />
      </button>
    </header>
    <div class="changes-summary">
      <button class="text-button" @click="organizerOpen = true">
        Organize changes
      </button>
      <span
        ><i class="status-dot" />{{ store.snapshot?.files.length }} changed
        files</span
      ><button
        class="text-button"
        :disabled="!store.trusted || store.busy || !items.length"
        @click="
          store.mutate({
            kind: 'stage',
            paths: store
              .snapshot!.files.filter((f) => !f.conflict)
              .map((f) => f.path),
          })
        "
      >
        Stage all
      </button>
    </div>
    <div
      v-bind="containerProps"
      class="files-scroll"
      role="list"
      aria-label="Changed files"
    >
      <div v-bind="wrapperProps">
        <div
          v-for="row in list"
          :key="`${row.data.staged}-${row.data.file.path}`"
          class="file-row"
          role="listitem"
        >
          <button
            class="file-select"
            @click="
              row.data.file.conflict
                ? emit('conflict', row.data.file.path)
                : emit('select', row.data.file.path, row.data.staged)
            "
          >
            <span
              class="file-status"
              :class="{ conflicted: row.data.file.conflict }"
              >{{
                row.data.file.conflict
                  ? "!"
                  : row.data.staged
                    ? row.data.file.index
                    : row.data.file.worktree === "?"
                      ? "U"
                      : row.data.file.worktree
              }}</span
            ><File :size="15" /><span
              >{{ row.data.file.path
              }}<small v-if="row.data.file.originalPath"
                >from {{ row.data.file.originalPath }}</small
              ></span
            ><span class="file-location">{{
              row.data.file.conflict
                ? "Conflict"
                : row.data.staged
                  ? "Staged"
                  : "Unstaged"
            }}</span></button
          ><button
            class="icon-button"
            :aria-label="`View file history for ${row.data.file.path}`"
            :disabled="row.data.file.untracked"
            @click="emit('fileHistory', row.data.file.path)"
          >
            <History :size="14" /></button
          ><button
            class="icon-button"
            :aria-label="`${row.data.staged ? 'Unstage' : 'Stage'} ${row.data.file.path}`"
            :disabled="!store.trusted || store.busy || row.data.file.conflict"
            @click="toggle(row.data.file, row.data.staged)"
          >
            <Minus v-if="row.data.staged" :size="15" /><Plus
              v-else
              :size="15"
            />
          </button>
        </div>
      </div>
      <div v-if="!items.length" class="empty-panel">
        <Check :size="30" />
        <h2>Working tree is clean</h2>
        <p>Nothing to commit. A good place to start.</p>
      </div>
    </div>
    <form
      class="commit-composer"
      @submit.prevent="store.mutate({ kind: 'commit', message, amend: false })"
    >
      <div class="section-heading">
        <h2><GitCommitHorizontal :size="17" />Commit</h2>
        <span>{{ staged }} staged</span>
      </div>
      <button
        type="button"
        class="ai-generate"
        :disabled="!staged || store.busy"
        @click="aiOpen = true"
      >
        <Sparkles :size="15" />Generate with AI
      </button>
      <label class="sr-only" for="commit-message">Commit message</label
      ><textarea
        ref="composer"
        id="commit-message"
        v-model="message"
        placeholder="What changed, and why?"
        rows="4"
        required
        :disabled="store.busy"
      />
      <div class="composer-footer">
        <button
          type="button"
          class="text-button"
          :disabled="!store.trusted || store.busy"
          @click="emit('action', 'commit', { message, amend: true })"
        >
          Amend last commit…</button
        ><button
          class="primary"
          :disabled="!store.trusted || store.busy || !staged || !message.trim()"
        >
          <Check :size="16" />Commit staged
        </button>
      </div>
    </form>
    <AiCommitDialog
      v-model:open="aiOpen"
      :existing-message="message"
      @use="message = $event"
    />
  </section>
</template>
