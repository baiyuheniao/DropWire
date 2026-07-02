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
    >
      <input ref="fileInput" type="file" multiple hidden @change="onFileChange" />
      <div class="drop-content">
        <div class="drop-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
        </div>
        <p class="drop-title">拖拽文件到此处</p>
        <p class="drop-sub">或点击选择 · 支持多文件 · 2 MB 分片 · 并发 3</p>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="tasks.size === 0" class="empty-state">
      <span>等待文件上传...</span>
    </div>

    <!-- Task list -->
    <TransitionGroup v-else name="task" tag="div" class="task-list">
      <div v-for="[id, task] in tasks" :key="id" class="task-card" :class="task.status">
        <div class="task-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
            <polyline points="13 2 13 9 20 9" />
          </svg>
        </div>

        <div class="task-body">
          <div class="task-header">
            <span class="filename" :title="task.filename">{{ task.filename }}</span>
            <span class="badge" :class="task.status">{{ label(task.status) }}</span>
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

          <p v-if="task.error" class="error-msg">{{ task.error }}</p>
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useUpload, type UploadTask, type TaskStatus } from '../composables/useUpload'

const fileInput = ref<HTMLInputElement>()
const isDragging = ref(false)
const { tasks, uploadFile } = useUpload()

function onDrop(e: DragEvent) {
  isDragging.value = false
  Array.from(e.dataTransfer?.files ?? []).forEach(uploadFile)
}

function onFileChange(e: Event) {
  Array.from((e.target as HTMLInputElement).files ?? []).forEach(uploadFile)
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
  border: 2px dashed #d1d5db;
  border-radius: 16px;
  padding: 56px 40px;
  text-align: center;
  cursor: pointer;
  background: #fff;
  transition: all 0.2s ease;
}

.drop-zone:hover,
.drop-zone.drag-over {
  border-color: #3b82f6;
  background: #eff6ff;
  box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.08);
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
  background: #eff6ff;
  display: grid;
  place-items: center;
  color: #3b82f6;
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
  color: #1f2937;
  margin-bottom: 4px;
}

.drop-zone.has-tasks .drop-title {
  font-size: 14px;
}

.drop-sub {
  font-size: 13px;
  color: #6b7280;
}

/* Empty state */
.empty-state {
  text-align: center;
  padding: 24px;
  color: #9ca3af;
  font-size: 14px;
  border-radius: 12px;
  border: 1px dashed #e5e7eb;
  background: #fff;
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
  background: #fff;
  border: 1px solid #e5e7eb;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
  transition: all 0.2s ease;
}

.task-card:hover {
  border-color: #d1d5db;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
}

.task-icon {
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: #eff6ff;
  display: grid;
  place-items: center;
  color: #3b82f6;
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

.filename {
  font-weight: 500;
  font-size: 14px;
  color: #1f2937;
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
  background: #f3f4f6;
  color: #6b7280;
  border: 1px solid #e5e7eb;
}

.badge.uploading { color: #2563eb; background: #dbeafe; border-color: #bfdbfe; }
.badge.merging   { color: #b45309; background: #fef3c7; border-color: #fde68a; }
.badge.done      { color: #15803d; background: #dcfce7; border-color: #bbf7d0; }
.badge.error     { color: #b91c1c; background: #fee2e2; border-color: #fecaca; }

/* Progress bar */
.progress-track {
  height: 7px;
  background: #e5e7eb;
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 8px;
}

.progress-bar {
  height: 100%;
  border-radius: 4px;
  background: linear-gradient(90deg, #3b82f6, #60a5fa);
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
  color: #6b7280;
}

.error-msg {
  font-size: 12px;
  color: #dc2626;
  margin-top: 6px;
  line-height: 1.4;
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
</style>
