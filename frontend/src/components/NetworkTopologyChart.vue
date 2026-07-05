<template>
  <div class="topology-chart">
    <svg :viewBox="`0 0 ${width} ${height}`" class="topology-svg">
      <!-- Edges from self to each peer -->
      <line
        v-for="node in peerNodes"
        :key="`edge-${node.id}`"
        :x1="centerX"
        :y1="centerY"
        :x2="node.x"
        :y2="node.y"
        class="topology-edge"
      />
      <!-- Self node -->
      <circle :cx="centerX" :cy="centerY" :r="selfR" class="topology-node self" />
      <text :x="centerX" :y="centerY" class="topology-label self">{{ selfLabel }}</text>
      <!-- Peer nodes -->
      <g v-for="node in peerNodes" :key="`peer-${node.id}`">
        <circle :cx="node.x" :cy="node.y" :r="peerR" class="topology-node peer" />
        <text :x="node.x" :y="node.y - peerR - 6" class="topology-label peer">{{ node.name }}</text>
        <text :x="node.x" :y="node.y + peerR + 14" class="topology-ip">{{ node.ip }}</text>
      </g>
    </svg>
    <p v-if="peerNodes.length === 0" class="topology-empty">局域网内未发现其他设备</p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { DeviceInfo, NetworkStatus } from '../composables/useNetworkTest'

const props = defineProps<{
  status: NetworkStatus | null
}>()

const width = 360
const height = 240
const centerX = width / 2
const centerY = height / 2
const radius = 80
const selfR = 28
const peerR = 18

const selfLabel = computed(() => {
  const name = props.status?.self_name || '本机'
  return name.length > 4 ? name.slice(0, 4) + '…' : name
})

const peerNodes = computed(() => {
  const peers = props.status?.peers || []
  const count = peers.length
  return peers.map((peer: DeviceInfo, index: number) => {
    const angle = (Math.PI * 2 * index) / Math.max(1, count) - Math.PI / 2
    return {
      ...peer,
      x: centerX + radius * Math.cos(angle),
      y: centerY + radius * Math.sin(angle),
    }
  })
})
</script>

<style scoped>
.topology-chart {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.topology-svg {
  width: 100%;
  max-width: 400px;
  height: auto;
}

.topology-edge {
  stroke: var(--border-strong);
  stroke-width: 1.5;
  stroke-dasharray: 4 4;
}

.topology-node {
  stroke-width: 2;
}

.topology-node.self {
  fill: var(--primary);
  stroke: var(--primary-hover);
}

.topology-node.peer {
  fill: var(--bg-card);
  stroke: var(--primary);
}

.topology-label {
  text-anchor: middle;
  dominant-baseline: middle;
  font-size: 12px;
  font-weight: 600;
  pointer-events: none;
}

.topology-label.self {
  fill: #fff;
}

.topology-label.peer {
  fill: var(--text-primary);
}

.topology-ip {
  text-anchor: middle;
  font-size: 10px;
  fill: var(--text-tertiary);
}

.topology-empty {
  font-size: 13px;
  color: var(--text-tertiary);
}
</style>
