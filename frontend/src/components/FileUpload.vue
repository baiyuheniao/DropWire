<template>
  <div class="upload-container">
    <!-- Drop zone -->
    <div
      class="drop-zone"
      :class="{ 'drag-over': isDragging }"
      @dragover.prevent="isDragging = true"
      @dragleave.prevent="isDragging = false"
      @drop.prevent="onDrop"
      @click="fileInput?.click()"
    >
      <input ref="fileInput" type="file" multiple hidden @change="onFileChange" />
      <div class="drop-icon">&#128193;</div>
      <p class="drop-hint">拖拽文件到此处，或点击选择</p>
      <p class="drop-sub">支持多文件 · 分片大小 2 MB · 并发 3</p>
    </div>

    <!-- Task list -->
    <TransitionGroup name="task" tag="div" class="task-list">
      <div v-for="[id, task] in tasks" :key="id" class="task-card">
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

        <div class="task-footer">
          <span>{{ task.uploadedChunks }} / {{ task.totalChunks }} 分片</span>
          <span>{{ pct(task) }}%</span>
        </div>

        <p v-if="task.error" class="error-msg">{{ task.error }}</p>
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
  pending:   '等待中',
  uploading: '上传中',
  merging:   '合并中',
  done:      '完成',
  error:     '失败',
}
function label(s: TaskStatus) { return STATUS_LABELS[s] }
</script>

<style scoped>
.upload-container { display: flex; flex-direction: column; gap: 20px; }

/* Drop zone */
.drop-zone {
  border: 2px dashed #c0c8d8;
  border-radius: 14px;
  padding: 56px 40px;
  text-align: center;
  cursor: pointer;
  background: #fff;
  transition: border-color 0.2s, background 0.2s;
  user-select: none;
}
.drop-zone:hover,
.drop-zone.drag-over {
  border-color: #4a9eff;
  background: #f0f7ff;
}
.drop-icon { font-size: 52px; margin-bottom: 14px; }
.drop-hint { font-size: 16px; font-weight: 500; color: #444; margin-bottom: 6px; }
.drop-sub  { font-size: 12px; color: #999; }

/* Task cards */
.task-list { display: flex; flex-direction: column; gap: 12px; }

.task-card {
  background: #fff;
  border-radius: 10px;
  padding: 16px 18px;
  box-shadow: 0 1px 4px rgba(0,0,0,.08);
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
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}
.badge {
  font-size: 11px;
  padding: 2px 9px;
  border-radius: 10px;
  white-space: nowrap;
  font-weight: 500;
  background: #e9ecef;
  color: #555;
}
.badge.uploading { background: #cce5ff; color: #004085; }
.badge.merging   { background: #fff3cd; color: #856404; }
.badge.done      { background: #d4edda; color: #155724; }
.badge.error     { background: #f8d7da; color: #721c24; }

/* Progress bar */
.progress-track {
  height: 6px;
  background: #eee;
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: 8px;
}
.progress-bar {
  height: 100%;
  background: #4a9eff;
  border-radius: 3px;
  transition: width 0.25s ease;
}
.progress-bar.done      { background: #28a745; }
.progress-bar.error     { background: #dc3545; }
.progress-bar.merging   { background: #ffc107; }

.task-footer {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: #999;
}
.error-msg {
  font-size: 12px;
  color: #dc3545;
  margin-top: 6px;
}

/* Transition */
.task-enter-active { transition: all 0.25s ease; }
.task-enter-from   { opacity: 0; transform: translateY(-8px); }
</style>
