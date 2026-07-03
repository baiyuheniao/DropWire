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
        <div class="avatar" :class="{ logged: user }" @click="showAccountModal = true">
          <img v-if="user?.avatar" :src="user.avatar" alt="avatar" />
          <span v-else>{{ user ? user.nickname.charAt(0) : '头像' }}</span>
        </div>
        <span class="status-label">状态:</span>
        <span class="status-badge" :class="wsStatus">{{ wsStatusLabel }}</span>
      </div>
    </header>

    <main class="main-content">
      <SendView v-if="activeTab === 'send'" :user="user" />
      <ReceiveView v-else-if="activeTab === 'receive'" :user="user" />
      <SettingsView
        v-else-if="activeTab === 'settings'"
        :user="user"
        @edit-account="showAccountModal = true"
        @logout="onUserUpdate(null)"
      />
    </main>

    <AccountModal
      v-if="showAccountModal"
      :user="user"
      @close="showAccountModal = false"
      @update:user="onUserUpdate"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useWebSocket } from './composables/useWebSocket'
import { loadSettings, settings } from './composables/useSettings'
import SendView from './components/SendView.vue'
import ReceiveView from './components/ReceiveView.vue'
import SettingsView from './components/SettingsView.vue'
import AccountModal, { type User } from './components/AccountModal.vue'

loadSettings()

const wsUrl = computed(() => {
  const base = settings.value.apiBase.trim()
  if (!base) return '/ws'
  return base.replace(/^http/, 'ws') + '/ws'
})

const { connected } = useWebSocket(wsUrl.value)
const wsStatus = computed(() => (connected.value ? 'online' : 'offline'))
const wsStatusLabel = computed(() => (connected.value ? '已连接' : '断线中'))

const activeTab = ref('send')
const tabs = [
  { key: 'send', label: '发送' },
  { key: 'receive', label: '接收' },
  { key: 'settings', label: '设置' },
]

const showAccountModal = ref(false)
const user = ref<User | null>(null)

function onUserUpdate(newUser: User | null) {
  user.value = newUser
  if (newUser) {
    showAccountModal.value = false
    localStorage.setItem('dropwire_current_user', JSON.stringify(newUser))
  } else {
    localStorage.removeItem('dropwire_current_user')
  }
}

onMounted(() => {
  const saved = localStorage.getItem('dropwire_current_user')
  if (saved) {
    try {
      user.value = JSON.parse(saved)
    } catch {
      user.value = null
    }
  }
})
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
  justify-content: flex-start;
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
  width: 56px;
  height: 44px;
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
  margin-left: 40px;
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

.tab {
  transition: transform 0.2s ease, background 0.2s ease, color 0.2s ease;
  transform-origin: bottom center;
}

.tab:hover {
  background: #e5e7eb;
  color: #4b5563;
  transform: scale(1.05);
}

.tab.active {
  background: #f5f6f8;
  color: #1f2937;
  font-weight: 600;
  border-color: #e5e7eb #e5e7eb #f5f6f8;
  z-index: 1;
  transform: scale(1.08);
}

.status-area {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-left: auto;
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
  cursor: pointer;
  overflow: hidden;
  user-select: none;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.avatar:hover {
  transform: scale(1.08);
  border-color: #3b82f6;
}

.avatar.logged {
  background: #dbeafe;
  color: #1d4ed8;
  font-weight: 600;
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
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
