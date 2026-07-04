import { reactive } from 'vue'
import axios from 'axios'

const CHUNK_SIZE = 2 * 1024 * 1024 // 2 MB，与上传保持一致
const CONCURRENCY = 3
const RETRIES = 3

export type DownloadStatus = 'downloading' | 'paused' | 'done' | 'error'

export interface DownloadTask {
  /** 用于唯一标识一个下载任务（文件路径） */
  key: string
  filename: string
  fileSize: number
  receivedBytes: number
  totalChunks: number
  downloadedChunks: number
  status: DownloadStatus
  error?: string
  /** 底层已下载的 ArrayBuffer 片段，按 chunk 顺序拼接 */
  chunks: ArrayBuffer[]
  /** 中止控制器，暂停时 abort 当前进行中的请求 */
  abortController?: AbortController
}

/** 全局下载任务表，key 为文件路径 */
const tasks = reactive<Record<string, DownloadTask>>({})

export function useDownload() {
  return { tasks }
}

/**
 * 获取指定文件的下载进度（0-100）。仅读取，不创建任务。
 */
export function getDownloadProgress(key: string): number {
  const t = tasks[key]
  if (!t || t.fileSize === 0) return 0
  if (t.status === 'done') return 100
  return Math.min(100, Math.round((t.receivedBytes / t.fileSize) * 100))
}

function ensureTask(key: string, filename: string, fileSize: number): DownloadTask {
  if (!tasks[key]) {
    tasks[key] = {
      key,
      filename,
      fileSize,
      receivedBytes: 0,
      totalChunks: Math.max(1, Math.ceil(fileSize / CHUNK_SIZE)),
      downloadedChunks: 0,
      status: 'downloading',
      chunks: [],
    }
  }
  return tasks[key]
}

async function downloadChunkWithRetry(
  url: string,
  start: number,
  end: number,
  signal: AbortSignal,
): Promise<ArrayBuffer> {
  let lastErr: any
  for (let attempt = 0; attempt < RETRIES; attempt++) {
    if (signal.aborted) throw new DOMException('aborted', 'AbortError')
    try {
      const res = await axios.get(url, {
        responseType: 'arraybuffer',
        headers: { Range: `bytes=${start}-${end}` },
        signal,
      })
      return res.data
    } catch (err: any) {
      // 暂停导致的 abort 直接抛出，不重试
      if (err?.name === 'AbortError' || signal.aborted) throw err
      lastErr = err
      if (attempt < RETRIES - 1) {
        await new Promise((r) => setTimeout(r, 500 * (attempt + 1)))
      }
    }
  }
  throw lastErr
}

/**
 * 启动（或继续）一个分片下载任务。
 * - 若任务不存在则创建。
 * - 若任务处于 paused 状态则恢复。
 * - 若任务已完成则直接返回其 buffer。
 *
 * 返回完整文件的 ArrayBuffer。
 */
export async function startDownload(
  key: string,
  filename: string,
  fileSize: number,
  url: string,
): Promise<ArrayBuffer> {
  const task = ensureTask(key, filename, fileSize)

  // 已完成，直接返回
  if (task.status === 'done') {
    return concatenate(task.chunks)
  }

  // 若正在下载中，不重复启动
  if (task.status === 'downloading') {
    return concatenate(task.chunks)
  }

  task.status = 'downloading'
  task.error = undefined
  task.abortController = new AbortController()
  const { signal } = task.abortController

  // 计算尚未完成的分片
  const totalChunks = task.totalChunks
  const completedSet = new Set<number>()
  for (let i = 0; i < task.downloadedChunks; i++) completedSet.add(i)

  const queue: number[] = []
  for (let i = 0; i < totalChunks; i++) {
    if (!completedSet.has(i)) queue.push(i)
  }

  // 确保 chunks 数组长度足够
  while (task.chunks.length < totalChunks) task.chunks.push(new ArrayBuffer(0))

  const worker = async () => {
    while (queue.length > 0) {
      if (signal.aborted) return
      const idx = queue.shift()!
      const start = idx * CHUNK_SIZE
      const end = Math.min(start + CHUNK_SIZE - 1, fileSize - 1)

      try {
        const buf = await downloadChunkWithRetry(url, start, end, signal)
        task.chunks[idx] = buf
        task.downloadedChunks++
        task.receivedBytes += buf.byteLength
      } catch (err: any) {
        if (signal.aborted) return
        task.status = 'error'
        task.error = String(err?.message || err)
        return
      }
    }
  }

  try {
    await Promise.all(Array.from({ length: CONCURRENCY }, worker))
  } catch (err: any) {
    if (!signal.aborted) {
      task.status = 'error'
      task.error = String(err?.message || err)
    }
    throw err
  }

  // 暂停后 worker 会静默退出，此时不标记完成
  if (signal.aborted) {
    return concatenate(task.chunks)
  }

  // 校验完整性
  if (task.downloadedChunks < totalChunks) {
    task.status = 'error'
    task.error = '部分分片下载失败'
    throw new Error(task.error)
  }

  task.status = 'done'
  return concatenate(task.chunks)
}

/**
 * 暂停一个正在下载的任务。进行中的分片请求会被 abort，
 * 已下载的分片保留在内存中，后续可继续。
 */
export function pauseDownload(key: string) {
  const task = tasks[key]
  if (!task) return
  if (task.status !== 'downloading') return
  task.abortController?.abort()
  task.status = 'paused'
}

/**
 * 取消并清除一个下载任务，释放内存。
 */
export function cancelDownload(key: string) {
  const task = tasks[key]
  if (!task) return
  task.abortController?.abort()
  delete tasks[key]
}

function concatenate(chunks: ArrayBuffer[]): ArrayBuffer {
  const total = chunks.reduce((sum, b) => sum + b.byteLength, 0)
  const result = new Uint8Array(total)
  let offset = 0
  for (const b of chunks) {
    result.set(new Uint8Array(b), offset)
    offset += b.byteLength
  }
  return result.buffer
}
