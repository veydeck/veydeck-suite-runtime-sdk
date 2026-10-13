"""语义操作事件构建、序列化和校验。"""

import re
import secrets
import time
import uuid
from collections.abc import Mapping
from dataclasses import dataclass, field

from .errors import InvalidEvent
from .types import OperationImpact, OperationOutcome, OperationTarget, ParameterValue

EVENT_CODE = re.compile(r"^[a-z][a-z0-9_]{2,63}$")
SENSITIVE_KEYS = (
    "password",
    "token",
    "authorization",
    "secret",
    "cookie",
    "command",
    "environment",
)
OPERATION_CONTEXT_HEADER = "x-veydeck-operation-context"


def operation_context_from_headers(headers: Mapping[str, str]) -> str | None:
    """从套件代理请求头提取可信操作上下文 ID。"""
    value = headers.get(OPERATION_CONTEXT_HEADER)
    if value is None:
        return None
    value = value.strip()
    return value if value and len(value) <= 128 else None


def uuid7() -> uuid.UUID:
    """生成符合 RFC 9562 布局的 UUIDv7。"""
    timestamp = int(time.time() * 1000) & ((1 << 48) - 1)
    value = timestamp << 80
    value |= 0x7 << 76
    value |= secrets.randbits(12) << 64
    value |= 0b10 << 62
    value |= secrets.randbits(62)
    return uuid.UUID(int=value)


@dataclass
class OperationEvent:
    event_code: str
    zh_cn: str
    en_us: str
    outcome: OperationOutcome
    impact: OperationImpact
    event_id: str = field(default_factory=lambda: str(uuid7()))
    operation_context_id: str | None = None
    target: OperationTarget | None = None
    task_id: str | None = None
    parameters: dict[str, ParameterValue] = field(default_factory=dict)
    error_code: str | None = None
    error_summary: str | None = None

    def validate(self) -> None:
        try:
            identifier = uuid.UUID(self.event_id)
        except ValueError as error:
            raise InvalidEvent("eventId must be a UUIDv7") from error
        if identifier.version != 7:
            raise InvalidEvent("eventId must be a UUIDv7")
        if self.operation_context_id is not None and (
            not self.operation_context_id.strip() or len(self.operation_context_id) > 128
        ):
            raise InvalidEvent("operationContextId must contain 1 to 128 characters")
        if not EVENT_CODE.fullmatch(self.event_code):
            raise InvalidEvent("eventCode must use lower snake case")
        if any(not value.strip() or len(value) > 128 for value in (self.zh_cn, self.en_us)):
            raise InvalidEvent("event labels must contain 1 to 128 characters")
        if len(self.parameters) > 32 or any(
            marker in key.lower() for key in self.parameters for marker in SENSITIVE_KEYS
        ):
            raise InvalidEvent("operation parameters contain unsupported fields")

    def to_dict(self) -> dict[str, object]:
        self.validate()
        target: dict[str, object] | None = None
        if self.target is not None:
            target = {
                "kind": self.target.kind,
                "id": self.target.id,
                "displayName": self.target.display_name,
                "ownership": self.target.ownership,
            }
        return {
            "eventId": self.event_id,
            "operationContextId": self.operation_context_id,
            "eventCode": self.event_code,
            "eventLabel": {"zhCn": self.zh_cn, "enUs": self.en_us},
            "outcome": self.outcome.value,
            "impact": self.impact.value,
            "target": target,
            "taskId": self.task_id,
            "parameters": self.parameters,
            "errorCode": self.error_code[:128] if self.error_code else None,
            "errorSummary": redact_error(self.error_summary) if self.error_summary else None,
        }


def redact_error(value: str) -> str:
    sanitized = value.replace("\r", " ").replace("\n", " ")
    lowered = sanitized.lower()
    positions = [
        lowered.find(marker)
        for marker in ("bearer ", "token=", "password=")
        if lowered.find(marker) >= 0
    ]
    if positions:
        sanitized = sanitized[: min(positions)] + "[REDACTED]"
    return sanitized[:512]
