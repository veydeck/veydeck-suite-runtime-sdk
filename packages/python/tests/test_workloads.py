import json
from pathlib import Path

import httpx
import pytest

from veydeck_suite_runtime import (
    RuntimeClient,
    RuntimeDescriptor,
    RuntimeEndpoint,
    StartWorkloadRequest,
    WorkloadPort,
    WorkloadResources,
    WorkloadTransport,
)


def test_workload_fixture_matches_python_wire_types() -> None:
    fixture_path = (
        Path(__file__).parents[3] / "contracts" / "fixtures" / "workloads" / "start-request.json"
    )
    expected = json.loads(fixture_path.read_text(encoding="utf-8"))
    request = StartWorkloadRequest(
        workload_kind="simulation-rule",
        workload_name="dns-decoy",
        image="guowenju/veydeck-protocol-simulation-engine:0.1.0-alpha.1",
        ports=(
            WorkloadPort("dns-tcp", 1053, 53, WorkloadTransport.TCP),
            WorkloadPort("dns-udp", 1053, 53, WorkloadTransport.UDP),
        ),
        env={},
        config_json={"protocol": "dns"},
        resources=WorkloadResources(memory_mb=256, cpu_shares=256),
    )
    assert request.to_dict() == expected


@pytest.mark.asyncio
async def test_workload_client_uses_authenticated_v1_routes_and_decodes_responses() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        path = request.url.path
        if request.method == "POST" and path.endswith("/workloads"):
            return httpx.Response(
                200,
                json={
                    "workloadId": "workload-1",
                    "containerId": "container-1",
                    "status": "running",
                },
                request=request,
            )
        if request.method == "GET" and path.endswith("/workloads"):
            return httpx.Response(
                200,
                json=[
                    {
                        "workloadId": "workload-1",
                        "suiteId": "veydeck.protocol-simulation",
                        "suiteInstanceId": "instance-1",
                        "workloadKind": "simulation-rule",
                        "name": "dns-decoy",
                        "image": "engine:alpha",
                        "status": "running",
                        "containerId": "container-1",
                    }
                ],
                request=request,
            )
        if path.endswith("/captures"):
            return httpx.Response(
                200,
                json={
                    "captureId": "capture-1",
                    "status": "capturing",
                    "endpoints": [{"endpointId": "dns-udp", "hostPort": 1053, "protocol": "udp"}],
                },
                request=request,
            )
        if path.endswith("/finish"):
            return httpx.Response(200, content=b"pcap", request=request)
        return httpx.Response(204, request=request)

    descriptor = RuntimeDescriptor(
        schema_version=1,
        platform_version="0.1.0-alpha.1",
        suite_id="veydeck.protocol-simulation",
        instance_id="instance-1",
        endpoint=RuntimeEndpoint(kind="unix", base_url="http://local"),
        token_path=Path(__file__),
        capabilities=("workloads.manage", "captures.manage"),
    )
    http = httpx.AsyncClient(transport=httpx.MockTransport(handler), base_url="http://local")
    client = RuntimeClient(descriptor, http, "trusted-token")
    payload = StartWorkloadRequest(
        workload_kind="simulation-rule",
        workload_name="dns-decoy",
        image="engine:alpha",
    )

    assert (await client.start_workload(payload)).workload_id == "workload-1"
    assert (await client.list_workloads())[0].container_id == "container-1"
    capture = await client.start_capture("workload-1")
    assert capture.endpoints[0].protocol is WorkloadTransport.UDP
    assert await client.finish_capture("workload-1", "capture-1") == b"pcap"
    await client.delete_workload("workload-1")
    await client.aclose()

    assert [(request.method, request.url.path) for request in requests] == [
        ("POST", "/api/v1/agent/suite-runtime/workloads"),
        ("GET", "/api/v1/agent/suite-runtime/workloads"),
        ("POST", "/api/v1/agent/suite-runtime/workloads/workload-1/captures"),
        (
            "POST",
            "/api/v1/agent/suite-runtime/workloads/workload-1/captures/capture-1/finish",
        ),
        ("DELETE", "/api/v1/agent/suite-runtime/workloads/workload-1"),
    ]
    assert all(request.headers["authorization"] == "Bearer trusted-token" for request in requests)
