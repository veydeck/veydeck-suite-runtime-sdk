# Python Runtime SDK

通过 `RuntimeClient.from_environment()` 读取 Agent 注入的 v1 Runtime 描述，并使用 `start_workload()`、`start_capture()`、`finish_capture()` 管理多端点工作负载与整实例取证，使用 `submit_operation_event()` 提交语义操作事件。

用户请求触发事件时，使用 `operation_context_from_headers(request.headers)` 读取
`x-veydeck-operation-context`，并写入事件的 `operation_context_id`。SDK 不接受
用户名或客户端 IP，由 Agent 根据上下文恢复可信用户身份。
