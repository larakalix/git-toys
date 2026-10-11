import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useRepositoryStore } from "../../src/stores/repository";
import { useNotificationStore } from "../../src/stores/notifications";
import type { OperationEvent, Snapshot } from "../../src/lib/api";
const transport = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, (event: unknown) => Promise<void>>(),
}));
vi.mock("../../src/lib/api", () => ({
  desktop: true,
  defaultAiSettings: () => ({
    enabled: false,
    provider: "ollama",
    model: "",
    ollamaUrl: "http://127.0.0.1:11434",
  }),
  call: (...args: unknown[]) => transport.invoke(...args),
  asError: (value: unknown) => value,
  onEvent: async (
    name: string,
    callback: (event: unknown) => Promise<void>,
  ) => {
    transport.listeners.set(name, callback);
    return () => {};
  },
}));
const snapshot = {
  stateToken: "test-state",
  warnings: [],
  head: "abc",
  name: "fixture",
  path: "/fixture",
} as unknown as Snapshot;
describe("operation feedback lifecycle", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    transport.listeners.clear();
    transport.invoke.mockReset();
    transport.invoke.mockImplementation(async (command: string) => {
      if (command === "snapshot") return snapshot;
      if (command === "history") return [];
      if (command === "load_preferences")
        return { recent: [], favorites: [], theme: "system", journal: [] };
      return "operation-id";
    });
  });
  it("shows pending push immediately and clears it after completion", async () => {
    const store = useRepositoryStore();
    await store.init();
    store.activeId = "repo";
    store.snapshot = snapshot;
    await store.mutate({
      kind: "push",
      remote: "origin",
      publish: false,
      force: false,
    });
    expect(store.busy).toBe(true);
    expect(store.runningKind).toBe("push");
    const event: OperationEvent = {
      id: "operation-id",
      repoId: "repo",
      kind: "push",
      status: "completed",
      output: "",
      error: null,
    };
    await transport.listeners.get("operation")!(event);
    expect(store.busy).toBe(false);
    expect(store.runningKind).toBe("");
    expect(useNotificationStore().notices.at(-1)?.title).toBe("Changes pushed");
  });
  it("clears pending state and retains Git output on a rejected invocation", async () => {
    const store = useRepositoryStore();
    store.activeId = "repo";
    store.snapshot = snapshot;
    transport.invoke.mockRejectedValueOnce({
      code: "rejected",
      message: "Push rejected",
      detail: "Fetch before pushing",
    });
    await store.mutate({
      kind: "push",
      remote: "origin",
      publish: false,
      force: false,
    });
    expect(store.busy).toBe(false);
    expect(store.runningKind).toBe("");
    expect(useNotificationStore().notices[0]).toMatchObject({
      kind: "error",
      title: "Push rejected",
      detail: "Fetch before pushing",
    });
  });
  it("normalizes panel widths at the IPC save boundary", async () => {
    const store = useRepositoryStore();
    store.preferences.sidebarWidth = 240.75;
    store.preferences.detailWidth = 732.17578125;
    await store.save();
    expect(transport.invoke).toHaveBeenCalledWith("save_preferences", {
      preferences: expect.objectContaining({
        sidebarWidth: 241,
        detailWidth: 732,
      }),
    });
  });
});

describe("repository navigation ordering", () => {
  it("does not let a delayed open steal focus from a newer tab selection", async () => {
    setActivePinia(createPinia());
    let resolve!: (value: unknown) => void;
    const pending = new Promise((yes) => (resolve = yes));
    transport.invoke.mockImplementation(async (command: string) => {
      if (command === "open_repository") return pending;
      if (command === "snapshot") return snapshot;
      if (command === "history") return [];
    });
    const store = useRepositoryStore();
    store.tabs = [{ id: "chosen", snapshot, trusted: true }];
    const opening = store.open("/delayed");
    await store.activate("chosen");
    resolve({
      id: "delayed",
      snapshot: { ...snapshot, path: "/delayed", name: "delayed" },
      trusted: false,
    });
    await opening;
    expect(store.activeId).toBe("chosen");
    expect(store.tabs.map((tab) => tab.id)).toEqual(["chosen", "delayed"]);
  });
});
