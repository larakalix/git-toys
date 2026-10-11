<script setup lang="ts">
import {
  FolderOpen,
  GitFork,
  Plus,
  ArrowUpRight,
  Star,
  GitBranch,
} from "lucide-vue-next";
import { useRepositoryStore } from "../stores/repository";
import { desktop } from "../lib/api";
const store = useRepositoryStore();
defineEmits<{ open: []; create: [mode: "clone" | "init"] }>();
</script>
<template>
  <div class="welcome">
    <div class="welcome-intro">
      <div>
        <div class="eyebrow">
          <GitBranch :size="14" aria-hidden="true" />Your local Git workspace
        </div>
        <h1>Understand your code.<br /><span>Move it forward.</span></h1>
      </div>
      <p>
        Review changes, explore history, and work across branches. Everything
        you need for your next commit, in one place.
      </p>
    </div>
    <div v-if="!desktop" class="notice">
      Browser preview · Launch the desktop app to work with local Git
      repositories.
    </div>
    <div class="welcome-actions">
      <button
        class="welcome-open"
        aria-label="Open repository"
        :disabled="!desktop"
        @click="$emit('open')"
      >
        <FolderOpen :size="20" aria-hidden="true" /><strong
          >Open repository</strong
        ><small>Continue in a local project.</small></button
      ><button
        aria-label="Clone repository"
        :disabled="!desktop"
        @click="$emit('create', 'clone')"
      >
        <GitFork :size="20" aria-hidden="true" /><strong
          >Clone repository</strong
        ><small>Bring a remote project to your computer.</small></button
      ><button
        aria-label="Initialize new"
        :disabled="!desktop"
        @click="$emit('create', 'init')"
      >
        <Plus :size="20" aria-hidden="true" /><strong>Initialize new</strong
        ><small>Start tracking a project with Git.</small>
      </button>
    </div>
    <section class="recent">
      <div class="section-heading">
        <h2>Recent repositories</h2>
        <span>{{ store.preferences.recent.length }} repositories</span>
      </div>
      <div v-if="!store.preferences.recent.length" class="empty-recent">
        <GitBranch :size="20" /><span
          >Your recent repositories will appear here.<small
            >Everything stays on your computer. No account needed.</small
          ></span
        >
      </div>
      <div
        v-for="path in store.preferences.recent"
        :key="path"
        class="recent-row"
      >
        <button class="recent-open" @click="store.open(path)">
          <FolderOpen :size="20" /><span
            ><strong>{{ path.split(/[\\/]/).pop() }}</strong
            ><small>{{ path }}</small></span
          ><ArrowUpRight :size="16" /></button
        ><button
          class="icon-button"
          :aria-label="`${store.preferences.favorites.includes(path) ? 'Unfavorite' : 'Favorite'} ${path}`"
          @click="store.favorite(path)"
        >
          <Star
            :size="16"
            :fill="
              store.preferences.favorites.includes(path)
                ? 'currentColor'
                : 'none'
            "
          />
        </button>
      </div>
    </section>
    <footer class="welcome-footer">
      <span class="status-dot" />Local-first, always<span>macOS + Windows</span>
    </footer>
  </div>
</template>
