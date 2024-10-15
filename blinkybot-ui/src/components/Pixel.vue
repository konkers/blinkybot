<script setup lang="ts">
const props = defineProps<{
  state: boolean;
}>();
const emit = defineEmits<{
  (event: 'update:state', payload: boolean): void;
}>();

const updateState = () => {
  emit('update:state', !props.state);
};
</script>

<template>
  <button class="pixel" :class="{ active: state }" @click="updateState"></button>
</template>
<style>
.pixel {
  border: solid 1px #aaaaaa;
  width: 16px;
  height: 16px;
  background-color: #000000;
}

.pixel.active {
  background-color: #8888ff;
}
</style>
