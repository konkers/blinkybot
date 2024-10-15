<script setup lang="ts">
import { useBlinkyBotStore } from '@/stores/blinkybot';

const blinkyBot = useBlinkyBotStore();

const connect = async () => {
  await blinkyBot.connect();
};
const disconnect = async () => {
  await blinkyBot.disconnect();
};
const handler = async () => {
  await blinkyBot.testTransaction();
};
</script>

<template>
  <main>
    <div v-if="blinkyBot.isConnected">
      <v-btn @click="handler()">do usb stuff</v-btn>
    </div>
  </main>
</template>
