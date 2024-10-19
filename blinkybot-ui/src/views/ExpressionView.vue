<script setup lang="ts">
import { ref } from 'vue';
import type { Ref } from 'vue';
import { watch } from 'vue';
import { useRoute } from 'vue-router';

import { useBlinkyBotStore } from '@/stores/blinkybot';
import Expression from '@/components/Expression.vue';
import { Expression as ExpressionData, ExpressionIndex } from '@/stores/blinkybot';

const route = useRoute();

const blinkyBot = useBlinkyBotStore();

const pixelWidth = 15;
const pixelHeight = 7;
const pixels: Ref<boolean[][]> = ref(
  new Array(pixelHeight).fill(false).map(() => new Array(pixelWidth).fill(false))
);

let index: ExpressionIndex | null = ExpressionIndex.Default;

watch(() => route.params.id, fecthExpression, { immediate: true });

async function fecthExpression(id: string | string[]) {
  index = expressionIndex(id as string);
  if (index !== null && blinkyBot.isConnected) {
    const data = await blinkyBot.get_expression(index);
    let newPixels: boolean[][] = new Array(pixelHeight)
      .fill(false)
      .map(() => new Array(pixelWidth).fill(false));
    for (const y in newPixels) {
      for (const x in newPixels[y]) {
        newPixels[y][x] = data.get_pixel(Number(x), Number(y));
      }
    }
    pixels.value = newPixels;
  }
}

function expressionIndex(name: string): ExpressionIndex | null {
  if (name == 'default') {
    return ExpressionIndex.Default;
  } else if (name == 'blink') {
    return ExpressionIndex.Blink;
  } else if (name == 'friend') {
    return ExpressionIndex.Friend;
  } else if (name == 'friend_blink') {
    return ExpressionIndex.FriendBlink;
  } else {
    return null;
  }
}

async function saveExpression() {
  if (index === null) {
    return;
  }

  let data = new ExpressionData();
  for (const y in pixels.value) {
    const row = pixels.value[y];
    for (const x in row) {
      data.set_pixel(Number(x), Number(y), row[x]);
    }
  }
  await blinkyBot.set_expression(index, data);
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
      <v-btn @click="saveExpression()">Save</v-btn>
    </div>
  </main>
</template>
