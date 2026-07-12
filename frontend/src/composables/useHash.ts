export const HASH_ALGORITHMS = [
  { value: 'sha-256', label: 'SHA-256' },
  { value: 'sha-384', label: 'SHA-384' },
  { value: 'sha-512', label: 'SHA-512' },
  { value: 'sha-1', label: 'SHA-1' },
  { value: 'md5', label: 'MD5' },
  { value: 'crc32', label: 'CRC32' },
]

export function normalizeHashType(type?: string): string {
  if (!type) return 'sha-256'
  const t = type.toLowerCase().trim()
  if (['sha1', 'sha-1'].includes(t)) return 'sha-1'
  if (['sha256', 'sha-256'].includes(t)) return 'sha-256'
  if (['sha384', 'sha-384'].includes(t)) return 'sha-384'
  if (['sha512', 'sha-512'].includes(t)) return 'sha-512'
  if (['md5'].includes(t)) return 'md5'
  if (['crc32', 'crc-32'].includes(t)) return 'crc32'
  return 'sha-256'
}

export async function computeHash(buffer: ArrayBuffer, algorithm: string): Promise<string> {
  const normalized = normalizeHashType(algorithm)
  if (normalized === 'md5') {
    return computeMd5(buffer)
  }
  if (normalized === 'crc32') {
    return computeCrc32(buffer)
  }
  if (typeof crypto === 'undefined' || !crypto.subtle) {
    throw new Error('当前环境不支持 Web Crypto API，请在 HTTPS 或 localhost 下使用校验功能')
  }
  const digest = await crypto.subtle.digest(normalized, buffer)
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

// ------------------------------------------------------------------
// MD5 (RFC 1321) pure TypeScript implementation
// ------------------------------------------------------------------

function computeMd5(buffer: ArrayBuffer): string {
  const data = new Uint8Array(buffer)
  const bitLen = data.length * 8

  // Padding
  const padLen = (data.length % 64 < 56 ? 56 : 120) - (data.length % 64)
  const padded = new Uint8Array(data.length + padLen + 8)
  padded.set(data)
  padded[data.length] = 0x80
  const view = new DataView(padded.buffer)
  view.setUint32(padded.length - 8, bitLen & 0xffffffff, true)
  view.setUint32(padded.length - 4, (bitLen / 0x100000000) | 0, true)

  let a = 0x67452301
  let b = 0xefcdab89
  let c = 0x98badcfe
  let d = 0x10325476

  const s = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
  ]
  const K = new Uint32Array(64)
  for (let i = 0; i < 64; i++) {
    K[i] = Math.floor(Math.abs(Math.sin(i + 1)) * 0x100000000)
  }

  for (let offset = 0; offset < padded.length; offset += 64) {
    const chunk = new DataView(padded.buffer, offset, 64)
    const M = new Uint32Array(16)
    for (let i = 0; i < 16; i++) {
      M[i] = chunk.getUint32(i * 4, true)
    }

    let AA = a
    let BB = b
    let CC = c
    let DD = d

    for (let i = 0; i < 64; i++) {
      let F: number
      let g: number
      if (i < 16) {
        F = (BB & CC) | (~BB & DD)
        g = i
      } else if (i < 32) {
        F = (DD & BB) | (~DD & CC)
        g = (5 * i + 1) % 16
      } else if (i < 48) {
        F = BB ^ CC ^ DD
        g = (3 * i + 5) % 16
      } else {
        F = CC ^ (BB | ~DD)
        g = (7 * i) % 16
      }
      const temp = DD
      DD = CC
      CC = BB
      BB = BB + leftRotate((AA + F + K[i] + M[g]) >>> 0, s[i])
      AA = temp
    }

    a = (a + AA) >>> 0
    b = (b + BB) >>> 0
    c = (c + CC) >>> 0
    d = (d + DD) >>> 0
  }

  return [a, b, c, d]
    .map((v) => {
      const hex = v.toString(16)
      return hex.padStart(8, '0').match(/../g)!.reverse().join('')
    })
    .join('')
}

function leftRotate(x: number, c: number): number {
  return ((x << c) | (x >>> (32 - c))) >>> 0
}

// ------------------------------------------------------------------
// CRC32 (IEEE 802.3) pure TypeScript implementation
// ------------------------------------------------------------------

function makeCrc32Table(): Uint32Array {
  const table = new Uint32Array(256)
  for (let i = 0; i < 256; i++) {
    let c = i
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    }
    table[i] = c >>> 0
  }
  return table
}

let crc32Table: Uint32Array | null = null

function computeCrc32(buffer: ArrayBuffer): string {
  if (!crc32Table) crc32Table = makeCrc32Table()
  const data = new Uint8Array(buffer)
  let c = 0xffffffff
  for (let i = 0; i < data.length; i++) {
    c = (crc32Table[(c ^ data[i]) & 0xff] ^ (c >>> 8)) >>> 0
  }
  return ((c ^ 0xffffffff) >>> 0).toString(16).padStart(8, '0')
}
