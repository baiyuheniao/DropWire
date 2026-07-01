<template>
  <div class="app">
    <header>
      <h1>DropWire</h1>
      <span class="ws-status" :class="wsStatus">{{ wsStatusLabel }}</span>
    </header>
    <FileUpload />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import FileUpload from './components/FileUpload.vue'
import { useWebSocket } from './composables/useWebSocket'

const { connected } = useWebSocket(`ws://${location.hostname}:3000/ws`)
const wsStatus = computed(() => (connected.value ? 'online' : 'offline'))
const wsStatusLabel = computed(() => (connected.value ? '已连接' : '断线中...'))
</script>

<style>
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: system-ui, -apple-system, sans-serif; background: #f0f2f5; color: #333; }

.app { max-width: 860px; margin: 0 auto; padding: 32px 20px; }

header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 32px;
}
h1 { font-size: 28px; font-weight: 700; letter-spacing: -0.5px; }

.ws-status {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 12px;
  font-weight: 500;
}
.ws-status.online  { background: #d4edda; color: #155724; }
.ws-status.offline { background: #f8d7da; color: #721c24; }
</style>
