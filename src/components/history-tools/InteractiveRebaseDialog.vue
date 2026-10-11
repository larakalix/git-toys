<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import AppDialog from "../AppDialog.vue";
import RebaseSteps from "./RebaseSteps.vue";
import PreviewSummary from "./PreviewSummary.vue";
import {
  call,
  asError,
  type RebaseRange,
  type RebaseStep,
} from "../../lib/api";
import { useRepositoryStore } from "../../stores/repository";
import { useGitPreview } from "../../composables/useGitPreview";
const open = defineModel<boolean>("open", { default: false });
const store = useRepositoryStore(),
  base = ref(""),
  range = shallowRef<RebaseRange>(),
  steps = ref<RebaseStep[]>([]),
  loadingRange = ref(false),
  errorRange = ref(""),
  confirmed = ref(false);
const preview = useGitPreview();
const { result, loading, error, progress } = preview;
let version = 0;
const subjects = computed(() =>
  Object.fromEntries(range.value?.commits.map((c) => [c.oid, c.subject]) ?? []),
);
const plan = computed(() =>
  range.value ? { ...range.value.plan, steps: steps.value } : null,
);
watch([open, () => store.activeId, base], () => {
  ++version;
  range.value = undefined;
  steps.value = [];
  confirmed.value = false;
  preview.invalidate();
});
watch(
  steps,
  () => {
    preview.invalidate();
    confirmed.value = false;
  },
  { deep: true },
);
async function load() {
  const current = ++version;
  loadingRange.value = true;
  errorRange.value = "";
  try {
    const response = await call<RebaseRange>("rebase_range", {
      id: store.activeId,
      base: base.value,
    });
    if (current !== version) return;
    range.value = response;
    steps.value = response.plan.steps.map((s) => ({ ...s }));
  } catch (e) {
    if (current === version) {
      errorRange.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === version) loadingRange.value = false;
  }
}
async function apply() {
  if (!plan.value || !result.value || !confirmed.value) return;
  await store.mutate(
    { kind: "interactiveRebase", plan: plan.value },
    preview.expected(),
  );
  if (store.busy) open.value = false;
}
</script>
<template>
  <AppDialog
    v-model:open="open"
    title="Edit branch history"
    description="Reorder or combine local commits. This rewrites commit hashes; Git Toys creates a recovery branch first."
    content-class="feature-dialog"
  >
    <div class="feature-toolbar">
      <label
        >Base commit (excluded from editing)<input
          v-model="base"
          placeholder="HEAD~3 or a branch" /></label
      ><button
        class="button"
        :disabled="!base || loadingRange || store.busy"
        @click="load"
      >
        {{ loadingRange ? "Loading…" : "Load commits" }}
      </button>
    </div>
    <p v-if="errorRange" role="alert">{{ errorRange }}</p>
    <p v-if="range?.published" role="alert">
      Some selected commits appear in remote-tracking history. That information
      may be stale. Coordinate history changes with collaborators; pushing may
      require an explicit force-with-lease.
    </p>
    <div v-if="range" class="feature-split">
      <section>
        <h3>Original history</h3>
        <p v-for="commit in range.commits" :key="commit.oid">
          <code>{{ commit.oid.slice(0, 8) }}</code> {{ commit.subject }}
        </p>
      </section>
      <section>
        <h3>Proposed sequence</h3>
        <RebaseSteps
          v-model="steps"
          :subjects="subjects"
          :disabled="loading || store.busy"
        />
      </section>
    </div>
    <div v-if="plan" class="feature-toolbar">
      <button
        class="button"
        :disabled="loading || store.busy || !store.trusted"
        @click="preview.run({ kind: 'interactive', target: plan.base, plan })"
      >
        Preview edited history</button
      ><button v-if="loading" class="button" @click="preview.cancel">
        Cancel preview</button
      ><span v-if="loading" role="status">{{ progress }}</span>
    </div>
    <p v-if="error" role="alert">{{ error }}</p>
    <PreviewSummary v-if="result" :result="result" />
    <label v-if="result"
      ><input v-model="confirmed" type="checkbox" /> Rewrite
      {{ store.snapshot?.branch }} using this reviewed sequence.</label
    >
    <footer>
      <button
        class="button primary"
        :disabled="
          !result || !confirmed || loading || store.busy || !store.trusted
        "
        @click="apply"
      >
        Create recovery branch and rebase
      </button>
    </footer>
  </AppDialog>
</template>
