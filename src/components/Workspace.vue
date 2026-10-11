<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import {
  GitBranch,
  ArrowDown,
  ArrowUp,
  RefreshCw,
  ChevronDown,
  GitMerge,
  LoaderCircle,
} from "lucide-vue-next";
import { useRepositoryStore } from "../stores/repository";
import RepositorySidebar from "./RepositorySidebar.vue";
import HistoryPanel from "./HistoryPanel.vue";
import CommitDetails from "./CommitDetails.vue";
import WorkingChanges from "./WorkingChanges.vue";
import DiffPanel from "./DiffPanel.vue";
import FileTimeMachine from "./FileTimeMachine.vue";
import MergePreviewDialog from "./history-tools/MergePreviewDialog.vue";
import InteractiveRebaseDialog from "./history-tools/InteractiveRebaseDialog.vue";
import PullRequestsPanel from "./github/PullRequestsPanel.vue";
import WorktreesPanel from "./WorktreesPanel.vue";
import RecoveryPanel from "./RecoveryPanel.vue";
import { normalizePanelWidth } from "../lib/layout";
import { actionDefinitions } from "../lib/actions";
const props = defineProps<{ theme: string }>(),
  store = useRepositoryStore();
const emit = defineEmits<{
  action: [kind: string, defaults?: Record<string, unknown>];
  trust: [];
  conflict: [path: string];
}>();
const previewOpen = shallowRef(false),
  previewKind = shallowRef<"merge" | "rebase">("merge"),
  rebaseOpen = shallowRef(false);
const timeMachineOpen = shallowRef(false),
  historyPath = shallowRef(""),
  historyRevision = shallowRef("");
function fileHistory(path: string, revision = "") {
  historyPath.value = path;
  historyRevision.value = revision;
  timeMachineOpen.value = true;
}
const file = shallowRef(""),
  staged = shallowRef(false);
watch(
  () => store.activeId,
  () => {
    file.value = "";
    staged.value = false;
  },
);
const width = computed(
  () =>
    `${store.preferences.sidebarWidth ?? 240}px 5px minmax(300px,1fr) 5px minmax(340px,${store.preferences.detailWidth ?? 440}px)`,
);
const disabled = computed(() => !store.trusted || store.busy);
function select(path: string, isStaged: boolean) {
  file.value = path;
  staged.value = isStaged;
}
function adjust(which: "sidebarWidth" | "detailWidth", delta: number) {
  const current =
    store.preferences[which] ?? (which === "sidebarWidth" ? 240 : 440);
  store.preferences[which] = normalizePanelWidth(which, current + delta);
}
function resize(which: "sidebarWidth" | "detailWidth", event: PointerEvent) {
  const start = event.clientX;
  const current =
    store.preferences[which] ?? (which === "sidebarWidth" ? 240 : 440);
  const move = (e: PointerEvent) => {
    store.preferences[which] = current;
    adjust(which, (e.clientX - start) * (which === "detailWidth" ? -1 : 1));
  };
  const stop = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", stop);
    store.save();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", stop, { once: true });
}
function chooseAction(event: Event) {
  const el = event.target as HTMLSelectElement;
  if (el.value === "previewMerge" || el.value === "previewRebase") {
    previewKind.value = el.value === "previewMerge" ? "merge" : "rebase";
    previewOpen.value = true;
  } else if (el.value === "interactiveRebase") rebaseOpen.value = true;
  else if (el.value) emit("action", el.value);
  el.value = "";
}
</script>
<template>
  <div class="workspace">
    <header class="repo-toolbar">
      <div class="branch-indicator">
        <GitBranch :size="17" /><strong>{{
          store.snapshot?.branch === "(detached)"
            ? "Detached HEAD"
            : store.snapshot?.branch
        }}</strong
        ><ChevronDown :size="13" /><span class="toolbar-divider" /><span
          class="muted"
          >{{ store.snapshot?.upstream || "No upstream" }}</span
        ><span class="sync-count" title="Ahead / behind"
          >↑ {{ store.snapshot?.ahead }} ↓ {{ store.snapshot?.behind }}</span
        >
      </div>
      <div class="toolbar-actions">
        <button
          :disabled="disabled || !store.snapshot?.remotes.length"
          @click="emit('action', 'fetch')"
        >
          <RefreshCw :size="15" />Fetch</button
        ><button
          :disabled="disabled || !store.snapshot?.remotes.length"
          @click="emit('action', 'pull')"
        >
          <ArrowDown :size="15" />Pull</button
        ><button
          class="primary push-button"
          :aria-busy="store.busy && store.runningKind === 'push'"
          :disabled="disabled || !store.snapshot?.remotes.length"
          @click="emit('action', 'push')"
        >
          <LoaderCircle
            v-if="store.busy && store.runningKind === 'push'"
            :size="15"
            class="spin"
            aria-hidden="true"
          />
          <ArrowUp v-else :size="15" aria-hidden="true" />{{
            store.busy && store.runningKind === "push" ? "Pushing…" : "Push"
          }}</button
        ><select
          aria-label="Repository actions"
          :disabled="disabled"
          @change="chooseAction"
        >
          <option value="">More actions…</option>
          <option value="previewMerge">Preview merge</option>
          <option value="previewRebase">Preview rebase</option>
          <option value="interactiveRebase">Edit branch history</option>
          <option
            v-for="action in actionDefinitions.filter(
              (a) => !['discard', 'commit'].includes(a.kind),
            )"
            :key="action.kind"
            :value="action.kind"
          >
            {{ action.title }}
          </option>
        </select>
      </div>
    </header>
    <div
      v-if="store.busy"
      class="git-progress"
      role="status"
      aria-live="polite"
    >
      <LoaderCircle :size="16" class="spin" aria-hidden="true" />
      <strong>{{
        store.runningKind === "push"
          ? "Pushing to remote…"
          : `${store.runningKind || "Git operation"} in progress…`
      }}</strong>
      <span>Git is working. This view will refresh when it finishes.</span>
      <div class="git-progress-track" aria-hidden="true"><span /></div>
    </div>
    <div
      v-if="store.snapshot?.operation"
      class="operation-banner"
      role="status"
    >
      <GitMerge :size="17" /><strong
        >{{ store.snapshot.operation }} in progress</strong
      ><span>Resolve conflicts, stage changes, then continue.</span
      ><button :disabled="disabled" @click="emit('action', 'continue')">
        Continue…</button
      ><button :disabled="disabled" @click="emit('action', 'abort')">
        Abort…
      </button>
    </div>
    <div
      v-for="warning in store.snapshot?.warnings"
      :key="warning"
      class="repo-warning"
    >
      {{ warning }}
    </div>
    <div class="workspace-grid" :style="{ gridTemplateColumns: width }">
      <RepositorySidebar
        @action="(kind, defaults) => emit('action', kind, defaults)"
        @trust="emit('trust')"
      />
      <div
        class="resize-handle"
        role="separator"
        tabindex="0"
        aria-label="Resize navigation"
        aria-orientation="vertical"
        :aria-valuenow="store.preferences.sidebarWidth ?? 240"
        @pointerdown="resize('sidebarWidth', $event)"
        @keydown.left.prevent="
          adjust('sidebarWidth', -10);
          store.save();
        "
        @keydown.right.prevent="
          adjust('sidebarWidth', 10);
          store.save();
        "
      />
      <template v-if="store.view === 'history'"
        ><HistoryPanel />
        <div
          class="resize-handle"
          role="separator"
          tabindex="0"
          aria-label="Resize details"
          aria-orientation="vertical"
          :aria-valuenow="store.preferences.detailWidth ?? 440"
          @pointerdown="resize('detailWidth', $event)"
          @keydown.left.prevent="
            adjust('detailWidth', 10);
            store.save();
          "
          @keydown.right.prevent="
            adjust('detailWidth', -10);
            store.save();
          " />
        <CommitDetails
          @file-history="fileHistory"
          :theme="props.theme"
          @action="
            (kind, defaults) => emit('action', kind, defaults)
          " /></template
      ><template v-else-if="store.view === 'changes'"
        ><WorkingChanges
          @file-history="fileHistory"
          @select="select"
          @action="(kind, defaults) => emit('action', kind, defaults)"
          @conflict="emit('conflict', $event)" />
        <div
          class="resize-handle"
          role="separator"
          tabindex="0"
          aria-label="Resize diff"
          aria-orientation="vertical"
          :aria-valuenow="store.preferences.detailWidth ?? 440"
          @pointerdown="resize('detailWidth', $event)"
          @keydown.left.prevent="
            adjust('detailWidth', 10);
            store.save();
          "
          @keydown.right.prevent="
            adjust('detailWidth', -10);
            store.save();
          " />
        <DiffPanel
          :key="store.activeId"
          :path="file"
          :staged="staged"
          :theme="props.theme"
          @action="
            (kind, defaults) => emit('action', kind, defaults)
          " /></template
      ><PullRequestsPanel
        v-else-if="store.view === 'pullRequests'"
      /><WorktreesPanel v-else-if="store.view === 'worktrees'" /><RecoveryPanel
        v-else
        class="wide-panel"
        @action="(kind, defaults) => emit('action', kind, defaults)"
      />
    </div>
    <MergePreviewDialog v-model:open="previewOpen" :kind="previewKind" />
    <InteractiveRebaseDialog v-model:open="rebaseOpen" />
    <FileTimeMachine
      v-model:open="timeMachineOpen"
      :path="historyPath"
      :revision="historyRevision || undefined"
      :theme="props.theme"
    />
  </div>
</template>
