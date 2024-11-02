<script setup lang="ts">
import { ref } from 'vue';
import type { Ref } from 'vue';
import { watch } from 'vue';
import { useRoute } from 'vue-router';

import { useBlinkyBotStore } from '@/stores/blinkybot';

const blinkyBot = useBlinkyBotStore();
const adc_val = ref('');
const brightness: Ref<number | null> = ref(null);

blinkyBot.get_brightness().then((value: number) => {
  console.log(`brightness: ${value}`);
  brightness.value = value;
});

async function getAdc() {
  adc_val.value = (await blinkyBot.get_adc()).toString(16);
}

async function updateBrightness(value: number) {
  blinkyBot.set_brightness(value);
  console.log(value);
}
</script>

<template>
  <main>
    <div v-if="blinkyBot.isConnected">
      <div id="adc_val">{{ adc_val }}</div>
      <v-btn @click="getAdc()">Get ADC</v-btn>
      <v-slider
        v-if="brightness !== null"
        min="0"
        max="255"
        v-model="brightness"
        @update:modelValue="updateBrightness($event)"
      ></v-slider>
    </div>
  </main>
</template>
