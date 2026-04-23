"""Shared test helpers for aot-core tests."""

from __future__ import annotations

from typing import Any

import pytest
from interpreter_ir import IIRFunction, IIRInstr, IIRModule
from interpreter_ir.function import FunctionTypeStatus
from jit_core.cir import CIRInstr

# ---------------------------------------------------------------------------
# IIR construction helpers
# ---------------------------------------------------------------------------

def make_instr(
    op: str,
    dest: str | None = None,
    srcs: list | None = None,
    type_hint: str = "any",
    observed_type: str | None = None,
    observation_count: int = 0,
    deopt_anchor: int | None = None,
) -> IIRInstr:
    instr = IIRInstr(op=op, dest=dest, srcs=srcs or [], type_hint=type_hint)
    if observed_type is not None:
        for _ in range(observation_count):
            instr.record_observation(observed_type)
    if deopt_anchor is not None:
        instr.deopt_anchor = deopt_anchor
    return instr


def make_fn(
    name: str,
    params: list[tuple[str, str]],
    *instrs: IIRInstr,
    return_type: str = "any",
    type_status: FunctionTypeStatus = FunctionTypeStatus.FULLY_TYPED,
) -> IIRFunction:
    return IIRFunction(
        name=name,
        params=params,
        return_type=return_type,
        instructions=list(instrs),
        register_count=max(8, len(params) + len(instrs)),
        type_status=type_status,
    )


def make_mod(*fns: IIRFunction) -> IIRModule:
    return IIRModule(name="test", functions=list(fns))


# ---------------------------------------------------------------------------
# CIR construction helper
# ---------------------------------------------------------------------------

def make_cir(
    op: str,
    dest: str | None = None,
    srcs: list | None = None,
    type: str = "any",
    deopt_to: int | None = None,
) -> CIRInstr:
    return CIRInstr(op=op, dest=dest, srcs=srcs or [], type=type, deopt_to=deopt_to)


# ---------------------------------------------------------------------------
# Mock backend
# ---------------------------------------------------------------------------

class MockAOTBackend:
    """Records compile() calls; returns configurable bytes or None."""

    name: str = "mock-aot"

    def __init__(self, return_value: bytes = b"\xde\xad", fail: bool = False) -> None:
        self._return_value = return_value
        self._fail = fail
        self.compile_calls: list[list[CIRInstr]] = []

    def fail_next(self) -> None:
        self._fail = True

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        self.compile_calls.append(list(cir))
        if self._fail:
            self._fail = False
            return None
        return self._return_value

    def run(self, binary: bytes, args: list[Any]) -> Any:
        return sum(a for a in args if isinstance(a, int))


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def mock_backend() -> MockAOTBackend:
    return MockAOTBackend()
