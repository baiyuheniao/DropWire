<template>
  <div class="upload-container">
    <!-- Drop zone -->
    <div
      class="drop-zone"
      :class="{ 'drag-over': isDragging, 'has-tasks': tasks.size > 0 }"
      @dragover.prevent="isDragging = true"
      @dragleave.prevent="isDragging = false"
      @drop.prevent="onDrop"
      @click="fileInput?.click()"
    >      <input ref="fileInput" type="file" multiple webkitdirectory hidden @change="onFileChange" />
      <div class="drop-content">
        <div class="drop-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
        </div>
        <p class="drop-title">拖拽文件到此处</p>
        <p class="drop-sub">或点击选择 · 支持多文件/文件夹 · 2 MB 分片 · 并发 3</p>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="tasks.size === 0" class="empty-state">
      <span>等待文件上传...</span>
    </div>

    <!-- Task list -->
    <TransitionGroup v-else name="task" tag="div" class="task-list">
      <div
        v-for="[id, task] in tasks"
        :key="id"
        class="task-card"
        :class="[task.status, { clickable: task.status === 'done' }]"
        @click="task.status === 'done' && openDetail(task)"
      >
        <div class="task-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
            <polyline points="13 2 13 9 20 9" />
          </svg>
        </div>

        <div class="task-body">
          <div class="task-header">
            <span class="filename" :title="task.filename">{{ task.filename }}</span>
            <div class="badges">
              <span v-if="task.receivedAt" class="badge received">对方已接收</span>
              <span class="badge" :class="task.status">{{ label(task.status) }}</span>
            </div>
          </div>

          <div class="progress-track">
            <div
              class="progress-bar"
              :class="task.status"
              :style="{ width: pct(task) + '%' }"
            />
          </div>

          <div class="task-meta">
            <span>{{ task.uploadedChunks }} / {{ task.totalChunks }} 分片</span>
            <span>{{ pct(task) }}%</span>
          </div>

          <p v-if="task.status === 'done'" class="detail-hint">点击打开详情页</p>
          <p v-if="task.error" class="error-msg">{{ task.error }}</p>
        </div>
      </div>
    </TransitionGroup>

    <FileDetailModal
      v-if="selectedTask"
      :task="selectedTask"
      :sender="props.options?.sender"
      :receiver="props.options?.receiver"
      :remark="props.options?.remark"
      :encrypted="!!props.options?.password"
      @close="selectedTask = null"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useUpload, type UploadOptions, type UploadTask, type TaskStatus } from '../composables/useUpload'
import { addHistory } from '../composables/useHistory'
import { useWebSocket } from '../composables/useWebSocket'
import FileDetailModal from './FileDetailModal.vue'

const props = defineProps<{
  options?: UploadOptions
}>()

const fileInput = ref<HTMLInputElement>()
const isDragging = ref(false)
const selectedTask = ref<UploadTask | null>(null)
const { tasks, uploadFile } = useUpload()
const { received } = useWebSocket('/ws')

const recordedIds = ref<Set<string>>(new Set())

watch(
  () => tasks.value,
  (next) => {
    next.forEach((task) => {
      if (task.status === 'done' && !recordedIds.value.has(task.uploadId)) {
        recordedIds.value.add(task.uploadId)
        addHistory({
          type: 'send',
          filename: task.filename,
          size: task.fileSize || 0,
          sender: props.options?.sender,
          receiver: props.options?.receiver,
          remark: props.options?.remark,
          encrypted: !!props.options?.password,
          expiresAt: task.expiresAt,
          url: task.downloadUrl,
        })
      }
    })
  },
  { deep: true, immediate: true },
)

watch(
  () => received.value,
  (next) => {
    next.forEach((evt) => {
      for (const [id, task] of tasks.value) {
        if (task.filename === evt.filename && task.status === 'done' && !task.receivedAt) {
          tasks.value = new Map(tasks.value).set(id, {
            ...task,
            receivedAt: evt.received_at,
            receivedBy: evt.received_by,
          })
        }
      }
    })
  },
  { deep: true },
)

function openDetail(task: UploadTask) {
  selectedTask.value = task
}

async function collectFiles(items: DataTransferItemList): Promise<File[]> {
  const files: File[] = []

  const readEntry = (entry: any): Promise<void> => {
    return new Promise((resolve) => {
      if (entry.isFile) {
        entry.file((file: File) => {
          const relativePath = entry.fullPath.replace(/^\//, '')
          if (!(file as any).webkitRelativePath) {
            try {
              Object.defineProperty(file, 'webkitRelativePath', {
                value: relativePath,
                configurable: true,
              })
            } catch { /* ignore */ }
          }
          files.push(file)
          resolve()
        })
      } else if (entry.isDirectory) {
        const reader = entry.createReader()
        reader.readEntries(async (entries: any[]) => {
          await Promise.all(entries.map((child) => readEntry(child)))
          resolve()
        })
      } else {
        resolve()
      }
    })
  }

  const entries = Array.from(items)
    .map((item) => item.webkitGetAsEntry())
    .filter(Boolean)
  await Promise.all(entries.map((entry) => readEntry(entry)))
  return files
}

async function onDrop(e: DragEvent) {
  isDragging.value = false
  if (e.dataTransfer?.items && e.dataTransfer.items.length > 0 && typeof e.dataTransfer.items[0].webkitGetAsEntry === 'function') {
    const files = await collectFiles(e.dataTransfer.items)
    files.forEach((file) => uploadFile(file, props.options))
  } else {
    Array.from(e.dataTransfer?.files ?? []).forEach((file) => uploadFile(file, props.options))
  }
}

function onFileChange(e: Event) {
  Array.from((e.target as HTMLInputElement).files ?? []).forEach((file) => uploadFile(file, props.options))
  ;(e.target as HTMLInputElement).value = ''
}

function pct(t: UploadTask) {
  if (t.totalChunks === 0) return 0
  if (t.status === 'done') return 100
  return Math.round((t.uploadedChunks / t.totalChunks) * 100)
}

const STATUS_LABELS: Record<TaskStatus, string> = {
  pending: '等待中',
  uploading: '上传中',
  merging: '合并中',
  done: '已完成',
  error: '失败',
}
function label(s: TaskStatus) { return STATUS_LABELS[s] }
</script>

<style scoped>
.upload-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* Drop zone */
.drop-zone {
  position: relative;
  border: 2px dashed var(--border-strong);
  border-radius: 16px;
  padding: 56px 40px;
  text-align: center;
  cursor: pointer;
  background: var(--bg-card);
  transition: all 0.2s ease;
}

.drop-zone:hover,
.drop-zone.drag-over {
  border-color: var(--primary);
  background: var(--bg-primary-soft);
  box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.12);
}

.drop-zone.has-tasks {
  padding: 32px 40px;
}

.drop-content {
  position: relative;
  z-index: 1;
}

.drop-icon {
  width: 56px;
  height: 56px;
  margin: 0 auto 16px;
  border-radius: 16px;
  background: var(--bg-primary-soft);
  display: grid;
  place-items: center;
  color: var(--primary);
}

.drop-zone.has-tasks .drop-icon {
  width: 40px;
  height: 40px;
  margin-bottom: 10px;
}

.drop-icon svg {
  width: 28px;
  height: 28px;
}

.drop-zone.has-tasks .drop-icon svg {
  width: 20px;
  height: 20px;
}

.drop-title {
  font-size: 17px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.drop-zone.has-tasks .drop-title {
  font-size: 14px;
}

.drop-sub {
  font-size: 13px;
  color: var(--text-tertiary);
}

/* Empty state */
.empty-state {
  text-align: center;
  padding: 24px;
  color: var(--text-tertiary);
  font-size: 14px;
  border-radius: 12px;
  border: 1px dashed var(--border-color);
  background: var(--bg-card);
}

/* Task cards */
.task-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.task-card {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 16px 18px;
  border-radius: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  box-shadow: 0 1px 2px var(--shadow);
  transition: all 0.2s ease;
}

.task-card:hover {
  border-color: var(--border-strong);
  box-shadow: 0 2px 6px var(--shadow);
}

.task-card.clickable {
  cursor: pointer;
}

.task-card.clickable:hover {
  border-color: var(--primary);
}

.task-icon {
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: var(--bg-primary-soft);
  display: grid;
  place-items: center;
  color: var(--primary);
}

.task-icon svg {
  width: 20px;
  height: 20px;
}

.task-body {
  flex: 1;
  min-width: 0;
}

.task-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  gap: 12px;
}

.badges {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.filename {
  font-weight: 500;
  font-size: 14px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.badge {
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 10px;
  white-space: nowrap;
  font-weight: 600;
  background: var(--bg-soft);
  color: var(--text-tertiary);
  border: 1px solid var(--border-color);
}

.badge.uploading { color: var(--primary-text); background: var(--bg-primary-soft); border-color: rgba(59, 130, 246, 0.25); }
.badge.merging   { color: var(--warning-text); background: var(--warning-bg); border-color: rgba(245, 158, 11, 0.25); }
.badge.done      { color: var(--success-text); background: rgba(52, 211, 153, 0.12); border-color: rgba(34, 197, 94, 0.25); }
.badge.error     { color: var(--danger-text); background: var(--danger-bg); border-color: rgba(239, 68, 68, 0.25); }
.badge.received  { color: var(--success-text); background: rgba(52, 211, 153, 0.12); border-color: rgba(34, 197, 94, 0.25); }

/* Progress bar */
.progress-track {
  height: 7px;
  background: var(--border-color);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 8px;
}

.progress-bar {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, var(--primary), #60a5fa);
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  overflow: hidden;
}

.progress-bar::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.35), transparent);
  transform: translateX(-100%);
  animation: shimmer 1.8s infinite;
}

.progress-bar.done {
  background: linear-gradient(90deg, #22c55e, #4ade80);
}

.progress-bar.done::after,
.progress-bar.error::after {
  animation: none;
}

.progress-bar.error {
  background: linear-gradient(90deg, #ef4444, #f87171);
}

.progress-bar.merging {
  background: linear-gradient(90deg, #f59e0b, #fbbf24);
}

@keyframes shimmer {
  100% { transform: translateX(100%); }
}

.task-meta {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-tertiary);
}

.error-msg {
  font-size: 12px;
  color: var(--danger-text);
  margin-top: 6px;
  line-height: 1.4;
}

.detail-hint {
  font-size: 12px;
  color: var(--primary-text);
  margin-top: 6px;
}

/* Transition */
.task-enter-active,
.task-leave-active {
  transition: all 0.25s ease;
}

.task-enter-from {
  opacity: 0;
  transform: translateY(-10px);
}

.task-leave-to {
  opacity: 0;
  transform: translateX(16px);
}

@media (max-width: 640px) {
  .drop-zone {
    padding: 36px 20px;
    border-radius: 12px;
  }

  .drop-zone.has-tasks {
    padding: 20px;
  }

  .drop-icon {
    width: 44px;
    height: 44px;
  }

  .drop-icon svg {
    width: 22px;
    height: 22px;
  }

  .drop-title {
    font-size: 15px;
  }

  .drop-sub {
    font-size: 12px;
  }

  .task-card {
    padding: 12px;
    gap: 12px;
  }

  .task-icon {
    width: 34px;
    height: 34px;
  }

  .task-icon svg {
    width: 16px;
    height: 16px;
  }

  .filename {
    font-size: 13px;
  }
}
</style>
