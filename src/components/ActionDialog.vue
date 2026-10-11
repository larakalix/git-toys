<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import AppDialog from "./AppDialog.vue";
import { useRepositoryStore } from "../stores/repository";
import { actionDefinition } from "../lib/actions";
import type { Action } from "../lib/api";
const props = defineProps<{
  kind: string;
  defaults?: Record<string, unknown>;
}>();
const open = defineModel<boolean>("open", { default: false });
const store = useRepositoryStore(),
  form = reactive<Record<string, string | boolean | number>>({});
const definition = computed(() => actionDefinition(props.kind));
watch(
  () => [props.kind, open.value],
  () => {
    if (!open.value) return;
    for (const key of Object.keys(form)) delete form[key];
    for (const f of definition.value?.fields ?? [])
      form[f.key] =
        f.type === "checkbox"
          ? false
          : f.type === "number"
            ? 0
            : (f.options?.[0] ?? "");
    Object.assign(
      form,
      {
        remote: store.snapshot?.remotes[0]?.name ?? "origin",
        target: store.selectedCommit?.oid ?? "HEAD",
        branch: store.snapshot?.branch ?? "",
        old: store.snapshot?.branch ?? "",
        start: store.selectedCommit?.oid ?? "",
        publish: !store.snapshot?.upstream,
      },
      props.defaults,
    );
  },
  { immediate: true },
);
function submit() {
  const payload: Record<string, unknown> = { kind: props.kind };
  for (const f of definition.value?.fields ?? [])
    payload[f.key] = f.type === "number" ? Number(form[f.key]) : form[f.key];
  if (props.kind === "createBranch" && !payload.start) payload.start = null;
  if (props.kind === "discard") payload.paths = props.defaults?.paths ?? [];
  store.mutate(payload as Action);
  open.value = false;
}
</script>
<template>
  <AppDialog
    v-model:open="open"
    :title="definition?.title ?? 'Git operation'"
    :description="definition?.description"
    ><form v-if="definition" class="operation-form" @submit.prevent="submit">
      <div class="action-context">
        <span>CURRENT BRANCH</span><code>{{ store.snapshot?.branch }}</code
        ><span>HEAD {{ store.snapshot?.head.slice(0, 7) }}</span>
      </div>
      <p v-if="kind === 'discard'" class="notice">{{ defaults?.paths }}</p>
      <label
        v-for="field in definition.fields"
        :key="field.key"
        :class="field.type === 'checkbox' ? 'checkbox-field' : 'field'"
        ><input
          v-if="field.type === 'checkbox'"
          v-model="form[field.key]"
          type="checkbox" /><span>{{ field.label }}</span
        ><select v-if="field.options" v-model="form[field.key]">
          <option v-for="option in field.options" :key="option" :value="option">
            {{ option }}
          </option></select
        ><textarea
          v-else-if="field.type === 'textarea'"
          v-model="form[field.key] as string"
          :required="!field.optional"
          rows="4" /><input
          v-else-if="field.type !== 'checkbox'"
          v-model="form[field.key]"
          :type="field.type === 'number' ? 'number' : 'text'"
          :min="field.type === 'number' ? 0 : undefined"
          :required="!field.optional"
          :placeholder="field.placeholder"
      /></label>
      <footer class="dialog-actions">
        <button type="button" @click="open = false">Cancel</button
        ><button
          :class="definition.danger ? 'danger' : 'primary'"
          :disabled="store.busy || !store.trusted"
        >
          {{ definition.title }}
        </button>
      </footer>
    </form></AppDialog
  >
</template>
