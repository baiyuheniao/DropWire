import { ref } from 'vue'
import axios from 'axios'

export type ThemeMode = 'light' | 'dark' | 'system'

export type SpeedUnit = 'B' | 'KB' | 'MB' | 'GB'

export const SPEED_UNIT_MULTIPLIERS: Record<SpeedUnit, number> = {
  B: 1,
  KB: 1024,
  MB: 1024 * 1024,
  GB: 1024 * 1024 * 1024,
}

export interface AppSettings {
  apiBase: string
  autoRefresh: boolean
  refreshInterval: number
  theme: ThemeMode
  qrValidityMinutes: number
  notificationsEnabled: boolean
  uploadRateLimitEnabled: boolean
  uploadRateLimit: number
  uploadRateLimitUnit: SpeedUnit
  downloadRateLimitEnabled: boolean
  downloadRateLimit: number
  downloadRateLimitUnit: SpeedUnit
}

export interface QrValidityOption {
  label: string
  value: number
}

export const QR_VALIDITY_OPTIONS: QrValidityOption[] = [
  { label: '15 分钟', value: 15 },
  { label: '1 小时', value: 60 },
  { label: '24 小时', value: 1440 },
  { label: '7 天', value: 10080 },
  { label: '永久', value: 0 },
]

const SETTINGS_KEY = 'dropwire_settings'

const defaultSettings: AppSettings = {
  apiBase: '',
  autoRefresh: false,
  refreshInterval: 10,
  theme: 'system',
  qrValidityMinutes: 60,
  notificationsEnabled: true,
  uploadRateLimitEnabled: false,
  uploadRateLimit: 1,
  uploadRateLimitUnit: 'MB',
  downloadRateLimitEnabled: false,
  downloadRateLimit: 1,
  downloadRateLimitUnit: 'MB',
}

export const settings = ref<AppSettings>({ ...defaultSettings })

export function loadSettings() {
  const raw = localStorage.getItem(SETTINGS_KEY)
  if (raw) {
    try {
      const parsed = JSON.parse(raw)
      settings.value = { ...defaultSettings, ...parsed }
    } catch {
      // ignore malformed settings
    }
  }
  applySettings()
}

export function saveSettings(next: Partial<AppSettings>) {
  settings.value = { ...settings.value, ...next }
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings.value))
  applySettings()
}

export function applySettings() {
  axios.defaults.baseURL = settings.value.apiBase || ''
}

export function useSettings() {
  return { settings, saveSettings }
}
