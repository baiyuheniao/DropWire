import { ref, onUnmounted } from 'vue'

export interface UploadProgress {
  upload_id: string
  filename: string
  total_chunks: number
  received_chunks: number
  status: 'Uploading' | 'Merging' | 'Completed' | { type: 'Failed'; reason: string }
}

export interface ReceivedEvent {
  filename: string
  received_at?: number
  received_by?: string
}

// Singleton so App.vue and FileUpload.vue share the same connection.
let ws: WebSocket | null = null
const progress = ref<Map<string, UploadProgress>>(new Map())
const received = ref<Map<string, ReceivedEvent>>(new Map())
const connected = ref(false)
let refCount = 0

function connect(url: string) {
  ws = new WebSocket(url)

  ws.onopen = () => { connected.value = true }

  ws.onmessage = (e) => {
    try {
      const payload = JSON.parse(e.data)
      if (payload.event === 'progress' && payload.data) {
        const data: UploadProgress = payload.data
        progress.value = new Map(progress.value).set(data.upload_id, data)
      } else if (payload.event === 'received' && payload.data) {
        const data: ReceivedEvent = payload.data
        received.value = new Map(received.value).set(data.filename, data)
      }
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

  return { progress, connected, received }
}
