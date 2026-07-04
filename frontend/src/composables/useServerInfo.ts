import axios from 'axios'

export interface ServerInfo {
  ip: string
  port: number
  download_url_prefix: string
}

const CACHE_TTL_MS = 60_000

let cached: ServerInfo | null = null
let cachedAt = 0
let fetching: Promise<ServerInfo> | null = null

export async function fetchServerInfo(): Promise<ServerInfo> {
  if (cached && Date.now() - cachedAt < CACHE_TTL_MS) return cached
  if (fetching) return fetching

  fetching = axios.get('/server-info').then((res) => {
    const info = res.data as ServerInfo
    cached = info
    cachedAt = Date.now()
    return info
  }).catch(() => {
    // fallback to current origin
    const fallback: ServerInfo = {
      ip: window.location.hostname,
      port: Number(window.location.port) || (window.location.protocol === 'https:' ? 443 : 80),
      download_url_prefix: `${window.location.origin}/download/`,
    }
    cached = fallback
    cachedAt = Date.now()
    return fallback
  })

  return fetching.finally(() => {
    fetching = null
  })
}

export function clearServerInfoCache() {
  cached = null
  cachedAt = 0
  fetching = null
}

export function buildLanDownloadUrl(filename: string, prefix?: string): string {
  const encoded = encodeURIComponent(filename)
  if (prefix) {
    return `${prefix.replace(/\/$/, '')}/${encoded}`
  }
  return `${window.location.origin}/download/${encoded}`
}
