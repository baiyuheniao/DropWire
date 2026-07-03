<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-header">
        <h3>文件详情</h3>
        <button class="close-btn" @click="emit('close')">&times;</button>
      </div>

      <div class="modal-body">
        <div class="file-title">
          <span class="filename" :title="task.filename">{{ task.filename }}</span>
          <span class="badge done">已完成</span>
        </div>

        <div class="qr-section">
          <div class="qr-code">
            <QRCodeVue v-if="task.downloadUrl" :value="task.downloadUrl" :size="180" level="M" />
          </div>
          <p class="qr-hint">同一局域网内扫码即可下载</p>
          <p v-if="task.expiresAt" class="qr-expires">
            有效期至 {{ formatExpireTime(task.expiresAt) }}
          </p>
          <p v-else class="qr-expires">永久有效</p>
        </div>

        <div class="link-section">
          <input :value="task.downloadUrl" readonly class="link-input" />
          <button class="copy-btn" @click="copyUrl">
            {{ copied ? '已复制' : '复制链接' }}
          </button>
        </div>

        <div v-if="props.sender || props.receiver || props.remark" class="meta-section">
          <div v-if="props.sender" class="meta-row">
            <span class="meta-label">发送者</span>
            <span class="meta-value">{{ props.sender }}</span>
          </div>
          <div v-if="props.receiver" class="meta-row">
            <span class="meta-label">接收者</span>
            <span class="meta-value">{{ props.receiver }}</span>
          </div>
          <div v-if="props.remark" class="meta-row">
            <span class="meta-label">备注</span>
            <span class="meta-value">{{ props.remark }}</span>
          </div>
          <div v-if="props.encrypted" class="meta-row">
            <span class="meta-label">加密</span>
            <span class="meta-value">是</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import QRCodeVue from 'qrcode.vue'
import type { UploadTask } from '../composables/useUpload'

const props = defineProps<{
  task: UploadTask
  sender?: string
  receiver?: string
  remark?: string
  encrypted?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const copied = ref(false)

async function copyUrl() {
  if (!props.task.downloadUrl) return
  try {
    await navigator.clipboard.writeText(props.task.downloadUrl)
  } catch {
    const input = document.createElement('input')
    input.value = props.task.downloadUrl
    document.body.appendChild(input)
    input.select()
    document.execCommand('copy')
    document.body.removeChild(input)
  }
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}

function formatExpireTime(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  width: 420px;
  max-width: 90vw;
  background: var(--bg-card);
  border-radius: 16px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 20px;
  border-bottom: 1px solid var(--border-color);
}

.modal-header h3 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.close-btn {
  background: none;
  border: none;
  font-size: 24px;
  color: var(--text-tertiary);
  cursor: pointer;
  line-height: 1;
}

.close-btn:hover {
  color: var(--text-secondary);
}

.modal-body {
  padding: 24px 20px;
}

.file-title {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 20px;
}

.filename {
  font-size: 15px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

.badge {
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 10px;
  white-space: nowrap;
  font-weight: 600;
  background: rgba(52, 211, 153, 0.12);
  color: var(--success-text);
  border: 1px solid rgba(34, 197, 94, 0.25);
}

.qr-section {
  text-align: center;
  margin-bottom: 20px;
}

.qr-code {
  width: 180px;
  height: 180px;
  margin: 0 auto 12px;
  border-radius: 12px;
  overflow: hidden;
  background: #fff;
  border: 1px solid var(--border-color);
}

.qr-hint {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.qr-expires {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-tertiary);
}

.link-section {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

.link-input {
  flex: 1;
  min-width: 0;
  padding: 10px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  font-size: 13px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
}

.copy-btn {
  padding: 9px 16px;
  background: var(--primary);
  color: #fff;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
  white-space: nowrap;
}

.copy-btn:hover {
  background: var(--primary-hover);
}

.meta-section {
  padding-top: 16px;
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.meta-row {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
}

.meta-label {
  color: var(--text-tertiary);
}

.meta-value {
  color: var(--text-primary);
  font-weight: 500;
}
</style>
