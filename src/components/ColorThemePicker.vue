<script setup lang="ts">
import { CheckCircle2, Palette } from "lucide-vue-next";
import { colorThemes, getColorTheme } from "../lib/themes";
defineProps<{ modelValue?: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <fieldset class="color-themes">
    <legend><Palette :size="18" aria-hidden="true" />Color theme</legend>
    <p class="muted">
      Choose your palette. Works with light, dark, and system appearance.
    </p>
    <div class="theme-grid">
      <label
        v-for="theme in colorThemes"
        :key="theme.id"
        class="theme-card"
        :class="{ selected: getColorTheme(modelValue).id === theme.id }"
      >
        <input
          type="radio"
          name="color-theme"
          :value="theme.id"
          :checked="getColorTheme(modelValue).id === theme.id"
          @change="emit('update:modelValue', theme.id)"
        />
        <span class="theme-swatches" aria-hidden="true">
          <span
            v-for="(color, index) in theme.swatches"
            :key="index"
            :style="{ background: color }"
          />
        </span>
        <span class="theme-name">{{ theme.name }}</span>
        <CheckCircle2
          v-if="getColorTheme(modelValue).id === theme.id"
          :size="16"
          aria-hidden="true"
        />
      </label>
    </div>
  </fieldset>
</template>

<style scoped>
.color-themes {
  border: 0;
  padding: 0;
  margin: 24px 0;
  min-width: 0;
}
legend {
  display: flex;
  align-items: center;
  gap: 9px;
  font-size: 14px;
  font-weight: 600;
}
legend svg {
  color: var(--accent);
}
p {
  margin: 7px 0 16px;
  font-size: 12px;
}
.theme-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}
.theme-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 9px;
  cursor: pointer;
  transition:
    background 140ms,
    border-color 140ms;
}
.theme-card:hover {
  background: var(--raised);
}
.theme-card.selected {
  background: var(--accent-soft);
  border-color: var(--accent);
}
.theme-card:has(input:focus-visible) {
  outline: 2px solid var(--accent);
  outline-offset: 3px;
}
input {
  position: absolute;
  opacity: 0;
  width: 1px;
  height: 1px;
  margin: 0;
}
.theme-swatches {
  display: flex;
  flex-shrink: 0;
  padding-left: 4px;
}
.theme-swatches span {
  width: 17px;
  height: 17px;
  border: 2px solid var(--surface);
  border-radius: 50%;
  margin-left: -4px;
  box-shadow: 0 0 0 1px #0000000a;
}
.theme-name {
  flex: 1;
  font-size: 12px;
}
.theme-card > svg {
  color: var(--accent);
}
@media (max-width: 700px) {
  .theme-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
@media (max-width: 430px) {
  .theme-grid {
    grid-template-columns: 1fr;
  }
}
@media (prefers-reduced-motion: reduce) {
  .theme-card {
    transition: none;
  }
}
</style>
