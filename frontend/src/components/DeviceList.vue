<template>
  <div class="device-list">
    <div class="device-header">
      <h3>在线设备</h3>
      <span v-if="devicesLoading" class="loading-hint">刷新中...</span>
      <span v-else-if="devicesError" class="error-hint">{{ devicesError }}</span>
      <span v-else class="count-hint">{{ devices.length }} 台在线</span>
    </div>

    <div class="devices">
      <div
        class="device-card"
        :class="{ active: modelValue === null, all: true }"
        @click="selectGlobal"
      >
        <div class="device-avatar all">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 16v-4" />
            <path d="M12 8h.01" />
          </svg>
        </div>
        <div class="device-meta">
          <div class="device-name">全局发送</div>
          <div class="device-ip">局域网内所有人可见</div>
        </div>
      </div>

      <div
        v-for="device in devices"
        :key="device.id"
        class="device-card"
        :class="{ active: modelValue?.id === device.id, offline: !device.online }"
        @click="selectDevice(device)"
      >
        <div class="device-avatar" :class="{ logged: device.avatar }">
          <img v-if="device.avatar" :src="device.avatar" alt="avatar" />
          <span v-else>{{ device.name.charAt(0) }}</span>
        </div>
        <div class="device-meta">
          <div class="device-name">{{ device.name }}</div>
          <div class="device-ip">{{ device.ip }}:{{ device.port }}</div>
        </div>
        <div class="status-dot" :class="device.online ? 'online' : 'offline'" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { devices, devicesLoading, devicesError, startDeviceRefresh, type DeviceInfo } from '../composables/useDevices'
import { onMounted, onUnmounted } from 'vue'

const props = defineProps<{
  modelValue: DeviceInfo | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', device: DeviceInfo | null): void
}>()

let timer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  timer = startDeviceRefresh(3000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})

function selectGlobal() {
  emit('update:modelValue', null)
}

function selectDevice(device: DeviceInfo) {
  emit('update:modelValue', device)
}
</script>

<style scoped>
.device-list {
  margin-bottom: 20px;
  padding: 18px 20px;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 16px;
}

.device-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
}

.device-header h3 {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-secondary);
}

.loading-hint,
.count-hint {
  font-size: 12px;
  color: var(--text-tertiary);
}

.error-hint {
  font-size: 12px;
  color: var(--danger-text);
}

.devices {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.device-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--bg-soft);
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s, transform 0.15s;
  position: relative;
}

.device-card:hover {
  border-color: var(--primary);
  background: var(--bg-card-hover);
}

.device-card.active {
  border-color: var(--primary);
  background: var(--bg-primary-soft);
}

.device-card.offline {
  opacity: 0.6;
}

.device-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  display: grid;
  place-items: center;
  font-size: 14px;
  color: var(--text-tertiary);
  overflow: hidden;
  flex-shrink: 0;
}

.device-avatar.all {
  color: var(--primary);
  background: var(--bg-primary-soft);
}

.device-avatar.logged {
  background: var(--bg-primary-soft);
  color: var(--primary-text);
  font-weight: 600;
}

.device-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.device-meta {
  min-width: 0;
  flex: 1;
}

.device-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.device-ip {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.online {
  background: #22c55e;
  box-shadow: 0 0 0 2px rgba(34, 197, 94, 0.2);
}

.status-dot.offline {
  background: var(--text-tertiary);
}
</style>
