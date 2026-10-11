<script setup lang="ts">
import { computed } from "vue";
import { useVirtualList } from "@vueuse/core";
import { selectableLines } from "../../lib/selections";
const props = defineProps<{ patch: string; disabled?: boolean }>();
const selected = defineModel<number[]>({ default: () => [] });
const rows = computed(() => selectableLines(props.patch));
const { list, containerProps, wrapperProps } = useVirtualList(rows, {
  itemHeight: 28,
  overscan: 12,
});
function toggle(index: number) {
  selected.value = selected.value.includes(index)
    ? selected.value.filter((v) => v !== index)
    : [...selected.value, index];
}
</script>
<template>
  <div
    v-bind="containerProps"
    class="selection-lines"
    aria-label="Select changed lines"
  >
    <div v-bind="wrapperProps">
      <label
        v-for="row in list"
        :key="row.index"
        class="selection-line"
        :class="{
          addition: row.data.selectable && row.data.text.startsWith('+'),
          deletion: row.data.selectable && row.data.text.startsWith('-'),
        }"
      >
        <input
          v-if="row.data.selectable"
          type="checkbox"
          :aria-label="`Select change line ${row.data.index + 1}`"
          :checked="selected.includes(row.data.index)"
          :disabled="disabled"
          @change="toggle(row.data.index)"
        />
        <span v-else class="line-spacer" /><code>{{ row.data.text }}</code>
      </label>
    </div>
  </div>
</template>
