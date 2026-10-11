<script setup lang="ts">
import { computed, shallowRef } from "vue";
import { Search, GitBranch, FolderOpen } from "lucide-vue-next";
import AppDialog from "./AppDialog.vue";
import { actionDefinitions } from "../lib/actions";
import { useRepositoryStore } from "../stores/repository";
const open = defineModel<boolean>("open", { default: false }),
  query = shallowRef(""),
  store = useRepositoryStore();
const emit = defineEmits<{ action: [kind: string]; open: [] }>();
const actions = computed(() =>
  actionDefinitions.filter((a) =>
    a.title.toLowerCase().includes(query.value.toLowerCase()),
  ),
);
function choose(kind: string) {
  open.value = false;
  query.value = "";
  emit("action", kind);
}
</script>
<template>
  <AppDialog
    v-model:open="open"
    :motion="false"
    title="Command palette"
    description="Find a Git operation. Use Tab to navigate and Enter to select."
    ><label class="search-field palette-search"
      ><Search :size="18" /><input
        v-model="query"
        autofocus
        placeholder="What would you like to do?"
        aria-label="Find a command"
    /></label>
    <div class="palette-list">
      <button
        @click="
          open = false;
          emit('open');
        "
      >
        <FolderOpen :size="17" />Open repository</button
      ><button
        v-for="action in actions"
        :key="action.kind"
        :disabled="!store.trusted || store.busy"
        @click="choose(action.kind)"
      >
        <GitBranch :size="16" /><span>{{ action.title }}</span
        ><small v-if="action.danger">Review required</small>
      </button>
      <p v-if="!actions.length" class="muted">No matching Git operations.</p>
    </div></AppDialog
  >
</template>
