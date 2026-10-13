import asyncio

from veydeck_suite_runtime import (
    OperationEvent,
    OperationImpact,
    OperationOutcome,
    RuntimeClient,
)


async def main() -> None:
    client = await RuntimeClient.from_environment("operation-logs.write")
    try:
        await client.submit_operation_event(
            OperationEvent(
                event_code="task_submitted",
                zh_cn="提交任务",
                en_us="Submit task",
                outcome=OperationOutcome.SUCCESS,
                impact=OperationImpact.INFO,
            )
        )
    finally:
        await client.aclose()


asyncio.run(main())
