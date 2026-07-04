import { ref } from 'vue'
import axios from 'axios'

export interface DeviceInfo {
  id: string
  name: string
  avatar?: string
  ip: string
  port: number
  last_seen: number
  online: boolean
}

export interface SelfDevice {
  id: string
  name: string
  avatar?: string
  ip: string
  port: number
}

export const devices = ref<DeviceInfo[]>([])
export const selfDevice = ref<SelfDevice | null>(null)
export const devicesLoading = ref(false)
export const devicesError = ref('')

export async function fetchDevices() {
  devicesLoading.value = true
  devicesError.value = ''
  try {
    const res = await axios.get('/devices')
    if (res.data) {
      devices.value = res.data
    }
  } catch (err: any) {
    devicesError.value = err?.response?.data?.message || '获取设备列表失败'
  } finally {
    devicesLoading.value = false
  }
}

export async function fetchSelfDevice() {
  try {
    const res = await axios.get('/device')
    if (res.data) {
      selfDevice.value = res.data
    }
  } catch {
    // ignore
  }
}

export async function updateSelfDevice(updates: { name?: string; avatar?: string }) {
  const res = await axios.post('/device', updates)
  selfDevice.value = res.data
  return res.data as SelfDevice
}

export function startDeviceRefresh(intervalMs = 3000) {
  fetchDevices()
  fetchSelfDevice()
  return setInterval(() => {
    fetchDevices()
  }, intervalMs)
}
