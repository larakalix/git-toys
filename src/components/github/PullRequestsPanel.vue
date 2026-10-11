<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import {
  call,
  asError,
  type PullRequest,
  type PullRequestPage,
} from "../../lib/api";
import { githubRepository } from "../../lib/github";
import { useRepositoryStore } from "../../stores/repository";
import GithubConnection from "./GithubConnection.vue";
import PullRequestDetails from "./PullRequestDetails.vue";
const store = useRepositoryStore(),
  remote = ref(""),
  state = ref("open"),
  search = ref(""),
  page = ref(1),
  rows = shallowRef<PullRequestPage>(),
  selected = shallowRef<PullRequest>(),
  loading = ref(false),
  error = ref("");
let version = 0,
  detailVersion = 0;
const remotes = computed(
  () => store.snapshot?.remotes.filter((r) => githubRepository(r.url)) ?? [],
);
const repository = computed(
  () =>
    githubRepository(
      remotes.value.find((r) => r.name === remote.value)?.url ?? "",
    ) ?? "",
);
watch(
  () => store.activeId,
  () => {
    ++version;
    ++detailVersion;
    remote.value = remotes.value.length === 1 ? remotes.value[0]!.name : "";
    selected.value = undefined;
    rows.value = undefined;
    page.value = 1;
  },
  { immediate: true },
);
watch([remote, state], () => {
  page.value = 1;
  load();
});
async function load() {
  const current = ++version;
  ++detailVersion;
  rows.value = undefined;
  selected.value = undefined;
  error.value = "";
  if (!repository.value) return;
  loading.value = true;
  try {
    const next = await call<PullRequestPage>("github_pulls", {
      id: store.activeId,
      remote: remote.value,
      query: { state: state.value, search: search.value, page: page.value },
    });
    if (current === version) rows.value = next;
  } catch (e) {
    if (current === version) {
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === version) loading.value = false;
  }
}
async function select(number: number) {
  const current = ++detailVersion;
  selected.value = undefined;
  error.value = "";
  try {
    const detail = await call<PullRequest>("github_pull", {
      id: store.activeId,
      remote: remote.value,
      number,
    });
    if (current === detailVersion) selected.value = detail;
  } catch (e) {
    if (current === detailVersion) {
      error.value = asError(e).message;
      store.fail(e);
    }
  }
}
function turn(delta: number) {
  page.value += delta;
  load();
}
// The initial remote is assigned before the remote watcher is installed.
if (remote.value) void load();
</script>
<template>
  <section class="wide-panel pull-requests-panel">
    <header class="panel-heading">
      <div>
        <h1>Pull requests</h1>
        <p>Review GitHub changes and check out a branch locally.</p>
      </div>
    </header>
    <div class="pull-requests-content">
      <details>
        <summary>GitHub connection</summary>
        <GithubConnection />
      </details>
      <p v-if="!remotes.length">
        Add a GitHub.com HTTPS or SSH remote to use pull requests. GitLab and
        Enterprise hosting are not supported yet.
      </p>
      <form
        v-else
        class="feature-toolbar"
        @submit.prevent="
          page = 1;
          load();
        "
      >
        <label
          >Repository<select v-model="remote">
            <option value="">Choose a remote</option>
            <option v-for="item in remotes" :key="item.name" :value="item.name">
              {{ item.name }} · {{ githubRepository(item.url) }}
            </option>
          </select></label
        ><label
          >State<select v-model="state">
            <option value="open">Open</option>
            <option value="closed">Closed</option>
            <option value="all">All</option>
          </select></label
        ><label
          >Search<input
            v-model="search"
            maxlength="200"
            placeholder="Search title and description" /></label
        ><button class="button" :disabled="!repository || loading">
          {{ loading ? "Loading…" : "Search / refresh" }}
        </button>
      </form>
      <p v-if="error" role="alert">{{ error }}</p>
      <div class="feature-split">
        <div class="feature-list">
          <button
            v-for="item in rows?.items"
            :key="item.number"
            class="button pr-list-item"
            :aria-pressed="selected?.number === item.number"
            @click="select(item.number)"
          >
            <strong>#{{ item.number }} {{ item.title }}</strong
            ><small
              >{{ item.author }} ·
              {{ item.draft ? "Draft" : item.state }}</small
            >
          </button>
          <p v-if="rows && !rows.items.length">No pull requests found.</p>
          <div class="feature-toolbar" v-if="rows">
            <button
              class="button"
              :disabled="page === 1 || loading"
              @click="turn(-1)"
            >
              Previous</button
            ><span>{{ page }}</span
            ><button
              class="button"
              :disabled="!rows.more || loading"
              @click="turn(1)"
            >
              Next
            </button>
          </div>
        </div>
        <PullRequestDetails
          v-if="selected"
          :request="selected"
          :repository="repository"
          :remote="remote"
        />
        <p v-else class="muted">
          Select a pull request to inspect its checks, commits, reviews, and
          changed files.
        </p>
      </div>
    </div>
  </section>
</template>
