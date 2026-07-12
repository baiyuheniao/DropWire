<template>
  <section class="section">
    <h3>网络诊断</h3>
    <p class="hint">分析局域网拓扑、测试内外网传输速度</p>

    <div class="network-summary-card" @click="openModal">
      <div class="summary-main">
        <div class="summary-icon">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 2a10 10 0 0 1 10 10" />
            <path d="M12 12 4.5 6.5" />
            <path d="M12 12v10" />
          </svg>
        </div>
        <div class="summary-body">
          <div class="summary-title">网络状态概览</div>
          <div v-if="statusLoading" class="summary-status">检测中...</div>
          <div v-else-if="status" class="summary-status">
            本机 {{ status.self_ip }} · 局域网 {{ status.peer_count }} 台设备 ·
            {{ status.has_public_internet ? '公网正常' : '公网受限' }}
          </div>
          <div v-else class="summary-status">点击进行网络诊断</div>
        </div>
      </div>
      <button class="summary-btn" @click.stop="openModal">开始诊断</button>
    </div>

    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showModal" class="modal-overlay" @click="closeModal">
          <div class="modal-panel" @click.stop>
            <div class="modal-header">
              <h3>网络诊断</h3>
              <button class="modal-close" @click="closeModal">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>

            <div class="modal-body">
              <div class="network-card">
                <div class="network-status">
                  <h4>网络现状</h4>
                  <div v-if="statusLoading" class="network-loading">检测中...</div>
                  <div v-else-if="status" class="status-grid">
                    <div class="status-item">
                      <span class="status-key">本机名称</span>
                      <span class="status-value">{{ status.self_name }}</span>
                    </div>
                    <div class="status-item">
                      <span class="status-key">本机 IP</span>
                      <span class="status-value">{{ status.self_ip }}:{{ status.port }}</span>
                    </div>
                    <div class="status-item">
                      <span class="status-key">公网 IP</span>
                      <span class="status-value">{{ status.public_ip || '未知' }}</span>
                    </div>
                    <div class="status-item">
                      <span class="status-key">局域网设备</span>
                      <span class="status-value">{{ status.peer_count }} 台</span>
                    </div>
                  </div>
                  <p v-if="status" class="status-summary">{{ status.summary }}</p>
                  <button class="btn-primary" :disabled="statusLoading" @click="fetchNetworkStatus">
                    刷新状态
                  </button>
                </div>

                <div class="topology-section">
                  <h4>局域网拓扑</h4>
                  <NetworkTopologyChart :status="status" />
                </div>

                <div class="speed-section">
                  <h4>内网测速</h4>
                  <div class="speed-controls">
                    <select v-model="internalSize" class="speed-select">
                      <option :value="1">1 MB</option>
                      <option :value="10">10 MB</option>
                      <option :value="50">50 MB</option>
                    </select>
                    <button
                      class="btn-primary"
                      :disabled="internalResult.loading"
                      @click="runInternalSpeedTest(internalSize)"
                    >
                      {{ internalResult.loading ? '测试中...' : '开始内网测速' }}
                    </button>
                  </div>
                  <div v-if="internalResult.loading" class="speed-pulse">
                    <div class="pulse-ring" />
                    <span>正在传输测试数据...</span>
                  </div>
                  <div v-else-if="internalResult.upload || internalResult.download" class="speed-visual">
                    <SpeedGauge
                      :speed-mbps="internalResult.upload?.speedMbps || 0"
                      label="上行速度"
                      class="gauge"
                    />
                    <SpeedGauge
                      :speed-mbps="internalResult.download?.speedMbps || 0"
                      label="下行速度"
                      class="gauge"
                    />
                  </div>
                  <div v-if="internalResult.upload || internalResult.download" class="speed-bars">
                    <div class="bar-row">
                      <span class="bar-label">上行</span>
                      <div class="bar-track">
                        <div
                          class="bar-fill upload"
                          :style="{ width: barPct(internalResult.upload) + '%' }"
                        />
                      </div>
                      <span class="bar-value">{{ formatResult(internalResult.upload) }}</span>
                    </div>
                    <div class="bar-row">
                      <span class="bar-label">下行</span>
                      <div class="bar-track">
                        <div
                          class="bar-fill download"
                          :style="{ width: barPct(internalResult.download) + '%' }"
                        />
                      </div>
                      <span class="bar-value">{{ formatResult(internalResult.download) }}</span>
                    </div>
                  </div>
                </div>

                <div class="speed-section">
                  <h4>公网测速</h4>
                  <div class="speed-controls">
                    <select v-model="publicSize" class="speed-select">
                      <option :value="1">1 MB</option>
                      <option :value="10">10 MB</option>
                      <option :value="50">50 MB</option>
                    </select>
                    <button
                      class="btn-primary"
                      :disabled="publicResult.loading"
                      @click="runPublicSpeedTest(publicSize)"
                    >
                      {{ publicResult.loading ? '测试中...' : '开始公网测速' }}
                    </button>
                  </div>
                  <div v-if="publicResult.loading" class="speed-pulse">
                    <div class="pulse-ring" />
                    <span>正在传输测试数据...</span>
                  </div>
                  <div v-else-if="publicResult.upload || publicResult.download" class="speed-visual">
                    <SpeedGauge
                      :speed-mbps="publicResult.upload?.speedMbps || 0"
                      label="上行速度"
                      class="gauge"
                    />
                    <SpeedGauge
                      :speed-mbps="publicResult.download?.speedMbps || 0"
                      label="下行速度"
                      class="gauge"
                    />
                  </div>
                  <div v-if="publicResult.upload || publicResult.download" class="speed-bars">
                    <div class="bar-row">
                      <span class="bar-label">上行</span>
                      <div class="bar-track">
                        <div
                          class="bar-fill upload"
                          :style="{ width: barPct(publicResult.upload) + '%' }"
                        />
                      </div>
                      <span class="bar-value">{{ formatResult(publicResult.upload) }}</span>
                    </div>
                    <div class="bar-row">
                      <span class="bar-label">下行</span>
                      <div class="bar-track">
                        <div
                          class="bar-fill download"
                          :style="{ width: barPct(publicResult.download) + '%' }"
                        />
                      </div>
                      <span class="bar-value">{{ formatResult(publicResult.download) }}</span>
                    </div>
                  </div>
                  <p class="speed-note">公网测速由后端代理到 Cloudflare Speed Test。</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </section>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  status,
  statusLoading,
  internalResult,
  publicResult,
  fetchNetworkStatus,
  runInternalSpeedTest,
  runPublicSpeedTest,
  formatMbps,
} from '../composables/useNetworkTest'
import NetworkTopologyChart from './NetworkTopologyChart.vue'
import SpeedGauge from './SpeedGauge.vue'

const internalSize = ref(10)
const publicSize = ref(10)
const showModal = ref(false)

function openModal() {
  showModal.value = true
  fetchNetworkStatus()
}

function closeModal() {
  showModal.value = false
}

function formatResult(result: { speedMbps: number } | null) {
  return result ? formatMbps(result.speedMbps) : '-'
}

function barPct(result: { speedMbps: number } | null): number {
  if (!result) return 0
  const max = Math.max(result.speedMbps, 100)
  return Math.min((result.speedMbps / max) * 100, 100)
}

onMounted(fetchNetworkStatus)
</script>

<style scoped>
.network-summary-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 16px;
  background: var(--bg-soft);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.network-summary-card:hover {
  background: var(--bg-card-hover);
  border-color: var(--primary);
}

.summary-main {
  display: flex;
  align-items: center;
  gap: 14px;
  min-width: 0;
}

.summary-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  background: var(--bg-primary-soft);
  color: var(--primary);
  display: grid;
  place-items: center;
  flex-shrink: 0;
}

.summary-icon svg {
  width: 22px;
  height: 22px;
}

.summary-body {
  min-width: 0;
}

.summary-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.summary-status {
  font-size: 13px;
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.summary-btn {
  padding: 8px 16px;
  background: var(--primary);
  color: #fff;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.2s;
}

.summary-btn:hover {
  background: var(--primary-hover);
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.modal-panel {
  width: 100%;
  max-width: 720px;
  max-height: 90vh;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 16px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
}

.modal-header h3 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.modal-close {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  display: grid;
  place-items: center;
  border-radius: 8px;
  transition: background 0.2s, color 0.2s;
}

.modal-close:hover {
  background: var(--bg-soft);
  color: var(--text-primary);
}

.modal-close svg {
  width: 18px;
  height: 18px;
}

.modal-body {
  overflow-y: auto;
  padding: 20px;
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.network-card {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.network-status h4,
.topology-section h4,
.speed-section h4 {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 12px;
}

.network-loading {
  font-size: 14px;
  color: var(--text-tertiary);
  margin-bottom: 12px;
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 12px;
  margin-bottom: 12px;
}

.status-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  background: var(--bg-soft);
  border-radius: 8px;
}

.status-key {
  font-size: 12px;
  color: var(--text-tertiary);
}

.status-value {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  word-break: break-all;
}

.status-summary {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 12px;
  line-height: 1.5;
}

.speed-controls {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.speed-select {
  padding: 8px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
}

.speed-visual {
  display: flex;
  gap: 24px;
  flex-wrap: wrap;
  justify-content: center;
  margin-bottom: 12px;
}

.gauge {
  flex: 1 1 220px;
  max-width: 280px;
}

.speed-pulse {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px 0;
  color: var(--text-secondary);
  font-size: 14px;
}

.pulse-ring {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--primary);
  animation: pulse 1.2s ease-out infinite;
}

@keyframes pulse {
  0% {
    transform: scale(0.6);
    opacity: 1;
  }
  100% {
    transform: scale(2.2);
    opacity: 0;
  }
}

.speed-bars {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 12px;
}

.bar-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.bar-label {
  width: 40px;
  font-size: 13px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.bar-track {
  flex: 1;
  height: 10px;
  background: var(--bg-soft);
  border-radius: 999px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  border-radius: 999px;
  transition: width 0.4s ease;
}

.bar-fill.upload {
  background: linear-gradient(90deg, var(--primary), var(--primary-light, var(--primary)));
}

.bar-fill.download {
  background: linear-gradient(90deg, #10b981, #34d399);
}

.bar-value {
  width: 90px;
  text-align: right;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  flex-shrink: 0;
}

.speed-note {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 8px;
}

@media (max-width: 640px) {
  .network-summary-card {
    flex-direction: column;
    align-items: flex-start;
  }

  .summary-btn {
    width: 100%;
  }

  .modal-overlay {
    padding: 0;
    align-items: flex-end;
  }

  .modal-panel {
    max-height: 92vh;
    border-radius: 16px 16px 0 0;
  }
}
</style>
