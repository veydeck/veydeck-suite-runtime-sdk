from pathlib import Path

import httpx
import pytest

from veydeck_suite_runtime import (
    AgentError,
    InvalidEvent,
    OperationEvent,
    OperationImpact,
    OperationOutcome,
    RuntimeClient,
    RuntimeDescriptor,
    RuntimeEndpoint,
    operation_context_from_headers,
)


def event() -> OperationEvent:
    return OperationEvent(
        event_code="scan_submitted",
        zh_cn="提交扫描",
        en_us="Submit scan",
        outcome=OperationOutcome.SUCCESS,
        impact=OperationImpact.INFO,
    )


def test_rejects_sensitive_parameters() -> None:
    value = event()
    value.parameters["accessToken"] = "secret"
    with pytest.raises(InvalidEvent):
        value.to_dict()


def test_operation_context_is_copied_from_trusted_proxy_header() -> None:
    context = operation_context_from_headers({"x-veydeck-operation-context": " context-1 "})
    value = event()
    value.operation_context_id = context
    assert value.to_dict()["operationContextId"] == "context-1"


@pytest.mark.asyncio
async def test_retry_reuses_event_id() -> None:
    identifiers: list[str] = []

    def handler(request: httpx.Request) -> httpx.Response:
        identifiers.append(request.read().decode())
        status = 503 if len(identifiers) < 3 else 200
        return httpx.Response(status, request=request)

    descriptor = RuntimeDescriptor(
        schema_version=1,
        platform_version="0.1.0-alpha.1",
        suite_id="veydeck.host-scanner",
        instance_id="instance-1",
        endpoint=RuntimeEndpoint(kind="unix", base_url="http://local"),
        token_path=Path(__file__),
        capabilities=("operation-logs.write",),
    )
    http = httpx.AsyncClient(transport=httpx.MockTransport(handler), base_url="http://local")
    client = RuntimeClient(descriptor, http, "token")
    await client.submit_operation_event(event())
    assert len(identifiers) == 3
    assert identifiers[0] == identifiers[1] == identifiers[2]
    await client.aclose()


@pytest.mark.asyncio
async def test_https_mock_uses_bearer_authentication() -> None:
    requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(request)
        return httpx.Response(200, request=request)

    descriptor = RuntimeDescriptor(
        schema_version=1,
        platform_version="0.1.0-alpha.1",
        suite_id="veydeck.packet",
        instance_id="instance-2",
        endpoint=RuntimeEndpoint(kind="https", base_url="https://agent.example"),
        token_path=Path(__file__),
        capabilities=("operation-logs.write",),
    )
    http = httpx.AsyncClient(
        transport=httpx.MockTransport(handler),
        base_url=descriptor.endpoint.base_url,
    )
    client = RuntimeClient(descriptor, http, "trusted-token")
    await client.submit_operation_event(event())

    assert len(requests) == 1
    assert requests[0].url.scheme == "https"
    assert requests[0].headers["authorization"] == "Bearer trusted-token"
    await client.aclose()


@pytest.mark.asyncio
async def test_client_error_is_not_retried() -> None:
    attempts = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal attempts
        attempts += 1
        return httpx.Response(400, text="invalid event", request=request)

    descriptor = RuntimeDescriptor(
        schema_version=1,
        platform_version="0.1.0-alpha.1",
        suite_id="veydeck.host-scanner",
        instance_id="instance-1",
        endpoint=RuntimeEndpoint(kind="unix", base_url="http://local"),
        token_path=Path(__file__),
        capabilities=("operation-logs.write",),
    )
    http = httpx.AsyncClient(transport=httpx.MockTransport(handler), base_url="http://local")
    client = RuntimeClient(descriptor, http, "token")

    with pytest.raises(AgentError):
        await client.submit_operation_event(event())

    assert attempts == 1
    await client.aclose()
