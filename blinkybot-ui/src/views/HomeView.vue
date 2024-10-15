<script setup lang="ts">
import { useBlinkyBotStore } from '@/stores/blinkybot';
import Expression from '@/components/Expression.vue';

const blinkyBot = useBlinkyBotStore();

const handler = async () => {
  const response = await blinkyBot.ping(1);
  console.log(response);
};
</script>

<template>
  <main>
    <Expression></Expression>
    <div v-if="blinkyBot.isConnected">
      <v-btn @click="handler()">do usb stuff</v-btn>
    </div>
  </main>
</template>
