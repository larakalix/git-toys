<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import AppDialog from "./AppDialog.vue";
import { call } from "../lib/api";
import type { Conflict } from "../lib/api";
import { useRepositoryStore } from "../stores/repository";
const props = defineProps<{ path: string }>(),
  open = defineModel<boolean>("open", { default: false });
const store = useRepositoryStore(),
  conflict = shallowRef<Conflict | null>(null),
  result = shallowRef(""),
  tab = shallowRef<"base" | "ours" | "theirs">("ours");
let generation = 0;
watch(
  () => [props.path, open.value],
  async () => {
    const current = ++generation;
    conflict.value = null;
    if (!props.path || !open.value) return;
    try {
      const data = await call<Conflict>("conflict", {
        id: store.activeId,
        path: props.path,
      });
      if (current === generation) {
        conflict.value = data;
        result.value = data.result;
      }
    } catch (e) {
      store.fail(e);
    }
  },
  { immediate: true },
);
const markers = computed(() =>
  result.value
    .split("\n")
    .some(
      (l) =>
        l.startsWith("<<<<<<< ") || l === "=======" || l.startsWith(">>>>>>> "),
    ),
);
function resolve() {
  store.mutate({ kind: "resolve", path: props.path, content: result.value });
  open.value = false;
}
</script>
<template>
  <AppDialog
    v-model:open="open"
    :title="`Resolve ${path}`"
    description="During a rebase, ours is the branch being rebased onto; theirs is the replayed commit. Review each version before choosing."
    ><template v-if="conflict"
      ><div class="conflict-tabs">
        <button
          v-for="side in ['base', 'ours', 'theirs'] as const"
          :key="side"
          :class="{ active: tab === side }"
          @click="tab = side"
        >
          {{ side }}
        </button>
      </div>
      <div v-if="conflict.binary" class="notice">
        Binary conflict: choose a complete side or resolve with an external
        tool.
      </div>
      <template v-else>
        <pre class="conflict-source">{{
          conflict[tab] || "(File absent in this version)"
        }}</pre>
        <div class="conflict-tools">
          <button @click="result = conflict.ours">Use ours as result</button
          ><button @click="result = conflict.theirs">
            Use theirs as result
          </button>
        </div>
        <label class="field"
          >Resolved result<textarea
            v-model="result"
            class="conflict-result"
            rows="10"
            spellcheck="false"
          />
        </label>
        <p v-if="markers" class="warning-text">
          Remove conflict markers before marking resolved.
        </p></template
      >
      <footer class="dialog-actions">
        <template v-if="conflict.binary"
          ><button
            @click="
              store.mutate({ kind: 'resolveSide', path, side: 'ours' });
              open = false;
            "
          >
            Resolve using ours</button
          ><button
            @click="
              store.mutate({ kind: 'resolveSide', path, side: 'theirs' });
              open = false;
            "
          >
            Resolve using theirs
          </button></template
        ><button
          v-else
          class="primary"
          :disabled="markers || store.busy || !store.trusted"
          @click="resolve"
        >
          Save & mark resolved
        </button>
      </footer></template
    >
    <p v-else>Loading conflict…</p></AppDialog
  >
</template>
