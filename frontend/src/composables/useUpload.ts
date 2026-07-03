import { ref } from 'vue'
import axios from 'axios'
import { encryptFile } from './useCrypto'
import { settings } from './useSettings'

const CHUNK_SIZE = 2 * 1024 * 1024 // 2 MB
const CONCURRENCY = 3

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
  expiresInMinutes?: number
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
}

const tasks = ref<Map<string, UploadTask>>(new Map())

function setTask(t: UploadTask) {
  tasks.value = new Map(tasks.value).set(t.uploadId, t)
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

  const form = new FormData()
  form.append('upload_id', uploadId)
  form.append('filename', file.name)
  form.append('chunk_index', String(index))
  form.append('total_chunks', String(totalChunks))
  form.append('chunk', chunk)
  if (options?.sender) form.append('sender', options.sender)
  if (options?.receiver) form.append('receiver', options.receiver)
  if (options?.remark) form.append('remark', options.remark)

  await axios.post('/upload/chunk', form)
}

function buildDownloadUrl(filename: string): string {
  const encoded = encodeURIComponent(filename)
  const base = settings.value.apiBase.trim()
  if (base) {
    return `${base.replace(/\/$/, '')}/download/${encoded}`
  }
  return `${window.location.origin}/download/${encoded}`
}

export function useUpload() {
  async function uploadFile(file: File, options?: UploadOptions) {
    const uploadId = generateUUID()

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
    setTask({ ...tasks.value.get(uploadId)!, totalChunks })

    // Bounded-concurrency queue
    const queue = Array.from({ length: totalChunks }, (_, i) => i)

    const worker = async () => {
      while (queue.length > 0) {
        const idx = queue.shift()!
        await uploadChunk(uploadId, fileToUpload, idx, totalChunks, options)
        const cur = tasks.value.get(uploadId)!
        setTask({ ...cur, uploadedChunks: cur.uploadedChunks + 1 })
      }
    }

    try {
      await Promise.all(Array.from({ length: CONCURRENCY }, worker))
    } catch (err) {
      setTask({ ...tasks.value.get(uploadId)!, status: 'error', error: String(err) })
      return
    }

    setTask({ ...tasks.value.get(uploadId)!, status: 'merging' })

    try {
      const expiresInMinutes = options?.expiresInMinutes ?? settings.value.qrValidityMinutes
      const res = await axios.post('/upload/merge', {
        upload_id: uploadId,
        filename: file.name,
        total_chunks: totalChunks,
        sender: options?.sender || null,
        receiver: options?.receiver || null,
        remark: options?.remark || null,
        encrypted: encryptedMeta.encrypted,
        salt: encryptedMeta.salt || null,
        iv: encryptedMeta.iv || null,
        expires_in_minutes: expiresInMinutes > 0 ? expiresInMinutes : null,
      })
      const returnedFilename = res.data.data as string | undefined
      const finalFilename = returnedFilename || file.name
      const expiresAt = expiresInMinutes > 0
        ? Math.floor(Date.now() / 1000) + expiresInMinutes * 60
        : undefined
      setTask({
        ...tasks.value.get(uploadId)!,
        status: 'done',
        filename: finalFilename,
        downloadUrl: buildDownloadUrl(finalFilename),
        expiresAt,
      })
    } catch (err) {
      setTask({ ...tasks.value.get(uploadId)!, status: 'error', error: String(err) })
    }
  }

  return { tasks, uploadFile }
}
