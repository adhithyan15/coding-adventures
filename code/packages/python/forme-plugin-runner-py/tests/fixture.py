from __future__ import annotations

import asyncio
import sys
from collections.abc import AsyncIterator
from typing import Any

from forme_plugin_runner import StageError, define_stage, run_plugin

MODE = sys.argv[3] if len(sys.argv) > 3 else "single"
CONSUMES = "Stream<ContentNode>" if MODE in {"stream", "stream-single"} else "ContentNode"
PRODUCES = "Stream<ContentNode>" if MODE in {"stream", "single-stream"} else "ContentNode"


@define_stage(
    name="@forme/conformance",
    version="1.0.0",
    api_version=1,
    consumes=CONSUMES,
    produces=PRODUCES,
    capabilities=[
        "storage:read",
        "storage:write",
        "env:ALLOWED",
        "filesystem:user",
        "network:example.com",
        "system:time:wallclock",
        "system:shell",
    ],
)
async def fixture(input_value: Any, _config: Any, ctx: Any) -> Any:
    if MODE == "stream":

        async def passthrough() -> AsyncIterator[Any]:
            async for value in input_value:
                yield value

        return passthrough()
    if MODE == "stream-single":
        return {"values": [value async for value in input_value]}
    if MODE == "single-stream":

        async def duplicate() -> AsyncIterator[Any]:
            yield input_value
            yield {"copy": input_value}

        return duplicate()
    operation = input_value.get("operation")
    if operation == "error":
        raise StageError("FIXTURE", "fixture failed", fields={"line": 3})
    if operation == "waitForCancel":
        while True:
            ctx.cancellation.throw_if_cancelled()
            await asyncio.sleep(0.002)
    if operation == "capabilityDenied":
        return {"value": await ctx.env.get("DENIED")}
    if operation == "malformedResponse":
        return {"value": await ctx.env.get("MALFORMED")}
    if operation == "context":
        data = await ctx.storage.read("posts/a.md")
        await ctx.storage.write("out/a.md", data)
        entries = [entry async for entry in ctx.storage.list("posts")]
        watched = [entry async for entry in ctx.storage.watch("posts")]
        await ctx.storage.remove("out/stale.md")
        response = await ctx.network.fetch(
            "https://example.com/data",
            {"method": "POST", "body": bytes([1, 2, 3]), "headers": {"x-test": "yes"}},
        )
        await ctx.filesystem.write_absolute("/safe/out", data)
        shell = await ctx.shell.run("tool", ["arg"], {"stdin": data})
        ctx.logger.info("runner fixture", {"ok": True})
        return {
            "bytes": data,
            "bounded": await ctx.storage.read_bounded("posts/a.md", 16),
            "exists": await ctx.storage.exists("posts/a.md"),
            "stat": await ctx.storage.stat("posts/a.md"),
            "entries": entries,
            "watched": watched,
            "env": await ctx.env.get("ALLOWED"),
            "envRequired": await ctx.env.get_or_throw("ALLOWED"),
            "nowMs": await ctx.time.now_ms(),
            "nowIso": await ctx.time.now_iso(),
            "home": await ctx.filesystem.home_dir(),
            "temp": await ctx.filesystem.temp_dir(),
            "absolute": await ctx.filesystem.read_absolute("/safe/a"),
            "absoluteBounded": await ctx.filesystem.read_absolute_bounded("/safe/a", 16),
            "status": response.status,
            "body": await response.array_buffer(),
            "shell": shell,
        }
    return input_value["value"]


run_plugin(
    fixture,
    max_frame_bytes=4096,
    max_header_bytes=512,
    max_buffered_stream_values=4,
    max_buffered_stream_bytes=1024,
)
