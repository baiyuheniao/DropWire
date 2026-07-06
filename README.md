# DropWire

局域网文件传输工具。前端分片上传（支持断点续传、可选端到端加密），Rust 后端接收合并，WebSocket 实时推送进度，并提供设备发现、账户认证、文件下载与网络诊断能力。

## 技术栈

| 层 | 技术 |
|----|------|
| 前端 | Vue 3 · TypeScript · Vite · Axios · qrcode.vue |
| 后端 | Rust · axum 0.7 · tokio · serde · mdns-sd · argon2 |
| 通信 | HTTP multipart (分片上传) · WebSocket (进度推送) · UDP 广播 + mDNS (设备发现) |

## API 概览

**受保护路由**（需登录，携带 `Authorization` 头）：

```
POST /upload/chunk              分片上传 (multipart，单片限 8 MB)
GET  /upload/status/:upload_id  查询已收到的分片（断点续传）
POST /upload/merge              合并分片
GET  /files                     文件列表
POST /files/received            标记文件已接收
GET  /download/*path            下载文件
GET/POST /device                查看 / 更新本机设备信息
GET  /devices                   局域网设备列表
POST /auth/profile              更新个人资料
```

**公开路由**：

```
GET  /server-info               服务器信息（IP、下载前缀等）
POST /auth/register|login|logout
GET  /ws                        WebSocket，后端主动推送 UploadProgress JSON
POST /network/speed-test/upload     上传测速 (限 128 MB)
GET  /network/speed-test/download   下载测速
GET  /network/speed-test/public     公网测速
GET  /network/latency               延迟测量
GET  /network/status                网络状态
```

## 上传流程

1. 前端用 `Blob.slice` 将文件切成 2 MB 分片。`upload_id` 由文件指纹（名称/大小/修改时间的 djb2 哈希）派生以支持断点续传；加密上传因密文每次不同，改用随机 UUID
2. 若设置了密码，先在前端用 PBKDF2 + AES-256-GCM 加密整个文件（端到端加密）
3. 先查询 `/upload/status/:upload_id` 跳过服务器已有的分片，其余分片最多 3 路并发 POST，每片附带 SHA-256 校验和，失败自动重试（最多 3 次，指数退避）；文件级并发为 2
4. 后端每收一片写入 `temp_chunks/<upload_id>/<index>.chunk`，并通过 broadcast channel 向所有 WS 客户端推送最新进度
5. 全部分片完成后，前端 POST `/upload/merge`（可携带发送人/接收人/备注/过期时间/加密元数据），后端按序合并到 `uploads/`，元数据写入 `uploads_meta/`，清理临时目录
6. 合并完成再推一条 `status: Completed` 的 WS 消息

**状态流转**（后端 WS 消息）

```
Uploading → Merging → Completed
                    ↘ Failed(reason)
```

前端任务状态为 `pending → uploading → merging → done / error`。

## 目录结构

```
DropWire/
├── backend/
│   └── src/
│       ├── main.rs              # 路由注册、CORS、认证中间件、服务启动
│       ├── state.rs             # AppState：uploads Map + broadcast Sender + 会话
│       ├── discovery.rs         # UDP 广播 + mDNS 设备发现
│       ├── models/mod.rs        # ApiResponse, MergeRequest 等
│       └── routes/
│           ├── upload.rs        # 分片上传 / 合并 / 文件列表 / 下载
│           ├── auth.rs          # 注册 / 登录 / 登出 / 资料 (argon2 哈希)
│           ├── auth_middleware.rs
│           ├── info.rs          # /server-info
│           ├── network.rs       # 测速 / 延迟 / 网络状态
│           └── ws.rs            # GET /ws
└── frontend/
    └── src/
        ├── composables/         # useUpload, useDownload, useWebSocket,
        │                        # useCrypto, useAuth, useDevices, useHistory,
        │                        # useNetworkTest, useSettings 等
        ├── components/          # SendView, ReceiveView, SettingsView,
        │                        # FileUpload, DeviceList, HistoryModal,
        │                        # NetworkTestPanel 等
        └── workers/
            └── crypto.worker.ts # 加解密 Web Worker
```

## 快速开始

```bash
# 后端 (需要 Rust 1.75+)
cd backend
cargo run

# 前端 (需要 Node 18+ 和 pnpm)
cd frontend
pnpm install
pnpm dev
```

访问 `http://localhost:5173`，局域网内其他设备访问 `http://<本机IP>:5173`。

Vite 开发服务器已将 `/upload`、`/files`、`/download`、`/server-info`、`/device(s)`、`/auth`、`/network` 和 `/ws` 代理到 `:3000`，生产部署时需自行配置反向代理（nginx 等）。

## 配置项

| 位置 | 变量 | 默认值 | 说明 |
|------|------|--------|------|
| 环境变量 | `PORT` | `3000` | 后端监听端口 (监听 `0.0.0.0`) |
| 环境变量 | `ALLOWED_ORIGINS` | 允许所有 | CORS 允许来源，逗号分隔 |
| `useUpload.ts` | `CHUNK_SIZE` | 2 MB | 单片大小 |
| `useUpload.ts` | `CHUNK_CONCURRENCY` | 3 | 分片并发上传数 |
| `useUpload.ts` | `CHUNK_RETRIES` | 3 | 单片失败重试次数 |
| `useUpload.ts` | `FILE_CONCURRENCY` | 2 | 同时上传的文件数 |
| `upload.rs` | `TEMP_DIR` | `./temp_chunks` | 分片临时目录 |
| `upload.rs` | `OUTPUT_DIR` | `./uploads` | 合并输出目录 |
| `upload.rs` | `META_DIR` | `./uploads_meta` | 文件元数据目录 |
| `discovery.rs` | `DISCOVERY_PORT` | `3001` | UDP 设备发现端口 |
| `discovery.rs` | `MDNS_SERVICE_TYPE` | `_dropwire._tcp.local.` | mDNS 服务类型 |
