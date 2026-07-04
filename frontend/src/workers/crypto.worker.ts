/// <reference lib="webworker" />

const SALT_LEN = 16
const IV_LEN = 12
const ITERATIONS = 100000
const CHUNK_SIZE = 1024 * 1024 // 1 MB plaintext per chunk

function bytesToBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
}

async function deriveKey(password: string, salt: BufferSource): Promise<CryptoKey> {
  const encoder = new TextEncoder()
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    { name: 'PBKDF2' },
    false,
    ['deriveKey'],
  )
  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations: ITERATIONS,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  )
}

interface EncryptMessage {
  file: File
  password: string
}

self.onmessage = async (event: MessageEvent<EncryptMessage>) => {
  const { file, password } = event.data
  try {
    if (typeof crypto === 'undefined' || !crypto.subtle) {
      throw new Error('当前环境不支持 Web Crypto API')
    }

    const salt = new Uint8Array(SALT_LEN)
    crypto.getRandomValues(salt)

    const key = await deriveKey(password, salt)

    const totalSize = file.size
    const numChunks = Math.ceil(totalSize / CHUNK_SIZE) || 1
    const headerSize = SALT_LEN + 4

    const parts: Uint8Array[] = []
    let firstIv = ''

    for (let i = 0; i < numChunks; i++) {
      const start = i * CHUNK_SIZE
      const end = Math.min(start + CHUNK_SIZE, totalSize)
      const slice = file.slice(start, end)
      const plaintext = await slice.arrayBuffer()

      const iv = new Uint8Array(IV_LEN)
      crypto.getRandomValues(iv)
      if (i === 0) {
        firstIv = bytesToBase64(iv.buffer)
      }

      const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, plaintext)

      const lenBuf = new ArrayBuffer(4)
      new DataView(lenBuf).setUint32(0, ciphertext.byteLength, false)

      parts.push(iv)
      parts.push(new Uint8Array(lenBuf))
      parts.push(new Uint8Array(ciphertext))

      self.postMessage({ type: 'progress', chunkIndex: i, totalChunks: numChunks })
    }

    const totalLength = headerSize + parts.reduce((sum, p) => sum + p.length, 0)
    const result = new Uint8Array(totalLength)
    result.set(salt, 0)
    new DataView(result.buffer).setUint32(SALT_LEN, numChunks, false)

    let offset = headerSize
    for (const part of parts) {
      result.set(part, offset)
      offset += part.length
    }

    self.postMessage(
      {
        type: 'done',
        blob: result.buffer,
        salt: bytesToBase64(salt.buffer),
        iv: firstIv,
      },
      [result.buffer],
    )
  } catch (err: any) {
    self.postMessage({ type: 'error', error: err?.message || '加密失败' })
  }
}
