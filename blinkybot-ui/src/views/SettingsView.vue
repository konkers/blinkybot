<script setup lang="ts">
import { ref } from 'vue';
import type { Ref } from 'vue';
import { watch } from 'vue';
import { useRoute } from 'vue-router';

import { useBlinkyBotStore } from '@/stores/blinkybot';

const blinkyBot = useBlinkyBotStore();
const adc_val = ref('');

async function getAdc() {
  adc_val.value = (await blinkyBot.get_adc()).toString(16);
}
</script>

<template>
  <main>
    <div v-if="blinkyBot.isConnected">
      <div id="adc_val">{{ adc_val }}</div>
      <v-btn @click="getAdc()">Get ADC</v-btn>
    </div>
  </main>
</template>
