export const HASH_ALGORITHMS = [
  { value: 'sha-256', label: 'SHA-256' },
  { value: 'sha-384', label: 'SHA-384' },
  { value: 'sha-512', label: 'SHA-512' },
  { value: 'sha-1', label: 'SHA-1' },
]

export function normalizeHashType(type?: string): string {
  if (!type) return 'sha-256'
  const t = type.toLowerCase().trim()
  if (['sha1', 'sha-1'].includes(t)) return 'sha-1'
  if (['sha256', 'sha-256'].includes(t)) return 'sha-256'
  if (['sha384', 'sha-384'].includes(t)) return 'sha-384'
  if (['sha512', 'sha-512'].includes(t)) return 'sha-512'
  return 'sha-256'
}

export async function computeHash(buffer: ArrayBuffer, algorithm: string): Promise<string> {
  const normalized = normalizeHashType(algorithm)
  if (typeof crypto === 'undefined' || !crypto.subtle) {
    throw new Error('当前环境不支持 Web Crypto API，请在 HTTPS 或 localhost 下使用校验功能')
  }
  const digest = await crypto.subtle.digest(normalized, buffer)
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}
