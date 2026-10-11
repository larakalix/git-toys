<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { GitCommitHorizontal, File, ArrowUpRight } from "lucide-vue-next";
import { useRepositoryStore } from "../stores/repository";
import { call } from "../lib/api";
import DiffPanel from "./DiffPanel.vue";
const props = defineProps<{ theme: string }>();
const store = useRepositoryStore(),
  files = shallowRef<string[]>([]),
  path = shallowRef(""),
  base = shallowRef("");
const emit = defineEmits<{
  action: [kind: string, defaults?: Record<string, unknown>];
  fileHistory: [path: string, revision: string];
}>();
const commit = computed(() => store.selectedCommit);
let generation = 0;
watch(
  () => [commit.value?.oid, base.value, store.activeId],
  async () => {
    const current = ++generation;
    path.value = "";
    files.value = [];
    if (!commit.value) return;
    try {
      const result = await call<string[]>("commit_files", {
        id: store.activeId,
        revision: commit.value.oid,
        base: base.value || null,
      });
      if (current === generation) {
        files.value = result;
        path.value = result[0] ?? "";
      }
    } catch (e) {
      store.fail(e);
    }
  },
  { immediate: true },
);
watch(
  () => store.activeId,
  () => (base.value = ""),
);
</script>
<template>
  <aside class="commit-details">
    <div v-if="!commit" class="empty-panel">
      <GitCommitHorizontal :size="32" />
      <h2>Follow the story</h2>
      <p>Select a commit to see its changes,<br />parents, and author.</p>
    </div>
    <template v-else
      ><section class="commit-info">
        <div class="eyebrow">COMMIT DETAILS</div>
        <h2>{{ commit.subject }}</h2>
        <p
          v-if="commit.message.split('\n').slice(1).join('\n').trim()"
          class="commit-body"
        >
          {{ commit.message.split("\n").slice(1).join("\n").trim() }}
        </p>
        <div class="author-row">
          <span class="avatar">{{
            commit.author.slice(0, 2).toUpperCase()
          }}</span>
          <div>
            <strong>{{ commit.author }}</strong
            ><small>{{ new Date(commit.date).toLocaleString() }}</small>
          </div>
        </div>
        <dl>
          <dt>Commit</dt>
          <dd>
            <code :title="commit.oid">{{ commit.oid.slice(0, 12) }}</code>
          </dd>
          <dt>Parents</dt>
          <dd>
            <code>{{
              commit.parents.map((p) => p.slice(0, 7)).join(", ") ||
              "Root commit"
            }}</code>
          </dd>
        </dl>
        <div class="commit-tools">
          <button
            :disabled="!store.trusted"
            @click="emit('action', 'createBranch', { start: commit.oid })"
          >
            Branch here<ArrowUpRight :size="13" /></button
          ><select
            aria-label="Commit action"
            :disabled="!store.trusted"
            @change="
              emit('action', ($event.target as HTMLSelectElement).value, {
                target: commit.oid,
              });
              ($event.target as HTMLSelectElement).value = '';
            "
          >
            <option value="">More actions…</option>
            <option value="cherryPick">Cherry-pick</option>
            <option value="revert">Revert</option>
            <option value="reset">Reset current branch</option>
            <option value="createTag">Tag commit</option>
          </select>
        </div>
        <label class="compare-field"
          >Compare from<input
            v-model.lazy="base"
            placeholder="Parent (default) or commit/ref"
        /></label>
      </section>
      <div class="commit-file-list">
        <div class="section-heading">
          <h3>Changed files</h3>
          <span>{{ files.length }}</span
          ><button
            v-if="path"
            class="text-button"
            @click="emit('fileHistory', path, commit.oid)"
          >
            View file history
          </button>
        </div>
        <button
          v-for="file in files"
          :key="file"
          :class="{ active: path === file }"
          @click="path = file"
        >
          <File :size="14" /><span>{{ file }}</span>
        </button>
      </div>
      <DiffPanel
        :path="path"
        :commit="commit.oid"
        :base="base || undefined"
        :theme="props.theme"
    /></template>
  </aside>
</template>
