<template>
  <div class="app">
    <header class="top-bar">
      <div class="brand">
        <img class="brand-logo" src="/logo.png" alt="DropWire" />
        <span class="brand-name">DropWire</span>
      </div>

      <nav class="tabs">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="tab"
          :class="{ active: activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </nav>

      <div class="status-area">
        <div class="avatar">头像</div>
        <span class="status-label">状态:</span>
        <span class="status-badge" :class="wsStatus">{{ wsStatusLabel }}</span>
      </div>
    </header>

    <main class="main-content">
      <SendView v-if="activeTab === 'send'" />
      <ReceiveView v-else-if="activeTab === 'receive'" />
      <SettingsView v-else-if="activeTab === 'settings'" />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useWebSocket } from './composables/useWebSocket'
import SendView from './components/SendView.vue'
import ReceiveView from './components/ReceiveView.vue'
import SettingsView from './components/SettingsView.vue'

const { connected } = useWebSocket(`/ws`)
const wsStatus = computed(() => (connected.value ? 'online' : 'offline'))
const wsStatusLabel = computed(() => (connected.value ? '已连接' : '断线中'))

const activeTab = ref('send')
const tabs = [
  { key: 'send', label: '发送' },
  { key: 'receive', label: '接收' },
  { key: 'settings', label: '设置' },
]
</script>

<style>
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  background: #f5f6f8;
  color: #1f2937;
  min-height: 100vh;
}

.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.top-bar {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  height: 64px;
  background: #fff;
  border-bottom: 1px solid #e5e7eb;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-logo {
  height: 36px;
  width: auto;
  object-fit: contain;
}

.brand-name {
  font-size: 20px;
  font-weight: 700;
  color: #374151;
  letter-spacing: -0.3px;
}

.tabs {
  display: flex;
  align-items: flex-end;
  gap: 4px;
  height: 100%;
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.tab {
  padding: 10px 28px;
  border: 1px solid #d1d5db;
  border-bottom: none;
  border-radius: 12px 12px 0 0;
  background: #f3f4f6;
  color: #6b7280;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
  top: 1px;
}

.tab:hover {
  background: #e5e7eb;
  color: #4b5563;
}

.tab.active {
  background: #f5f6f8;
  color: #1f2937;
  font-weight: 600;
  border-color: #e5e7eb #e5e7eb #f5f6f8;
  z-index: 1;
}

.status-area {
  display: flex;
  align-items: center;
  gap: 10px;
}

.avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: #f3f4f6;
  border: 1px solid #e5e7eb;
  display: grid;
  place-items: center;
  font-size: 12px;
  color: #6b7280;
}

.status-label {
  font-size: 13px;
  color: #6b7280;
}

.status-badge {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  background: #fee2e2;
  color: #dc2626;
}

.status-badge.online {
  background: #3b82f6;
  color: #fff;
}

.main-content {
  flex: 1;
  padding: 32px 24px;
}
</style>
