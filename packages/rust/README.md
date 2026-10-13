# Rust Runtime SDK

使用 `RuntimeClient::from_environment` 读取 Agent 注入的 v1 运行时描述，并通过 `start_workload`、`start_capture`、`finish_capture` 管理多端点工作负载与整实例取证，通过 `submit_operation_event` 提交语义操作事件。

用户请求触发事件时，使用 `operation_context_from_header` 读取代理注入的
`x-veydeck-operation-context`，并通过构建器的 `operation_context_id` 绑定事件。
SDK 不接受用户名或客户端 IP，由 Agent 根据上下文恢复可信用户身份。
