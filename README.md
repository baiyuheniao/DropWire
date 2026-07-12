# DropWire

> 🔥 Trae 创造力大赛参赛作品 —— 局域网高速文件传输工具

![DropWire Demo](https://trae-api-cn.mchost.guru/api/ide/v1/text_to_image?prompt=Modern%20dark%20theme%20file%20transfer%20application%20interface%20showing%20upload%20progress%2C%20devices%20list%2C%20speed%20gauges%2C%20professional%20UI%20design%2C%20blue%20accent%20color&image_size=landscape_16_9)

## 📋 项目概述

DropWire 是一款面向局域网环境的高速文件传输工具，采用 Vue 3 + Rust 全栈技术栈，实现无需公网依赖、安全加密的文件互传体验。

## ✨ 核心特性

| 特性 | 描述 |
|------|------|
| **高速传输** | 充分利用局域网带宽，传输速度接近物理网络上限 |
| **分片并发上传** | 2MB 分片 + 3路并发，断点续传支持 |
| **端到端加密** | AES-256-GCM 加密，密码仅存于内存，页面关闭即清除 |
| **文件完整性校验** | 支持 SHA-256/MD5/CRC32 等多种哈希算法 |
| **设备自动发现** | mDNS + UDP 广播，自动扫描局域网设备 |
| **实时进度推送** | WebSocket 实时推送上传/下载进度 |
| **网络诊断工具** | 内/公网测速、局域网拓扑分析、速度可视化仪表盘 |
| **安全存储** | Argon2 密码哈希存储，Bearer Token 服务端鉴权 |

## 🎯 技术亮点

### 1. 前后端分离架构

- **前端**: Vue 3 + TypeScript + Vite，组件化开发，响应式设计
- **后端**: Rust + axum 0.7，高性能异步运行时，内存安全保障
- **通信**: HTTP/1.1 (分片上传) + WebSocket (实时推送)

### 2. 安全加固体系

- **密码安全**: Argon2 算法哈希存储，避免明文存储
- **服务端鉴权**: Bearer Token 中间件保护敏感路由
- **路径遍历防护**: 路径规范化 + 组件检查，过滤危险路径
- **分片完整性校验**: 上传时计算分片哈希，合并前验证所有分片
- **CORS 收窄**: 根据环境变量动态配置允许的源

### 3. 性能优化

- **大文件流式加密**: Web Worker 分片加密，避免浏览器 OOM
- **异步并发控制**: tokio::sync::Mutex 替代 std::sync::Mutex
- **传输速度可视化**: 半圆弧仪表盘 + 进度条直观展示

### 4. 用户体验

- **三页签设计**: 发送、接收、设置，职责清晰
- **暗黑模式**: 完整的主题系统，支持浅色/深色切换
- **响应式布局**: 适配桌面端和移动端
- **文件预览**: 支持图片、文本等常见格式在线预览

## 🛠️ 技术栈

| 层 | 技术 | 版本 |
|----|------|------|
| 前端框架 | Vue | 3 |
| 前端语言 | TypeScript | - |
| 构建工具 | Vite | 5 |
| HTTP 客户端 | Axios | - |
| 后端框架 | axum | 0.7 |
| 后端语言 | Rust | 1.75+ |
| 异步运行时 | tokio | 1 |
| WebSocket | tokio-tungstenite | - |
| 加密算法 | AES-256-GCM / PBKDF2-SHA-256 | - |
| 密码哈希 | Argon2 | - |

## 🏗️ 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                        Browser                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐ │
│  │ SendView │  │ReceiveView│  │Settings  │  │AccountModal│ │
│  └────┬─────┘  └────┬──────┘  └────┬──────┘  └─────┬──────┘ │
│       │             │              │               │       │
│  ┌────▼─────┐  ┌────▼──────┐  ┌────▼──────┐           │    │
│  │useUpload │  │useDownload│  │useNetwork │           │    │
│  │useCrypto │  │useHash    │  │useDevices │           │    │
│  └────┬─────┘  └────┬──────┘  └────┬──────┘           │    │
└───────┼─────────────┼──────────────┼───────────────────┼────┘
        │             │              │                   │
        │ HTTP        │ HTTP         │ HTTP              │ HTTP
        │ WebSocket   │ WebSocket    │ WebSocket         │ WebSocket
┌───────▼─────────────▼──────────────▼───────────────────▼────┐
│                     DropWire Backend (Rust)                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐   │
│  │  upload  │  │   auth   │  │  network │  │ discovery  │   │
│  │  routes  │  │  routes  │  │  routes  │  │  module   │   │
│  └────┬─────┘  └────┬──────┘  └────┬──────┘  └─────┬──────┘   │
│       │             │              │               │         │
│       ▼             ▼              ▼               ▼         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  AppState                            │   │
│  │  uploads Map + broadcast Sender + devices Registry  │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 📁 目录结构

```
DropWire/
├── backend/
│   ├── src/
│   │   ├── main.rs           # 路由注册、CORS、服务启动
│   │   ├── state.rs          # AppState：全局状态管理
│   │   ├── discovery.rs      # mDNS + UDP 设备发现
│   │   ├── models/mod.rs     # 数据模型定义
│   │   └── routes/
│   │       ├── upload.rs     # 分片上传、合并、下载、文件列表
│   │       ├── auth.rs       # 注册、登录、Profile 更新
│   │       ├── auth_middleware.rs  # Bearer Token 鉴权
│   │       ├── network.rs    # 网络测速、状态查询、延迟测量
│   │       ├── ws.rs         # WebSocket 进度推送
│   │       └── info.rs       # 服务器信息
│   ├── Cargo.toml
│   └── Cargo.lock
├── frontend/
│   ├── src/
│   │   ├── App.vue           # 根组件，主题系统，状态管理
│   │   ├── main.ts           # 入口文件
│   │   ├── components/
│   │   │   ├── SendView.vue          # 发送页面
│   │   │   ├── ReceiveView.vue       # 接收页面（含文件校验）
│   │   │   ├── SettingsView.vue      # 设置页面（含网络诊断）
│   │   │   ├── FileUpload.vue        # 文件上传组件
│   │   │   ├── AccountModal.vue      # 账户弹窗
│   │   │   ├── SpeedGauge.vue        # 速度仪表盘组件
│   │   │   ├── NetworkTestPanel.vue  # 网络测试面板
│   │   │   ├── NetworkTopologyChart.vue  # 局域网拓扑图
│   │   │   └── DeviceList.vue        # 设备列表
│   │   ├── composables/
│   │   │   ├── useUpload.ts          # 分片上传逻辑
│   │   │   ├── useDownload.ts        # 文件下载逻辑
│   │   │   ├── useCrypto.ts          # AES-256-GCM 加密
│   │   │   ├── useHash.ts            # SHA/MD5/CRC32 哈希计算
│   │   │   ├── useWebSocket.ts       # WebSocket 连接管理
│   │   │   ├── useNetworkSpeed.ts    # 实时速度追踪
│   │   │   ├── useNetworkTest.ts     # 网络测试功能
│   │   │   ├── useAuth.ts            # 用户认证管理
│   │   │   ├── useDevices.ts         # 设备发现与管理
│   │   │   └── useSettings.ts        # 设置管理
│   │   └── workers/
│   │       └── crypto.worker.ts      # 加密 Worker
│   ├── vite.config.ts
│   ├── tsconfig.json
│   └── package.json
├── README.md
└── .gitignore
```

## 🚀 快速开始

### 前置要求

- **后端**: Rust 1.75+
- **前端**: Node 18+ 和 pnpm

### 启动步骤

```bash
# 1. 启动后端
cd backend
cargo run
# 后端运行在 http://localhost:3000

# 2. 启动前端
cd frontend
pnpm install
pnpm dev
# 前端运行在 http://localhost:5173
```

### 访问方式

- 本机访问：`http://localhost:5173`
- 局域网其他设备访问：`http://<本机IP>:5173`

## 🔧 配置项

| 位置 | 变量 | 默认值 | 说明 |
|------|------|--------|------|
| `useUpload.ts` | `CHUNK_SIZE` | 2 MB | 单片大小 |
| `useUpload.ts` | `CHUNK_CONCURRENCY` | 3 | 并发上传数 |
| `main.rs` | `PORT` | 3000 | 后端监听端口 |
| `main.rs` | `ALLOWED_ORIGINS` | - | 允许的 CORS 源 |

## 📊 功能演示

### 1. 文件发送
- 选择文件或拖拽上传
- 设置加密密码（可选）
- 选择校验算法
- 实时显示上传进度

### 2. 文件接收
- 查看可接收文件列表
- 预览、下载文件
- 文件完整性校验（支持 SHA-256/MD5/CRC32）

### 3. 网络诊断
- 局域网设备发现与拓扑图
- 内网测速（上行/下行速度可视化）
- 公网测速（Cloudflare Speed Test）
- 实时速度指示器

### 4. 用户认证
- 注册/登录账户
- Bearer Token 鉴权
- 密码哈希安全存储

## 🔒 安全特性

1. **密码安全**: 使用 Argon2 算法进行密码哈希，不存储明文密码
2. **传输加密**: 文件上传支持 AES-256-GCM 端到端加密
3. **服务端鉴权**: 所有敏感接口需携带有效的 Bearer Token
4. **路径遍历防护**: 上传合并时对文件路径进行规范化和安全检查
5. **分片完整性**: 每个分片计算 SHA-256 哈希，合并前验证所有分片
6. **CORS 限制**: 生产环境可通过环境变量限制允许的源

## 📈 性能优化

1. **大文件处理**: Web Worker 分片加密，避免主线程阻塞和内存溢出
2. **异步并发**: 使用 tokio 异步运行时和 tokio::sync::Mutex
3. **缓存策略**: 合理的浏览器缓存和服务端缓存
4. **按需加载**: 组件和资源按需加载

## 📝 License

MIT License

## 🤝 贡献

欢迎提交 Issue 和 PR！

---

> 💡 **Trae 创造力大赛参赛作品** —— 感谢您的关注！
