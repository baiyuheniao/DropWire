<template>
  <div class="receive-view">
    <div class="receive-card">
      <div class="receive-header">
        <h2>可接收文件</h2>
        <button class="refresh-btn" :disabled="loading" @click="fetchFiles">
          {{ loading ? '刷新中...' : '刷新列表' }}
        </button>
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
        <li v-for="file in filteredFiles" :key="file.filename" class="file-item">
          <div class="file-info">
            <div class="file-name" :title="file.filename">{{ file.filename }}</div>
            <div class="file-tags">
              <span v-if="file.encrypted" class="tag encrypted">🔒 加密</span>
              <span v-if="file.sender" class="tag sender">来自: {{ file.sender }}</span>
              <span v-if="file.receiver" class="tag receiver">发给: {{ file.receiver }}</span>
              <span v-if="file.remark" class="tag remark">备注: {{ file.remark }}</span>
            </div>
            <div class="file-meta">
              <span>{{ formatSize(file.size) }}</span>
              <span v-if="file.modified_at">{{ formatTime(file.modified_at) }}</span>
            </div>
          </div>

          <div class="download-area">
            <template v-if="file.encrypted">
              <input
                v-model="decryptPasswords[file.filename]"
                type="password"
                class="decrypt-input"
                placeholder="输入密码"
              />
              <button class="download-btn" @click="downloadDecrypted(file)">
                解密下载
              </button>
            </template>
            <button v-else class="download-btn" @click="download(file.filename)">
              下载
            </button>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import axios from 'axios'
import { type User } from './AccountModal.vue'
import { settings } from '../composables/useSettings'
import { decryptFile } from '../composables/useCrypto'

interface FileInfo {
  filename: string
  size: number
  modified_at?: number
  sender?: string
  receiver?: string
  remark?: string
  encrypted?: boolean
  salt?: string
  iv?: string
}

const props = defineProps<{
  user: User | null
}>()

const files = ref<FileInfo[]>([])
const loading = ref(false)
const error = ref('')
const decryptPasswords = reactive<Record<string, string>>({})

const filteredFiles = computed(() => {
  if (!props.user) {
    return files.value.filter((f) => !f.receiver)
  }
  return files.value.filter((f) => !f.receiver || f.receiver === props.user!.username)
})

async function fetchFiles() {
  loading.value = true
  error.value = ''
  try {
    const res = await axios.get('/files')
    if (res.data.success) {
      files.value = res.data.data || []
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

async function download(filename: string) {
  try {
    const res = await axios.get(`/download/${encodeURIComponent(filename)}`, {
      responseType: 'blob',
    })
    triggerDownload(new Blob([res.data]), filename)
  } catch (err: any) {
    error.value = err?.response?.data?.message || '下载失败'
  }
}

async function downloadDecrypted(file: FileInfo) {
  error.value = ''
  const password = decryptPasswords[file.filename]
  if (!password) {
    error.value = '请输入解密密码'
    return
  }
  if (!file.salt || !file.iv) {
    error.value = '缺少加密参数，无法解密'
    return
  }
  try {
    const res = await axios.get(`/download/${encodeURIComponent(file.filename)}`, {
      responseType: 'arraybuffer',
    })
    const plaintext = await decryptFile(res.data, password, file.salt, file.iv)
    triggerDownload(new Blob([plaintext]), file.filename)
  } catch (err: any) {
    error.value = '解密失败：密码错误或文件损坏'
  }
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
  fetchFiles()
  startAutoRefresh()
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
  background: #fff;
  border: 1px solid #e5e7eb;
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
  color: #1f2937;
}

.refresh-btn {
  padding: 8px 16px;
  background: #f3f4f6;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  font-size: 14px;
  color: #374151;
  cursor: pointer;
  transition: background 0.2s;
}

.refresh-btn:hover:not(:disabled) {
  background: #e5e7eb;
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-msg {
  margin-bottom: 16px;
  padding: 10px 14px;
  background: #fee2e2;
  color: #b91c1c;
  border-radius: 8px;
  font-size: 14px;
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: #6b7280;
}

.empty-state .icon {
  width: 64px;
  height: 64px;
  margin: 0 auto 16px;
  border-radius: 50%;
  background: #eff6ff;
  color: #3b82f6;
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
  border: 1px solid #e5e7eb;
  border-radius: 12px;
  transition: background 0.2s, border-color 0.2s;
}

.file-item:hover {
  background: #f9fafb;
  border-color: #d1d5db;
}

.file-info {
  min-width: 0;
  flex: 1;
}

.file-name {
  font-size: 15px;
  font-weight: 500;
  color: #1f2937;
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
  background: #dbeafe;
  color: #1d4ed8;
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
  background: #fef3c7;
  color: #b45309;
}

.file-meta {
  margin-top: 6px;
  font-size: 13px;
  color: #6b7280;
  display: flex;
  gap: 12px;
}

.download-area {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.decrypt-input {
  width: 120px;
  padding: 8px 10px;
  border: 1px solid #d1d5db;
  border-radius: 8px;
  font-size: 13px;
  outline: none;
}

.decrypt-input:focus {
  border-color: #3b82f6;
}

.download-btn {
  padding: 8px 16px;
  background: #3b82f6;
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
  background: #2563eb;
}
</style>
