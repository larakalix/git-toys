<script setup lang="ts">
import { ref } from "vue";
import type { RebaseStep } from "../../lib/api";
const steps = defineModel<RebaseStep[]>({ required: true });
defineProps<{ disabled?: boolean; subjects: Record<string, string> }>();
const dragged = ref<number | null>(null);
function move(from: number, to: number) {
  if (to < 0 || to >= steps.value.length) return;
  const list = [...steps.value];
  const [item] = list.splice(from, 1);
  if (item) list.splice(to, 0, item);
  steps.value = list;
}
function action(step: RebaseStep, value: string, index: number) {
  step.action = value;
  if (value === "squash") {
    const previous = steps.value
      .slice(0, index)
      .filter((s) => !["drop", "fixup"].includes(s.action))
      .at(-1);
    if (previous)
      step.message = `${previous.message.trim()}\n\n${step.message.trim()}`;
  }
}
function drop(index: number) {
  if (dragged.value !== null) move(dragged.value, index);
  dragged.value = null;
}
</script>
<template>
  <ol class="rebase-steps">
    <li
      v-for="(step, index) in steps"
      :key="step.oid"
      @dragover.prevent
      @drop.prevent="!disabled && drop(index)"
    >
      <span
        :draggable="!disabled"
        @dragstart="dragged = index"
        @dragend="dragged = null"
        title="Drag to reorder"
        >⠿</span
      ><code>{{ step.oid.slice(0, 8) }}</code
      ><span>{{ subjects[step.oid] }}</span>
      <select
        :aria-label="`Action for ${subjects[step.oid]}`"
        :value="step.action"
        :disabled="disabled"
        @change="
          action(step, ($event.target as HTMLSelectElement).value, index)
        "
      >
        <option
          v-for="value in ['pick', 'reword', 'squash', 'fixup', 'drop']"
          :key="value"
          :value="value"
        >
          {{ value }}
        </option>
      </select>
      <button
        class="button"
        :disabled="disabled || index === 0"
        :aria-label="`Move ${subjects[step.oid]} up`"
        @click="move(index, index - 1)"
      >
        ↑</button
      ><button
        class="button"
        :disabled="disabled || index === steps.length - 1"
        :aria-label="`Move ${subjects[step.oid]} down`"
        @click="move(index, index + 1)"
      >
        ↓
      </button>
      <label
        v-if="['reword', 'squash'].includes(step.action)"
        class="rebase-message"
        >{{ step.action === "squash" ? "Final combined message" : "New message"
        }}<textarea
          v-model="step.message"
          :disabled="disabled"
          maxlength="8192"
          rows="3"
        />
      </label>
    </li>
  </ol>
</template>
