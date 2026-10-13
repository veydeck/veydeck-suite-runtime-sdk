"""操作事件公开类型。"""

from dataclasses import dataclass
from enum import StrEnum
from typing import Any, TypeAlias


class OperationOutcome(StrEnum):
    SUCCESS = "success"
    FAILURE = "failure"
    PARTIAL = "partial"
    CANCELED = "canceled"
    TIMED_OUT = "timedOut"


class OperationImpact(StrEnum):
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"


@dataclass(frozen=True)
class OperationTarget:
    kind: str
    id: str
    display_name: str | None = None
    ownership: str | None = None


ParameterValue: TypeAlias = str | int | float | bool


class WorkloadTransport(StrEnum):
    TCP = "tcp"
    UDP = "udp"


@dataclass(frozen=True)
class WorkloadPort:
    endpoint_id: str
    host_port: int
    container_port: int
    protocol: WorkloadTransport

    def to_dict(self) -> dict[str, Any]:
        return {
            "endpointId": self.endpoint_id,
            "hostPort": self.host_port,
            "containerPort": self.container_port,
            "protocol": self.protocol.value,
        }


@dataclass(frozen=True)
class WorkloadResources:
    memory_mb: int | None = None
    cpu_shares: int | None = None

    def to_dict(self) -> dict[str, Any]:
        return {"memoryMb": self.memory_mb, "cpuShares": self.cpu_shares}


@dataclass(frozen=True)
class StartWorkloadRequest:
    workload_kind: str
    workload_name: str
    image: str
    ports: tuple[WorkloadPort, ...] = ()
    env: dict[str, Any] | None = None
    config_json: Any = None
    resources: WorkloadResources = WorkloadResources()

    def to_dict(self) -> dict[str, Any]:
        return {
            "workloadKind": self.workload_kind,
            "workloadName": self.workload_name,
            "image": self.image,
            "ports": [port.to_dict() for port in self.ports],
            "env": self.env or {},
            "configJson": self.config_json,
            "resources": self.resources.to_dict(),
        }


@dataclass(frozen=True)
class StartWorkloadResponse:
    workload_id: str
    container_id: str
    status: str


@dataclass(frozen=True)
class WorkloadSummary:
    workload_id: str
    suite_id: str
    suite_instance_id: str
    workload_kind: str
    name: str
    image: str
    status: str
    container_id: str


@dataclass(frozen=True)
class CaptureEndpoint:
    endpoint_id: str
    host_port: int
    protocol: WorkloadTransport


@dataclass(frozen=True)
class StartCaptureResponse:
    capture_id: str
    status: str
    endpoints: tuple[CaptureEndpoint, ...]
