<template>
  <div class="receive-view">
    <div class="receive-card">
      <div class="receive-header">
        <h2>可接收文件</h2>
        <div class="header-actions">
          <button class="history-btn" @click="showHistory = true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
            </svg>
            历史记录
          </button>
          <button class="refresh-btn" :disabled="loading" @click="fetchFiles">
            {{ loading ? '刷新中...' : '刷新列表' }}
          </button>
        </div>
      </div>

      <div v-if="error" class="error-msg">{{ error }}</div>

      <div v-if="filteredFiles.length === 0 && !loading" class="empty-state">
        <div class="icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
        </div>
        <p>暂无文件，请让发送方先上传</p>
      </div>

      <ul v-else class="file-list">
        <li v-for="file in filteredFiles" :key="filePath(file)" class="file-item">
          <div class="file-info">
            <div class="file-name" :title="filePath(file)">{{ file.filename }}</div>
            <div class="file-tags">
              <span v-if="file.encrypted" class="tag encrypted">🔒 加密</span>
              <span v-if="file.sender" class="tag sender">来自: {{ file.sender }}</span>
              <span v-if="file.receiver" class="tag receiver">发给: {{ file.receiver }}</span>
              <span v-if="file.remark" class="tag remark">备注: {{ file.remark }}</span>
              <span v-if="file.received" class="tag confirmed">✓ 已接收</span>
            </div>
            <div class="file-meta">
              <span>{{ formatSize(file.size) }}</span>
              <span v-if="file.modified_at">{{ formatTime(file.modified_at) }}</span>
            </div>
          </div>

          <div class="download-area">
            <template v-if="file.encrypted">
              <input
                v-model="decryptPasswords[filePath(file)]"
                type="password"
                class="decrypt-input"
                placeholder="输入密码"
              />
              <button class="download-btn" @click="downloadDecrypted(file)">
                解密下载
              </button>
            </template>
            <button v-else class="download-btn" @click="download(file)">
              下载
            </button>
          </div>
        </li>
      </ul>
    </div>

    <HistoryModal
      v-if="showHistory"
      initial-tab="receive"
      @close="showHistory = false"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import axios from 'axios'
import { type User } from './AccountModal.vue'
import HistoryModal from './HistoryModal.vue'
import { settings } from '../composables/useSettings'
import { decryptFile } from '../composables/useCrypto'
import { addHistory } from '../composables/useHistory'
import { selfDevice, fetchSelfDevice } from '../composables/useDevices'
import { notify, requestNotificationPermission } from '../composables/useNotifications'
import { useWebSocket } from '../composables/useWebSocket'

interface FileInfo {
  filename: string
  relative_path?: string
  size: number
  modified_at?: number
  sender?: string
  receiver?: string
  remark?: string
  encrypted?: boolean
  salt?: string
  iv?: string
  received?: boolean
  received_at?: number
  received_by?: string
}

function filePath(file: FileInfo): string {
  if (file.relative_path) {
    return `${file.relative_path}/${file.filename}`
  }
  return file.filename
}

const props = defineProps<{
  user: User | null
}>()

const files = ref<FileInfo[]>([])
const loading = ref(false)
const error = ref('')
const showHistory = ref(false)
const decryptPasswords = reactive<Record<string, string>>({})
const knownFilenames = ref<Set<string>>(new Set())

const selfDeviceId = computed(() => selfDevice.value?.id || props.user?.username || '')

const wsUrl = computed(() => {
  const base = settings.value.apiBase.trim()
  if (!base) return '/ws'
  return base.replace(/^http/, 'ws') + '/ws'
})
const { received } = useWebSocket(wsUrl.value)

watch(
  () => received.value,
  (next) => {
    next.forEach((evt) => {
      const file = files.value.find((f) => filePath(f) === evt.filename)
      if (file) {
        file.received = true
        file.received_at = evt.received_at
        file.received_by = evt.received_by
      }
    })
  },
  { deep: true },
)

const filteredFiles = computed(() => {
  return files.value.filter((f) => {
    if (!f.receiver) return true
    return f.receiver === selfDeviceId.value || f.receiver === props.user?.username
  })
})

async function fetchFiles() {
  loading.value = true
  error.value = ''
  try {
    const res = await axios.get('/files')
    if (res.data.success) {
      const next = (res.data.data || []) as FileInfo[]
      if (settings.value.notificationsEnabled && knownFilenames.value.size > 0) {
        const newFiles = next.filter((f) => !knownFilenames.value.has(filePath(f)))
        for (const f of newFiles.slice(0, 3)) {
          notify('收到新文件', { body: filePath(f) })
        }
      }
      knownFilenames.value = new Set(next.map((f) => filePath(f)))
      files.value = next
    } else {
      error.value = res.data.message || '获取文件列表失败'
    }
  } catch (err: any) {
    error.value = err?.response?.data?.message || '获取文件列表失败'
  } finally {
    loading.value = false
  }
}

function triggerDownload(blob: Blob, filename: string) {
  const url = window.URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  window.URL.revokeObjectURL(url)
}

async function markReceived(path: string) {
  try {
    await axios.post('/files/received', {
      filename: path,
      received_by: selfDeviceId.value || undefined,
    })
    const file = files.value.find((f) => filePath(f) === path)
    if (file) {
      file.received = true
      file.received_at = Math.floor(Date.now() / 1000)
      file.received_by = selfDeviceId.value
    }
  } catch (err: any) {
    // Non-blocking: the receiver can still use the file.
    console.warn('标记已接收失败', err)
  }
}

async function download(file: FileInfo) {
  const path = filePath(file)
  try {
    const res = await axios.get(`/download/${encodeURIComponent(path)}`, {
      responseType: 'blob',
    })
    triggerDownload(new Blob([res.data]), file.filename)
    recordReceive({ ...file, filename: path })
    markReceived(path)
  } catch (err: any) {
    error.value = err?.response?.data?.message || '下载失败'
  }
}

async function downloadDecrypted(file: FileInfo) {
  error.value = ''
  const path = filePath(file)
  const password = decryptPasswords[path]
  if (!password) {
    error.value = '请输入解密密码'
    return
  }
  if (!file.salt || !file.iv) {
    error.value = '缺少加密参数，无法解密'
    return
  }
  try {
    const res = await axios.get(`/download/${encodeURIComponent(path)}`, {
      responseType: 'arraybuffer',
    })
    const plaintext = await decryptFile(res.data, password, file.salt, file.iv)
    triggerDownload(new Blob([plaintext]), file.filename)
    recordReceive({ ...file, filename: path })
    markReceived(path)
  } catch (err: any) {
    error.value = '解密失败：密码错误或文件损坏'
  }
}

function recordReceive(file: { filename: string; size: number; sender?: string; receiver?: string; remark?: string; encrypted?: boolean }) {
  addHistory({
    type: 'receive',
    filename: file.filename,
    size: file.size,
    sender: file.sender,
    receiver: file.receiver,
    remark: file.remark,
    encrypted: file.encrypted,
  })
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

function formatTime(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

let refreshTimer: ReturnType<typeof setInterval> | null = null

function startAutoRefresh() {
  stopAutoRefresh()
  if (settings.value.autoRefresh && settings.value.refreshInterval >= 3) {
    refreshTimer = setInterval(() => {
      if (!loading.value) fetchFiles()
    }, settings.value.refreshInterval * 1000)
  }
}

function stopAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

watch(
  () => ({ auto: settings.value.autoRefresh, interval: settings.value.refreshInterval }),
  startAutoRefresh,
  { deep: true },
)

onMounted(() => {
  fetchSelfDevice()
  fetchFiles()
  startAutoRefresh()
  if (settings.value.notificationsEnabled) {
    requestNotificationPermission()
  }
})

onUnmounted(stopAutoRefresh)
</script>

<style scoped>
.receive-view {
  max-width: 800px;
  margin: 0 auto;
  padding: 40px 20px;
}

.receive-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 16px;
  padding: 24px;
}

.receive-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.receive-header h2 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.history-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  background: var(--bg-soft);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s, color 0.2s;
}

.history-btn:hover {
  background: var(--bg-card);
  border-color: var(--primary);
  color: var(--primary);
}

.history-btn svg {
  width: 16px;
  height: 16px;
}

.refresh-btn {
  padding: 8px 16px;
  background: var(--bg-soft);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.2s;
}

.refresh-btn:hover:not(:disabled) {
  background: var(--border-color);
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-msg {
  margin-bottom: 16px;
  padding: 10px 14px;
  background: var(--danger-bg);
  color: var(--danger-text);
  border-radius: 8px;
  font-size: 14px;
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-tertiary);
}

.empty-state .icon {
  width: 64px;
  height: 64px;
  margin: 0 auto 16px;
  border-radius: 50%;
  background: var(--bg-primary-soft);
  color: var(--primary);
  display: grid;
  place-items: center;
}

.empty-state .icon svg {
  width: 28px;
  height: 28px;
}

.empty-state p {
  font-size: 14px;
}

.file-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.file-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  transition: background 0.2s, border-color 0.2s;
}

.file-item:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-strong);
}

.file-info {
  min-width: 0;
  flex: 1;
}

.file-name {
  font-size: 15px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-tags {
  margin-top: 6px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag {
  display: inline-flex;
  align-items: center;
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
}

.tag.sender {
  background: var(--bg-primary-soft);
  color: var(--primary-text);
}

.tag.receiver {
  background: #f3e8ff;
  color: #7e22ce;
}

.tag.remark {
  background: #ecfdf5;
  color: #047857;
}

.tag.encrypted {
  background: var(--warning-bg);
  color: var(--warning-text);
}

.tag.confirmed {
  background: rgba(52, 211, 153, 0.12);
  color: var(--success-text);
}

.file-meta {
  margin-top: 6px;
  font-size: 13px;
  color: var(--text-tertiary);
  display: flex;
  gap: 12px;
}

.download-area {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.decrypt-input {
  width: 120px;
  padding: 8px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  font-size: 13px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
}

.decrypt-input:focus {
  border-color: var(--primary);
}

.download-btn {
  padding: 8px 16px;
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

.download-btn:hover {
  background: var(--primary-hover);
}

@media (max-width: 640px) {
  .receive-view {
    max-width: 100%;
    padding: 0;
  }

  .receive-card {
    padding: 16px;
    border-radius: 12px;
  }

  .receive-header h2 {
    font-size: 16px;
  }

  .file-item {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
    padding: 12px;
  }

  .file-name {
    font-size: 14px;
  }

  .download-area {
    width: 100%;
    justify-content: flex-start;
  }

  .decrypt-input {
    flex: 1;
    width: auto;
    min-width: 0;
  }

  .download-btn {
    flex: 1;
    text-align: center;
  }
}
</style>
