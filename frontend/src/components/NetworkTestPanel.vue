<template>
  <section class="section">
    <h3>网络诊断</h3>
    <p class="hint">分析局域网拓扑、测试内外网传输速度</p>

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
        <div v-if="internalResult.upload || internalResult.download" class="speed-result">
          <div class="speed-row">
            <span class="speed-label">上行</span>
            <span class="speed-value">{{ formatResult(internalResult.upload) }}</span>
          </div>
          <div class="speed-row">
            <span class="speed-label">下行</span>
            <span class="speed-value">{{ formatResult(internalResult.download) }}</span>
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
        <div v-if="publicResult.upload || publicResult.download" class="speed-result">
          <div class="speed-row">
            <span class="speed-label">上行</span>
            <span class="speed-value">{{ formatResult(publicResult.upload) }}</span>
          </div>
          <div class="speed-row">
            <span class="speed-label">下行</span>
            <span class="speed-value">{{ formatResult(publicResult.download) }}</span>
          </div>
        </div>
        <p class="speed-note">公网测速由后端代理到 Cloudflare Speed Test。</p>
      </div>
    </div>
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

const internalSize = ref(10)
const publicSize = ref(10)

function formatResult(result: { speedMbps: number } | null) {
  return result ? formatMbps(result.speedMbps) : '-'
}

onMounted(fetchNetworkStatus)
</script>

<style scoped>
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

.speed-result {
  display: flex;
  gap: 24px;
  flex-wrap: wrap;
}

.speed-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.speed-label {
  font-size: 13px;
  color: var(--text-tertiary);
}

.speed-value {
  font-size: 15px;
  font-weight: 600;
  color: var(--primary);
}

.speed-note {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 8px;
}
</style>
