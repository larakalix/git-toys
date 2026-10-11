import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { flushPromises, mount } from "@vue/test-utils";
import AiCommitDialog from "../../src/components/AiCommitDialog.vue";
import { useRepositoryStore } from "../../src/stores/repository";
import type { AiContext, Snapshot, AiDraft } from "../../src/lib/api";
const transport = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("../../src/lib/api", async (original) => ({
  ...(await original<object>()),
  call: (...args: unknown[]) => transport.call(...args),
}));
const context: AiContext = {
  indexToken: "index-a",
  contextToken: "review-a",
  files: [
    { path: "hello.txt", excludedReason: null },
    { path: ".env", excludedReason: "Potential credential file" },
  ],
  selected: ["hello.txt"],
  context: "reviewed staged patch",
  oversized: false,
};
const dialogStub = {
  props: ["open", "title"],
  template: '<section v-if="open"><slot /></section>',
};
async function setup() {
  const store = useRepositoryStore();
  store.activeId = "repo";
  store.snapshot = { stateToken: "a", warnings: [] } as unknown as Snapshot;
  store.preferences.ai = {
    enabled: true,
    provider: "ollama",
    model: "fixture",
    ollamaUrl: "http://127.0.0.1:11434",
  };
  const wrapper = mount(AiCommitDialog, {
    props: { open: false, existingMessage: "My original message" },
    global: { stubs: { AppDialog: dialogStub } },
  });
  await wrapper.setProps({ open: true });
  await flushPromises();
  return { store, wrapper };
}
const button = (wrapper: ReturnType<typeof mount>, text: string) =>
  wrapper.findAll("button").find((item) => item.text() === text)!;
describe("AI commit draft review", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    transport.call.mockReset();
    transport.call.mockImplementation(async (command: string) => {
      if (command === "ai_context") return context;
      if (command === "ai_index_token") return "index-a";
      if (command === "ai_generate")
        return { message: "Improve greeting", indexToken: "index-a" };
    });
  });
  it("keeps the existing message until replacement is explicitly accepted", async () => {
    const { wrapper } = await setup();
    expect(wrapper.get('input[type="checkbox"]:disabled').element).toBeTruthy();
    await button(wrapper, "Generate draft").trigger("click");
    await flushPromises();
    expect(wrapper.emitted("use")).toBeUndefined();
    expect(button(wrapper, "Use message").attributes("disabled")).toBeDefined();
    await wrapper.get("textarea").setValue("Edited generated draft");
    await wrapper.findAll('input[type="checkbox"]').at(-1)!.setValue(true);
    await button(wrapper, "Use message").trigger("click");
    await flushPromises();
    expect(wrapper.emitted("use")).toEqual([["Edited generated draft"]]);
    expect(
      transport.call.mock.calls.some(([command]) => command === "mutate"),
    ).toBe(false);
    wrapper.unmount();
  });
  it("rejects a stale draft after the index changes", async () => {
    const { wrapper, store } = await setup();
    await button(wrapper, "Generate draft").trigger("click");
    await flushPromises();
    transport.call.mockImplementation(async (command: string) =>
      command === "ai_index_token" ? "index-b" : context,
    );
    store.snapshot = { ...store.snapshot!, stateToken: "b" };
    await flushPromises();
    expect(wrapper.text()).toContain("Staged changes changed");
    expect(button(wrapper, "Use message").attributes("disabled")).toBeDefined();
    expect(wrapper.emitted("use")).toBeUndefined();
    wrapper.unmount();
  });
  it("cancels pending requests and ignores their late responses", async () => {
    const { wrapper } = await setup();
    let resolve!: (value: AiDraft) => void;
    const pending = new Promise<AiDraft>((yes) => (resolve = yes));
    transport.call.mockImplementation((command: string) =>
      command === "ai_generate" ? pending : Promise.resolve(context),
    );
    await button(wrapper, "Generate draft").trigger("click");
    await flushPromises();
    await button(wrapper, "Stop generation").trigger("click");
    await flushPromises();
    expect(transport.call).toHaveBeenCalledWith(
      "ai_cancel",
      expect.objectContaining({ requestId: expect.any(String) }),
    );
    resolve({ message: "Too late", indexToken: "index-a" });
    await flushPromises();
    expect(wrapper.find("textarea").exists()).toBe(false);
    wrapper.unmount();
  });
  it("prevents oversized payload generation and clears selections through review", async () => {
    transport.call.mockResolvedValue({ ...context, oversized: true });
    const { wrapper } = await setup();
    expect(
      button(wrapper, "Generate draft").attributes("disabled"),
    ).toBeDefined();
    await wrapper.findAll('input[type="checkbox"]')[0]!.setValue(false);
    await flushPromises();
    expect(transport.call).toHaveBeenCalledWith(
      "ai_context",
      expect.objectContaining({ selected: [] }),
    );
    wrapper.unmount();
  });
});
