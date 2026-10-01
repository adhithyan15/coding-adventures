from __future__ import annotations

import pytest

from forme_plugin_runner import CancellationError, StageError, define_stage
from forme_plugin_runner.stage import CancellationToken


async def test_define_stage_records_an_immutable_contract() -> None:
    @define_stage(
        name="@example/test",
        version="1.0.0",
        api_version=1,
        consumes="ContentNode",
        produces="ContentNode",
        capabilities=["storage:read"],
    )
    async def stage(value: object, _config: object, _ctx: object) -> object:
        return value

    assert stage.name == "@example/test"
    assert stage.capabilities == ("storage:read",)
    assert await stage.run({"ok": True}, {}, object()) == {"ok": True}


def test_stage_and_cancellation_errors_are_typed() -> None:
    error = StageError("BROKEN", "broken", fields={"line": 3})
    assert error.code == "BROKEN"
    assert error.fields == {"line": 3}
    token = CancellationToken()
    token.cancel("stop")
    with pytest.raises(CancellationError, match="stop"):
        token.throw_if_cancelled()
