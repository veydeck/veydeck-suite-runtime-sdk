"""SDK 公开错误。"""


class RuntimeSdkError(Exception):
    """所有 Runtime SDK 错误的基类。"""


class InvalidDescriptor(RuntimeSdkError):
    """Runtime 描述无效。"""


class CapabilityDenied(RuntimeSdkError):
    """实例未被授予所需能力。"""


class InvalidEvent(RuntimeSdkError):
    """操作事件不满足公开约束。"""


class AgentError(RuntimeSdkError):
    """Agent 拒绝或无法处理请求。"""

    def __init__(self, status: int, message: str) -> None:
        super().__init__(f"Agent rejected the request with HTTP {status}: {message}")
        self.status = status
        self.message = message
