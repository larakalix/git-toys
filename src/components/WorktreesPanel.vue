<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import {
  FolderGit2,
  Plus,
  RefreshCw,
  FolderOpen,
  Trash2,
} from "lucide-vue-next";
import { open as directory } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../stores/repository";
import { call } from "../lib/api";
import type { Worktree } from "../lib/api";
import AppDialog from "./AppDialog.vue";
const store = useRepositoryStore();
const trees = shallowRef<Worktree[]>([]),
  loading = shallowRef(false),
  failed = shallowRef(false),
  createOpen = shallowRef(false),
  removeOpen = shallowRef(false),
  chosen = shallowRef<Worktree | null>(null),
  path = shallowRef(""),
  branch = shallowRef(""),
  newBranch = shallowRef(true),
  start = shallowRef("HEAD");
const branches = computed(
  () => store.snapshot?.refs.filter((ref) => ref.kind === "branch") ?? [],
);
let version = 0,
  removalExpected = "";
async function refresh() {
  const current = ++version,
    id = store.activeId;
  loading.value = true;
  failed.value = false;
  try {
    const result = await call<Worktree[]>("worktrees", { id });
    if (current === version && id === store.activeId) trees.value = result;
  } catch (error) {
    if (current === version) {
      failed.value = true;
      store.fail(error);
    }
  } finally {
    if (current === version) loading.value = false;
  }
}
watch(
  () => store.activeId,
  () => {
    trees.value = [];
    createOpen.value = false;
    removeOpen.value = false;
  },
);
watch(() => store.snapshot, refresh, { immediate: true });
function reason(tree: Worktree) {
  if (tree.main) return "Main workspace cannot be removed";
  if (tree.missing)
    return "Folder is missing; use Git worktree repair or prune in the terminal";
  if (tree.locked !== null)
    return `Locked${tree.locked ? ": " + tree.locked : ""}`;
  if (tree.operation) return `${tree.operation} in progress`;
  if (tree.prunable) return "Workspace metadata needs repair";
  if (tree.dirty === null) return "Workspace status could not be read";
  if (tree.dirty)
    return "Local changes or ignored files must be moved or saved first";
  return "";
}
function remove(tree: Worktree) {
  chosen.value = tree;
  removalExpected = store.snapshot?.stateToken ?? "";
  removeOpen.value = true;
}
async function chooseParent() {
  const parent = await directory({
    directory: true,
    title: "Choose parent folder for the new workspace",
  });
  if (typeof parent === "string")
    path.value = `${parent}${parent.includes("\\") ? "\\" : "/"}workspace`;
}
function create() {
  store.mutate({
    kind: "addWorktree",
    path: path.value,
    branch: branch.value,
    newBranch: newBranch.value,
    start: start.value,
  });
  createOpen.value = false;
}
</script>
<template>
  <section class="worktrees-panel wide-panel">
    <header class="panel-heading">
      <div>
        <h1>Parallel workspaces</h1>
        <p>Keep separate tasks checked out in their own folders.</p>
      </div>
      <div class="button-row">
        <button
          :disabled="loading"
          aria-label="Refresh worktrees"
          @click="refresh()"
        >
          <RefreshCw :size="15" /></button
        ><button
          class="primary"
          :disabled="!store.trusted || store.busy"
          @click="createOpen = true"
        >
          <Plus :size="15" />Create workspace
        </button>
      </div>
    </header>
    <p v-if="loading && !trees.length" class="panel-note" role="status">
      Reading workspaces…
    </p>
    <p v-if="failed" role="alert" class="panel-note">
      Could not refresh workspaces. <button @click="refresh()">Retry</button>
    </p>
    <div class="worktree-list">
      <article v-for="tree in trees" :key="tree.path" class="worktree-card">
        <FolderGit2 :size="24" class="accent" />
        <div class="worktree-info">
          <h2>
            {{
              tree.branch === "(detached)"
                ? `Detached · ${tree.head.slice(0, 8)}`
                : tree.branch
            }}<span v-if="tree.current" class="label-badge">Current</span
            ><span v-if="tree.main" class="label-badge">Main workspace</span>
          </h2>
          <p class="worktree-path">{{ tree.path }}</p>
          <p class="muted">
            {{
              tree.dirty === null
                ? "Status unavailable"
                : tree.dirty
                  ? "Has local changes or ignored files"
                  : "Clean"
            }}{{ tree.missing ? " · Missing folder" : ""
            }}{{ tree.locked !== null ? " · Locked" : ""
            }}{{ tree.operation ? " · " + tree.operation : "" }}
          </p>
        </div>
        <div class="worktree-buttons">
          <button
            :disabled="tree.missing || store.busy"
            @click="store.open(tree.path)"
          >
            <FolderOpen :size="15" />Open workspace</button
          ><button
            :disabled="
              !store.trusted ||
              store.busy ||
              failed ||
              loading ||
              !!reason(tree)
            "
            :title="reason(tree) || 'Remove folder and preserve branch'"
            :aria-label="`Remove workspace ${tree.branch}`"
            @click="remove(tree)"
          >
            <Trash2 :size="15" />
          </button>
        </div>
        <p v-if="reason(tree)" class="worktree-reason muted">
          {{ reason(tree) }}
        </p>
      </article>
    </div>
    <AppDialog
      v-model:open="createOpen"
      title="Create a parallel workspace"
      description="Git creates a separate working folder sharing this repository’s history. Your current branch stays checked out."
    >
      <form @submit.prevent="create()">
        <label class="field"
          >New workspace folder<input
            v-model="path"
            required
            placeholder="Absolute path to a folder that does not exist" /></label
        ><button type="button" @click="chooseParent()">
          Choose parent folder…
        </button>
        <p class="muted">
          The parent folder must exist. The new workspace folder must not exist.
        </p>
        <label class="check-field"
          ><input
            v-model="newBranch"
            type="checkbox"
            @change="branch = ''"
          />Create a new branch</label
        >
        <label class="field"
          >Workspace branch<input
            v-if="newBranch"
            v-model="branch"
            required
            placeholder="feature/my-task"
          /><select v-else v-model="branch" required>
            <option value="" disabled>Choose a branch</option>
            <option
              v-for="ref in branches"
              :key="ref.name"
              :value="ref.name"
              :disabled="trees.some((tree) => tree.branch === ref.name)"
            >
              {{ ref.name }}
            </option>
          </select></label
        >
        <label v-if="newBranch" class="field"
          >Starting ref<input v-model="start" required placeholder="HEAD"
        /></label>
        <footer class="dialog-actions">
          <button type="button" @click="createOpen = false">Cancel</button
          ><button
            class="primary"
            :disabled="
              !path.trim() || !branch.trim() || store.busy || !store.trusted
            "
          >
            Create workspace
          </button>
        </footer>
      </form>
    </AppDialog>
    <AppDialog
      v-model:open="removeOpen"
      title="Remove this workspace?"
      description="The clean workspace folder will be removed. Its branch and commits remain in the repository."
    >
      <template v-if="chosen"
        ><p>
          <strong>{{ chosen.branch }}</strong>
        </p>
        <code class="worktree-path">{{ chosen.path }}</code>
        <p class="muted">
          Git Toys will recheck local changes, locks, and active operations.
          Open tabs for this folder close only after successful removal.
        </p>
        <footer class="dialog-actions">
          <button @click="removeOpen = false">Cancel</button
          ><button
            class="danger"
            :disabled="store.busy || !store.trusted"
            @click="
              store.mutate(
                { kind: 'removeWorktree', path: chosen.path },
                removalExpected,
              );
              removeOpen = false;
            "
          >
            Remove workspace
          </button>
        </footer></template
      >
    </AppDialog>
  </section>
</template>
<style scoped>
.worktrees-panel {
  overflow: auto;
  background: var(--surface);
}
.button-row,
.worktree-buttons {
  display: flex;
  gap: 8px;
}
.worktree-list {
  display: grid;
  gap: 12px;
  padding: 24px;
}
.worktree-card {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  align-items: center;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 18px;
}
.worktree-info {
  flex: 1;
  min-width: 200px;
}
h2 {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  font-weight: 600;
}
.worktree-path {
  overflow-wrap: anywhere;
  font:
    11px/1.7 "JetBrains Mono",
    monospace;
  margin: 6px 0;
}
.worktree-reason {
  flex-basis: 100%;
  font-size: 11px;
}
.accent {
  color: var(--accent);
}
.panel-note {
  padding: 24px;
}
form > p,
.dialog p {
  margin: 12px 0;
}
</style>
