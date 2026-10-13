# Runtime wire contract

本目录描述 Veydeck Agent 已实现的套件 Runtime v1 wire contract。`fixtures/` 中存放 Rust、Python SDK 与 Agent 共用的契约样例，用于验证各实现的序列化与反序列化结果保持一致，并与 `veydeck-contracts` 的契约一致性测试保持同步。

- `schemas/runtime-descriptor.schema.json`：套件实例身份与 Agent 端点。
- `schemas/suite-workload.schema.json`：多端点受控工作负载请求。
- `fixtures/workloads/`：跨实现共用的 workload 标准契约样例。

所有公开契约均保持 v1。alpha 阶段允许同步重写 v1，但不得引入 v2。runtime 描述中的 `platformVersion` 由 Agent 注入，供套件校验扩展资产声明的 `minVeydeckVersion`。
