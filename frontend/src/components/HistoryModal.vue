<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-header">
        <h3>历史记录</h3>
        <button class="close-btn" @click="emit('close')">&times;</button>
      </div>

      <div class="tabs">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="tab"
          :class="{ active: activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </div>

      <div class="modal-body">
        <div v-if="filteredHistory.length === 0" class="empty-state">
          暂无{{ activeTab === 'send' ? '发送' : '接收' }}记录
        </div>

        <ul v-else class="history-list">
          <li v-for="item in filteredHistory" :key="item.id" class="history-item">
            <div class="history-main">
              <div class="history-title">
                <span class="filename" :title="item.filename">{{ item.filename }}</span>
                <span class="size">{{ formatSize(item.size) }}</span>
              </div>
              <div class="history-tags">
                <span v-if="item.encrypted" class="tag encrypted">🔒 加密</span>
                <span v-if="item.sender" class="tag">来自: {{ item.sender }}</span>
                <span v-if="item.receiver" class="tag">发给: {{ item.receiver }}</span>
                <span v-if="item.remark" class="tag remark">备注: {{ item.remark }}</span>
              </div>
              <div class="history-meta">
                <span>{{ formatTime(item.timestamp) }}</span>
                <span v-if="item.expiresAt" :class="{ expired: isExpired(item.expiresAt) }">
                  {{ isExpired(item.expiresAt) ? '已过期' : `有效期至 ${formatExpireTime(item.expiresAt)}` }}
                </span>
              </div>
            </div>

            <div class="history-actions">
              <template v-if="activeTab === 'send' && item.url">
                <div class="qr-thumb">
                  <QRCodeVue :value="item.url" :size="72" level="M" />
                </div>
                <button class="icon-btn" title="复制链接" @click="copyUrl(item.url, item.id)">
                  {{ copiedId === item.id ? '已复制' : '复制' }}
                </button>
              </template>
              <button class="icon-btn danger" title="删除" @click="removeHistory(item.id)">
                删除
              </button>
            </div>
          </li>
        </ul>
      </div>

      <div class="modal-footer">
        <button class="clear-btn" @click="clearActive">
          清空{{ activeTab === 'send' ? '发送' : '接收' }}记录
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import QRCodeVue from 'qrcode.vue'
import { loadHistory, saveHistory, type HistoryEntry, type HistoryType } from '../composables/useHistory'

const props = defineProps<{
  initialTab?: HistoryType
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const activeTab = ref<HistoryType>(props.initialTab || 'send')
const history = ref<HistoryEntry[]>(loadHistory())
const copiedId = ref<string>('')

const tabs = [
  { key: 'send' as HistoryType, label: '发送' },
  { key: 'receive' as HistoryType, label: '接收' },
]

const filteredHistory = computed(() =>
  history.value.filter((h) => h.type === activeTab.value),
)

function refresh() {
  history.value = loadHistory()
}

function removeHistory(id: string) {
  history.value = history.value.filter((h) => h.id !== id)
  saveHistory(history.value)
}

function clearActive() {
  history.value = history.value.filter((h) => h.type !== activeTab.value)
  saveHistory(history.value)
}

async function copyUrl(url?: string, id?: string) {
  if (!url || !id) return
  try {
    await navigator.clipboard.writeText(url)
  } catch {
    const input = document.createElement('input')
    input.value = url
    document.body.appendChild(input)
    input.select()
    document.execCommand('copy')
    document.body.removeChild(input)
  }
  copiedId.value = id
  setTimeout(() => {
    if (copiedId.value === id) copiedId.value = ''
  }, 2000)
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

function formatTime(timestamp: number): string {
  const date = new Date(timestamp)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
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

function isExpired(timestamp: number): boolean {
  return timestamp * 1000 <= Date.now()
}

refresh()
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
  width: 600px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
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

.tabs {
  display: flex;
  border-bottom: 1px solid var(--border-color);
}

.tab {
  flex: 1;
  padding: 12px;
  background: none;
  border: none;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}

.tab:hover {
  background: var(--bg-soft);
}

.tab.active {
  color: var(--primary);
  font-weight: 600;
  box-shadow: inset 0 -2px 0 var(--primary);
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}

.empty-state {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-tertiary);
  font-size: 14px;
}

.history-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.history-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--bg-soft);
}

.history-main {
  flex: 1;
  min-width: 0;
}

.history-title {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}

.filename {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.size {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-tertiary);
}

.history-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 6px;
}

.tag {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 500;
  background: var(--bg-card);
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}

.tag.encrypted {
  background: var(--warning-bg);
  color: var(--warning-text);
}

.tag.remark {
  background: #ecfdf5;
  color: #047857;
}

.history-meta {
  font-size: 12px;
  color: var(--text-tertiary);
  display: flex;
  gap: 12px;
}

.history-meta .expired {
  color: var(--danger-text);
}

.history-actions {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
  flex-shrink: 0;
}

.qr-thumb {
  width: 72px;
  height: 72px;
  border-radius: 8px;
  overflow: hidden;
  background: #fff;
  border: 1px solid var(--border-color);
}

.icon-btn {
  padding: 5px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.2s;
}

.icon-btn:hover {
  background: var(--border-color);
}

.icon-btn.danger {
  border-color: var(--danger-bg);
  color: var(--danger-text);
  background: var(--danger-bg);
}

.icon-btn.danger:hover {
  opacity: 0.9;
}

.modal-footer {
  padding: 12px 20px;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: flex-end;
}

.clear-btn {
  padding: 8px 14px;
  border: 1px solid var(--danger-bg);
  border-radius: 8px;
  background: var(--danger-bg);
  color: var(--danger-text);
  font-size: 13px;
  cursor: pointer;
  transition: opacity 0.2s;
}

.clear-btn:hover {
  opacity: 0.9;
}
</style>
