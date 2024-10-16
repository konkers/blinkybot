<script setup lang="ts">
import { ref } from 'vue';
import type { Ref } from 'vue';

import Pixel from './Pixel.vue';

const props = defineProps<{
  pixels: boolean[][];
}>();
const emit = defineEmits<{
  (event: 'update:pixels', payload: boolean[][]): void;
}>();

const updatePixel = (row: number, col: number, value: boolean) => {
  let pixels = props.pixels;

  pixels[row][col] = value;
  emit('update:pixels', pixels);
};
</script>

<template>
  <div class="pixels" v-if="pixels">
    <div class="pixel_row" v-for="(row, row_index) in pixels" :key="row_index">
      <Pixel
        v-for="(pixel, col_index) in row"
        :key="col_index"
        :state="pixel"
        @update:state="($event) => updatePixel(row_index, col_index, $event)"
      ></Pixel>
    </div>
  </div>
</template>

<style>
.pixels {
  display: flex;
  flex-wrap: nowrap;
  flex-direction: column;
}
.pixel_row {
  display: flex;
  flex-wrap: nowrap;
  flex-direction: row;
}
</style>
