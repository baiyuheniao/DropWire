<template>
  <div class="receive-view">
    <div class="placeholder-card">
      <div class="icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
          <polyline points="7 10 12 15 17 10" />
          <line x1="12" y1="15" x2="12" y2="3" />
        </svg>
      </div>
      <h2>接收文件</h2>
      <p>该功能正在开发中，敬请期待。</p>
    </div>
  </div>
</template>

<style scoped>
.receive-view {
  max-width: 720px;
  margin: 0 auto;
  padding: 80px 20px;
}

.placeholder-card {
  text-align: center;
  padding: 60px 40px;
  border-radius: 16px;
  background: #fff;
  border: 1px solid #e5e7eb;
}

.icon {
  width: 64px;
  height: 64px;
  margin: 0 auto 20px;
  border-radius: 50%;
  background: #eff6ff;
  color: #3b82f6;
  display: grid;
  place-items: center;
}

.icon svg {
  width: 28px;
  height: 28px;
}

h2 {
  font-size: 18px;
  font-weight: 600;
  color: #1f2937;
  margin-bottom: 8px;
}

p {
  font-size: 14px;
  color: #6b7280;
}
</style>
