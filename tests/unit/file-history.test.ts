import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mount, flushPromises } from "@vue/test-utils";
import FileTimeMachine from "../../src/components/FileTimeMachine.vue";
import { useRepositoryStore } from "../../src/stores/repository";
import type { Snapshot, FileRevision } from "../../src/lib/api";
const transport = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("../../src/lib/api", async (original) => ({
  ...(await original<object>()),
  call: (...args: unknown[]) => transport.call(...args),
}));
vi.mock("../../src/components/CodeEditor.vue", () => ({
  __esModule: true,
  default: {
    props: ["original", "modified"],
    template: '<div class="test-editor">{{ modified }}</div>',
  },
}));
const entry = (path: string) => ({
  path,
  previousPath: null,
  deleted: false,
  commit: {
    oid: "abc",
    parents: [],
    author: "Author",
    email: "test@example.invalid",
    date: "2026-10-11",
    subject: "Update file",
    message: "Update file",
    refs: "",
  },
});
const revision = (path: string): FileRevision => ({
  revision: "abc",
  path,
  original: "before",
  content: `version of ${path}`,
  exists: true,
  unsupported: null,
});
async function setup() {
  const store = useRepositoryStore();
  store.activeId = "repo";
  store.snapshot = {
    head: "abc",
    stateToken: "state-a",
    warnings: [],
  } as unknown as Snapshot;
  store.tabs = [{ id: "repo", trusted: true, snapshot: store.snapshot }];
  const wrapper = mount(FileTimeMachine, {
    props: { open: false, path: "old.txt", theme: "light" },
    global: {
      stubs: {
        AppDialog: {
          props: ["open"],
          template: '<section v-if="open"><slot /></section>',
        },
      },
    },
  });
  await wrapper.setProps({ open: true });
  await flushPromises();
  return { store, wrapper };
}
describe("file history response ownership", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    transport.call.mockReset();
    transport.call.mockImplementation(
      async (command: string, args: { path: string }) =>
        command === "file_history" ? [entry(args.path)] : revision(args.path),
    );
  });
  it("ignores a previous file’s delayed content after switching files", async () => {
    let resolve!: (value: FileRevision) => void;
    const pending = new Promise<FileRevision>((yes) => (resolve = yes));
    transport.call.mockImplementation(
      (command: string, args: { path: string }) => {
        if (command === "file_history")
          return Promise.resolve([entry(args.path)]);
        return args.path === "old.txt"
          ? pending
          : Promise.resolve(revision(args.path));
      },
    );
    const { wrapper } = await setup();
    await wrapper.setProps({ path: "new.txt" });
    await flushPromises();
    expect(wrapper.get(".test-editor").text()).toBe("version of new.txt");
    resolve(revision("old.txt"));
    await flushPromises();
    expect(wrapper.get(".test-editor").text()).toBe("version of new.txt");
    wrapper.unmount();
  });
  it("binds restore confirmation to its preview state and preserves the target path", async () => {
    const { wrapper, store } = await setup();
    transport.call.mockImplementation(async (command: string) =>
      command === "file_restore_preview"
        ? revision("historical-name.txt")
        : "operation",
    );
    await wrapper
      .findAll("button")
      .find((button) => button.text() === "Restore this version")!
      .trigger("click");
    await flushPromises();
    store.snapshot = { ...store.snapshot!, stateToken: "newer-state" };
    await wrapper
      .findAll("button")
      .find((button) => button.text() === "Restore as unstaged change")!
      .trigger("click");
    await flushPromises();
    expect(transport.call).toHaveBeenCalledWith(
      "mutate",
      expect.objectContaining({
        expected: "state-a",
        action: {
          kind: "restoreFileVersion",
          target: "old.txt",
          source: "historical-name.txt",
          revision: "abc",
        },
      }),
    );
    wrapper.unmount();
  });
});
