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
        <h3>接收列表</h3>
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
import { settings, saveSettings } from '../composables/useSettings'

defineProps<{
  user: User | null
}>()

const emit = defineEmits<{
  (e: 'edit-account'): void
  (e: 'logout'): void
}>()

const form = reactive({ ...settings.value })
const serverSaved = ref(false)

function saveServer() {
  saveSettings({ apiBase: form.apiBase.trim() })
  serverSaved.value = true
  setTimeout(() => (serverSaved.value = false), 3000)
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
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 16px;
  padding: 24px;
}

.settings-card h2 {
  font-size: 18px;
  font-weight: 600;
  color: #1f2937;
  margin-bottom: 20px;
}

.section {
  padding: 20px 0;
  border-bottom: 1px solid #f3f4f6;
}

.section:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.section h3 {
  font-size: 15px;
  font-weight: 600;
  color: #374151;
  margin-bottom: 14px;
}

.hint {
  font-size: 13px;
  color: #6b7280;
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
  background: #f3f4f6;
  border: 1px solid #e5e7eb;
  display: grid;
  place-items: center;
  font-size: 18px;
  color: #6b7280;
  overflow: hidden;
  flex-shrink: 0;
}

.avatar-large.logged {
  background: #dbeafe;
  color: #1d4ed8;
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
  color: #1f2937;
}

.username {
  font-size: 13px;
  color: #6b7280;
  margin-top: 2px;
}

.actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
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
  border: 1px solid #d1d5db;
  border-radius: 10px;
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.input-row input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.interval-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 12px 0;
  font-size: 14px;
  color: #374151;
}

.interval-row input {
  width: 80px;
  padding: 8px 10px;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  font-size: 14px;
  outline: none;
}

.checkbox-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #374151;
  cursor: pointer;
}

.checkbox-row input {
  width: 16px;
  height: 16px;
  accent-color: #3b82f6;
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
  background: #3b82f6;
  color: #fff;
}

.btn-primary:hover {
  background: #2563eb;
}

.btn-danger {
  background: #fee2e2;
  color: #b91c1c;
}

.btn-danger:hover {
  background: #fecaca;
}

.success-msg {
  margin-top: 10px;
  font-size: 13px;
  color: #047857;
}
</style>
