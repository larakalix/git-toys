<script setup lang="ts">
import { ref, watch } from "vue";
import AppDialog from "../AppDialog.vue";
import PreviewSummary from "./PreviewSummary.vue";
import { useGitPreview } from "../../composables/useGitPreview";
import { useRepositoryStore } from "../../stores/repository";
const open = defineModel<boolean>("open", { default: false });
const props = defineProps<{
  kind: "merge" | "rebase";
  initialTarget?: string;
}>();
const store = useRepositoryStore(),
  target = ref(""),
  confirmed = ref(false);
const preview = useGitPreview();
const { result, loading, error, progress } = preview;
watch([open, () => props.initialTarget, () => props.kind], () => {
  preview.invalidate();
  confirmed.value = false;
  target.value = props.initialTarget ?? "";
});
watch(target, () => {
  preview.invalidate();
  confirmed.value = false;
});
async function apply() {
  if (!result.value || !confirmed.value) return;
  await store.mutate(
    {
      kind: "applyPreview",
      kindName: props.kind,
      head: result.value.head,
      target: result.value.target,
      targetRef: result.value.targetRef,
    },
    preview.expected(),
  );
  if (store.busy) open.value = false;
}
</script>
<template>
  <AppDialog
    v-model:open="open"
    :title="`Preview ${kind}`"
    description="Inspect committed changes in a disposable repository before applying them to your current branch."
    content-class="feature-dialog"
  >
    <label class="field"
      >{{
        kind === "merge"
          ? "Branch or commit to merge"
          : "Branch or commit to rebase onto"
      }}<input v-model="target" list="preview-refs" /><datalist
        id="preview-refs"
      >
        <option
          v-for="branch in store.snapshot?.refs"
          :key="branch.name"
          :value="branch.name"
        /></datalist
    ></label>
    <div class="feature-toolbar">
      <button
        class="button"
        :disabled="!target.trim() || loading || !store.trusted || store.busy"
        @click="preview.run({ kind, target, plan: null })"
      >
        Generate preview</button
      ><button v-if="loading" class="button" @click="preview.cancel">
        Cancel preview</button
      ><span v-if="loading" role="status">{{ progress }}</span>
    </div>
    <p v-if="error" role="alert">{{ error }}</p>
    <PreviewSummary v-if="result" :result="result" />
    <p v-if="store.snapshot?.files.length" class="muted">
      Commit or stash local changes before applying this operation.
    </p>
    <label v-if="result"
      ><input v-model="confirmed" type="checkbox" /> I reviewed the affected
      commits and conflicts{{
        kind === "rebase" ? " and understand this rewrites history" : ""
      }}.</label
    >
    <footer>
      <button
        class="button primary"
        :disabled="
          !result ||
          !confirmed ||
          loading ||
          !store.trusted ||
          store.busy ||
          !!store.snapshot?.files.length ||
          !!store.snapshot?.operation
        "
        @click="apply"
      >
        Apply {{ kind }} to {{ store.snapshot?.branch }}
      </button>
    </footer>
  </AppDialog>
</template>
