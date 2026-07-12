<template>
  <Teleport to="body">
    <div class="preview-overlay" @click.self="$emit('close')">
      <div class="preview-modal">
        <div class="preview-header">
          <div class="preview-title">
            <span class="preview-icon">{{ fileTypeIcon }}</span>
            <span class="preview-filename">{{ filename }}</span>
          </div>
          <button class="close-btn" @click="$emit('close')">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div class="preview-body">
          <div v-if="loading" class="preview-loading">
            <svg class="spinner" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-dasharray="60" stroke-dashoffset="30" />
            </svg>
            <p>加载中...</p>
          </div>

          <div v-else-if="error" class="preview-error">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <p>{{ error }}</p>
          </div>

          <!-- 图片预览 -->
          <div v-else-if="isImage" class="preview-image-wrapper">
            <img :src="previewUrl" :alt="filename" class="preview-image" @error="handleImageError" />
          </div>

          <!-- 视频预览 -->
          <div v-else-if="isVideo" class="preview-video-wrapper">
            <video :src="previewUrl" controls class="preview-video" @error="handleMediaError">
              您的浏览器不支持视频播放
            </video>
          </div>

          <!-- 音频预览 -->
          <div v-else-if="isAudio" class="preview-audio-wrapper">
            <audio :src="previewUrl" controls class="preview-audio" @error="handleMediaError">
              您的浏览器不支持音频播放
            </audio>
          </div>

          <!-- 文本预览 -->
          <div v-else-if="isText" class="preview-text-wrapper">
            <div class="text-actions">
              <button class="btn-secondary" @click="copyText">复制</button>
              <select v-model="textWrap" class="wrap-select">
                <option :value="false">不换行</option>
                <option :value="true">自动换行</option>
              </select>
            </div>
            <pre :class="{ wrap: textWrap }" class="preview-text">{{ textContent }}</pre>
          </div>

          <!-- PDF 预览 -->
          <div v-else-if="isPdf" class="preview-pdf-wrapper">
            <object :data="previewUrl" type="application/pdf" class="preview-pdf">
              <p>无法直接预览 PDF，请下载查看</p>
              <a :href="previewUrl" download class="download-link">下载 PDF</a>
            </object>
          </div>

          <!-- 不支持预览 -->
          <div v-else class="preview-unsupported">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
              <polyline points="13 2 13 9 20 9" />
              <path d="M9 15l2 2 4-4" />
            </svg>
            <p>该文件类型不支持预览</p>
            <p class="hint">文件大小：{{ formatSize }}</p>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, watch } from 'vue'

interface Props {
  file?: File | null
  url?: string
  filename?: string
  size?: number
}

const props = defineProps<Props>()
defineEmits<{
  close: []
}>()

const loading = ref(true)
const error = ref('')
const textContent = ref('')
const textWrap = ref(false)
const objectUrl = ref('')

const fileType = computed(() => {
  const name = props.file?.name || props.filename || ''
  const ext = name.split('.').pop()?.toLowerCase() || ''
  return ext
})

const fileTypeIcon = computed(() => {
  const icons: Record<string, string> = {
    png: '🖼️',
    jpg: '🖼️',
    jpeg: '🖼️',
    gif: '🖼️',
    webp: '🖼️',
    svg: '📐',
    bmp: '🖼️',
    mp4: '🎬',
    webm: '🎬',
    ogg: '🎬',
    mp3: '🎵',
    wav: '🎵',
    txt: '📝',
    md: '📄',
    json: '📋',
    xml: '📋',
    pdf: '📕',
    html: '🌐',
    css: '🎨',
    js: '📜',
    ts: '📜',
  }
  return icons[fileType.value] || '📦'
})

const isImage = computed(() => ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp'].includes(fileType.value))
const isVideo = computed(() => ['mp4', 'webm', 'ogg'].includes(fileType.value))
const isAudio = computed(() => ['mp3', 'wav', 'ogg', 'webm'].includes(fileType.value))
const isText = computed(() => ['txt', 'md', 'json', 'xml', 'html', 'css', 'js', 'ts'].includes(fileType.value))
const isPdf = computed(() => fileType.value === 'pdf')

const previewUrl = computed(() => {
  if (props.url) return props.url
  if (objectUrl.value) return objectUrl.value
  return ''
})

const formatSize = computed(() => {
  const s = props.file?.size || props.size || 0
  if (s < 1024) return `${s} B`
  if (s < 1024 * 1024) return `${(s / 1024).toFixed(1)} KB`
  return `${(s / 1024 / 1024).toFixed(2)} MB`
})

async function loadFile() {
  loading.value = true
  error.value = ''
  textContent.value = ''

  if (props.file) {
    if (isImage.value || isVideo.value || isAudio.value || isPdf.value) {
      objectUrl.value = URL.createObjectURL(props.file)
    } else if (isText.value) {
      try {
        textContent.value = await props.file.text()
      } catch (err) {
        error.value = '读取文本失败'
      }
    }
    loading.value = false
    return
  }

  if (props.url) {
    if (isText.value) {
      try {
        const res = await fetch(props.url)
        textContent.value = await res.text()
      } catch (err) {
        error.value = '加载文本失败'
      }
    }
    loading.value = false
  }
}

function handleImageError() {
  error.value = '图片加载失败'
}

function handleMediaError() {
  error.value = '媒体加载失败'
}

async function copyText() {
  try {
    await navigator.clipboard.writeText(textContent.value)
  } catch {
    // fallback
    const textarea = document.createElement('textarea')
    textarea.value = textContent.value
    document.body.appendChild(textarea)
    textarea.select()
    document.execCommand('copy')
    document.body.removeChild(textarea)
  }
}

watch(() => [props.file, props.url], loadFile, { immediate: true })

onMounted(() => {
  document.body.style.overflow = 'hidden'
})
</script>

<style scoped>
.preview-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
  backdrop-filter: blur(4px);
}

.preview-modal {
  background: var(--bg-card);
  border-radius: 16px;
  max-width: 90vw;
  max-height: 90vh;
  width: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card);
}

.preview-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.preview-icon {
  font-size: 20px;
}

.preview-filename {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 300px;
}

.close-btn {
  padding: 6px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.2s, color 0.2s;
}

.close-btn:hover {
  background: var(--bg-soft);
  color: var(--text-primary);
}

.close-btn svg {
  width: 20px;
  height: 20px;
}

.preview-body {
  flex: 1;
  overflow: auto;
  padding: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.preview-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--text-tertiary);
}

.spinner {
  width: 40px;
  height: 40px;
  color: var(--primary);
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.preview-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--danger-text);
}

.preview-error svg {
  width: 48px;
  height: 48px;
}

.preview-image-wrapper {
  max-width: 100%;
  max-height: calc(90vh - 120px);
}

.preview-image {
  max-width: 100%;
  max-height: calc(90vh - 120px);
  border-radius: 8px;
  object-fit: contain;
  background: var(--bg-soft);
}

.preview-video-wrapper,
.preview-audio-wrapper {
  width: 100%;
  max-width: 800px;
}

.preview-video {
  width: 100%;
  max-height: calc(90vh - 120px);
  border-radius: 8px;
}

.preview-audio {
  width: 100%;
}

.preview-text-wrapper {
  width: 100%;
  max-width: 900px;
  max-height: calc(90vh - 150px);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.text-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  justify-content: flex-end;
}

.btn-secondary {
  padding: 6px 14px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.btn-secondary:hover {
  border-color: var(--primary);
  color: var(--primary);
}

.wrap-select {
  padding: 6px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 13px;
}

.preview-text {
  flex: 1;
  overflow: auto;
  padding: 16px;
  border-radius: 8px;
  background: var(--bg-soft);
  color: var(--text-primary);
  font-family: 'SF Mono', 'Monaco', 'Consolas', monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre;
}

.preview-text.wrap {
  white-space: pre-wrap;
  word-break: break-all;
}

.preview-pdf-wrapper {
  width: 100%;
  max-width: 900px;
  height: calc(90vh - 120px);
}

.preview-pdf {
  width: 100%;
  height: 100%;
  border-radius: 8px;
}

.preview-unsupported {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--text-tertiary);
}

.preview-unsupported svg {
  width: 64px;
  height: 64px;
}

.preview-unsupported p {
  font-size: 14px;
}

.preview-unsupported .hint {
  font-size: 12px;
  opacity: 0.7;
}

.download-link {
  display: inline-block;
  margin-top: 12px;
  padding: 10px 20px;
  background: var(--primary);
  color: #fff;
  border-radius: 8px;
  text-decoration: none;
  font-weight: 500;
  transition: background 0.2s;
}

.download-link:hover {
  background: var(--primary-hover);
}

@media (max-width: 640px) {
  .preview-overlay {
    padding: 10px;
  }

  .preview-modal {
    border-radius: 12px;
  }

  .preview-header {
    padding: 12px 16px;
  }

  .preview-body {
    padding: 12px;
  }

  .preview-filename {
    max-width: 200px;
    font-size: 14px;
  }

  .preview-text {
    font-size: 12px;
    padding: 12px;
  }
}
</style>
