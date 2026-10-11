import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import DiffPanel from "../../src/components/DiffPanel.vue";
import { useRepositoryStore } from "../../src/stores/repository";
import type { Diff, Snapshot } from "../../src/lib/api";
const transport = vi.hoisted(() => ({
  invoke: vi.fn(),
  mounted: vi.fn(),
  unmounted: vi.fn(),
}));
vi.mock("../../src/lib/api", async (original) => ({
  ...(await original<object>()),
  call: (...args: unknown[]) => transport.invoke(...args),
}));
vi.mock("../../src/components/CodeEditor.vue", async () => {
  const { defineComponent, onMounted, onBeforeUnmount } = await import("vue");
  return {
    __esModule: true,
    default: defineComponent({
      props: ["original", "modified"],
      setup() {
        onMounted(transport.mounted);
        onBeforeUnmount(transport.unmounted);
      },
      template: '<div class="test-editor">{{ modified }}</div>',
    }),
  };
});
const initial: Diff = {
  path: "hello.txt",
  original: "before\n",
  modified: "after\n",
  patch: "patch",
  hunks: ["@@ -1 +1 @@\n-before\n+after\n"],
  binary: false,
  truncated: false,
};
function deferred<T>() {
  let resolve!: (value: T) => void, reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
async function setup() {
  const store = useRepositoryStore();
  store.activeId = "repo";
  store.tabs = [{ id: "repo", trusted: true, snapshot: {} as Snapshot }];
  store.snapshot = { stateToken: "a", warnings: [] } as unknown as Snapshot;
  const panel = mount(DiffPanel, {
    props: { path: "hello.txt", theme: "light" },
  });
  await flushPromises();
  return { store, panel };
}
describe("diff refresh stability", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    transport.invoke.mockReset();
    transport.mounted.mockClear();
    transport.unmounted.mockClear();
    transport.invoke.mockResolvedValue(initial);
  });
  it("keeps the editor and scroll position through unchanged polling snapshots", async () => {
    const { store, panel } = await setup();
    const editor = panel.get(".test-editor").element;
    editor.scrollTop = 123;
    for (let i = 0; i < 4; i++) {
      store.snapshot = { ...store.snapshot! };
      await nextTick();
    }
    await flushPromises();
    expect(transport.invoke).toHaveBeenCalledTimes(1);
    expect(transport.mounted).toHaveBeenCalledTimes(1);
    expect(transport.unmounted).not.toHaveBeenCalled();
    expect(panel.get(".test-editor").element).toBe(editor);
    expect(editor.scrollTop).toBe(123);
    panel.unmount();
  });
  it("retains content while refreshing, disables actions, and updates without a remount", async () => {
    const { store, panel } = await setup();
    const editor = panel.get(".test-editor").element;
    const pending = deferred<Diff>();
    transport.invoke.mockReturnValueOnce(pending.promise);
    store.snapshot = { ...store.snapshot!, stateToken: "b" };
    await nextTick();
    expect(panel.get(".test-editor").text()).toBe("after");
    expect(panel.text()).not.toContain("Loading diff…");
    expect(
      panel.get(".diff-actions > button").attributes("disabled"),
    ).toBeDefined();
    pending.resolve({ ...initial, modified: "new text\n" });
    await flushPromises();
    expect(panel.get(".test-editor").element).toBe(editor);
    expect(panel.get(".test-editor").text()).toBe("new text");
    expect(
      panel.get(".diff-actions > button").attributes("disabled"),
    ).toBeUndefined();
    expect(transport.mounted).toHaveBeenCalledTimes(1);
    panel.unmount();
  });
  it("keeps failed refresh content read-only until retry succeeds", async () => {
    const { store, panel } = await setup();
    transport.invoke.mockRejectedValueOnce({
      code: "io",
      message: "Unable to read diff",
      detail: "File unavailable",
    });
    store.snapshot = { ...store.snapshot!, stateToken: "b" };
    await flushPromises();
    expect(panel.get(".test-editor").text()).toBe("after");
    expect(
      panel.get(".diff-actions > button").attributes("disabled"),
    ).toBeDefined();
    expect(store.error?.message).toBe("Unable to read diff");
    await panel.get('[aria-label="Retry diff"]').trigger("click");
    await flushPromises();
    expect(panel.find('[aria-label="Retry diff"]').exists()).toBe(false);
    expect(
      panel.get(".diff-actions > button").attributes("disabled"),
    ).toBeUndefined();
    panel.unmount();
  });
  it("rejects late responses after changing files or repositories", async () => {
    const { store, panel } = await setup();
    const older = deferred<Diff>();
    transport.invoke.mockReturnValueOnce(older.promise);
    store.snapshot = { ...store.snapshot!, stateToken: "b" };
    await nextTick();
    await panel.setProps({ path: "second.txt" });
    await flushPromises();
    store.activeId = "other-repo";
    await flushPromises();
    older.resolve({ ...initial, modified: "WRONG REPOSITORY" });
    await flushPromises();
    expect(panel.text()).not.toContain("WRONG REPOSITORY");
    expect(transport.invoke.mock.calls.at(-1)?.[1]).toMatchObject({
      id: "other-repo",
      request: { path: "second.txt" },
    });
    panel.unmount();
  });
});
