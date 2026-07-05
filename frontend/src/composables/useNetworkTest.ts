import { reactive, ref } from 'vue'
import axios from 'axios'

export interface SpeedResult {
  bytes: number
  durationMs: number
  speedMbps: number
}

export interface DeviceInfo {
  id: string
  name: string
  avatar?: string
  ip: string
  port: number
  last_seen: number
  online: boolean
}

export interface NetworkStatus {
  self_id: string
  self_name: string
  self_ip: string
  port: number
  peer_count: number
  peers: DeviceInfo[]
  public_ip?: string
  has_public_internet: boolean
  summary: string
}

export const status = ref<NetworkStatus | null>(null)
export const statusLoading = ref(false)
export const internalResult = reactive<{
  upload: SpeedResult | null
  download: SpeedResult | null
  loading: boolean
}>({ upload: null, download: null, loading: false })

export const publicResult = reactive<{
  upload: SpeedResult | null
  download: SpeedResult | null
  loading: boolean
}>({ upload: null, download: null, loading: false })

export async function fetchNetworkStatus(): Promise<NetworkStatus | null> {
  statusLoading.value = true
  try {
    const res = await axios.get('/network/status')
    status.value = res.data
    return res.data
  } catch (err: any) {
    console.warn('获取网络状态失败', err)
    return null
  } finally {
    statusLoading.value = false
  }
}

function makeTestPayload(mb: number): Blob {
  const size = mb * 1024 * 1024
  const chunk = new Uint8Array(64 * 1024)
  // fill with pseudo-random bytes so payload is not overly compressible
  for (let i = 0; i < chunk.length; i++) chunk[i] = i % 256
  const parts: BlobPart[] = []
  const count = Math.floor(size / chunk.length)
  for (let i = 0; i < count; i++) parts.push(chunk.slice())
  const remainder = size % chunk.length
  if (remainder > 0) parts.push(chunk.slice(0, remainder))
  return new Blob(parts)
}

async function measureDownload(url: string): Promise<SpeedResult> {
  const start = performance.now()
  const res = await axios.get(url, { responseType: 'arraybuffer' })
  const durationMs = Math.max(1, performance.now() - start)
  const bytes = (res.data as ArrayBuffer).byteLength
  const speedMbps = (bytes * 8) / 1_000_000 / (durationMs / 1000)
  return { bytes, durationMs: Math.round(durationMs), speedMbps }
}

async function measureUpload(url: string, blob: Blob): Promise<SpeedResult> {
  const start = performance.now()
  const res = await axios.post(url, blob, {
    headers: { 'Content-Type': 'application/octet-stream' },
  })
  const durationMs = Math.max(1, performance.now() - start)
  const bytes = blob.size
  const speedMbps = (bytes * 8) / 1_000_000 / (durationMs / 1000)
  return { bytes, durationMs: Math.round(durationMs), speedMbps: res.data?.speed_mbps ?? speedMbps }
}

export async function runInternalSpeedTest(sizeMb = 10) {
  internalResult.loading = true
  internalResult.upload = null
  internalResult.download = null
  try {
    const payload = makeTestPayload(sizeMb)
    internalResult.upload = await measureUpload('/network/speed-test/upload', payload)
    internalResult.download = await measureDownload(`/network/speed-test/download?size_mb=${sizeMb}`)
  } finally {
    internalResult.loading = false
  }
}

export async function runPublicSpeedTest(sizeMb = 10) {
  publicResult.loading = true
  publicResult.upload = null
  publicResult.download = null
  try {
    const payload = makeTestPayload(sizeMb)
    publicResult.upload = await measureUpload('/network/speed-test/upload', payload)
    publicResult.download = await measureDownload(`/network/speed-test/public?size_mb=${sizeMb}`)
  } catch (err: any) {
    console.warn('公网测速失败', err)
  } finally {
    publicResult.loading = false
  }
}

export function formatMbps(mbps: number): string {
  return `${mbps.toFixed(2)} Mbps`
}
