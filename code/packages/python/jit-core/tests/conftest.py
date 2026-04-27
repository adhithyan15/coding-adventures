"""Shared fixtures and helpers for jit-core tests."""

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
        # Simulate profiler observations.
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
    type_status: FunctionTypeStatus = FunctionTypeStatus.UNTYPED,
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
# Mock backend
# ---------------------------------------------------------------------------

class MockBackend:
    """Minimal backend that records calls and returns predictable results."""

    name = "mock"

    def __init__(self, return_value: Any = 42) -> None:
        self._return_value = return_value
        self.compile_calls: list[list[CIRInstr]] = []
        self.run_calls: list[tuple[bytes, list]] = []
        self._fail_compile = False

    def fail_next_compile(self) -> None:
        self._fail_compile = True

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        self.compile_calls.append(cir)
        if self._fail_compile:
            self._fail_compile = False
            return None
        return b"mock_binary:" + str(len(cir)).encode()

    def run(self, binary: bytes, args: list[Any]) -> Any:
        self.run_calls.append((binary, args))
        return self._return_value


class SummingBackend:
    """Backend that sums its arguments (useful for integration tests)."""

    name = "summing"

    def compile(self, cir: list[CIRInstr]) -> bytes:
        return b"sum_binary"

    def run(self, binary: bytes, args: list[Any]) -> Any:
        return sum(int(a) for a in args) if args else 0


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def mock_backend() -> MockBackend:
    return MockBackend()


@pytest.fixture
def summing_backend() -> SummingBackend:
    return SummingBackend()
