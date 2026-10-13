"""Agent 注入的套件 Runtime 描述。"""

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal

from .errors import CapabilityDenied, InvalidDescriptor

SEMVER_PATTERN = re.compile(
    r"^(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)"
    r"(?:-(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)


@dataclass(frozen=True)
class RuntimeEndpoint:
    kind: Literal["unix", "https"]
    base_url: str
    socket_path: Path | None = None
    ca_path: Path | None = None
    client_cert_path: Path | None = None
    client_key_path: Path | None = None


@dataclass(frozen=True)
class RuntimeDescriptor:
    schema_version: int
    platform_version: str
    suite_id: str
    instance_id: str
    endpoint: RuntimeEndpoint
    token_path: Path
    capabilities: tuple[str, ...]

    @classmethod
    def load(cls, path: Path) -> "RuntimeDescriptor":
        try:
            value: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
            endpoint_value = value["endpoint"]
            kind = endpoint_value["kind"]
            if kind == "unix":
                endpoint = RuntimeEndpoint(
                    kind="unix",
                    base_url=endpoint_value["baseUrl"],
                    socket_path=Path(endpoint_value["socketPath"]),
                )
            elif kind == "https":
                endpoint = RuntimeEndpoint(
                    kind="https",
                    base_url=endpoint_value["baseUrl"],
                    ca_path=Path(endpoint_value["caPath"]),
                    client_cert_path=Path(endpoint_value["clientCertPath"]),
                    client_key_path=Path(endpoint_value["clientKeyPath"]),
                )
            else:
                raise InvalidDescriptor(f"unsupported endpoint kind: {kind}")
            descriptor = cls(
                schema_version=int(value["schemaVersion"]),
                platform_version=str(value["platformVersion"]),
                suite_id=str(value["suiteId"]),
                instance_id=str(value["instanceId"]),
                endpoint=endpoint,
                token_path=Path(value["credential"]["tokenPath"]),
                capabilities=tuple(str(item) for item in value["capabilities"]),
            )
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise InvalidDescriptor(str(error)) from error
        descriptor.validate()
        return descriptor

    def validate(self) -> None:
        if (
            self.schema_version != 1
            or SEMVER_PATTERN.fullmatch(self.platform_version) is None
            or not self.suite_id
            or not self.instance_id
        ):
            raise InvalidDescriptor("schema version and instance identity are required")
        if any(not capability.strip() for capability in self.capabilities):
            raise InvalidDescriptor("capabilities must not contain empty values")

    def require_capability(self, capability: str) -> None:
        if capability not in self.capabilities:
            raise CapabilityDenied(f"runtime capability is not granted: {capability}")

    def require_identity(self, suite_id: str, instance_id: str) -> None:
        """校验调用进程身份与 Agent 描述一致。"""
        if self.suite_id != suite_id or self.instance_id != instance_id:
            raise InvalidDescriptor("runtime descriptor identity does not match suite instance")
