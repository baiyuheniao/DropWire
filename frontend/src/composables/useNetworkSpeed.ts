import { computed, reactive } from 'vue'

interface SpeedWindow {
  bytes: number
  time: number
}

const WINDOW_MS = 2000

const windows = reactive({
  upload: [] as SpeedWindow[],
  download: [] as SpeedWindow[],
})

function prune(list: SpeedWindow[]) {
  const now = Date.now()
  while (list.length && now - list[0].time > WINDOW_MS) {
    list.shift()
  }
}

function record(direction: 'upload' | 'download', bytes: number) {
  const list = direction === 'upload' ? windows.upload : windows.download
  list.push({ bytes, time: Date.now() })
  prune(list)
}

function currentSpeed(direction: 'upload' | 'download'): number {
  const list = direction === 'upload' ? windows.upload : windows.download
  prune(list)
  if (list.length < 2) return 0
  const totalBytes = list.reduce((sum, w) => sum + w.bytes, 0)
  const duration = list[list.length - 1].time - list[0].time
  if (duration <= 0) return 0
  return totalBytes / (duration / 1000)
}

export const uploadSpeed = computed(() => currentSpeed('upload'))
export const downloadSpeed = computed(() => currentSpeed('download'))
export const hasActiveTransfer = computed(
  () => uploadSpeed.value > 0 || downloadSpeed.value > 0,
)

export function recordUpload(bytes: number) {
  record('upload', bytes)
}

export function recordDownload(bytes: number) {
  record('download', bytes)
}

export function formatSpeed(bps: number): string {
  if (bps <= 0) return '0 B/s'
  if (bps < 1024) return `${bps.toFixed(0)} B/s`
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`
  return `${(bps / 1024 / 1024).toFixed(2)} MB/s`
}
