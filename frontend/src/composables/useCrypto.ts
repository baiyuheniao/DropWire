const SALT_LEN = 16
const IV_LEN = 12
const ITERATIONS = 100000

function bytesToBase64(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
}

function base64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return Uint8Array.from(bytes)
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

function arraysEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false
  }
  return true
}

export async function encryptFile(
  file: File,
  password: string,
  onProgress?: (current: number, total: number) => void,
): Promise<{ blob: Blob; salt: string; iv: string }> {
  if (typeof crypto === 'undefined' || !crypto.subtle) {
    throw new Error('当前环境不支持 Web Crypto API，请在 HTTPS 或 localhost 下使用加密功能')
  }

  const CryptoWorker = new Worker(new URL('../workers/crypto.worker.ts', import.meta.url), {
    type: 'module',
  })

  return new Promise((resolve, reject) => {
    CryptoWorker.onmessage = (event) => {
      const msg = event.data
      if (msg.type === 'progress') {
        onProgress?.(msg.chunkIndex + 1, msg.totalChunks)
      } else if (msg.type === 'done') {
        CryptoWorker.terminate()
        resolve({
          blob: new Blob([msg.blob]),
          salt: msg.salt,
          iv: msg.iv,
        })
      } else if (msg.type === 'error') {
        CryptoWorker.terminate()
        reject(new Error(msg.error))
      }
    }

    CryptoWorker.onerror = (err) => {
      CryptoWorker.terminate()
      reject(err)
    }

    CryptoWorker.postMessage({ file, password })
  })
}

export async function decryptFile(
  encryptedData: ArrayBuffer,
  password: string,
  saltB64: string,
  ivB64: string,
): Promise<ArrayBuffer> {
  if (typeof crypto === 'undefined' || !crypto.subtle) {
    throw new Error('当前环境不支持 Web Crypto API，请在 HTTPS 或 localhost 下使用解密功能')
  }

  const data = new Uint8Array(encryptedData)

  // Detect chunked format: salt(16) + numChunks(4) + [iv(12) + len(4) + ciphertext]*
  if (data.byteLength >= SALT_LEN + 4) {
    const headerSalt = data.slice(0, SALT_LEN)
    const expectedSalt = base64ToBytes(saltB64)
    const numChunks = new DataView(data.buffer).getUint32(SALT_LEN, false)

    if (numChunks > 0 && arraysEqual(headerSalt, expectedSalt)) {
      const key = await deriveKey(password, expectedSalt.buffer as ArrayBuffer)
      const plaintexts: Uint8Array[] = []
      let offset = SALT_LEN + 4

      for (let i = 0; i < numChunks; i++) {
        if (offset + IV_LEN + 4 > data.byteLength) {
          throw new Error('加密文件损坏')
        }
        const iv = data.slice(offset, offset + IV_LEN)
        const len = new DataView(data.buffer).getUint32(offset + IV_LEN, false)
        offset += IV_LEN + 4
        if (offset + len > data.byteLength) {
          throw new Error('加密文件损坏')
        }
        const ciphertext = data.slice(offset, offset + len)
        offset += len

        const plain = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ciphertext)
        plaintexts.push(new Uint8Array(plain))
      }

      const totalLength = plaintexts.reduce((sum, p) => sum + p.length, 0)
      const result = new Uint8Array(totalLength)
      let pos = 0
      for (const p of plaintexts) {
        result.set(p, pos)
        pos += p.length
      }
      return result.buffer
    }
  }

  // Legacy single-chunk format fallback.
  const salt = base64ToBytes(saltB64)
  const iv = base64ToBytes(ivB64)
  const key = await deriveKey(password, salt.buffer as ArrayBuffer)
  return crypto.subtle.decrypt({ name: 'AES-GCM', iv: iv as BufferSource }, key, encryptedData)
}
