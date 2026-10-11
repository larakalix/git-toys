import { shallowRef } from "vue";
import { defineStore } from "pinia";
export type NoticeKind = "success" | "error" | "warning" | "info";
export interface Notice {
  id: number;
  kind: NoticeKind;
  title: string;
  message?: string;
  detail?: string;
  key?: string;
}
export const useNotificationStore = defineStore("notifications", () => {
  const notices = shallowRef<Notice[]>([]);
  let nextId = 0;
  function dismiss(id: number) {
    notices.value = notices.value.filter((notice) => notice.id !== id);
  }
  function notify(notice: Omit<Notice, "id">) {
    const existing = notices.value.find(
      (item) => notice.key && item.key === notice.key,
    );
    if (existing) return existing.id;
    const id = ++nextId;
    const next = [...notices.value, { ...notice, id }];
    const transient = next.filter(
      (item) => item.kind === "success" || item.kind === "info",
    );
    const expired = new Set(transient.slice(0, -3).map((item) => item.id));
    notices.value = next.filter((item) => !expired.has(item.id));
    return id;
  }
  return { notices, notify, dismiss };
});
