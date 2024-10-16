<script setup lang="ts">
import { ref } from 'vue';
import type { Ref } from 'vue';

import { useBlinkyBotStore } from '@/stores/blinkybot';
import Expression from '@/components/Expression.vue';
import { Expression as ExpressionData, ExpressionIndex } from '@/stores/blinkybot';

const blinkyBot = useBlinkyBotStore();

const pixelWidth = 15;
const pixelHeight = 7;
const pixels: Ref<boolean[][]> = ref(
  new Array(pixelHeight).fill(false).map(() => new Array(pixelWidth).fill(false))
);

async function handler() {
  let data = new ExpressionData();
  for (const y in pixels.value) {
    const row = pixels.value[y];
    for (const x in row) {
      data.set_pixel(Number(x), Number(y), row[x]);
    }
  }
  await blinkyBot.set_expression(ExpressionIndex.Default, data);
}
function updatePixels(newPixels: boolean[][]) {
  pixels.value = newPixels;
  console.log(newPixels);
}
</script>

<template>
  <main>
    <div v-if="blinkyBot.isConnected">
      <Expression :pixels="pixels" @update:pixels="($event) => updatePixels($event)"></Expression>
      <v-btn @click="handler()">do usb stuff</v-btn>
    </div>
  </main>
</template>
