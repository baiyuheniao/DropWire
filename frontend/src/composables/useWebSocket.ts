import { ref, onUnmounted } from 'vue'

export interface UploadProgress {
  upload_id: string
  filename: string
  total_chunks: number
  received_chunks: number
  status: 'Uploading' | 'Merging' | 'Completed' | { type: 'Failed'; reason: string }
}

// Singleton so App.vue and FileUpload.vue share the same connection.
let ws: WebSocket | null = null
const progress = ref<Map<string, UploadProgress>>(new Map())
const connected = ref(false)
let refCount = 0

function connect(url: string) {
  ws = new WebSocket(url)

  ws.onopen = () => { connected.value = true }

  ws.onmessage = (e) => {
    try {
      const data: UploadProgress = JSON.parse(e.data)
      progress.value = new Map(progress.value).set(data.upload_id, data)
    } catch { /* ignore malformed frames */ }
  }

  ws.onclose = () => {
    connected.value = false
    setTimeout(() => connect(url), 3000)
  }

  ws.onerror = () => ws?.close()
}

export function useWebSocket(url: string) {
  if (refCount === 0) connect(url)
  refCount++

  onUnmounted(() => {
    refCount--
    if (refCount === 0) {
      ws?.close()
      ws = null
    }
  })

  return { progress, connected }
}
