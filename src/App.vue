<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, shallowRef, watch } from "vue";
import { usePreferredDark } from "@vueuse/core";
import {
  GitFork,
  Plus,
  Command,
  Settings2,
  X,
  RefreshCw,
  ShieldCheck,
  LoaderCircle,
} from "lucide-vue-next";
import { open as directory } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "./stores/repository";
import { desktop } from "./lib/api";
import NotificationViewport from "./components/NotificationViewport.vue";
import EmptyWorkspace from "./components/EmptyWorkspace.vue";
import Workspace from "./components/Workspace.vue";
import AppDialog from "./components/AppDialog.vue";
import GithubConnection from "./components/github/GithubConnection.vue";
import AiSettings from "./components/AiSettings.vue";
import ColorThemePicker from "./components/ColorThemePicker.vue";
import { applyColorTheme } from "./lib/themes";
import ActionDialog from "./components/ActionDialog.vue";
import CreateRepositoryDialog from "./components/CreateRepositoryDialog.vue";
import CommandPalette from "./components/CommandPalette.vue";
import ConflictEditor from "./components/ConflictEditor.vue";
const store = useRepositoryStore(),
  preferredDark = usePreferredDark();
const palette = shallowRef(false),
  settings = shallowRef(false),
  trust = shallowRef(false),
  create = shallowRef(false),
  createMode = shallowRef<"clone" | "init">("clone");
const actionOpen = shallowRef(false),
  actionKind = shallowRef(""),
  actionDefaults = shallowRef<Record<string, unknown>>({}),
  conflictOpen = shallowRef(false),
  conflictPath = shallowRef(""),
  secret = shallowRef("");
const theme = computed(() =>
  store.preferences.theme === "system"
    ? preferredDark.value
      ? "dark"
      : "light"
    : store.preferences.theme,
);
watch(
  [() => store.preferences.colorTheme, theme],
  ([palette, appearance]) => {
    applyColorTheme(document.documentElement, palette, appearance);
  },
  { immediate: true },
);
async function openRepository() {
  await store.attempt(async () => {
    const path = await directory({
      directory: true,
      title: "Open Git repository",
    });
    if (typeof path === "string") await store.open(path);
  });
}
function createRepository(mode: "clone" | "init") {
  createMode.value = mode;
  create.value = true;
}
function action(kind: string, defaults?: Record<string, unknown>) {
  if (!store.trusted) {
    trust.value = true;
    return;
  }
  actionKind.value = kind;
  actionDefaults.value = defaults ?? {};
  actionOpen.value = true;
}
function keydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    palette.value = !palette.value;
  }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "o") {
    event.preventDefault();
    openRepository();
  }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "r") {
    event.preventDefault();
    store.refresh(true);
  }
}
let interval: ReturnType<typeof setInterval> | undefined,
  unlisten: (() => void) | undefined;
const focus = () => store.refresh();
onMounted(async () => {
  unlisten = await store.init();
  window.addEventListener("keydown", keydown);
  window.addEventListener("focus", focus);
  interval = setInterval(() => {
    if (document.visibilityState === "visible") store.refresh();
  }, 3000);
});
onBeforeUnmount(() => {
  unlisten?.();
  window.removeEventListener("keydown", keydown);
  window.removeEventListener("focus", focus);
  if (interval) clearInterval(interval);
});
const authOpen = computed(() => store.authQueue.length > 0);
async function authenticate(value: string | null) {
  await store.reply(value);
  secret.value = "";
}
</script>
<template>
  <div class="app-shell">
    <a class="skip-link" href="#main">Skip to workspace</a>
    <header class="app-header">
      <div class="brand">
        <span class="brand-symbol"><GitFork :size="20" /></span
        ><strong>git toys<span class="brand-period">.</span></strong>
      </div>
      <div class="repo-tabs" role="tablist" aria-label="Open repositories">
        <div
          v-for="tab in store.tabs"
          :key="tab.id"
          class="repo-tab"
          :class="{ selected: tab.id === store.activeId }"
        >
          <button
            role="tab"
            :aria-selected="tab.id === store.activeId"
            @click="store.activate(tab.id)"
          >
            {{ tab.snapshot.name }}</button
          ><button
            class="tab-close"
            :aria-label="`Close ${tab.snapshot.name}`"
            @click="store.close(tab.id)"
          >
            <X :size="12" />
          </button>
        </div>
        <button
          class="icon-button"
          :disabled="!desktop"
          aria-label="Open repository"
          title="Open repository (⌘/Ctrl O)"
          @click="openRepository"
        >
          <Plus :size="18" />
        </button>
      </div>
      <div class="header-tools">
        <button class="palette-trigger" @click="palette = true">
          <Command :size="14" /><span>Commands</span><kbd>⌘ K</kbd></button
        ><button
          class="icon-button"
          aria-label="Settings and Git diagnostics"
          @click="settings = true"
        >
          <Settings2 :size="18" />
        </button>
      </div>
    </header>
    <main id="main" tabindex="-1">
      <Workspace
        v-if="store.snapshot"
        :theme="theme"
        @action="action"
        @trust="trust = true"
        @conflict="
          (path) => {
            conflictPath = path;
            conflictOpen = true;
          }
        "
      /><EmptyWorkspace
        v-else
        @open="openRepository"
        @create="createRepository"
      />
    </main>
    <footer class="status-bar">
      <div>
        <LoaderCircle v-if="store.busy" :size="13" class="spin" /><span
          v-else
          class="status-dot"
        /><span>{{
          store.busy
            ? store.runningKind
              ? `Running ${store.runningKind}…`
              : "Working…"
            : store.snapshot
              ? "Ready"
              : "Your next commit starts here"
        }}</span>
      </div>
      <div>
        <span v-if="store.snapshot"
          ><code>{{ store.snapshot.head.slice(0, 7) }}</code
          ><button
            class="icon-button"
            aria-label="Refresh repository"
            :disabled="store.busy"
            @click="store.refresh(true)"
          >
            <RefreshCw :size="12" /></button></span
        ><span>{{ store.diagnostic?.version ?? "Local-first Git client" }}</span
        ><span>v0.1.0 · Preview</span>
      </div>
    </footer>
    <NotificationViewport />
    <ActionDialog
      v-model:open="actionOpen"
      :kind="actionKind"
      :defaults="actionDefaults"
    /><CreateRepositoryDialog
      v-model:open="create"
      :mode="createMode"
    /><CommandPalette
      v-model:open="palette"
      @action="action"
      @open="openRepository"
    /><ConflictEditor
      v-model:open="conflictOpen"
      :path="conflictPath"
    /><AppDialog
      v-model:open="trust"
      title="Trust this repository?"
      description="Git repositories can configure hooks, filters, credential helpers, and signing programs. These can execute software on your computer."
      ><p class="notice">{{ store.snapshot?.path }}</p>
      <p class="muted">
        Only trust sources you know. Trust is limited to this app session;
        read-only browsing remains available without it.
      </p>
      <footer class="dialog-actions">
        <button @click="trust = false">Keep read-only</button
        ><button
          class="primary"
          @click="
            store.trust();
            trust = false;
          "
        >
          <ShieldCheck :size="16" />Trust repository
        </button>
      </footer></AppDialog
    ><AppDialog
      v-model:open="settings"
      content-class="settings-dialog"
      title="Make yourself at home"
      description="Preferences stay on this computer."
      ><label class="field"
        >Appearance<select
          v-model="store.preferences.theme"
          @change="store.save()"
        >
          <option value="system">Follow system</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select></label
      >
      <ColorThemePicker
        v-model="store.preferences.colorTheme"
        @update:model-value="store.save()"
      />
      <AiSettings />
      <GithubConnection />
      <section class="diagnostics">
        <h3>Git diagnostics</h3>
        <code>{{ store.diagnostic?.version ?? "Desktop app required" }}</code>
        <p>
          {{
            store.diagnostic?.message ??
            "Browser preview cannot access your local Git installation. Run pnpm tauri dev."
          }}
        </p>
        <p>
          Uses your existing SSH agent, credential helpers, hooks, and signing
          configuration in trusted repositories. VS Code requires the code
          command in PATH.
        </p>
        <h3>About this preview</h3>
        <p>
          Free and closed source. No account or telemetry. Update checks will be
          enabled when a public release feed is configured.
        </p>
      </section></AppDialog
    ><AppDialog
      :open="authOpen"
      title="Git authentication"
      :description="store.authQueue[0]?.prompt"
      @update:open="
        (value) => {
          if (!value) authenticate(null);
        }
      "
      ><form @submit.prevent="authenticate(secret)">
        <label class="field"
          >Response<input
            v-model="secret"
            type="password"
            autocomplete="off"
            autofocus
            required
        /></label>
        <p class="muted">
          Sent only to the requesting local Git process. Not saved in
          preferences or the activity journal.
        </p>
        <footer class="dialog-actions">
          <button type="button" @click="authenticate(null)">Cancel</button
          ><button class="primary">Continue</button>
        </footer>
      </form></AppDialog
    >
  </div>
</template>
