# DropWire 技术亮点与功能详解

> 🔥 Trae 创造力大赛参赛作品

---

## 🎯 技术亮点

### 1. 安全加固体系

#### 1.1 密码哈希存储
- **问题**: 原始代码将密码原文明文写入 localStorage
- **解决方案**: 使用 Argon2 算法对密码进行哈希处理
- **实现**: 
  - 后端 [auth.rs](file:///workspace/DropWire/backend/src/routes/auth.rs) 使用 `argon2::password_hash` 进行密码哈希
  - 前端 [useAuth.ts](file:///workspace/DropWire/frontend/src/composables/useAuth.ts) 仅在内存中保留 token，页面卸载立即清除

#### 1.2 服务端鉴权
- **问题**: `/files` 接口返回全部文件，仅在前端按 receiver 过滤
- **解决方案**: 基于 Bearer Token 的认证中间件
- **实现**: 
  - [auth_middleware.rs](file:///workspace/DropWire/backend/src/routes/auth_middleware.rs) 实现 `require_auth` 中间件
  - 保护 `/upload/chunk`、`/upload/merge`、`/files`、`/download/*` 等敏感路由
  - 文件访问控制确保用户只能查看自己有权限的文件

#### 1.3 路径遍历防护
- **问题**: `/upload/merge` 中 `output_dir.join(&req.filename)` 未净化文件名
- **解决方案**: 路径规范化 + 组件检查
- **实现**: 在 [upload.rs](file:///workspace/DropWire/backend/src/routes/upload.rs) 中使用 `canonicalize` 校验路径安全性

#### 1.4 分片完整性校验
- **问题**: 合并不检查所有分片是否到齐，缺片时直接 500
- **解决方案**: 上传时计算分片哈希，合并前验证所有分片存在及哈希匹配
- **实现**: 
  - 前端 [useUpload.ts](file:///workspace/DropWire/frontend/src/composables/useUpload.ts) 计算每个分片的 SHA-256 哈希
  - 后端合并前验证所有 `.chunk` 文件存在并校验哈希
  - 最终文件生成 SHA-256 校验值

#### 1.5 CORS 配置收窄
- **问题**: `allow_origin(Any) + allow_methods(Any) + allow_headers(Any)`
- **解决方案**: 根据环境变量动态配置允许的源
- **实现**: 在 [main.rs](file:///workspace/DropWire/backend/src/main.rs) 中通过 `ALLOWED_ORIGINS` 环境变量限制 CORS

### 2. 性能优化

#### 2.1 大文件流式加密
- **问题**: 使用 `file.arrayBuffer()` 读取整个文件，大文件导致 OOM
- **解决方案**: Web Worker 分片加密
- **实现**: 
  - [crypto.worker.ts](file:///workspace/DropWire/frontend/src/workers/crypto.worker.ts) 在独立线程中执行加密
  - 支持加密进度反馈
  - 避免主线程阻塞和内存溢出

#### 2.2 异步并发控制
- **问题**: `std::sync::Mutex` 在高并发下可能阻塞 async executor
- **解决方案**: 使用 `tokio::sync::Mutex`
- **实现**: 在 [state.rs](file:///workspace/DropWire/backend/src/state.rs) 中使用 tokio 的异步互斥锁

#### 2.3 传输速度可视化
- **问题**: 测速结果仅文本展示，不够直观
- **解决方案**: 半圆弧仪表盘 + 进度条
- **实现**: 
  - [SpeedGauge.vue](file:///workspace/DropWire/frontend/src/components/SpeedGauge.vue) 实现速度仪表盘组件
  - 使用 SVG 绘制半圆弧，动态显示速度值
  - 支持动画过渡效果

### 3. 网络诊断功能

#### 3.1 局域网拓扑分析
- **实现**: [NetworkTopologyChart.vue](file:///workspace/DropWire/frontend/src/components/NetworkTopologyChart.vue)
- **功能**: 获取网络状态信息，生成并展示局域网设备拓扑图

#### 3.2 内/公网测速
- **实现**: 
  - 后端 [network.rs](file:///workspace/DropWire/backend/src/routes/network.rs) 提供测速 API
  - 前端 [useNetworkTest.ts](file:///workspace/DropWire/frontend/src/composables/useNetworkTest.ts) 封装测速逻辑
- **功能**: 支持 1MB/10MB/50MB 测试文件，测量上行和下行速度

#### 3.3 实时速度追踪
- **实现**: [useNetworkSpeed.ts](file:///workspace/DropWire/frontend/src/composables/useNetworkSpeed.ts)
- **功能**: 全局追踪上传/下载速度，用于任务栏指示器

---

## ✨ 核心功能

### 1. 文件发送
- 拖拽或点击选择文件上传
- 支持 AES-256-GCM 加密发送
- 可选校验算法（SHA-256/MD5/CRC32）
- 实时上传进度显示
- 3路并发上传，断点续传支持

### 2. 文件接收
- 查看可接收文件列表
- 文件预览（图片、文本等）
- 文件下载，显示速度与剩余时间
- 文件完整性校验（可折叠）

### 3. 网络诊断
- 网络状态概览卡片（默认折叠）
- 内网测速（可视化仪表盘）
- 公网测速（Cloudflare Speed Test）
- 局域网设备拓扑图

### 4. 用户认证
- 注册/登录账户
- 密码哈希安全存储
- Bearer Token 鉴权
- 账户信息管理

---

## 🔧 技术实现细节

### 上传流程

```
1. 前端用 Blob.slice 将文件切成 2MB 分片
2. 生成 upload_id（加密上传使用 UUID，非加密使用文件指纹）
3. 最多 3 路并发 POST /upload/chunk，每片携带 upload_id/filename/chunk_index/total_chunks
4. 后端每收一片写入 temp_chunks/<upload_id>/<index>.chunk
5. 通过 broadcast channel 向所有 WS 客户端推送最新进度
6. 全部分片上传完后，前端 POST /upload/merge
7. 后端按序合并到 uploads/<filename> 并清理临时目录
8. 合并完成再推一条 status: Completed 的 WS 消息
```

### 加密流程

```
1. 用户输入加密密码
2. 使用 PBKDF2-SHA-256 生成加密密钥（10万轮）
3. 生成随机 salt 和 IV
4. 使用 AES-256-GCM 加密文件内容
5. 将 salt、IV、加密后的数据一起上传
6. 接收方使用相同密码解密
```

### 设备发现流程

```
1. 后端启动 UDP 广播和 mDNS 服务
2. 定期发送设备信息到局域网
3. 监听其他设备的广播消息
4. 维护在线设备列表
5. 通过 WebSocket 实时推送到前端
```

---

## 📊 数据安全

### 加密算法
- **密钥派生**: PBKDF2-SHA-256（10万轮）
- **对称加密**: AES-256-GCM
- **密码哈希**: Argon2
- **文件校验**: SHA-256/MD5/CRC32

### 安全措施
- 密码不存储明文，仅存储 Argon2 哈希
- Token 存储在内存中，页面关闭即清除
- 敏感接口需 Bearer Token 鉴权
- 路径遍历防护
- CORS 限制
- 分片完整性校验

---

## 🎨 用户体验

### 界面设计
- 三页签设计：发送、接收、设置
- 暗黑/浅色主题切换
- 响应式布局，适配移动端
- 速度可视化仪表盘
- 网络诊断折叠面板
- 文件校验折叠区域

### 交互体验
- 拖拽上传
- 实时进度推送
- 传输速度显示
- 下载剩余时间估算
- 文件预览
- 历史记录

---

## 📈 性能指标

### 上传性能
- 分片大小: 2MB
- 并发数: 3
- 支持断点续传

### 加密性能
- Web Worker 异步加密
- 避免主线程阻塞
- 大文件内存友好

### 网络性能
- WebSocket 实时推送
- 传输速度可视化
- 内/公网测速

---

## 🚀 部署方案

### 开发环境
```bash
# 后端
cd backend
cargo run

# 前端
cd frontend
pnpm install
pnpm dev
```

### 生产环境
```bash
# 后端编译
cd backend
cargo build --release

# 前端构建
cd frontend
pnpm install
pnpm build

# 环境变量
export PORT=3000
export ALLOWED_ORIGINS=https://your-domain.com
```

---

## 📝 License

MIT License

---

> 💡 **Trae 创造力大赛参赛作品** —— DropWire 局域网高速文件传输工具
