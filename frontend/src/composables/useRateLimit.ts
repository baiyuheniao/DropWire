import { settings, SPEED_UNIT_MULTIPLIERS, type SpeedUnit } from './useSettings'

/**
 * 基于时间窗口的令牌桶限速器。
 * 所有并发 worker 共享同一个实例，保证全局速率不超过上限。
 */
class RateLimiter {
  private bytesPerSecond = 0
  private windowStart = 0
  private bytesConsumed = 0

  /** 设置速率上限（字节/秒），0 表示不限速 */
  setLimit(bytesPerSecond: number) {
    this.bytesPerSecond = Math.max(0, bytesPerSecond)
    this.reset()
  }

  get enabled(): boolean {
    return this.bytesPerSecond > 0
  }

  reset() {
    this.windowStart = performance.now()
    this.bytesConsumed = 0
  }

  /**
   * 消费指定字节数，若超出速率上限则等待。
   * 应在每次网络传输完成后调用。
   */
  async consume(bytes: number) {
    if (!this.enabled || bytes <= 0) return

    const now = performance.now()
    const elapsedMs = now - this.windowStart

    // 窗口超过 1 秒则重置
    if (elapsedMs >= 1000) {
      this.windowStart = now
      this.bytesConsumed = 0
    }

    this.bytesConsumed += bytes

    // 计算当前窗口内已经花费的实际时间 vs 应花费的时间
    const expectedMs = (this.bytesConsumed / this.bytesPerSecond) * 1000
    const actualMs = performance.now() - this.windowStart

    if (expectedMs > actualMs) {
      const waitMs = expectedMs - actualMs
      await new Promise((r) => setTimeout(r, waitMs))
    }
  }
}

/** 上传限速器（全局单例，所有上传任务共享） */
const uploadLimiter = new RateLimiter()

/** 下载限速器（全局单例，所有下载任务共享） */
const downloadLimiter = new RateLimiter()

/** 将用户设置的数值 + 单位转换为字节/秒 */
function toBps(value: number, unit: SpeedUnit): number {
  if (value <= 0) return 0
  return Math.round(value * SPEED_UNIT_MULTIPLIERS[unit])
}

/** 根据当前设置刷新上传限速器 */
export function refreshUploadLimiter() {
  const s = settings.value
  if (s.uploadRateLimitEnabled) {
    uploadLimiter.setLimit(toBps(s.uploadRateLimit, s.uploadRateLimitUnit))
  } else {
    uploadLimiter.setLimit(0)
  }
}

/** 根据当前设置刷新下载限速器 */
export function refreshDownloadLimiter() {
  const s = settings.value
  if (s.downloadRateLimitEnabled) {
    downloadLimiter.setLimit(toBps(s.downloadRateLimit, s.downloadRateLimitUnit))
  } else {
    downloadLimiter.setLimit(0)
  }
}

export { uploadLimiter, downloadLimiter }
