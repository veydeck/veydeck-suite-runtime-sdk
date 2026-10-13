# Veydeck Suite Runtime SDK

Veydeck 套件后端访问 Agent 受信运行时 API 的跨语言 SDK。仓库同时维护 Rust crate、Python package 和共享 wire contract fixture。

SDK 只面向套件后端，负责读取含 `platformVersion` 的 v1 `VEYDECK_AGENT_RUNTIME`、校验实例能力、连接 UDS 或 mTLS HTTPS、携带实例令牌、管理具名 TCP/UDP 多端点工作负载、执行整工作负载抓包并提交语义操作事件。浏览器 iframe Bridge 继续由 `@veydeck/suite-sdk` 提供，前端不得读取 Agent 令牌。

## 目录

- `contracts/`：跨语言协议 Schema 与契约一致性测试样例。
- `packages/rust/`：Rust Runtime SDK。
- `packages/python/`：Python Runtime SDK。
