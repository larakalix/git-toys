<script setup lang="ts">
import { shallowRef, watch } from "vue";
import { open as chooseDirectory } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-vue-next";
import { useRepositoryStore } from "../stores/repository";
import AppDialog from "./AppDialog.vue";
const props = defineProps<{ mode: "clone" | "init" }>(),
  open = defineModel<boolean>("open", { default: false });
const store = useRepositoryStore(),
  url = shallowRef(""),
  path = shallowRef(""),
  trusted = shallowRef(false);
watch(open, (value) => {
  if (value) {
    url.value = "";
    path.value = "";
    trusted.value = false;
  }
});
async function browse() {
  const selected = await chooseDirectory({
    directory: true,
    title: "Choose an empty destination directory",
  });
  if (typeof selected === "string") path.value = selected;
}
async function submit() {
  await store.create(path.value, props.mode === "clone" ? url.value : null);
  if (!store.error) open.value = false;
}
</script>
<template>
  <AppDialog
    v-model:open="open"
    :title="mode === 'clone' ? 'Clone repository' : 'Initialize repository'"
    :description="
      mode === 'clone'
        ? 'Bring an existing project onto your computer.'
        : 'Start a repository in a new or empty folder.'
    "
    ><form class="operation-form" @submit.prevent="submit">
      <label v-if="mode === 'clone'" class="field"
        >Repository URL<input
          v-model="url"
          required
          placeholder="https://host/owner/repository.git or SSH URL" /></label
      ><label class="field"
        >Destination directory
        <div class="input-with-button">
          <input
            v-model="path"
            aria-label="Destination directory"
            required
            placeholder="Absolute path to a new or empty folder"
          /><button
            type="button"
            aria-label="Choose destination directory"
            @click="store.attempt(browse)"
          >
            <FolderOpen :size="17" />
          </button></div></label
      ><label class="checkbox-field"
        ><input v-model="trusted" type="checkbox" required /><span
          >I trust this repository source and my Git configuration. Git filters,
          helpers, and hooks can execute local programs.</span
        ></label
      >
      <p v-if="store.busy" role="status" class="notice">
        {{
          mode === "clone"
            ? "Cloning repository… Authentication may require a response."
            : "Initializing…"
        }}
        Keep this window open.
      </p>
      <footer class="dialog-actions">
        <button type="button" :disabled="store.busy" @click="open = false">
          Cancel</button
        ><button class="primary" :disabled="!trusted || store.busy">
          {{ mode === "clone" ? "Clone repository" : "Initialize repository" }}
        </button>
      </footer>
    </form></AppDialog
  >
</template>
