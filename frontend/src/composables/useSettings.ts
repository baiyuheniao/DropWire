import { ref } from 'vue'
import axios from 'axios'

export interface AppSettings {
  apiBase: string
  autoRefresh: boolean
  refreshInterval: number
}

const SETTINGS_KEY = 'dropwire_settings'

const defaultSettings: AppSettings = {
  apiBase: '',
  autoRefresh: false,
  refreshInterval: 10,
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
