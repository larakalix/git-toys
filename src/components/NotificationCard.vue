<script setup lang="ts">
import { onMounted, onBeforeUnmount, shallowRef } from "vue";
import {
  CheckCircle2,
  CircleAlert,
  TriangleAlert,
  Info,
  X,
} from "lucide-vue-next";
import type { Notice } from "../stores/notifications";
const props = defineProps<{ notice: Notice }>();
const emit = defineEmits<{ dismiss: [id: number] }>();
const icons = {
  success: CheckCircle2,
  error: CircleAlert,
  warning: TriangleAlert,
  info: Info,
};
const hovered = shallowRef(false),
  focused = shallowRef(false);
let timer: ReturnType<typeof setTimeout> | undefined;
function pause() {
  clearTimeout(timer);
}
function resume() {
  pause();
  if (
    !hovered.value &&
    !focused.value &&
    ["success", "info"].includes(props.notice.kind)
  )
    timer = setTimeout(() => emit("dismiss", props.notice.id), 5000);
}
function leaveFocus(event: FocusEvent) {
  if (
    !(event.currentTarget as HTMLElement).contains(
      event.relatedTarget as Node | null,
    )
  ) {
    focused.value = false;
    resume();
  }
}
onMounted(resume);
onBeforeUnmount(pause);
</script>
<template>
  <article
    class="notification-card"
    :class="`notice-${notice.kind}`"
    :role="notice.kind === 'error' ? 'alert' : 'status'"
    aria-atomic="true"
    @mouseenter="
      hovered = true;
      pause();
    "
    @mouseleave="
      hovered = false;
      resume();
    "
    @focusin="
      focused = true;
      pause();
    "
    @focusout="leaveFocus"
  >
    <component
      :is="icons[notice.kind]"
      :size="19"
      class="notification-icon"
      aria-hidden="true"
    />
    <div class="notification-content">
      <span class="notification-kind">{{ notice.kind }}</span>
      <strong>{{ notice.title }}</strong>
      <p v-if="notice.message">{{ notice.message }}</p>
      <details v-if="notice.detail">
        <summary>View Git output</summary>
        <pre>{{ notice.detail }}</pre>
      </details>
    </div>
    <button
      class="icon-button"
      :aria-label="`Dismiss ${notice.kind} notification: ${notice.title}`"
      @click="emit('dismiss', notice.id)"
    >
      <X :size="15" aria-hidden="true" />
    </button>
  </article>
</template>
