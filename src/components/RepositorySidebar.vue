<script setup lang="ts">
import { computed } from "vue";
import {
  GitBranch,
  History,
  Files,
  Archive,
  Radio,
  ChevronDown,
  Plus,
  Tag,
  Activity,
  ShieldCheck,
  Terminal,
  FolderOpen,
  Code,
  Star,
  FolderGit2,
  GitPullRequest,
} from "lucide-vue-next";
import { call } from "../lib/api";
import { useRepositoryStore } from "../stores/repository";
const store = useRepositoryStore();
function launch(target: string) {
  store.attempt(() => call("desktop_open", { id: store.activeId, target }));
}
const branches = computed(
  () => store.snapshot?.refs.filter((v) => v.kind === "branch") ?? [],
);
const tags = computed(
  () => store.snapshot?.refs.filter((v) => v.kind === "tag") ?? [],
);
defineEmits<{
  action: [kind: string, defaults?: Record<string, string>];
  trust: [];
}>();
</script>
<template>
  <aside class="sidebar">
    <div class="repo-heading">
      <div class="repo-avatar"><GitBranch :size="20" /></div>
      <div>
        <strong>{{ store.snapshot?.name }}</strong
        ><small>{{ store.snapshot?.path }}</small>
      </div>
      <button
        class="icon-button"
        aria-label="Favorite repository"
        @click="store.favorite(store.snapshot!.path)"
      >
        <Star
          :size="15"
          :fill="
            store.preferences.favorites.includes(store.snapshot!.path)
              ? 'currentColor'
              : 'none'
          "
        />
      </button>
    </div>
    <nav aria-label="Repository views">
      <button
        :class="{ active: store.view === 'changes' }"
        @click="store.view = 'changes'"
      >
        <Files :size="17" />Working changes<span class="count">{{
          store.snapshot?.files.length
        }}</span></button
      ><button
        :class="{ active: store.view === 'history' }"
        @click="
          store.view = 'history';
          store.revision = '';
          store.loadHistory();
        "
      >
        <History :size="17" />History</button
      ><button
        :class="{ active: store.view === 'stashes' }"
        @click="store.view = 'stashes'"
      >
        <Archive :size="17" />Stashes</button
      ><button
        :class="{ active: store.view === 'reflog' }"
        @click="store.view = 'reflog'"
      >
        <Activity :size="17" />Reflog & recovery</button
      ><button
        :class="{ active: store.view === 'activity' }"
        @click="store.view = 'activity'"
      >
        <Radio :size="17" />Activity</button
      ><button
        :class="{ active: store.view === 'worktrees' }"
        @click="store.view = 'worktrees'"
      >
        <FolderGit2 :size="17" />Worktrees
      </button>
      <button
        :class="{ active: store.view === 'pullRequests' }"
        @click="store.view = 'pullRequests'"
      >
        <GitPullRequest :size="17" />Pull requests
      </button>
    </nav>
    <div class="side-scroll">
      <div class="side-label">
        <ChevronDown :size="13" />LOCAL BRANCHES<button
          class="icon-button"
          aria-label="Create branch"
          :disabled="!store.trusted"
          @click="$emit('action', 'createBranch')"
        >
          <Plus :size="14" />
        </button>
      </div>
      <button
        v-for="branch in branches"
        :key="branch.name"
        class="branch-row"
        :class="{ current: branch.name === store.snapshot?.branch }"
        @click="
          store.revision = branch.name;
          store.view = 'history';
          store.loadHistory();
        "
        @dblclick="$emit('action', 'switchBranch', { name: branch.name })"
      >
        <GitBranch :size="15" /><span>{{ branch.name }}</span
        ><span
          v-if="branch.name === store.snapshot?.branch"
          class="status-dot"
        />
      </button>
      <div class="side-label">
        <ChevronDown :size="13" />REMOTES<button
          class="icon-button"
          aria-label="Add remote"
          :disabled="!store.trusted"
          @click="$emit('action', 'addRemote')"
        >
          <Plus :size="14" />
        </button>
      </div>
      <button
        v-for="remote in store.snapshot?.remotes"
        :key="remote.name"
        class="branch-row"
        @click="
          $emit('action', 'editRemote', { name: remote.name, url: remote.url })
        "
      >
        <Radio :size="15" /><span>{{ remote.name }}</span>
      </button>
      <p v-if="!store.snapshot?.remotes.length" class="side-empty">
        No remotes configured
      </p>
      <div class="side-label">
        <ChevronDown :size="13" />TAGS<button
          class="icon-button"
          aria-label="Create tag"
          :disabled="!store.trusted"
          @click="$emit('action', 'createTag')"
        >
          <Plus :size="14" />
        </button>
      </div>
      <button
        v-for="tag in tags"
        :key="tag.name"
        class="branch-row"
        @click="
          store.revision = tag.name;
          store.view = 'history';
          store.loadHistory();
        "
      >
        <Tag :size="15" /><span>{{ tag.name }}</span>
      </button>
    </div>
    <div class="sidebar-bottom">
      <button
        v-if="!store.trusted"
        class="trust-button"
        @click="$emit('trust')"
      >
        <ShieldCheck :size="16" />Trust repository to make changes</button
      ><span v-else class="trust-state"
        ><ShieldCheck :size="14" />Trusted for this session</span
      >
      <div class="tool-row">
        <button
          class="icon-button"
          title="Open terminal"
          aria-label="Open terminal here"
          @click="launch('terminal')"
        >
          <Terminal :size="17" /></button
        ><button
          class="icon-button"
          title="Open VS Code (code must be in PATH)"
          aria-label="Open in VS Code"
          @click="launch('editor')"
        >
          <Code :size="17" /></button
        ><button
          class="icon-button"
          title="Open file manager"
          aria-label="Open in file manager"
          @click="launch('files')"
        >
          <FolderOpen :size="17" />
        </button>
      </div>
    </div>
  </aside>
</template>
