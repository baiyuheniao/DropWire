# DropWire

局域网文件传输工具。前端分片上传，Rust 后端接收合并，WebSocket 实时推送进度。

## 技术栈

| 层 | 技术 |
|----|------|
| 前端 | Vue 3 · TypeScript · Vite · Axios |
| 后端 | Rust · axum 0.7 · tokio · serde |
| 通信 | HTTP multipart (分片上传) · WebSocket (进度推送) |

## 架构

```
Browser
  │
  ├─ POST /upload/chunk  (multipart, 并发 3)
  ├─ POST /upload/merge  (JSON)
  └─ WS   /ws            ← 后端主动推送 UploadProgress JSON
```

**上传流程**

1. 前端用 `Blob.slice` 将文件切成 2 MB 分片，生成 `upload_id` (UUID)
2. 最多 3 路并发 POST，每片携带 `upload_id / filename / chunk_index / total_chunks`
3. 后端每收一片写入 `temp_chunks/<upload_id>/<index>.chunk`，并通过 broadcast channel 向所有 WS 客户端推送最新进度
4. 全部分片上传完后，前端 POST `/upload/merge`，后端按序合并到 `uploads/<filename>` 并清理临时目录
5. 合并完成再推一条 `status: Completed` 的 WS 消息

**状态流转**

```
Uploading → Merging → Completed
                    ↘ Failed(reason)
```

## 目录结构

```
DropWire/
├── backend/
│   └── src/
│       ├── main.rs          # 路由注册、CORS、服务启动 (:3000)
│       ├── state.rs         # AppState：uploads Map + broadcast Sender
│       ├── models/mod.rs    # ApiResponse, MergeRequest
│       └── routes/
│           ├── upload.rs    # POST /upload/chunk  /upload/merge
│           └── ws.rs        # GET /ws
└── frontend/
    └── src/
        ├── composables/
        │   ├── useUpload.ts     # 切片 + 并发队列 + merge 请求
        │   └── useWebSocket.ts  # 单例 WS 连接 + 断线重连 (3 s)
        └── components/
            └── FileUpload.vue   # 拖拽区 + 进度卡片
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

Vite 开发服务器已将 `/upload` 和 `/ws` 代理到 `:3000`，生产部署时需自行配置反向代理（nginx 等）。

## 配置项

| 位置 | 变量 | 默认值 | 说明 |
|------|------|--------|------|
| `useUpload.ts` | `CHUNK_SIZE` | 2 MB | 单片大小 |
| `useUpload.ts` | `CONCURRENCY` | 3 | 并发上传数 |
| `upload.rs` | `TEMP_DIR` | `./temp_chunks` | 分片临时目录 |
| `upload.rs` | `OUTPUT_DIR` | `./uploads` | 合并输出目录 |
| `main.rs` | `SocketAddr` | `0.0.0.0:3000` | 后端监听地址 |
