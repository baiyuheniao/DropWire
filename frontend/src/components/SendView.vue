<template>
  <div class="send-view">
    <div class="send-header">
      <h2>发送文件</h2>
      <button class="history-btn" @click="showHistory = true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <polyline points="1 4 1 10 7 10" />
          <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
        </svg>
        历史记录
      </button>
    </div>

    <div class="send-options">
      <div class="option-group">
        <label>发送对象</label>
        <div class="receiver-field">
          <select v-model="receiverMode" class="receiver-select">
            <option value="">全局发送</option>
            <option v-for="u in knownUsers" :key="u.username" :value="u.username">
              {{ u.nickname || u.username }}
            </option>
            <option value="__custom__">指定用户...</option>
          </select>
          <input
            v-if="receiverMode === '__custom__'"
            v-model="customReceiver"
            type="text"
            class="custom-receiver"
            placeholder="输入接收者用户名"
            maxlength="32"
          />
        </div>
      </div>

      <div class="option-group">
        <label>备注</label>
        <input
          v-model="remark"
          type="text"
          class="remark-input"
          placeholder="可选：添加备注"
          maxlength="120"
        />
      </div>

      <div class="option-group">
        <label class="checkbox-label">
          <input v-model="enableEncryption" type="checkbox" />
          <span>加密发送（AES-256-GCM）</span>
        </label>
        <input
          v-if="enableEncryption"
          v-model="password"
          type="password"
          class="password-input"
          placeholder="输入加密密码"
          maxlength="64"
        />
        <p v-if="enableEncryption" class="encrypt-hint">
          加密后文件内容会经过浏览器本地加密再上传，接收方需要相同密码才能解密。
        </p>
      </div>
    </div>

    <FileUpload :options="uploadOptions" />

    <HistoryModal
      v-if="showHistory"
      initial-tab="send"
      @close="showHistory = false"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import FileUpload from './FileUpload.vue'
import HistoryModal from './HistoryModal.vue'
import { type User } from './AccountModal.vue'

const props = defineProps<{
  user: User | null
}>()

const receiverMode = ref('')
const customReceiver = ref('')
const remark = ref('')
const enableEncryption = ref(false)
const password = ref('')
const knownUsers = ref<User[]>([])
const showHistory = ref(false)

onMounted(() => {
  const users: User[] = []
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i)
    if (key?.startsWith('dropwire_user_')) {
      try {
        const data = JSON.parse(localStorage.getItem(key) || '{}')
        if (data.username) {
          users.push({
            username: data.username,
            nickname: data.nickname || data.username,
            avatar: data.avatar,
          })
        }
      } catch {
        // ignore invalid entries
      }
    }
  }
  knownUsers.value = users
})

const effectiveReceiver = computed(() => {
  if (receiverMode.value === '__custom__') return customReceiver.value.trim()
  return receiverMode.value
})

const uploadOptions = computed(() => ({
  sender: props.user?.nickname || props.user?.username || undefined,
  receiver: effectiveReceiver.value || undefined,
  remark: remark.value.trim() || undefined,
  password: enableEncryption.value ? password.value : undefined,
}))
</script>

<style scoped>
.send-view {
  max-width: 720px;
  margin: 0 auto;
}

.send-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.send-header h2 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.history-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.history-btn:hover {
  background: var(--bg-soft);
  border-color: var(--primary);
  color: var(--primary);
}

.history-btn svg {
  width: 16px;
  height: 16px;
}

.send-options {
  display: grid;
  gap: 16px;
  margin-bottom: 20px;
  padding: 18px 20px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 16px;
}

.option-group label {
  display: block;
  margin-bottom: 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--text-secondary);
}

.receiver-field {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
}

.receiver-select,
.custom-receiver,
.remark-input {
  padding: 10px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  font-size: 14px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.receiver-select:focus,
.custom-receiver:focus,
.remark-input:focus {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.receiver-select {
  min-width: 160px;
  background: var(--bg-input);
}

.custom-receiver,
.remark-input {
  flex: 1;
  min-width: 0;
}

.checkbox-label {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  margin-bottom: 10px;
}

.checkbox-label input {
  width: 16px;
  height: 16px;
  accent-color: var(--primary);
  flex-shrink: 0;
}

.password-input {
  width: 100%;
  max-width: 320px;
  padding: 10px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  font-size: 14px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.password-input:focus {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.encrypt-hint {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-tertiary);
  line-height: 1.5;
}
</style>
