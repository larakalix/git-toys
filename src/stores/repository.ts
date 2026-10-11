import { computed, ref, shallowRef, watch } from "vue";
import { defineStore } from "pinia";
import { useNotificationStore } from "./notifications";
import { normalizePanelWidth } from "../lib/layout";
import { operationSuccess } from "../lib/actions";
import { call, asError, onEvent, desktop, defaultAiSettings } from "../lib/api";
import type {
  Action,
  Snapshot,
  Commit,
  GitError,
  Opened,
  Diagnostic,
  Preferences,
  OperationEvent,
  AuthPrompt,
} from "../lib/api";
export const useRepositoryStore = defineStore("repositories", () => {
  const notifications = useNotificationStore();
  const pendingKind = shallowRef("");
  const currentOperation = computed(() =>
    [...operations.value]
      .reverse()
      .find((operation) => operation.status === "running"),
  );
  const runningKind = computed(
    () => pendingKind.value || currentOperation.value?.kind || "",
  );
  const tabs = ref<Opened[]>([]),
    activeId = shallowRef(""),
    snapshot = shallowRef<Snapshot | null>(null),
    commits = shallowRef<Commit[]>([]);
  const error = shallowRef<GitError | null>(null),
    busy = shallowRef(false),
    loading = shallowRef(false),
    view = shallowRef<
      | "history"
      | "changes"
      | "reflog"
      | "activity"
      | "stashes"
      | "worktrees"
      | "pullRequests"
    >("history");
  const query = shallowRef(""),
    revision = shallowRef(""),
    pathFilter = shallowRef(""),
    selectedCommit = shallowRef<Commit | null>(null);
  const preferences = ref<Preferences>({
    commitGroups: [],
    recent: [],
    favorites: [],
    theme: "system",
    colorTheme: "teal",
    ai: defaultAiSettings(),
    sidebarWidth: 240,
    detailWidth: 440,
    journal: [],
  });
  const operations = shallowRef<OperationEvent[]>([]),
    authQueue = ref<AuthPrompt[]>([]),
    diagnostic = shallowRef<Diagnostic | null>(null);
  const active = computed(() =>
      tabs.value.find((t) => t.id === activeId.value),
    ),
    trusted = computed(() => active.value?.trusted ?? false);
  const seenWarnings = new Map<string, Set<string>>();
  watch(snapshotWarnings, () => {
    const id = activeId.value;
    const warnings = snapshot.value?.warnings ?? [];
    const previous = seenWarnings.get(id) ?? new Set<string>();
    for (const message of warnings)
      if (!previous.has(message))
        notifications.notify({
          kind: "warning",
          title: "Repository warning",
          message,
          key: `warning:${id}:${message}`,
        });
    seenWarnings.set(id, new Set(warnings));
  });
  function snapshotWarnings() {
    return snapshot.value?.warnings;
  }
  let navigationVersion = 0;
  let refreshVersion = 0,
    historyVersion = 0;
  function fail(e: unknown) {
    error.value = asError(e);
    notifications.notify({
      kind: "error",
      title: error.value.message,
      detail: error.value.detail,
      key: `error:${error.value.code}:${error.value.message}:${error.value.detail}`,
    });
  }
  async function attempt<T>(fn: () => Promise<T>) {
    try {
      return await fn();
    } catch (e) {
      fail(e);
      return undefined;
    }
  }
  async function save() {
    preferences.value.sidebarWidth = normalizePanelWidth(
      "sidebarWidth",
      preferences.value.sidebarWidth,
    );
    preferences.value.detailWidth = normalizePanelWidth(
      "detailWidth",
      preferences.value.detailWidth,
    );
    await attempt(() =>
      call("save_preferences", { preferences: preferences.value }),
    );
  }
  async function open(path: string) {
    const navigation = ++navigationVersion;
    await attempt(async () => {
      const repo = await call<Opened>("open_repository", { path });
      await adopt(repo, navigation === navigationVersion);
      notifications.notify({
        kind: "info",
        title: "Repository opened",
        message: `${repo.snapshot.name} · ${tabs.value.find((tab) => tab.id === repo.id)?.trusted ? "Trusted for this session" : "Read-only until trusted"}`,
      });
    });
  }
  async function adopt(repo: Opened, select = true) {
    if (!tabs.value.some((t) => t.id === repo.id)) tabs.value.push(repo);
    preferences.value.recent = [
      repo.snapshot.path,
      ...preferences.value.recent.filter((p) => p !== repo.snapshot.path),
    ].slice(0, 30);
    if (select) await activate(repo.id);
    await save();
  }
  async function activate(id: string) {
    ++navigationVersion;
    activeId.value = id;
    snapshot.value = tabs.value.find((t) => t.id === id)?.snapshot ?? null;
    commits.value = [];
    selectedCommit.value = null;
    revision.value = "";
    query.value = "";
    pathFilter.value = "";
    await refresh(true);
  }
  async function close(id: string) {
    await attempt(() => call("close_repository", { id }));
    tabs.value = tabs.value.filter((t) => t.id !== id);
    if (activeId.value === id) {
      activeId.value = "";
      snapshot.value = null;
      commits.value = [];
      if (tabs.value[0]) await activate(tabs.value[0].id);
    }
  }
  async function refresh(force = false, afterOperation = false) {
    const id = activeId.value;
    if (!id || (busy.value && !afterOperation)) return;
    const version = ++refreshVersion;
    await attempt(async () => {
      const next = await call<Snapshot>("snapshot", { id });
      if (id !== activeId.value || version !== refreshVersion) return;
      const changed = next.stateToken !== snapshot.value?.stateToken;
      snapshot.value = next;
      const tab = tabs.value.find((t) => t.id === id);
      if (tab) tab.snapshot = next;
      if (changed || force) await loadHistory();
    });
  }
  async function loadHistory(more = false) {
    const id = activeId.value;
    if (!id) return;
    const version = ++historyVersion;
    loading.value = true;
    try {
      const rows = await call<Commit[]>("history", {
        id,
        query: {
          revision: revision.value || null,
          search: query.value || null,
          path: pathFilter.value || null,
          skip: more ? commits.value.length : 0,
          limit: 200,
        },
      });
      if (id !== activeId.value || version !== historyVersion) return;
      commits.value = more ? [...commits.value, ...rows] : rows;
    } catch (e) {
      fail(e);
    } finally {
      if (version === historyVersion) loading.value = false;
    }
  }
  async function trust() {
    const id = activeId.value;
    await attempt(async () => {
      await call("trust_repository", { id, trusted: true });
      const tab = tabs.value.find((t) => t.id === id);
      if (tab) tab.trusted = true;
      notifications.notify({
        kind: "info",
        title: "Repository trusted",
        message: "Git operations are enabled for this app session.",
      });
    });
  }
  async function mutate(action: Action, expected?: string) {
    if (!snapshot.value || busy.value) return;
    const id = activeId.value;
    busy.value = true;
    pendingKind.value = action.kind;
    error.value = null;
    try {
      await call<string>("mutate", {
        id,
        action,
        expected: expected ?? snapshot.value.stateToken,
      });
    } catch (e) {
      pendingKind.value = "";
      busy.value = false;
      fail(e);
      await refresh(true);
    }
  }
  async function create(path: string, url: string | null) {
    busy.value = true;
    pendingKind.value = url ? "clone" : "init";
    error.value = null;
    try {
      const repo = await call<Opened>("create_repository", {
        path,
        url,
        trusted: true,
      });
      busy.value = false;
      pendingKind.value = "";
      await adopt(repo);
      notifications.notify({
        kind: "success",
        title: url ? "Repository cloned" : "Repository initialized",
        message: repo.snapshot.name,
      });
    } catch (e) {
      pendingKind.value = "";
      busy.value = false;
      fail(e);
    }
  }
  async function favorite(path: string) {
    const values = preferences.value.favorites;
    preferences.value.favorites = values.includes(path)
      ? values.filter((p) => p !== path)
      : [...values, path];
    await save();
  }
  async function init() {
    if (!desktop) return;
    await attempt(async () => {
      diagnostic.value = await call<Diagnostic>("git_diagnostics");
      preferences.value = {
        ...preferences.value,
        ...(await call<Preferences>("load_preferences")),
      };
      preferences.value.ai = {
        ...defaultAiSettings(),
        ...preferences.value.ai,
      };
    });
    const unlisten = await onEvent<OperationEvent>(
      "operation",
      async (event) => {
        operations.value = [
          ...operations.value.filter((v) => v.id !== event.id),
          event,
        ].slice(-100);
        if (event.status !== "running") {
          if (event.error) fail(event.error);
          else if (event.status === "failed")
            fail(new Error(`${event.kind} failed`));
          else
            notifications.notify({
              kind: "success",
              title: operationSuccess(event.kind),
              message: tabs.value.find((tab) => tab.id === event.repoId)
                ?.snapshot.name,
            });
          if (event.status === "completed" && event.removedPath) {
            const removed = tabs.value.filter(
              (tab) => tab.snapshot.path === event.removedPath,
            );
            for (const tab of removed) await close(tab.id);
          }
          await refresh(true, true);
          busy.value = false;
          pendingKind.value = "";
          await attempt(async () => {
            preferences.value = {
              ...preferences.value,
              ...(await call<Preferences>("load_preferences")),
            };
            preferences.value.ai = {
              ...defaultAiSettings(),
              ...preferences.value.ai,
            };
          });
        }
      },
    );
    let changeTimer: ReturnType<typeof setTimeout> | undefined;
    const unChange = await onEvent<string>("repository-changed", (id) => {
      if (id !== activeId.value) return;
      if (changeTimer) clearTimeout(changeTimer);
      changeTimer = setTimeout(() => refresh(), 300);
    });
    const unAuth = await onEvent<AuthPrompt>("auth-prompt", (payload) =>
      authQueue.value.push(payload),
    );
    return () => {
      unlisten();
      unAuth();
      unChange();
      if (changeTimer) clearTimeout(changeTimer);
    };
  }
  async function reply(value: string | null) {
    const prompt = authQueue.value[0];
    if (!prompt) return;
    await attempt(() => call("auth_reply", { id: prompt.id, value }));
    authQueue.value.shift();
  }
  return {
    tabs,
    activeId,
    snapshot,
    commits,
    error,
    busy,
    loading,
    view,
    query,
    revision,
    pathFilter,
    selectedCommit,
    preferences,
    operations,
    runningKind,
    currentOperation,
    authQueue,
    diagnostic,
    active,
    trusted,
    fail,
    attempt,
    save,
    open,
    adopt,
    activate,
    close,
    refresh,
    loadHistory,
    trust,
    mutate,
    create,
    favorite,
    init,
    reply,
  };
});
