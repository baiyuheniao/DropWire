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

export async function encryptFile(
  file: File,
  password: string,
): Promise<{ blob: Blob; salt: string; iv: string }> {
  if (typeof crypto === 'undefined' || !crypto.subtle) {
    throw new Error('当前环境不支持 Web Crypto API，请在 HTTPS 或 localhost 下使用加密功能')
  }
  const salt = new Uint8Array(SALT_LEN)
  const iv = new Uint8Array(IV_LEN)
  crypto.getRandomValues(salt)
  crypto.getRandomValues(iv)
  const key = await deriveKey(password, salt)
  const plaintext = await file.arrayBuffer()
  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, plaintext)
  return {
    blob: new Blob([ciphertext]),
    salt: bytesToBase64(salt.buffer),
    iv: bytesToBase64(iv.buffer),
  }
}

export async function decryptFile(
  encryptedData: ArrayBuffer,
  password: string,
  saltB64: string,
  ivB64: string,
): Promise<ArrayBuffer> {
  const salt = new Uint8Array(base64ToBytes(saltB64))
  const iv = new Uint8Array(base64ToBytes(ivB64))
  const key = await deriveKey(password, salt)
  return crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, encryptedData)
}
