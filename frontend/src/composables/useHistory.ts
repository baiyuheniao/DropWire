export type HistoryType = 'send' | 'receive'

export interface HistoryEntry {
  id: string
  type: HistoryType
  filename: string
  size: number
  timestamp: number
  sender?: string
  receiver?: string
  remark?: string
  encrypted?: boolean
  expiresAt?: number
  url?: string
}

const HISTORY_KEY = 'dropwire_history'
const MAX_ENTRIES = 200

function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = Math.random() * 16 | 0
    const v = c === 'x' ? r : (r & 0x3 | 0x8)
    return v.toString(16)
  })
}

export function loadHistory(): HistoryEntry[] {
  const raw = localStorage.getItem(HISTORY_KEY)
  if (!raw) return []
  try {
    const list = JSON.parse(raw) as HistoryEntry[]
    return Array.isArray(list) ? list : []
  } catch {
    return []
  }
}

export function saveHistory(list: HistoryEntry[]) {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(list))
}

export function addHistory(entry: Omit<HistoryEntry, 'id' | 'timestamp'>) {
  const list = loadHistory()
  const newEntry: HistoryEntry = {
    ...entry,
    id: generateUUID(),
    timestamp: Date.now(),
  }
  list.unshift(newEntry)
  if (list.length > MAX_ENTRIES) {
    list.length = MAX_ENTRIES
  }
  saveHistory(list)
  return newEntry
}

export function removeHistory(id: string) {
  const list = loadHistory().filter((h) => h.id !== id)
  saveHistory(list)
}

export function clearHistory(type?: HistoryType) {
  if (!type) {
    saveHistory([])
    return
  }
  const list = loadHistory().filter((h) => h.type !== type)
  saveHistory(list)
}
