<template>
  <div class="speed-gauge">
    <svg class="gauge-svg" viewBox="0 0 200 120">
      <!-- Background arc -->
      <path
        class="gauge-bg"
        d="M 20 100 A 80 80 0 0 1 180 100"
        fill="none"
        stroke-width="14"
        stroke-linecap="round"
      />
      <!-- Active arc -->
      <path
        class="gauge-value"
        :d="arcPath"
        fill="none"
        stroke-width="14"
        stroke-linecap="round"
        :style="{ strokeDasharray, strokeDashoffset }"
      />
      <!-- Ticks -->
      <g class="ticks">
        <line
          v-for="tick in ticks"
          :key="tick.angle"
          :x1="tick.x1"
          :y1="tick.y1"
          :x2="tick.x2"
          :y2="tick.y2"
          stroke="var(--border-strong)"
          stroke-width="2"
        />
      </g>
      <!-- Labels -->
      <text x="20" y="115" class="gauge-min">0</text>
      <text x="180" y="115" class="gauge-max" text-anchor="end">{{ maxScale }}</text>
      <text x="100" y="75" class="gauge-speed" text-anchor="middle">{{ displaySpeed }}</text>
      <text x="100" y="95" class="gauge-unit" text-anchor="middle">Mbps</text>
    </svg>
    <div class="gauge-label">{{ label }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  speedMbps: number
  label: string
  maxScale?: number
}>()

const maxScale = computed(() => {
  const raw = props.maxScale || Math.max(100, Math.ceil(props.speedMbps / 50) * 50)
  return Math.max(raw, 1)
})

const ratio = computed(() => {
  const r = props.speedMbps / maxScale.value
  return Math.min(Math.max(r, 0), 1)
})

const arcLength = 251.2 // approximate length of the semi-circle arc with r=80
const strokeDasharray = `${arcLength} ${arcLength}`
const strokeDashoffset = computed(() => arcLength * (1 - ratio.value))

const arcPath = 'M 20 100 A 80 80 0 0 1 180 100'

const displaySpeed = computed(() => {
  if (props.speedMbps <= 0) return '0.00'
  return props.speedMbps.toFixed(2)
})

const ticks = computed(() => {
  const arr = []
  const totalTicks = 9
  for (let i = 0; i <= totalTicks; i++) {
    const t = i / totalTicks
    const angle = Math.PI * (1 - t)
    const rIn = 68
    const rOut = 78
    arr.push({
      angle: i,
      x1: 100 + rIn * Math.cos(angle),
      y1: 100 - rIn * Math.sin(angle),
      x2: 100 + rOut * Math.cos(angle),
      y2: 100 - rOut * Math.sin(angle),
    })
  }
  return arr
})
</script>

<style scoped>
.speed-gauge {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.gauge-svg {
  width: 200px;
  height: 120px;
}

.gauge-bg {
  stroke: var(--border-color);
}

.gauge-value {
  stroke: var(--primary);
  transition: stroke-dashoffset 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}

.gauge-min,
.gauge-max {
  font-size: 10px;
  fill: var(--text-tertiary);
}

.gauge-speed {
  font-size: 28px;
  font-weight: 700;
  fill: var(--text-primary);
}

.gauge-unit {
  font-size: 11px;
  fill: var(--text-tertiary);
}

.gauge-label {
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 500;
}
</style>
