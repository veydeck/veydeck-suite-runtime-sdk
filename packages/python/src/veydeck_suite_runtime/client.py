"""Runtime 描述驱动的异步 Agent 客户端。"""

import asyncio
import os
import ssl
from pathlib import Path
from typing import Any

import httpx

from .descriptor import RuntimeDescriptor
from .errors import AgentError, InvalidDescriptor
from .operation_logs import OperationEvent
from .types import (
    CaptureEndpoint,
    StartCaptureResponse,
    StartWorkloadRequest,
    StartWorkloadResponse,
    WorkloadSummary,
    WorkloadTransport,
)

DEFAULT_RUNTIME_PATH = "/run/veydeck-agent/runtime.json"
OPERATION_EVENT_PATH = "/api/v1/agent/suite-runtime/operation-events"
MAX_ATTEMPTS = 3
WORKLOADS_PATH = "/api/v1/agent/suite-runtime/workloads"


class RuntimeClient:
    """持有实例令牌和拓扑专属传输的 Agent 客户端。"""

    def __init__(
        self,
        descriptor: RuntimeDescriptor,
        http: httpx.AsyncClient,
        token: str,
    ) -> None:
        self.descriptor = descriptor
        self._http = http
        self._token = token

    @classmethod
    async def from_environment(cls, required_capability: str) -> "RuntimeClient":
        path = Path(os.environ.get("VEYDECK_AGENT_RUNTIME", DEFAULT_RUNTIME_PATH))
        return await cls.from_path(path, required_capability)

    @classmethod
    async def from_path(cls, path: Path, required_capability: str) -> "RuntimeClient":
        descriptor = RuntimeDescriptor.load(path)
        suite_id = os.environ.get("VEYDECK_SUITE_ID")
        instance_id = os.environ.get("VEYDECK_SUITE_INSTANCE_ID")
        if suite_id is not None and instance_id is not None:
            descriptor.require_identity(suite_id, instance_id)
        descriptor.require_capability(required_capability)
        token = descriptor.token_path.read_text(encoding="utf-8").strip()
        if not token:
            raise InvalidDescriptor("runtime token is empty")
        endpoint = descriptor.endpoint
        if endpoint.kind == "unix":
            if endpoint.socket_path is None:
                raise InvalidDescriptor("Unix socket path is missing")
            transport = httpx.AsyncHTTPTransport(uds=str(endpoint.socket_path))
        else:
            if not all((endpoint.ca_path, endpoint.client_cert_path, endpoint.client_key_path)):
                raise InvalidDescriptor("mTLS credential paths are missing")
            context = ssl.create_default_context(cafile=str(endpoint.ca_path))
            context.load_cert_chain(str(endpoint.client_cert_path), str(endpoint.client_key_path))
            transport = httpx.AsyncHTTPTransport(verify=context)
        http = httpx.AsyncClient(transport=transport, base_url=endpoint.base_url)
        return cls(descriptor, http, token)

    async def request(self, method: str, path: str, **kwargs: Any) -> httpx.Response:
        response = await self._http.request(method, path, headers=self._headers(), **kwargs)
        if not response.is_success:
            raise self._agent_error(response)
        return response

    async def submit_operation_event(self, event: OperationEvent) -> None:
        payload = event.to_dict()
        for attempt in range(MAX_ATTEMPTS):
            try:
                response = await self._http.post(
                    OPERATION_EVENT_PATH, headers=self._headers(), json=payload
                )
            except httpx.TransportError:
                if attempt + 1 == MAX_ATTEMPTS:
                    raise
            else:
                if response.is_success:
                    return
                if response.is_client_error or attempt + 1 == MAX_ATTEMPTS:
                    raise self._agent_error(response)
            await asyncio.sleep(0.1 * (2**attempt))

    async def start_workload(self, payload: StartWorkloadRequest) -> StartWorkloadResponse:
        response = await self.request("POST", WORKLOADS_PATH, json=payload.to_dict())
        body = response.json()
        return StartWorkloadResponse(
            workload_id=body["workloadId"],
            container_id=body["containerId"],
            status=body["status"],
        )

    async def list_workloads(self) -> list[WorkloadSummary]:
        response = await self.request("GET", WORKLOADS_PATH)
        return [
            WorkloadSummary(
                workload_id=item["workloadId"],
                suite_id=item["suiteId"],
                suite_instance_id=item["suiteInstanceId"],
                workload_kind=item["workloadKind"],
                name=item["name"],
                image=item["image"],
                status=item["status"],
                container_id=item["containerId"],
            )
            for item in response.json()
        ]

    async def delete_workload(self, workload_id: str) -> None:
        await self.request("DELETE", f"{WORKLOADS_PATH}/{workload_id}")

    async def start_capture(self, workload_id: str) -> StartCaptureResponse:
        response = await self.request("POST", f"{WORKLOADS_PATH}/{workload_id}/captures")
        body = response.json()
        return StartCaptureResponse(
            capture_id=body["captureId"],
            status=body["status"],
            endpoints=tuple(
                CaptureEndpoint(
                    endpoint_id=item["endpointId"],
                    host_port=item["hostPort"],
                    protocol=WorkloadTransport(item["protocol"]),
                )
                for item in body["endpoints"]
            ),
        )

    async def finish_capture(self, workload_id: str, capture_id: str) -> bytes:
        response = await self.request(
            "POST",
            f"{WORKLOADS_PATH}/{workload_id}/captures/{capture_id}/finish",
        )
        return response.content

    async def aclose(self) -> None:
        await self._http.aclose()

    def _headers(self) -> dict[str, str]:
        return {"Authorization": f"Bearer {self._token}"}

    @staticmethod
    def _agent_error(response: httpx.Response) -> AgentError:
        message = response.text.replace("\r", " ").replace("\n", " ")[:512]
        return AgentError(response.status_code, message)
