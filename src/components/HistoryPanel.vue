<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useVirtualList, watchDebounced } from "@vueuse/core";
import {
  Search,
  GitCommitHorizontal,
  ArrowDown,
  Filter,
} from "lucide-vue-next";
import { useRepositoryStore } from "../stores/repository";
import { graphColors, graphEdgePath, layoutGraph } from "../lib/graph";
import InteractiveRebaseDialog from "./history-tools/InteractiveRebaseDialog.vue";
const rebaseOpen = shallowRef(false);
const store = useRepositoryStore();
const rows = computed(() => {
  const graph = layoutGraph(store.commits);
  return store.commits.map((commit, index) => ({
    commit,
    graph: graph[index]!,
  }));
});
const graphWidth = computed(() =>
  Math.max(84, ...rows.value.map((row) => row.graph.lanes * 22 + 24)),
);
function refs(value: string) {
  return value.split(", ").filter(Boolean);
}
const { list, containerProps, wrapperProps } = useVirtualList(rows, {
  itemHeight: 58,
  overscan: 12,
});
const search = shallowRef(store.query),
  path = shallowRef(store.pathFilter);
watch(
  () => store.activeId,
  () => {
    search.value = "";
    path.value = "";
  },
);
watchDebounced(
  [search, path],
  () => {
    store.query = search.value;
    store.pathFilter = path.value;
    store.loadHistory();
  },
  { debounce: 250, maxWait: 1000 },
);
function shortDate(date: string) {
  return new Date(date).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}
</script>
<template>
  <InteractiveRebaseDialog v-model:open="rebaseOpen" />
  <section class="history-panel">
    <header class="panel-heading">
      <div>
        <h1>
          History<span class="subtle">
            / {{ store.revision || "All branches" }}</span
          >
        </h1>
        <p>A connected view of your project.</p>
      </div>
      <button
        class="button"
        :disabled="!store.trusted || store.busy"
        @click="rebaseOpen = true"
      >
        Edit branch history
      </button>
      <span class="label-badge">{{ store.commits.length }} loaded</span>
    </header>
    <div class="history-filters">
      <label class="search-field"
        ><Search :size="16" /><input
          v-model="search"
          aria-label="Search history"
          placeholder="Search commits, authors, or hashes…"
        /><kbd>⌘ K</kbd></label
      ><label class="path-filter"
        ><Filter :size="14" /><input
          v-model="path"
          aria-label="Filter history by path"
          placeholder="Filter by path"
      /></label>
    </div>
    <div class="list-heading">
      <span>GRAPH / COMMIT</span><span>AUTHOR · DATE</span>
    </div>
    <div
      v-bind="containerProps"
      class="commit-scroll"
      role="list"
      aria-label="Commit history"
    >
      <div v-bind="wrapperProps">
        <button
          v-for="row in list"
          :key="row.data.commit.oid"
          class="commit-row"
          role="listitem"
          :aria-label="`${row.data.commit.subject}, ${row.data.commit.author}${row.data.commit.parents.length > 1 ? ', merge commit' : ''}`"
          :class="{
            selected: store.selectedCommit?.oid === row.data.commit.oid,
          }"
          @click="store.selectedCommit = row.data.commit"
        >
          <svg
            class="commit-graph"
            :width="graphWidth"
            height="58"
            aria-hidden="true"
          >
            <path
              v-for="(edge, i) in row.data.graph.edges"
              :key="i"
              :d="graphEdgePath(edge, row.data.graph.lane)"
              fill="none"
              :stroke="graphColors[edge.color]"
              stroke-width="2.3"
              stroke-linecap="round"
            />
            <path
              v-if="row.data.graph.incoming"
              :d="`M ${row.data.graph.lane * 22 + 16} 0 V 29`"
              :stroke="graphColors[row.data.graph.color]"
              stroke-width="2.3"
              stroke-linecap="round"
            />
            <circle
              :cx="row.data.graph.lane * 22 + 16"
              cy="29"
              :r="row.data.commit.parents.length > 1 ? 6 : 4.5"
              :fill="graphColors[row.data.graph.color]"
              class="graph-dot"
              :class="{ 'merge-dot': row.data.commit.parents.length > 1 }"
            />
            <circle
              v-if="row.data.commit.parents.length > 1"
              :cx="row.data.graph.lane * 22 + 16"
              cy="29"
              r="2.5"
              fill="var(--graph-bg)"
            />
          </svg>
          <div class="commit-text">
            <strong>{{ row.data.commit.subject }}</strong
            ><small
              ><code>{{ row.data.commit.oid.slice(0, 7) }}</code
              ><span
                v-for="ref in refs(row.data.commit.refs)"
                :key="ref"
                class="ref-label"
                :style="{ '--ref-color': graphColors[row.data.graph.color] }"
                :title="ref"
                >{{ ref }}</span
              ></small
            >
          </div>
          <div class="commit-meta">
            <span>{{ row.data.commit.author }}</span
            ><small>{{ shortDate(row.data.commit.date) }}</small>
          </div>
        </button>
      </div>
      <div v-if="!store.commits.length" class="empty-panel">
        <GitCommitHorizontal :size="30" />
        <h2>{{ store.loading ? "Loading history…" : "No commits to show" }}</h2>
        <p>
          {{
            search || path
              ? "Try another search or path."
              : "Your first commit starts the story."
          }}
        </p>
      </div>
      <button
        v-if="store.commits.length >= 200"
        class="load-more"
        :disabled="store.loading"
        @click="store.loadHistory(true)"
      >
        <ArrowDown :size="15" />{{
          store.loading ? "Loading…" : "Load older commits"
        }}
      </button>
    </div>
  </section>
</template>
