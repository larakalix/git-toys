<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import AppDialog from "../AppDialog.vue";
import SelectionLines from "./SelectionLines.vue";
import {
  call,
  asError,
  type SelectionDiff,
  type CommitGroup,
} from "../../lib/api";
import { useRepositoryStore } from "../../stores/repository";
const open = defineModel<boolean>("open", { default: false });
const emit = defineEmits<{ staged: [name: string] }>();
const store = useRepositoryStore();
const groupId = ref(""),
  path = ref(""),
  selected = ref<number[]>([]);
const content = shallowRef<SelectionDiff>(),
  error = ref(""),
  loading = ref(false),
  pending = ref("");
let version = 0;
const groups = computed(() =>
  store.preferences.commitGroups.filter(
    (g) => g.repository === store.snapshot?.path,
  ),
);
const group = computed(() => groups.value.find((g) => g.id === groupId.value));
const files = computed(
  () => store.snapshot?.files.filter((f) => f.worktree !== ".") ?? [],
);
const indexOccupied = computed(() =>
  store.snapshot?.files.some((f) => !f.untracked && f.index !== "."),
);
async function load() {
  const current = ++version;
  content.value = undefined;
  selected.value = [];
  error.value = "";
  if (!open.value || !path.value) return;
  loading.value = true;
  try {
    const result = await call<SelectionDiff>("selection_diff", {
      id: store.activeId,
      path: path.value,
    });
    if (current !== version) return;
    content.value = result;
    const saved = group.value?.files.find((f) => f.path === path.value);
    if (saved?.token === result.token) selected.value = [...saved.lines];
    else if (saved)
      error.value =
        "This file changed. Select its lines again and replace the saved selection.";
  } catch (e) {
    if (current === version) error.value = asError(e).message;
  } finally {
    if (current === version) loading.value = false;
  }
}
function create() {
  const item: CommitGroup = {
    id: crypto.randomUUID(),
    repository: store.snapshot!.path,
    name: `Commit ${groups.value.length + 1}`,
    files: [],
  };
  store.preferences.commitGroups.push(item);
  groupId.value = item.id;
  store.save();
}
function assign() {
  if (!group.value || !content.value) return;
  // A changed line belongs to one group. Assigning it moves it from other groups.
  for (const g of groups.value) {
    g.files = g.files
      .map((f) =>
        f.path === path.value && f.token === content.value!.token
          ? { ...f, lines: f.lines.filter((n) => !selected.value.includes(n)) }
          : f,
      )
      .filter((f) => f.lines.length);
  }
  group.value.files = group.value.files.filter((f) => f.path !== path.value);
  if (selected.value.length)
    group.value.files.push({
      path: path.value,
      token: content.value.token,
      lines: [...selected.value],
    });
  store.save();
}
function move(delta: number) {
  const all = store.preferences.commitGroups,
    i = all.findIndex((g) => g.id === groupId.value),
    j = i + delta;
  if (j >= 0 && j < all.length && all[j]?.repository === store.snapshot?.path) {
    [all[i], all[j]] = [all[j]!, all[i]!];
    store.save();
  }
}
function remove() {
  store.preferences.commitGroups = store.preferences.commitGroups.filter(
    (g) => g.id !== groupId.value,
  );
  groupId.value = groups.value[0]?.id ?? "";
  store.save();
}
async function stage() {
  if (!group.value) return;
  pending.value = group.value.id;
  await store.mutate({ kind: "stageGroup", files: group.value.files });
  if (!store.busy) pending.value = "";
}
watch([open, () => store.activeId], () => {
  pending.value = "";
  groupId.value = groups.value[0]?.id ?? "";
  path.value = "";
  ++version;
  content.value = undefined;
  if (open.value && !groups.value.length) create();
});
watch([path, groupId, () => store.snapshot?.stateToken], load);
watch(
  () => store.operations,
  (ops) => {
    const operation = ops.at(-1);
    if (
      !pending.value ||
      operation?.kind !== "stageGroup" ||
      operation.repoId !== store.activeId ||
      operation.status === "running"
    )
      return;
    if (operation.status === "completed") {
      emit("staged", group.value?.name ?? "");
      remove();
      open.value = false;
    }
    pending.value = "";
  },
);
</script>
<template>
  <AppDialog
    v-model:open="open"
    title="Organize changes"
    description="Select changed lines and collect them into focused commits. Groups leave Git unchanged until staged."
    content-class="feature-dialog"
  >
    <div class="feature-toolbar">
      <label
        >Commit group<select v-model="groupId">
          <option v-for="g in groups" :key="g.id" :value="g.id">
            {{ g.name }}
          </option>
        </select></label
      >
      <button class="button" @click="create">New group</button
      ><button class="button" @click="move(-1)">Move up</button
      ><button class="button" @click="move(1)">Move down</button
      ><button class="button" :disabled="!group" @click="remove">
        Delete group
      </button>
    </div>
    <label v-if="group" class="field"
      >Group name<input
        v-model="group.name"
        maxlength="200"
        @change="store.save()"
    /></label>
    <div class="feature-split">
      <div class="feature-list">
        <button
          v-for="f in files"
          :key="f.path"
          class="button"
          :aria-pressed="path === f.path"
          @click="path = f.path"
        >
          {{ f.path }}
        </button>
        <p v-if="!files.length">No unstaged changes.</p>
      </div>
      <div>
        <p v-if="loading" role="status">Loading changes…</p>
        <p v-if="error" role="alert">{{ error }}</p>
        <SelectionLines
          v-if="content"
          v-model="selected"
          :patch="content.diff.patch"
          :disabled="loading || store.busy"
        /><button
          v-if="content"
          class="button"
          :disabled="!group || loading || store.busy"
          @click="assign"
        >
          Assign {{ selected.length }}
          {{ selected.length === 1 ? "line" : "lines" }} to group
        </button>
      </div>
    </div>
    <p v-if="indexOccupied" class="muted">
      Commit or unstage existing staged changes before staging a group.
    </p>
    <p class="muted">
      {{ group?.files.reduce((count, f) => count + f.lines.length, 0) ?? 0 }}
      saved lines. Changed files require selection again. Unsupported changes
      use whole-file staging.
    </p>
    <footer>
      <button
        class="button primary"
        :disabled="
          !group?.files.length ||
          indexOccupied ||
          !store.trusted ||
          store.busy ||
          loading
        "
        @click="stage"
      >
        {{ pending ? "Staging…" : "Stage group and write commit" }}
      </button>
    </footer>
  </AppDialog>
</template>
