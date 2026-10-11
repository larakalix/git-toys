<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import AppDialog from "../AppDialog.vue";
import {
  call,
  asError,
  type PullRequest,
  type GithubSection,
} from "../../lib/api";
import { useRepositoryStore } from "../../stores/repository";
const props = defineProps<{
  request: PullRequest;
  repository: string;
  remote: string;
}>();
const store = useRepositoryStore(),
  section = ref("checks"),
  page = ref(1),
  result = shallowRef<GithubSection>(),
  loading = ref(false),
  error = ref(""),
  confirm = ref(false),
  expected = ref("");
let version = 0;
const checks = computed(() =>
  section.value === "checks" ? (result.value?.items ?? []) : [],
);
const checkSummary = computed(() => {
  if (section.value !== "checks" || !result.value) return "";
  if (result.value.more || page.value > 1)
    return "Showing a page of checks; this is not the complete CI result.";
  if (!checks.value.length) return "No checks reported.";
  if (
    checks.value.some(
      (v) =>
        [
          "failure",
          "error",
          "cancelled",
          "timed_out",
          "action_required",
          "startup_failure",
        ].includes(v.status) ||
        [
          "failure",
          "cancelled",
          "timed_out",
          "action_required",
          "startup_failure",
        ].includes(v.detail),
    )
  )
    return "Some checks failed or need attention.";
  if (
    checks.value.some((v) =>
      ["queued", "in_progress", "pending", "waiting", "requested"].includes(
        v.status,
      ),
    )
  )
    return "Checks are still running.";
  return "Review individual check results below.";
});
async function load() {
  const current = ++version;
  loading.value = true;
  result.value = undefined;
  error.value = "";
  try {
    const next = await call<GithubSection>("github_section", {
      id: store.activeId,
      remote: props.remote,
      number: props.request.number,
      section: section.value,
      page: page.value,
    });
    if (current === version) result.value = next;
  } catch (e) {
    if (current === version) {
      error.value = asError(e).message;
      store.fail(e);
    }
  } finally {
    if (current === version) loading.value = false;
  }
}
watch(
  [
    () => props.request.head,
    () => props.request.number,
    () => props.remote,
    () => store.activeId,
  ],
  () => {
    ++version;
    confirm.value = false;
    section.value = "checks";
    page.value = 1;
    load();
  },
  { immediate: true },
);
watch([section, page], load);
function choose(value: string) {
  section.value = value;
  page.value = 1;
}
async function checkout() {
  await store.mutate(
    {
      kind: "checkoutPullRequest",
      remote: props.remote,
      number: props.request.number,
      head: props.request.head,
    },
    expected.value,
  );
  confirm.value = false;
}
</script>
<template>
  <section class="pull-request-details">
    <h2>#{{ request.number }} {{ request.title }}</h2>
    <p>
      {{ request.author }} · {{ request.draft ? "Draft" : request.state }} ·
      {{ request.mergeability }}
    </p>
    <p>
      <code>{{ request.headLabel }}</code> → <code>{{ request.base }}</code> ·
      <code>{{ request.head.slice(0, 12) }}</code>
    </p>
    <div class="feature-toolbar">
      <button
        class="button"
        @click="
          store.attempt(() =>
            call('github_open', { repository, number: request.number }),
          )
        "
      >
        Open on GitHub</button
      ><button
        class="button primary"
        :disabled="
          !store.trusted ||
          store.busy ||
          !request.head ||
          !!store.snapshot?.files.length ||
          !!store.snapshot?.operation
        "
        @click="
          expected = store.snapshot!.stateToken;
          confirm = true;
        "
      >
        Check out for review
      </button>
    </div>
    <details>
      <summary>Description</summary>
      <p class="pr-description">
        {{ request.body || "No description provided." }}
      </p>
      <p class="muted">
        Displayed as plain text. Remote images and HTML are disabled.
      </p>
    </details>
    <div class="feature-toolbar" aria-label="Pull request details">
      <button
        v-for="value in ['checks', 'files', 'commits', 'reviews']"
        :key="value"
        class="button"
        :aria-pressed="section === value"
        @click="choose(value)"
      >
        {{ value === "files" ? "Changed files" : value }}</button
      ><button class="button" :disabled="loading" @click="load">Refresh</button>
    </div>
    <p v-if="loading" role="status">Loading {{ section }}…</p>
    <p v-if="error" role="alert">{{ error }}</p>
    <p v-if="checkSummary" role="status">{{ checkSummary }}</p>
    <article
      v-for="(item, index) in result?.items"
      :key="`${section}-${index}`"
      class="pr-item"
    >
      <strong>{{ item.name }}</strong
      ><span> {{ item.status }} </span
      ><code v-if="item.oid">{{ item.oid.slice(0, 12) }}</code>
      <p class="pr-description">{{ item.detail }}</p>
      <pre v-if="item.patch" class="preview-patch" tabindex="0">{{
        item.patch
      }}</pre>
      <p v-else-if="section === 'files'" class="muted">
        GitHub did not provide a text patch for this file. Open on GitHub for
        binary or truncated changes.
      </p>
    </article>
    <p v-if="result && !result.items.length" class="muted">
      No {{ section }} on this page.
    </p>
    <div class="feature-toolbar">
      <button class="button" :disabled="page === 1 || loading" @click="page--">
        Previous</button
      ><span>Page {{ page }}</span
      ><button
        class="button"
        :disabled="!result?.more || loading"
        @click="page++"
      >
        Next
      </button>
    </div>
    <AppDialog
      v-model:open="confirm"
      title="Check out pull request"
      :description="`Fetch #${request.number} and switch to review/pr-${request.number}-${request.head.slice(0, 12)} at ${request.head}. Existing review branches are never overwritten.`"
      ><p>
        Current branch: {{ store.snapshot?.branch }}. Git Toys verifies the
        fetched commit before switching.
      </p>
      <footer>
        <button class="button primary" :disabled="store.busy" @click="checkout">
          Fetch and check out for review
        </button>
      </footer></AppDialog
    >
  </section>
</template>
