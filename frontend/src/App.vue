<template>
  <div class="app">
    <header class="top-bar">
      <div class="brand">
        <img class="brand-logo" src="/logo.png?v=2" alt="DropWire" />
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
import { computed, onMounted, ref, watch } from 'vue'
import { useWebSocket } from './composables/useWebSocket'
import { loadSettings, settings, type ThemeMode } from './composables/useSettings'
import SendView from './components/SendView.vue'
import ReceiveView from './components/ReceiveView.vue'
import SettingsView from './components/SettingsView.vue'
import AccountModal, { type User } from './components/AccountModal.vue'

loadSettings()

const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

function effectiveTheme(): Exclude<ThemeMode, 'system'> {
  if (settings.value.theme === 'system') {
    return mediaQuery.matches ? 'dark' : 'light'
  }
  return settings.value.theme
}

function applyTheme() {
  document.documentElement.classList.toggle('dark', effectiveTheme() === 'dark')
}

applyTheme()
watch(() => settings.value.theme, applyTheme)
mediaQuery.addEventListener('change', applyTheme)

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
:root {
  color-scheme: light;
  --bg-body: #f5f6f8;
  --bg-card: #ffffff;
  --bg-card-hover: #f9fafb;
  --bg-input: #ffffff;
  --bg-soft: #f3f4f6;
  --bg-primary-soft: #eff6ff;
  --border-color: #e5e7eb;
  --border-strong: #d1d5db;
  --text-primary: #1f2937;
  --text-secondary: #374151;
  --text-tertiary: #6b7280;
  --primary: #3b82f6;
  --primary-hover: #2563eb;
  --primary-text: #1d4ed8;
  --danger-bg: #fee2e2;
  --danger-text: #b91c1c;
  --success-text: #047857;
  --warning-bg: #fef3c7;
  --warning-text: #b45309;
  --shadow: rgba(0, 0, 0, 0.04);
}

html.dark {
  color-scheme: dark;
  --bg-body: #0f172a;
  --bg-card: #1f2937;
  --bg-card-hover: #27354f;
  --bg-input: #111827;
  --bg-soft: #1f2937;
  --bg-primary-soft: #1e3a8a;
  --border-color: #374151;
  --border-strong: #4b5563;
  --text-primary: #f9fafb;
  --text-secondary: #e5e7eb;
  --text-tertiary: #9ca3af;
  --primary: #60a5fa;
  --primary-hover: #3b82f6;
  --primary-text: #93c5fd;
  --danger-bg: #450a0a;
  --danger-text: #fca5a5;
  --success-text: #34d399;
  --warning-bg: #451a03;
  --warning-text: #fcd34d;
  --shadow: rgba(0, 0, 0, 0.25);
}

*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  background: var(--bg-body);
  color: var(--text-primary);
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
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  box-shadow: 0 1px 3px var(--shadow);
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
  color: var(--text-secondary);
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
  border: 1px solid var(--border-strong);
  border-bottom: none;
  border-radius: 12px 12px 0 0;
  background: var(--bg-soft);
  color: var(--text-tertiary);
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  transition: transform 0.2s ease, background 0.2s ease, color 0.2s ease;
  transform-origin: bottom center;
  position: relative;
  top: 1px;
}

.tab:hover {
  background: var(--border-color);
  color: var(--text-secondary);
  transform: scale(1.05);
}

.tab.active {
  background: var(--bg-body);
  color: var(--text-primary);
  font-weight: 600;
  border-color: var(--border-color) var(--border-color) var(--bg-body);
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
  background: var(--bg-soft);
  border: 1px solid var(--border-color);
  display: grid;
  place-items: center;
  font-size: 12px;
  color: var(--text-tertiary);
  cursor: pointer;
  overflow: hidden;
  user-select: none;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.avatar:hover {
  transform: scale(1.08);
  border-color: var(--primary);
}

.avatar.logged {
  background: var(--bg-primary-soft);
  color: var(--primary-text);
  font-weight: 600;
}

.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.status-label {
  font-size: 13px;
  color: var(--text-tertiary);
}

.status-badge {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  background: var(--danger-bg);
  color: var(--danger-text);
}

.status-badge.online {
  background: var(--primary);
  color: #fff;
}

.main-content {
  flex: 1;
  padding: 32px 24px;
}

@media (max-width: 640px) {
  .top-bar {
    flex-wrap: wrap;
    height: auto;
    padding: 10px 12px;
    gap: 8px;
  }

  .brand-logo {
    width: 40px;
    height: 32px;
  }

  .brand-name {
    font-size: 16px;
  }

  .tabs {
    order: 3;
    width: 100%;
    margin-left: 0;
    height: auto;
    justify-content: center;
    gap: 8px;
  }

  .tab {
    flex: 1;
    padding: 8px 12px;
    border-radius: 10px;
    border: 1px solid var(--border-strong);
    font-size: 14px;
    top: 0;
  }

  .tab.active {
    border-color: var(--primary);
    background: var(--bg-primary-soft);
  }

  .status-area {
    margin-left: auto;
    gap: 6px;
  }

  .status-label {
    display: none;
  }

  .main-content {
    padding: 16px 12px;
  }
}
</style>
