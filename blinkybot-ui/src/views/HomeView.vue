<script setup lang="ts">
import { useBlinkyBotStore } from '@/stores/blinkybot';
import Expression from '@/components/Expression.vue';

const blinkyBot = useBlinkyBotStore();

const handler = async () => {
  await blinkyBot.testTransaction();
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
