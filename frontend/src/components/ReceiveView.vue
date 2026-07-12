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
        <li v-for="file in filteredFiles" :key="`${filePath(file)}|${file.size}`" class="file-item">
          <div class="file-main">
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

            <div class="file-actions">
              <div class="download-area">
                <!-- 预览按钮：仅支持未加密的可预览文件类型 -->
                <button
                  v-if="!file.encrypted && canPreview(file.filename)"
                  class="preview-btn"
                  @click="openPreview(file)"
                  title="预览文件"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
                  </svg>
                  预览
                </button>

                <!-- 下载中 / 暂停 / 完成：显示进度条 + 控制按钮 -->
                <div
                  v-if="downloadTasks[filePath(file)]"
                  class="download-progress-wrapper"
                >
                  <div class="download-progress-track">
                    <div
                      class="download-progress-bar"
                      :class="downloadTasks[filePath(file)].status"
                      :style="{ width: downloadPct(filePath(file)) + '%' }"
                    />
                  </div>
                  <div class="download-progress-info">
                    <span class="download-progress-text">{{ downloadPct(filePath(file)) }}%</span>
                    <span class="download-progress-speed">
                      {{ formatDownloadSpeed(downloadTasks[filePath(file)].speedBps) }}
                    </span>
                    <span class="download-progress-eta">
                      剩余 {{ formatEta(downloadTasks[filePath(file)].etaSeconds) }}
                    </span>
                  </div>
                  <button
                    v-if="downloadTasks[filePath(file)].status === 'downloading'"
                    class="pause-btn"
                    @click="pauseDownload(filePath(file))"
                  >
                    暂停
                  </button>
                  <button
                    v-if="downloadTasks[filePath(file)].status === 'paused'"
                    class="resume-btn"
                    @click="resumeDownload(file)"
                  >
                    继续
                  </button>
                  <button
                    v-if="downloadTasks[filePath(file)].status === 'error'"
                    class="resume-btn"
                    @click="resumeDownload(file)"
                  >
                    重试
                  </button>
                </div>

                <!-- 初始下载按钮 -->
                <template v-else-if="file.encrypted">
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

              <div class="verify-toggle">
                <button
                  class="verify-toggle-btn"
                  :class="{ expanded: verifyExpanded[filePath(file)] }"
                  @click="toggleVerify(filePath(file))"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="6 9 12 15 18 9" />
                  </svg>
                  文件校验
                </button>
              </div>
            </div>
          </div>

          <div v-if="verifyExpanded[filePath(file)]" class="verify-area">
            <select v-model="getVerifyState(file).algorithm" class="verify-select">
              <option v-for="alg in HASH_ALGORITHMS" :key="alg.value" :value="alg.value">
                {{ alg.label }}
              </option>
            </select>
            <input
              v-model="getVerifyState(file).expected"
              type="text"
              class="verify-input"
              placeholder="预期校验值"
            />
            <button
              class="verify-btn"
              :disabled="!downloadedBuffers[filePath(file)]"
              @click="runVerify(file, downloadedBuffers[filePath(file)])"
            >
              校验
            </button>
            <span
              v-if="getVerifyState(file).result"
              class="verify-result"
              :class="getVerifyState(file).result"
            >
              {{ verifyLabel(getVerifyState(file).result) }}
            </span>
          </div>
        </li>
      </ul>
    </div>

    <HistoryModal
      v-if="showHistory"
      initial-tab="receive"
      @close="showHistory = false"
    />

    <FilePreviewModal
      v-if="previewingFile"
      :url="previewUrl"
      :filename="previewingFile.filename"
      :size="previewingFile.size"
      @close="previewingFile = null"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import axios from 'axios'
import { type User } from '../composables/useAuth'
import HistoryModal from './HistoryModal.vue'
import FilePreviewModal from './FilePreviewModal.vue'
import { settings } from '../composables/useSettings'
import { decryptFile } from '../composables/useCrypto'
import { addHistory } from '../composables/useHistory'
import { selfDevice, fetchSelfDevice } from '../composables/useDevices'
import { notify, requestNotificationPermission } from '../composables/useNotifications'
import { useWebSocket } from '../composables/useWebSocket'
import { HASH_ALGORITHMS, computeHash, normalizeHashType } from '../composables/useHash'
import {
  useDownload,
  startDownload,
  pauseDownload as pauseDownloadTask,
  getDownloadProgress,
  formatDownloadSpeed,
  formatEta,
} from '../composables/useDownload'

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
  hash_type?: string
  hash_value?: string
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
const downloadedBuffers = reactive<Record<string, ArrayBuffer>>({})
const { tasks: downloadTasks } = useDownload()
const previewingFile = ref<FileInfo | null>(null)
const verifyState = reactive<
  Record<string, {
    algorithm: string
    expected: string
    result: 'match' | 'mismatch' | 'computing' | ''
    actual?: string
  }>
>({})
const verifyExpanded = reactive<Record<string, boolean>>({})

const previewUrl = computed(() => {
  if (!previewingFile.value) return ''
  return `/download/${encodeURIComponent(filePath(previewingFile.value))}`
})

function canPreview(filename: string): boolean {
  const ext = filename.split('.').pop()?.toLowerCase() || ''
  return ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'mp4', 'webm', 'ogg', 'mp3', 'wav', 'txt', 'md', 'json', 'xml', 'pdf', 'html', 'css', 'js', 'ts'].includes(ext)
}

function openPreview(file: FileInfo) {
  previewingFile.value = file
}

function getVerifyState(file: FileInfo) {
  const path = filePath(file)
  if (!verifyState[path]) {
    verifyState[path] = {
      algorithm: normalizeHashType(file.hash_type),
      expected: file.hash_value || '',
      result: '',
    }
  }
  return verifyState[path]
}

function toggleVerify(path: string) {
  verifyExpanded[path] = !verifyExpanded[path]
}

async function runVerify(file: FileInfo, buffer: ArrayBuffer) {
  const state = getVerifyState(file)
  if (!state.expected) {
    state.result = ''
    return
  }
  state.result = 'computing'
  try {
    const actual = await computeHash(buffer, state.algorithm)
    state.actual = actual
    state.result = actual.toLowerCase() === state.expected.toLowerCase().trim() ? 'match' : 'mismatch'
  } catch (err: any) {
    state.result = 'mismatch'
    state.actual = err?.message || '计算失败'
  }
}

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
  const url = `/download/${encodeURIComponent(path)}`
  try {
    const buffer = await startDownload(path, file.filename, file.size, url)
    downloadedBuffers[path] = buffer
    triggerDownload(new Blob([buffer]), file.filename)
    recordReceive({ ...file, filename: path })
    markReceived(path)
    await runVerify(file, buffer)
  } catch (err: any) {
    if (downloadTasks[path]?.status === 'paused') return // 用户主动暂停，不报错
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
  const url = `/download/${encodeURIComponent(path)}`
  try {
    const ciphertext = await startDownload(path, file.filename, file.size, url)
    const plaintext = await decryptFile(ciphertext, password, file.salt, file.iv)
    downloadedBuffers[path] = plaintext
    triggerDownload(new Blob([plaintext]), file.filename)
    recordReceive({ ...file, filename: path })
    markReceived(path)
    await runVerify(file, plaintext)
  } catch (err: any) {
    if (downloadTasks[path]?.status === 'paused') return
    error.value = '解密失败：密码错误或文件损坏'
  }
}

/** 暂停下载 */
function pauseDownload(path: string) {
  pauseDownloadTask(path)
}

/** 继续 / 重试下载 */
function resumeDownload(file: FileInfo) {
  // startDownload 内部会处理 paused -> downloading 的状态转换
  if (file.encrypted) {
    downloadDecrypted(file)
  } else {
    download(file)
  }
}

/** 下载进度百分比 */
function downloadPct(path: string): number {
  return getDownloadProgress(path)
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

function verifyLabel(result: string) {
  switch (result) {
    case 'match': return '✓ 一致'
    case 'mismatch': return '✗ 不一致'
    case 'computing': return '计算中...'
    default: return ''
  }
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
  flex-direction: column;
  gap: 12px;
  padding: 14px 16px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  transition: background 0.2s, border-color 0.2s;
}

.file-item:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-strong);
}

.file-main {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  width: 100%;
}

.file-info {
  min-width: 0;
  flex: 1;
}

.file-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
  flex-shrink: 0;
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
  flex-wrap: wrap;
  justify-content: flex-end;
  width: auto;
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

.preview-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s, color 0.2s;
  white-space: nowrap;
}

.preview-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
  background: var(--bg-primary-soft);
}

.preview-btn svg {
  width: 16px;
  height: 16px;
}

.download-progress-wrapper {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 200px;
}

.download-progress-track {
  flex: 1;
  height: 8px;
  background: var(--border-color);
  border-radius: 4px;
  overflow: hidden;
}

.download-progress-bar {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, var(--primary), var(--primary-hover));
  transition: width 0.3s ease;
}

.download-progress-bar.paused {
  background: linear-gradient(90deg, #f59e0b, #fbbf24);
}

.download-progress-bar.error {
  background: linear-gradient(90deg, #ef4444, #f87171);
}

.download-progress-bar.done {
  background: linear-gradient(90deg, #22c55e, #4ade80);
}

.download-progress-info {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
}

.download-progress-text {
  color: var(--text-secondary);
  min-width: 36px;
  text-align: right;
}

.download-progress-speed {
  color: var(--primary);
  font-weight: 600;
  min-width: 70px;
}

.download-progress-eta {
  color: var(--text-tertiary);
}

.pause-btn,
.resume-btn {
  padding: 6px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
  white-space: nowrap;
}

.pause-btn:hover {
  border-color: #f59e0b;
  color: #b45309;
}

.resume-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}

.verify-toggle {
  width: auto;
}

.verify-toggle-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: var(--bg-soft);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s, color 0.2s;
}

.verify-toggle-btn:hover {
  background: var(--bg-card);
  border-color: var(--primary);
  color: var(--primary);
}

.verify-toggle-btn svg {
  width: 14px;
  height: 14px;
  transition: transform 0.2s ease;
}

.verify-toggle-btn.expanded svg {
  transform: rotate(180deg);
}

.verify-area {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  padding: 12px;
  background: var(--bg-soft);
  border: 1px solid var(--border-color);
  border-radius: 10px;
}

.verify-select {
  padding: 8px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  font-size: 13px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
}

.verify-input {
  flex: 1;
  min-width: 120px;
  padding: 8px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  font-size: 13px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
}

.verify-input:focus,
.verify-select:focus {
  border-color: var(--primary);
}

.verify-btn {
  padding: 8px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.verify-btn:hover:not(:disabled) {
  border-color: var(--primary);
  color: var(--primary);
}

.verify-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.verify-result {
  font-size: 13px;
  font-weight: 600;
}

.verify-result.match {
  color: var(--success-text);
}

.verify-result.mismatch {
  color: var(--danger-text);
}

.verify-result.computing {
  color: var(--text-tertiary);
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

  .verify-area {
    gap: 8px;
  }

  .verify-select,
  .verify-input,
  .verify-btn {
    width: 100%;
  }
}
</style>
