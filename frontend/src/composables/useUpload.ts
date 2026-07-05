import { ref } from 'vue'
import axios from 'axios'
import { encryptFile } from './useCrypto'
import { settings } from './useSettings'
import { recordUpload } from './useNetworkSpeed'
import { buildLanDownloadUrl, fetchServerInfo } from './useServerInfo'
import { notify } from './useNotifications'
import { computeHash } from './useHash'

const CHUNK_SIZE = 2 * 1024 * 1024 // 2 MB
const CHUNK_CONCURRENCY = 3
const CHUNK_RETRIES = 3
const FILE_CONCURRENCY = 2

function generateUUID(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = Math.random() * 16 | 0
    const v = c === 'x' ? r : (r & 0x3 | 0x8)
    return v.toString(16)
  })
}

function simpleHash(input: string): string {
  // djb2-like non-cryptographic hash. Avoids crypto.subtle which requires a
  // secure context (HTTPS/localhost); this fingerprint only needs to be stable
  // for the same file, not cryptographically secure.
  let hash = 5381
  for (let i = 0; i < input.length; i++) {
    hash = ((hash << 5) + hash + input.charCodeAt(i)) >>> 0
  }
  return hash.toString(16).padStart(8, '0')
}

function deriveUploadId(file: File, password?: string): string {
  // Encrypted uploads change ciphertext on every encryption (random IV/salt),
  // so resuming by file fingerprint does not work. Use a random id instead.
  if (password) {
    return generateUUID()
  }
  const relativePath = (file as any).webkitRelativePath || ''
  const raw = `${file.name}|${file.size}|${file.lastModified}|${relativePath}`
  return simpleHash(raw)
}

function getRelativePath(file: File): string | undefined {
  const rp = (file as any).webkitRelativePath
  return typeof rp === 'string' && rp.length > 0 ? rp : undefined
}

function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = Math.random() * 16 | 0
    const v = c === 'x' ? r : (r & 0x3 | 0x8)
    return v.toString(16)
  })
}

export type TaskStatus = 'pending' | 'uploading' | 'merging' | 'done' | 'error'

export interface UploadOptions {
  sender?: string
  receiver?: string
  remark?: string
  password?: string
  hashType?: string
  expiresInMinutes?: number
  targetUrl?: string
}

export interface UploadTask {
  uploadId: string
  filename: string
  totalChunks: number
  uploadedChunks: number
  status: TaskStatus
  error?: string
  fileSize?: number
  downloadUrl?: string
  expiresAt?: number
  receivedAt?: number
  receivedBy?: string
}

interface QueuedFile {
  file: File
  options?: UploadOptions
  resolve: () => void
  reject: (err: any) => void
}

const tasks = ref<Map<string, UploadTask>>(new Map())
const fileQueue: QueuedFile[] = []
let activeFiles = 0

function setTask(t: UploadTask) {
  tasks.value = new Map(tasks.value).set(t.uploadId, t)
}

function resolveUrl(path: string, targetUrl?: string): string {
  if (targetUrl) {
    return `${targetUrl.replace(/\/$/, '')}${path}`
  }
  return path
}

async function fetchReceivedChunks(uploadId: string, targetUrl?: string): Promise<number[]> {
  try {
    const res = await axios.get<{ data?: { received_chunks?: number[] } }>(
      resolveUrl(`/upload/status/${uploadId}`, targetUrl),
    )
    return res.data.data?.received_chunks || []
  } catch {
    return []
  }
}

async function uploadChunk(
  uploadId: string,
  file: File,
  index: number,
  totalChunks: number,
  options?: UploadOptions,
) {
  const start = index * CHUNK_SIZE
  const chunk = file.slice(start, start + CHUNK_SIZE)
  const chunkBuffer = await chunk.arrayBuffer()
  let chunkHash = ''
  try {
    chunkHash = await computeHash(chunkBuffer, 'sha-256')
  } catch {
    // Non-secure contexts lack crypto.subtle; leave hash empty and rely on
    // transport-level integrity (HTTPS/TLS) or final file hash.
  }

  const form = new FormData()
  form.append('upload_id', uploadId)
  form.append('filename', file.name)
  form.append('chunk_index', String(index))
  form.append('total_chunks', String(totalChunks))
  if (chunkHash) form.append('chunk_hash', chunkHash)
  form.append('chunk', new Blob([chunkBuffer]))
  if (options?.sender) form.append('sender', options.sender)
  if (options?.receiver) form.append('receiver', options.receiver)
  if (options?.remark) form.append('remark', options.remark)

  await axios.post(resolveUrl('/upload/chunk', options?.targetUrl), form)
  recordUpload(chunkBuffer.byteLength)
}

async function uploadChunkWithRetry(
  uploadId: string,
  file: File,
  index: number,
  totalChunks: number,
  options?: UploadOptions,
) {
  let lastErr: any
  for (let attempt = 0; attempt < CHUNK_RETRIES; attempt++) {
    try {
      await uploadChunk(uploadId, file, index, totalChunks, options)
      return
    } catch (err) {
      lastErr = err
      if (attempt < CHUNK_RETRIES - 1) {
        await new Promise((r) => setTimeout(r, 500 * (attempt + 1)))
      }
    }
  }
  throw lastErr
}

async function buildDownloadUrl(filename: string, targetUrl?: string): Promise<string> {
  if (targetUrl) {
    return buildLanDownloadUrl(filename, `${targetUrl.replace(/\/$/, '')}/download/`)
  }
  const base = settings.value.apiBase.trim()
  if (base) {
    return buildLanDownloadUrl(filename, `${base.replace(/\/$/, '')}/download/`)
  }
  const info = await fetchServerInfo()
  return buildLanDownloadUrl(filename, info.download_url_prefix)
}

async function uploadSingleFile(file: File, options?: UploadOptions) {
  const uploadId = deriveUploadId(file, options?.password)
  const relativePath = getRelativePath(file)

  const existing = tasks.value.get(uploadId)
  if (existing?.status === 'done') {
    return
  }

  const task: UploadTask = {
    uploadId,
    filename: file.name,
    totalChunks: 0,
    uploadedChunks: 0,
    status: 'uploading',
    fileSize: file.size,
  }
  setTask(task)

  let fileToUpload = file
  const encryptedMeta: { encrypted: boolean; salt?: string; iv?: string } = { encrypted: false }

  if (options?.password) {
    try {
      const { blob, salt, iv } = await encryptFile(file, options.password)
      fileToUpload = new File([blob], file.name)
      encryptedMeta.encrypted = true
      encryptedMeta.salt = salt
      encryptedMeta.iv = iv
    } catch (err) {
      setTask({ ...tasks.value.get(uploadId)!, status: 'error', error: `加密失败: ${err}` })
      return
    }
  }

  const totalChunks = Math.ceil(fileToUpload.size / CHUNK_SIZE) || 1

  // Query server for already-received chunks to support resume.
  const receivedChunks = await fetchReceivedChunks(uploadId, options?.targetUrl)
  const receivedSet = new Set(receivedChunks)

  setTask({
    ...tasks.value.get(uploadId)!,
    totalChunks,
    uploadedChunks: receivedChunks.length,
  })

  // Only enqueue chunks that are missing on the server.
  const queue = Array.from({ length: totalChunks }, (_, i) => i).filter(
    (i) => !receivedSet.has(i),
  )

  const worker = async () => {
    while (queue.length > 0) {
      const idx = queue.shift()!
      await uploadChunkWithRetry(uploadId, fileToUpload, idx, totalChunks, options)
      const cur = tasks.value.get(uploadId)!
      if (cur.status === 'error') {
        // Stop further work once the task has been marked failed.
        return
      }
      setTask({ ...cur, uploadedChunks: cur.uploadedChunks + 1 })
    }
  }

  try {
    await Promise.all(Array.from({ length: CHUNK_CONCURRENCY }, worker))
  } catch (err) {
    setTask({ ...tasks.value.get(uploadId)!, status: 'error', error: String(err) })
    return
  }

  setTask({ ...tasks.value.get(uploadId)!, status: 'merging' })

  try {
    const expiresInMinutes = options?.expiresInMinutes ?? settings.value.qrValidityMinutes
    const res = await axios.post(resolveUrl('/upload/merge', options?.targetUrl), {
      upload_id: uploadId,
      filename: file.name,
      relative_path: relativePath || null,
      total_chunks: totalChunks,
      sender: options?.sender || null,
      receiver: options?.receiver || null,
      remark: options?.remark || null,
      encrypted: encryptedMeta.encrypted,
      salt: encryptedMeta.salt || null,
      iv: encryptedMeta.iv || null,
      expires_in_minutes: expiresInMinutes > 0 ? expiresInMinutes : null,
      hash_type: options?.hashType || 'sha-256',
    })
    const returnedFilename = res.data.data as string | undefined
    const finalFilename = returnedFilename || file.name
    const expiresAt = expiresInMinutes > 0
      ? Math.floor(Date.now() / 1000) + expiresInMinutes * 60
      : undefined
    const displayFilename = relativePath || finalFilename
    const downloadUrl = await buildDownloadUrl(displayFilename, options?.targetUrl)
    setTask({
      ...tasks.value.get(uploadId)!,
      status: 'done',
      filename: displayFilename,
      downloadUrl,
      expiresAt,
    })
    if (settings.value.notificationsEnabled) {
      notify('上传完成', { body: `${displayFilename} 已上传成功` })
    }
  } catch (err) {
    setTask({ ...tasks.value.get(uploadId)!, status: 'error', error: String(err) })
  }
}

function processQueue() {
  while (activeFiles < FILE_CONCURRENCY && fileQueue.length > 0) {
    const item = fileQueue.shift()!
    activeFiles++
    uploadSingleFile(item.file, item.options)
      .then(item.resolve)
      .catch(item.reject)
      .finally(() => {
        activeFiles--
        processQueue()
      })
  }
}

export function useUpload() {
  async function uploadFile(file: File, options?: UploadOptions) {
    return new Promise<void>((resolve, reject) => {
      fileQueue.push({ file, options, resolve, reject })
      processQueue()
    })
  }

  return { tasks, uploadFile }
}
