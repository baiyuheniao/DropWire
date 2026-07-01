import { ref } from 'vue'
import axios from 'axios'

const CHUNK_SIZE = 2 * 1024 * 1024 // 2 MB
const CONCURRENCY = 3

export type TaskStatus = 'pending' | 'uploading' | 'merging' | 'done' | 'error'

export interface UploadTask {
  uploadId: string
  filename: string
  totalChunks: number
  uploadedChunks: number
  status: TaskStatus
  error?: string
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
) {
  const start = index * CHUNK_SIZE
  const chunk = file.slice(start, start + CHUNK_SIZE)

  const form = new FormData()
  form.append('upload_id', uploadId)
  form.append('filename', file.name)
  form.append('chunk_index', String(index))
  form.append('total_chunks', String(totalChunks))
  form.append('chunk', chunk)

  await axios.post('/upload/chunk', form)
}

export function useUpload() {
  async function uploadFile(file: File) {
    const uploadId = crypto.randomUUID()
    const totalChunks = Math.ceil(file.size / CHUNK_SIZE) || 1

    const task: UploadTask = {
      uploadId,
      filename: file.name,
      totalChunks,
      uploadedChunks: 0,
      status: 'uploading',
    }
    setTask(task)

    // Bounded-concurrency queue
    const queue = Array.from({ length: totalChunks }, (_, i) => i)

    const worker = async () => {
      while (queue.length > 0) {
        const idx = queue.shift()!
        await uploadChunk(uploadId, file, idx, totalChunks)
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
      await axios.post('/upload/merge', {
        upload_id: uploadId,
        filename: file.name,
        total_chunks: totalChunks,
      })
      setTask({ ...tasks.value.get(uploadId)!, status: 'done' })
    } catch (err) {
      setTask({ ...tasks.value.get(uploadId)!, status: 'error', error: String(err) })
    }
  }

  return { tasks, uploadFile }
}
