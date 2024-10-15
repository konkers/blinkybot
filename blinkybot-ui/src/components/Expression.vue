<script setup lang="ts">
import { ref } from 'vue';
import type { Ref } from 'vue';

import Pixel from './Pixel.vue';

const pixelWidth = 15;
const pixelHeight = 7;
const pixels: Ref<boolean[][]> = ref(
  new Array(pixelHeight).fill(false).map(() => new Array(pixelWidth).fill(false))
);
</script>

<template>
  <div class="pixels" v-if="pixels">
    <div class="pixel_row" v-for="(row, row_index) in pixels">
      <Pixel
        v-for="(pixel, col_index) in row"
        :state="pixel"
        @update:state="($event) => (pixels[row_index][col_index] = $event)"
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
