import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mount } from "@vue/test-utils";
import { useNotificationStore } from "../../src/stores/notifications";
import NotificationCard from "../../src/components/NotificationCard.vue";

describe("notifications", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());
  it("deduplicates repeated errors without losing persistent notices to routine successes", () => {
    const store = useNotificationStore();
    const failure = {
      kind: "error" as const,
      title: "Push rejected",
      key: "rejected",
    };
    store.notify(failure);
    store.notify(failure);
    store.notify({ kind: "warning", title: "Conflict requires attention" });
    for (let i = 0; i < 8; i++)
      store.notify({ kind: "success", title: `Done ${i}` });
    expect(store.notices.map((notice) => notice.kind)).toEqual([
      "error",
      "warning",
      "success",
      "success",
      "success",
    ]);
    store.dismiss(store.notices[0]!.id);
    expect(store.notices.some((notice) => notice.kind === "error")).toBe(false);
  });
  it.each(["success", "info"] as const)(
    "auto-dismisses %s after five seconds and pauses while focused",
    async (kind) => {
      const card = mount(NotificationCard, {
        props: { notice: { id: 1, kind, title: "Saved" } },
      });
      await vi.advanceTimersByTimeAsync(4000);
      await card.trigger("focusin");
      await card.trigger("mouseenter");
      await card.trigger("mouseleave");
      await vi.advanceTimersByTimeAsync(10000);
      expect(card.emitted("dismiss")).toBeUndefined();
      await card.trigger("focusout", { relatedTarget: null });
      await vi.advanceTimersByTimeAsync(5000);
      expect(card.emitted("dismiss")).toEqual([[1]]);
      card.unmount();
    },
  );
  it.each(["error", "warning"] as const)(
    "keeps %s visible with output until dismissed",
    async (kind) => {
      const card = mount(NotificationCard, {
        props: {
          notice: {
            id: 2,
            kind,
            title: "Attention needed",
            detail: "Git hook output",
          },
        },
      });
      await vi.advanceTimersByTimeAsync(60000);
      expect(card.emitted("dismiss")).toBeUndefined();
      expect(card.find("article").attributes("role")).toBe(
        kind === "error" ? "alert" : "status",
      );
      expect(card.find("pre").text()).toBe("Git hook output");
      await card.get("button").trigger("click");
      expect(card.emitted("dismiss")).toEqual([[2]]);
      card.unmount();
    },
  );
});
