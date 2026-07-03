import axios from 'axios'

export interface ServerInfo {
  ip: string
  port: number
  download_url_prefix: string
}

let cached: ServerInfo | null = null
let fetching: Promise<ServerInfo> | null = null

export async function fetchServerInfo(): Promise<ServerInfo> {
  if (cached) return cached
  if (fetching) return fetching

  fetching = axios.get('/server-info').then((res) => {
    const info = res.data as ServerInfo
    cached = info
    return info
  }).catch(() => {
    // fallback to current origin
    const fallback: ServerInfo = {
      ip: window.location.hostname,
      port: Number(window.location.port) || (window.location.protocol === 'https:' ? 443 : 80),
      download_url_prefix: `${window.location.origin}/download/`,
    }
    cached = fallback
    return fallback
  })

  return fetching
}

export function clearServerInfoCache() {
  cached = null
  fetching = null
}

export function buildLanDownloadUrl(filename: string, prefix?: string): string {
  const encoded = encodeURIComponent(filename)
  if (prefix) {
    return `${prefix.replace(/\/$/, '')}/${encoded}`
  }
  return `${window.location.origin}/download/${encoded}`
}
