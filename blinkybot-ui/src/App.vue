<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router';

import HelloWorld from './components/HelloWorld.vue';
import { useBlinkyBotStore } from '@/stores/blinkybot';

const links = ['Dashboard', 'Messages', 'Profile', 'Updates'];

const blinkyBot = useBlinkyBotStore();
</script>

<template>
  <v-app id="inspire">
    <v-app-bar flat>
      <v-container class="mx-auto d-flex align-center justify-center">
        <RouterLink to="/">
          <v-avatar class="me-4" color="grey-darken-1" size="32" link to="/"></v-avatar>
        </RouterLink>

        <v-btn v-if="blinkyBot.isConnected" @click="blinkyBot.disconnect()">Disconnect</v-btn>
        <v-btn v-else @click="blinkyBot.connect()">Connect</v-btn>

        <v-spacer></v-spacer>

        <v-responsive max-width="160"> </v-responsive>
      </v-container>
    </v-app-bar>

    <v-main class="bg-grey-lighten-3">
      <v-container>
        <v-row>
          <v-col cols="2">
            <v-sheet rounded="lg">
              <v-list rounded="lg">
                <v-list-item title="Default" link to="/expression/default"></v-list-item>
                <v-list-item title="Blink" link to="/expression/blink"></v-list-item>
              </v-list>
            </v-sheet>
          </v-col>

          <v-col>
            <v-sheet min-height="70vh" rounded="lg">
              <RouterView />
              <!--  -->
            </v-sheet>
          </v-col>
        </v-row>
      </v-container>
    </v-main>
  </v-app>
</template>

<script lang="ts">
export default {
  data: () => ({
    links: ['Dashboard', 'Messages', 'Profile', 'Updates']
  })
};
</script>
