import { describe, it, expect, vi, beforeEach } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { flushPromises, mount } from "@vue/test-utils";
import MergePreviewDialog from "../../src/components/history-tools/MergePreviewDialog.vue";
import PullRequestDetails from "../../src/components/github/PullRequestDetails.vue";
import { selectableLines } from "../../src/lib/selections";
import { githubRepository } from "../../src/lib/github";
import { useRepositoryStore } from "../../src/stores/repository";
import type { Snapshot, PreviewResult, PullRequest } from "../../src/lib/api";
const transport = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("../../src/lib/api", async (original) => ({
  ...(await original<object>()),
  call: (...args: unknown[]) => transport.call(...args),
  onEvent: async () => () => {},
}));
const AppDialog = {
  props: ["open"],
  template: '<section v-if="open"><slot /></section>',
};
beforeEach(() => {
  setActivePinia(createPinia());
  transport.call.mockReset();
});
function setup() {
  const store = useRepositoryStore();
  store.activeId = "repo";
  store.snapshot = {
    path: "/repo",
    branch: "main",
    head: "a",
    files: [],
    refs: [],
    remotes: [],
    stateToken: "before",
    warnings: [],
    operation: null,
  } as unknown as Snapshot;
  store.tabs = [{ id: "repo", snapshot: store.snapshot, trusted: true }];
  return store;
}
describe("reviewed Git plans", () => {
  it("offers only changed lines, excluding patch headers and context", () => {
    const rows = selectableLines(
      "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-old\n+new\n context",
    );
    expect(rows.filter((v) => v.selectable).map((v) => v.index)).toEqual([
      4, 5,
    ]);
  });
  it("rejects lookalike GitHub hosts and preserves fork repositories", () => {
    expect(githubRepository("git@github.com:fork/project.git")).toBe(
      "fork/project",
    );
    expect(githubRepository("https://github.com/fork/project/")).toBe(
      "fork/project",
    );
    for (const value of [
      "https://github.com.evil.invalid/a/b",
      "https://github.com/../repo",
      "https://github.com/a/b?token=secret",
    ])
      expect(githubRepository(value)).toBeNull();
  });
  it("rejects a preview response when repository state changes in flight", async () => {
    const store = setup();
    let resolve!: (result: PreviewResult) => void;
    transport.call.mockImplementation((command) =>
      command === "preview_operation"
        ? new Promise((r) => {
            resolve = r;
          })
        : Promise.resolve(),
    );
    const wrapper = mount(MergePreviewDialog, {
      props: { open: true, kind: "merge" },
      global: { stubs: { AppDialog } },
    });
    await wrapper.find("input").setValue("feature");
    await wrapper
      .findAll("button")
      .find((b) => b.text() === "Generate preview")!
      .trigger("click");
    await flushPromises();
    store.snapshot = { ...store.snapshot!, stateToken: "after" };
    resolve({
      kind: "merge",
      head: "a",
      target: "b",
      targetRef: "feature",
      commits: [],
      files: [],
      conflicts: [],
      patch: "",
      stoppedCommit: null,
      remainingUntested: false,
    });
    await flushPromises();
    expect(wrapper.text()).toContain("Repository changed during preview");
    expect(
      wrapper
        .findAll("button")
        .find((b) => b.text().startsWith("Apply merge"))!
        .attributes("disabled"),
    ).toBeDefined();
    wrapper.unmount();
  });
  it("renders repository-controlled PR descriptions as text without executable HTML or images", async () => {
    setup();
    transport.call.mockResolvedValue({ items: [], more: false });
    const request = {
      number: 42,
      title: "Contribution",
      author: "forker",
      state: "open",
      draft: false,
      body: '<img src="https://tracker.invalid/x" onerror="alert(1)"><script>alert(1)</script>',
      head: "a".repeat(40),
      headLabel: "forker:feature",
      base: "main",
      mergeability: "Mergeable",
    } satisfies PullRequest;
    const wrapper = mount(PullRequestDetails, {
      props: { request, repository: "owner/repo", remote: "github" },
      global: { stubs: { AppDialog } },
    });
    await flushPromises();
    expect(wrapper.find("img").exists()).toBe(false);
    expect(wrapper.find("script").exists()).toBe(false);
    expect(wrapper.text()).toContain(request.body);
    wrapper.unmount();
  });
});
