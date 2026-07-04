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

    <DeviceList v-model="selectedDevice" />

    <div class="send-options">
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
import { computed, ref } from 'vue'
import FileUpload from './FileUpload.vue'
import HistoryModal from './HistoryModal.vue'
import DeviceList from './DeviceList.vue'
import { type User } from './AccountModal.vue'
import { selfDevice } from '../composables/useDevices'
import type { DeviceInfo } from '../composables/useDevices'

const props = defineProps<{
  user: User | null
}>()

const selectedDevice = ref<DeviceInfo | null>(null)
const remark = ref('')
const enableEncryption = ref(false)
const password = ref('')
const showHistory = ref(false)

const targetUrl = computed(() => {
  if (selectedDevice.value) {
    return `http://${selectedDevice.value.ip}:${selectedDevice.value.port}`
  }
  return undefined
})

const uploadOptions = computed(() => ({
  sender: props.user?.nickname || props.user?.username || selfDevice.value?.name || undefined,
  receiver: selectedDevice.value?.id || undefined,
  targetUrl: targetUrl.value,
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

.remark-input:focus {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
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

@media (max-width: 640px) {
  .send-view {
    max-width: 100%;
  }

  .send-header h2 {
    font-size: 16px;
  }

  .history-btn {
    padding: 6px 10px;
    font-size: 13px;
  }

  .send-options {
    padding: 14px;
    border-radius: 12px;
  }

  .password-input {
    max-width: 100%;
  }
}
</style>
