<script setup lang="ts">
import {
  DialogRoot,
  DialogPortal,
  DialogOverlay,
  DialogContent,
  DialogTitle,
  DialogDescription,
  DialogClose,
} from "reka-ui";
import { X } from "lucide-vue-next";
import { shallowRef, watch } from "vue";
withDefaults(
  defineProps<{
    title: string;
    description?: string;
    contentClass?: string;
    motion?: boolean;
  }>(),
  { motion: true, description: "", contentClass: "" },
);
const open = defineModel<boolean>("open", { default: false });
// Keep the lightweight Reka surface stable across exits so its presence cleanup runs
// once. Release heavy slot content when the close transition finishes.
const mounted = shallowRef(open.value);
const contentMounted = shallowRef(open.value);
watch(open, (value) => {
  if (value) {
    mounted.value = true;
    contentMounted.value = true;
  }
});
function afterLeave() {
  if (!open.value) contentMounted.value = false;
}
</script>
<template>
  <DialogRoot v-model:open="open"
    ><DialogPortal
      ><Transition :name="motion ? 'dialog-scrim' : undefined" :css="motion">
        <DialogOverlay
          v-if="mounted"
          v-show="open"
          force-mount
          class="dialog-overlay"
        />
      </Transition>
      <Transition
        :name="motion ? 'dialog-surface' : undefined"
        :css="motion"
        @after-leave="afterLeave"
      >
        <DialogContent
          v-if="mounted"
          v-show="open"
          force-mount
          :aria-hidden="!open || undefined"
          :inert="!open"
          class="dialog"
          :class="contentClass"
          ><template v-if="contentMounted"
            ><header>
              <div>
                <DialogTitle class="dialog-title">{{ title }}</DialogTitle
                ><DialogDescription class="muted">{{
                  description || "Review the details before continuing."
                }}</DialogDescription>
              </div>
              <DialogClose class="icon-button" aria-label="Close dialog"
                ><X :size="18"
              /></DialogClose>
            </header>
            <slot /></template
        ></DialogContent> </Transition></DialogPortal
  ></DialogRoot>
</template>
