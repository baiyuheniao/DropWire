<template>
  <div class="settings-view">
    <div class="settings-card">
      <h2>设置</h2>

      <section class="section">
        <h3>账号信息</h3>
        <div class="account-row">
          <div class="avatar-large" :class="{ logged: user }">
            <img v-if="user?.avatar" :src="user.avatar" alt="avatar" />
            <span v-else>{{ user ? user.nickname.charAt(0) : '?' }}</span>
          </div>
          <div class="account-meta">
            <div class="name">{{ user ? user.nickname : '未登录' }}</div>
            <div v-if="user" class="username">@{{ user.username }}</div>
          </div>
        </div>
        <div class="actions">
          <button class="btn-primary" @click="emit('edit-account')">
            {{ user ? '编辑账户' : '登录 / 注册' }}
          </button>
          <button v-if="user" class="btn-danger" @click="emit('logout')">
            退出登录
          </button>
        </div>
      </section>

      <section class="section">
        <h3>外观</h3>
        <div class="theme-options">
          <button
            v-for="opt in themeOptions"
            :key="opt.value"
            class="theme-btn"
            :class="{ active: form.theme === opt.value }"
            @click="saveTheme(opt.value)"
          >
            {{ opt.label }}
          </button>
        </div>
      </section>

      <section class="section">
        <h3>服务器地址</h3>
        <p class="hint">留空则使用当前站点代理；修改后需刷新页面生效</p>
        <div class="input-row">
          <input
            v-model="form.apiBase"
            type="text"
            placeholder="http://localhost:3000"
          />
          <button class="btn-primary" @click="saveServer">保存</button>
        </div>
        <p v-if="serverSaved" class="success-msg">已保存，刷新页面后生效</p>
      </section>

      <section class="section">
        <h3>发送二维码有效期</h3>
        <p class="hint">发送文件生成的二维码与下载链接在多久后自动失效</p>
        <div class="theme-options">
          <button
            v-for="opt in qrOptions"
            :key="opt.value"
            class="theme-btn"
            :class="{ active: form.qrValidityMinutes === opt.value }"
            @click="saveQr(opt.value)"
          >
            {{ opt.label }}
          </button>
        </div>
      </section>

      <section class="section">
        <h3>接收列表</h3>
        <div class="refresh-row">
          <label class="checkbox-row">
            <input v-model="form.autoRefresh" type="checkbox" />
            <span>自动刷新文件列表</span>
          </label>
          <div v-if="form.autoRefresh" class="interval-row">
            <span>刷新间隔（秒）</span>
            <input
              v-model.number="form.refreshInterval"
              type="number"
              min="3"
              max="300"
            />
          </div>
        </div>
        <button class="btn-primary save-btn" @click="saveRefresh">保存</button>
      </section>

      <section class="section">
        <h3>关于</h3>
        <p class="hint">DropWire v0.1.0 · 局域网文件传输</p>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { type User } from './AccountModal.vue'
import { QR_VALIDITY_OPTIONS, settings, saveSettings, type ThemeMode } from '../composables/useSettings'

defineProps<{
  user: User | null
}>()

const emit = defineEmits<{
  (e: 'edit-account'): void
  (e: 'logout'): void
}>()

const form = reactive({ ...settings.value })
const serverSaved = ref(false)

const themeOptions: { value: ThemeMode; label: string }[] = [
  { value: 'light', label: '浅色' },
  { value: 'dark', label: '深色' },
  { value: 'system', label: '跟随系统' },
]

const qrOptions = QR_VALIDITY_OPTIONS

function saveTheme(value: ThemeMode) {
  form.theme = value
  saveSettings({ theme: value })
}

function saveServer() {
  saveSettings({ apiBase: form.apiBase.trim() })
  serverSaved.value = true
  setTimeout(() => (serverSaved.value = false), 3000)
}

function saveQr(value: number) {
  form.qrValidityMinutes = value
  saveSettings({ qrValidityMinutes: value })
}

function saveRefresh() {
  const interval = Math.max(3, Math.min(300, Number(form.refreshInterval) || 10))
  saveSettings({
    autoRefresh: form.autoRefresh,
    refreshInterval: interval,
  })
}
</script>

<style scoped>
.settings-view {
  max-width: 720px;
  margin: 0 auto;
}

.settings-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 16px;
  padding: 24px;
}

.settings-card h2 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 20px;
}

.section {
  padding: 20px 0;
  border-bottom: 1px solid var(--border-color);
}

.section:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.section h3 {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 14px;
}

.hint {
  font-size: 13px;
  color: var(--text-tertiary);
  margin-bottom: 12px;
}

.account-row {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 16px;
}

.avatar-large {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  background: var(--bg-soft);
  border: 1px solid var(--border-color);
  display: grid;
  place-items: center;
  font-size: 18px;
  color: var(--text-tertiary);
  overflow: hidden;
  flex-shrink: 0;
}

.avatar-large.logged {
  background: var(--bg-primary-soft);
  color: var(--primary-text);
  font-weight: 600;
}

.avatar-large img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.account-meta {
  min-width: 0;
}

.name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.username {
  font-size: 13px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.theme-options {
  display: inline-flex;
  gap: 8px;
  flex-wrap: wrap;
}

.theme-btn {
  padding: 8px 16px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  background: var(--bg-soft);
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.theme-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}

.theme-btn.active {
  background: var(--primary);
  color: #fff;
  border-color: var(--primary);
}

.input-row {
  display: flex;
  gap: 12px;
  align-items: center;
}

.input-row input {
  flex: 1;
  min-width: 0;
  padding: 10px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  font-size: 14px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.input-row input:focus {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.refresh-row {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.interval-row {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
  color: var(--text-secondary);
}

.interval-row input {
  width: 80px;
  padding: 8px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  font-size: 14px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
}

.checkbox-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
}

.checkbox-row input {
  width: 16px;
  height: 16px;
  accent-color: var(--primary);
}

.save-btn {
  margin-top: 12px;
}

.btn-primary,
.btn-danger {
  padding: 9px 16px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-primary {
  background: var(--primary);
  color: #fff;
}

.btn-primary:hover {
  background: var(--primary-hover);
}

.btn-danger {
  background: var(--danger-bg);
  color: var(--danger-text);
}

.btn-danger:hover {
  opacity: 0.9;
}

.success-msg {
  margin-top: 10px;
  font-size: 13px;
  color: var(--success-text);
}
</style>
