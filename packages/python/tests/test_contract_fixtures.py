import json
import re
from pathlib import Path

import pytest

from veydeck_suite_runtime import OperationEvent, RuntimeDescriptor
from veydeck_suite_runtime.errors import InvalidDescriptor
from veydeck_suite_runtime.types import OperationImpact, OperationOutcome, OperationTarget

FIXTURES = Path(__file__).parents[3] / "contracts" / "fixtures"
RUNTIME_SCHEMA = FIXTURES.parent / "schemas" / "runtime-descriptor.schema.json"


def test_shared_runtime_fixture() -> None:
    descriptor = RuntimeDescriptor.load(FIXTURES / "runtime" / "uds-valid.json")
    assert descriptor.schema_version == 1
    descriptor.require_identity("veydeck.host-scanner", "instance-1")


@pytest.mark.parametrize(
    ("platform_version", "accepted"),
    [
        ("0.1.0-alpha.1", True),
        ("1.2.3+build.7", True),
        ("1.2.3-invalid@@", False),
        ("01.2.3", False),
        ("1.2.3-alpha..1", False),
        ("1.2", False),
    ],
)
def test_runtime_schema_uses_strict_semver(platform_version: str, accepted: bool) -> None:
    schema = json.loads(RUNTIME_SCHEMA.read_text(encoding="utf-8"))
    pattern = schema["properties"]["platformVersion"]["pattern"]
    assert (re.fullmatch(pattern, platform_version) is not None) is accepted


@pytest.mark.parametrize(
    "platform_version",
    ["1.2.3-invalid@@", "01.2.3", "1.2.3-alpha..1", "1.2"],
)
def test_runtime_descriptor_rejects_invalid_semver(platform_version: str) -> None:
    descriptor = RuntimeDescriptor(
        schema_version=1,
        platform_version=platform_version,
        suite_id="veydeck.host-scanner",
        instance_id="instance-1",
        endpoint=RuntimeDescriptor.load(FIXTURES / "runtime" / "uds-valid.json").endpoint,
        token_path=Path("/run/veydeck-agent/access-token"),
        capabilities=("operation-logs.write",),
    )
    with pytest.raises(InvalidDescriptor):
        descriptor.validate()


def test_shared_event_fixture() -> None:
    value = json.loads((FIXTURES / "operation-events" / "success.json").read_text())
    event = OperationEvent(
        event_id=value["eventId"],
        operation_context_id=value["operationContextId"],
        event_code=value["eventCode"],
        zh_cn=value["eventLabel"]["zhCn"],
        en_us=value["eventLabel"]["enUs"],
        outcome=OperationOutcome(value["outcome"]),
        impact=OperationImpact(value["impact"]),
        target=OperationTarget(kind=value["target"]["kind"], id=value["target"]["id"]),
        task_id=value["taskId"],
        parameters=value["parameters"],
    )
    assert event.to_dict()["eventCode"] == "scan_submitted"
