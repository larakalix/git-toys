<script setup lang="ts">
import { shallowRef, watch } from "vue";
import { Archive, Activity, GitBranch } from "lucide-vue-next";
import { useRepositoryStore } from "../stores/repository";
import { call } from "../lib/api";
import type { ReflogEntry } from "../lib/api";
const store = useRepositoryStore(),
  entries = shallowRef<ReflogEntry[]>([]);
const emit = defineEmits<{
  action: [kind: string, defaults?: Record<string, unknown>];
}>();
let version = 0;
watch(
  () => [store.view, store.activeId, store.snapshot?.stateToken],
  async () => {
    const current = ++version;
    entries.value = [];
    if (!["reflog", "stashes"].includes(store.view)) return;
    try {
      const result = await call<ReflogEntry[]>(store.view, {
        id: store.activeId,
      });
      if (current === version) entries.value = result;
    } catch (e) {
      store.fail(e);
    }
  },
  { immediate: true },
);
</script>
<template>
  <section class="recovery-panel">
    <header class="panel-heading">
      <div>
        <h1>
          {{
            store.view === "reflog"
              ? "Reflog & recovery"
              : store.view === "stashes"
                ? "Stashes"
                : "Activity"
          }}
        </h1>
        <p>
          {{
            store.view === "reflog"
              ? "Reference movements can help recover commits. They do not back up discarded working changes."
              : store.view === "stashes"
                ? "Work saved for another moment."
                : "A local record of Git operations. Credentials and command output are not persisted."
          }}
        </p>
      </div>
      <button
        v-if="store.view === 'stashes'"
        :disabled="!store.trusted || store.busy"
        @click="emit('action', 'stash')"
      >
        <Archive :size="16" />New stash
      </button>
    </header>
    <div class="recovery-scroll">
      <template v-if="store.view === 'activity'"
        ><details
          v-for="operation in [...store.operations].reverse()"
          :key="operation.id"
          class="session-output"
        >
          <summary>
            {{ operation.kind }} · {{ operation.status }} · current session
            output
          </summary>
          <pre>{{
            operation.error?.detail ||
            operation.output ||
            "Waiting for Git output…"
          }}</pre>
        </details>
        <article
          v-for="entry in [...store.preferences.journal].reverse()"
          :key="entry.id"
          class="activity-row"
        >
          <Activity :size="16" />
          <div>
            <strong>{{ entry.kind }}</strong
            ><small>{{ entry.repoId }}</small>
          </div>
          <span :class="entry.status === 'failed' ? 'danger-text' : 'muted'">{{
            entry.status
          }}</span
          ><time>{{ new Date(entry.timestamp * 1000).toLocaleString() }}</time>
        </article>
        <p v-if="!store.preferences.journal.length" class="empty-panel">
          Your operations will appear here.
        </p></template
      ><template v-else
        ><article
          v-for="(entry, index) in entries"
          :key="entry.selector"
          class="recovery-row"
        >
          <code>{{ entry.oid.slice(0, 7) }}</code>
          <div>
            <strong>{{ entry.subject }}</strong
            ><small>{{ entry.selector }}</small>
          </div>
          <template v-if="store.view === 'reflog'"
            ><button
              :disabled="!store.trusted || store.busy"
              @click="emit('action', 'createBranch', { start: entry.oid })"
            >
              <GitBranch :size="14" />Restore to new branch
            </button></template
          ><template v-else
            ><button
              :disabled="!store.trusted || store.busy"
              @click="emit('action', 'applyStash', { index })"
            >
              Apply…</button
            ><button
              class="danger-text"
              :disabled="!store.trusted || store.busy"
              @click="emit('action', 'dropStash', { index })"
            >
              Drop…
            </button></template
          >
        </article>
        <p v-if="!entries.length" class="empty-panel">
          No
          {{ store.view === "reflog" ? "reference movements" : "stashes" }} to
          show.
        </p></template
      >
    </div>
  </section>
</template>
