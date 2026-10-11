<script setup lang="ts">
import type { PreviewResult } from "../../lib/api";
defineProps<{ result: PreviewResult }>();
</script>
<template>
  <section class="preview-summary" aria-label="Operation preview">
    <p>
      <strong>{{ result.kind === "merge" ? "Merge" : "Rebase" }}</strong> ·
      current <code>{{ result.head.slice(0, 12) }}</code> · target
      <code>{{ result.target.slice(0, 12) }}</code>
    </p>
    <p>
      {{ result.files.length }} affected files ·
      {{ result.conflicts.length }} predicted conflicts
    </p>
    <p v-if="result.remainingUntested" role="status">
      Stopped at
      {{
        result.stoppedCommit?.slice(0, 12) || "the first conflicting commit"
      }}. Subsequent commits are untested.
    </p>
    <ul v-if="result.conflicts.length">
      <li v-for="file in result.conflicts" :key="file">Conflict: {{ file }}</li>
    </ul>
    <details>
      <summary>{{ result.commits.length }} commits (up to 200 shown)</summary>
      <p v-for="commit in result.commits" :key="commit.oid">
        <code>{{ commit.oid.slice(0, 8) }}</code> {{ commit.subject }}
      </p>
    </details>
    <details>
      <summary>Affected files</summary>
      <p v-for="file in result.files" :key="file">{{ file }}</p>
    </details>
    <pre class="preview-patch" tabindex="0" aria-label="Predicted changes">{{
      result.patch || "No file differences predicted."
    }}</pre>
    <p class="muted">
      This simulation uses committed content. Real execution can still fail
      because of hooks, signing, permissions, or external changes.
    </p>
  </section>
</template>
