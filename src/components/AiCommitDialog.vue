<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef, watch } from "vue";
import { LoaderCircle, Sparkles } from "lucide-vue-next";
import AppDialog from "./AppDialog.vue";
import { call, asError } from "../lib/api";
import type { AiContext, AiDraft } from "../lib/api";
import { useRepositoryStore } from "../stores/repository";
const props = defineProps<{ existingMessage: string }>();
const open = defineModel<boolean>("open", { default: false });
const emit = defineEmits<{ use: [message: string] }>();
const store = useRepositoryStore();
const context = shallowRef<AiContext | null>(null),
  selected = shallowRef<string[]>([]),
  style = shallowRef("plain"),
  draft = ref<AiDraft | null>(null),
  loading = shallowRef(false),
  running = shallowRef(false),
  stale = shallowRef(false),
  error = shallowRef(""),
  replace = shallowRef(false);
let version = 0,
  requestId = "",
  repositoryId = "";
const contextBytes = computed(() =>
  new TextEncoder()
    .encode(context.value?.context ?? "")
    .length.toLocaleString(),
);
const unavailable = computed(
  () => !store.preferences.ai.enabled || !store.preferences.ai.model.trim(),
);
const canGenerate = computed(
  () =>
    !unavailable.value &&
    !!context.value &&
    !context.value.oversized &&
    !!context.value.selected.length &&
    !loading.value &&
    !running.value,
);
async function cancel() {
  ++version;
  const id = requestId;
  requestId = "";
  running.value = false;
  if (id) await store.attempt(() => call("ai_cancel", { requestId: id }));
}
async function prepare(initial = false) {
  const current = ++version;
  loading.value = true;
  error.value = "";
  draft.value = null;
  try {
    const result = await call<AiContext>("ai_context", {
      id: repositoryId,
      selected: initial ? null : selected.value,
      style: style.value,
    });
    if (current !== version) return;
    context.value = result;
    selected.value = result.selected;
    stale.value = false;
  } catch (e) {
    if (current === version) {
      context.value = null;
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === version) loading.value = false;
  }
}
function toggle(path: string, checked: boolean) {
  selected.value = checked
    ? [...selected.value, path]
    : selected.value.filter((value) => value !== path);
  prepare();
}
watch(open, async (value) => {
  await cancel();
  if (open.value !== value) return;
  if (!value) {
    context.value = null;
    draft.value = null;
    return;
  }
  repositoryId = store.activeId;
  replace.value = false;
  await prepare(true);
});
watch(
  () => store.activeId,
  () => {
    open.value = false;
  },
);
watch(
  () => store.snapshot?.stateToken,
  async () => {
    if (!open.value || !context.value) return;
    const current = version;
    try {
      const token = await call<string>("ai_index_token", { id: repositoryId });
      if (
        current === version &&
        context.value &&
        token !== context.value.indexToken
      )
        stale.value = true;
    } catch {
      if (current === version) stale.value = true;
    }
  },
);
watch(
  () => [store.preferences.ai.provider, store.preferences.ai.model],
  () => {
    cancel();
    draft.value = null;
  },
);
async function generate() {
  if (!canGenerate.value || !context.value) return;
  const current = ++version;
  requestId = crypto.randomUUID();
  running.value = true;
  error.value = "";
  draft.value = null;
  try {
    const result = await call<AiDraft>("ai_generate", {
      id: repositoryId,
      requestId,
      context: context.value,
      style: style.value,
    });
    if (current === version) {
      draft.value = result;
      stale.value = false;
    }
  } catch (e) {
    if (current === version) {
      error.value = asError(e).message;
      if (asError(e).code !== "aiCancelled") store.fail(e);
    }
  } finally {
    if (current === version) {
      running.value = false;
      requestId = "";
    }
  }
}
async function accept() {
  if (!draft.value || stale.value || repositoryId !== store.activeId) return;
  const current = version;
  try {
    const token = await call<string>("ai_index_token", { id: repositoryId });
    if (current !== version || !draft.value) return;
    if (token !== draft.value.indexToken) {
      stale.value = true;
      return;
    }
    emit("use", draft.value.message);
    open.value = false;
  } catch (e) {
    stale.value = true;
    store.fail(e);
  }
}
onBeforeUnmount(() => {
  cancel();
});
</script>
<template>
  <AppDialog
    v-model:open="open"
    title="Draft a commit with AI"
    description="Review the exact outgoing context before generating. Nothing is committed automatically."
    content-class="ai-dialog"
  >
    <p class="ai-provider">
      <Sparkles :size="16" />{{ store.preferences.ai.provider }} ·
      {{ store.preferences.ai.model || "No model configured" }}
    </p>
    <p v-if="unavailable" class="repo-warning">
      Enable AI and enter a model in Settings → AI commit messages.
    </p>
    <p v-if="store.preferences.ai.provider !== 'ollama'" class="muted">
      This request sends the reviewed context to your cloud provider and may
      incur API charges.
    </p>
    <label class="field"
      >Message style<select
        v-model="style"
        :disabled="running"
        @change="prepare()"
      >
        <option value="plain">Plain</option>
        <option value="conventional">Conventional Commit</option>
      </select></label
    >
    <div v-if="loading" role="status">
      <LoaderCircle :size="16" class="spin" />Preparing staged context…
    </div>
    <template v-if="context">
      <fieldset class="ai-files" :disabled="running || loading">
        <legend>Staged files to include</legend>
        <label
          v-for="file in context.files"
          :key="file.path"
          class="check-field"
          ><input
            type="checkbox"
            :checked="selected.includes(file.path)"
            :disabled="!!file.excludedReason"
            @change="
              toggle(file.path, ($event.target as HTMLInputElement).checked)
            "
          /><span
            >{{ file.path
            }}<small v-if="file.excludedReason">
              — {{ file.excludedReason }}</small
            ></span
          ></label
        >
      </fieldset>
      <p class="muted">
        Only staged patches are included. Default exclusions cannot guarantee
        secret detection. Review the context before sharing.
      </p>
      <details class="ai-context">
        <summary>
          Exact outgoing context ·
          {{ contextBytes }}
          bytes / 65,536
        </summary>
        <pre>{{ context.context }}</pre>
      </details>
      <p v-if="context.oversized" class="repo-warning">
        Context exceeds 64 KiB. Deselect files before generating; nothing will
        be silently truncated.
      </p>
    </template>
    <p v-if="stale" role="alert" class="repo-warning">
      Staged changes changed. Refresh context and regenerate before using this
      draft.<button :disabled="running" @click="prepare(true)">
        Refresh context
      </button>
    </p>
    <p v-if="error" role="alert" class="danger">{{ error }}</p>
    <template v-if="draft">
      <label class="field"
        >AI draft<textarea v-model="draft.message" rows="5" />
      </label>
      <label v-if="props.existingMessage.trim()" class="check-field"
        ><input v-model="replace" type="checkbox" />Replace my existing commit
        message</label
      >
    </template>
    <footer class="dialog-actions">
      <button @click="open = false">Cancel</button
      ><button v-if="running" @click="cancel()">Stop generation</button
      ><button :disabled="!canGenerate || stale" @click="generate()">
        <LoaderCircle v-if="running" :size="15" class="spin" />{{
          running ? "Generating…" : draft ? "Regenerate" : "Generate draft"
        }}</button
      ><button
        v-if="draft"
        class="primary"
        :disabled="
          stale ||
          running ||
          !draft.message.trim() ||
          (!!props.existingMessage.trim() && !replace)
        "
        @click="accept()"
      >
        Use message
      </button>
    </footer>
  </AppDialog>
</template>
<style scoped>
.ai-provider {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
  color: var(--accent);
}
.ai-files {
  border: 1px solid var(--border);
  padding: 12px;
  border-radius: 7px;
  margin: 16px 0;
  max-height: 180px;
  overflow: auto;
}
.ai-files label {
  display: flex;
  gap: 8px;
  margin: 7px 0;
  overflow-wrap: anywhere;
}
.ai-context {
  margin: 16px 0;
  border: 1px solid var(--border);
  border-radius: 7px;
  padding: 10px;
}
pre {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  max-height: 240px;
  overflow: auto;
  font:
    11px/1.6 "JetBrains Mono",
    monospace;
  padding: 12px 0;
}
</style>
