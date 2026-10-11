<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useRepositoryStore } from "../stores/repository";
import { useNotificationStore } from "../stores/notifications";
import { call, asError } from "../lib/api";
const store = useRepositoryStore(),
  notifications = useNotificationStore();
const keyError = shallowRef(""),
  key = shallowRef(""),
  hasKey = shallowRef(false),
  saving = shallowRef(false);
const cloud = computed(() => store.preferences.ai.provider !== "ollama");
let generation = 0;
watch(
  () => store.preferences.ai.provider,
  async (provider) => {
    const current = ++generation;
    keyError.value = "";
    key.value = "";
    hasKey.value = false;
    if (provider === "ollama") return;
    try {
      const value = await call<boolean>("ai_key_status", { provider });
      if (current === generation) hasKey.value = value;
    } catch (error) {
      keyError.value = asError(error).message;
      store.fail(error);
    }
  },
  { immediate: true },
);
async function saveKey(remove = false) {
  saving.value = true;
  keyError.value = "";
  const provider = store.preferences.ai.provider;
  try {
    await call("ai_save_key", { provider, key: remove ? null : key.value });
    key.value = "";
    if (provider === store.preferences.ai.provider) hasKey.value = !remove;
    notifications.notify({
      kind: "success",
      title: remove
        ? "API key removed"
        : "API key saved in OS credential storage",
    });
  } catch (error) {
    keyError.value = asError(error).message;
    store.fail(error);
  } finally {
    saving.value = false;
  }
}
</script>
<template>
  <section class="ai-settings" aria-labelledby="ai-settings-heading">
    <h3 id="ai-settings-heading">AI commit messages</h3>
    <p class="muted">
      Optional drafts from staged changes. You review every request and decide
      what to commit.
    </p>
    <label class="check-field"
      ><input
        v-model="store.preferences.ai.enabled"
        type="checkbox"
        @change="store.save()"
      />Enable AI commit drafts</label
    >
    <div class="settings-columns">
      <label class="field"
        >AI provider<select
          v-model="store.preferences.ai.provider"
          @change="store.save()"
        >
          <option value="ollama">Ollama · local endpoint</option>
          <option value="openai">OpenAI · your API account</option>
          <option value="anthropic">Anthropic · your API account</option>
        </select></label
      >
      <label class="field"
        >AI model<input
          v-model="store.preferences.ai.model"
          placeholder="Enter an installed or available model ID"
          maxlength="200"
          @change="store.save()"
      /></label>
    </div>
    <p v-if="keyError" role="alert">{{ keyError }}</p>
    <template v-if="!cloud">
      <label class="field"
        >Ollama URL<input
          v-model="store.preferences.ai.ollamaUrl"
          placeholder="http://127.0.0.1:11434"
          @change="store.save()"
      /></label>
      <p class="muted">
        Start Ollama and choose an already-installed local model. Git Toys does
        not install models. For local-only operation, disable Ollama cloud
        features with OLLAMA_NO_CLOUD=1 and restart Ollama. A loopback endpoint
        alone does not guarantee a model runs locally.
      </p>
    </template>
    <template v-else>
      <p class="muted">
        Selected code will be sent to
        {{
          store.preferences.ai.provider === "openai" ? "OpenAI" : "Anthropic"
        }}
        after your review. Your API account may incur charges. Provider data
        policies apply.
      </p>
      <form class="key-form" @submit.prevent="saveKey()">
        <label class="field"
          >API key<input
            v-model="key"
            type="password"
            autocomplete="off"
            placeholder="Stored in Keychain or Windows Credential Manager"
            :disabled="saving"
        /></label>
        <div class="button-row">
          <button :disabled="saving || !key.trim()">
            {{ saving ? "Saving…" : "Save API key" }}</button
          ><button
            v-if="hasKey"
            type="button"
            :disabled="saving"
            @click="saveKey(true)"
          >
            Remove stored key</button
          ><span class="muted" role="status">{{
            hasKey ? "Key configured" : "No saved key"
          }}</span>
        </div>
      </form>
    </template>
  </section>
</template>
<style scoped>
.ai-settings {
  border-top: 1px solid var(--border);
  padding-top: 20px;
  margin: 24px 0;
}
h3 {
  font-size: 14px;
  font-weight: 600;
}
p {
  margin: 8px 0 16px;
}
.settings-columns {
  display: grid;
  grid-template-columns: 1fr 1.5fr;
  gap: 16px;
  margin-top: 16px;
}
.button-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
@media (max-width: 600px) {
  .settings-columns {
    grid-template-columns: 1fr;
  }
}
</style>
