"""Shared test fixtures for jit-profiling-insights.

All fixtures build on ``IIRInstr`` and ``IIRFunction`` from interpreter-ir
so that the tests exercise the real data model rather than mocks.

Fibonacci fixture mirrors the spec's canonical example:

    def fibonacci(n):          # n: any (no annotation)
        if n < 2: return n
        return fibonacci(n-1) + fibonacci(n-2)

The IIR for ``fibonacci`` would contain:
- A ``type_assert`` guard on ``%r0`` (= parameter ``n``)
- A ``cmp_lt`` for the base-case check (also guarded)
- Arithmetic ops

The ``main`` function calls ``fibonacci`` through a generic call.
"""

from __future__ import annotations

import pytest

from interpreter_ir import IIRFunction, IIRInstr


# ---------------------------------------------------------------------------
# Helper factories
# ---------------------------------------------------------------------------

def make_instr(
    op: str,
    dest: str | None,
    srcs: list,
    type_hint: str,
    *,
    observed_type: str | None = None,
    observation_count: int = 0,
    deopt_count: int = 0,
) -> IIRInstr:
    """Build an IIRInstr with optional profiler annotations."""
    instr = IIRInstr(op=op, dest=dest, srcs=srcs, type_hint=type_hint)
    instr.observed_type = observed_type
    instr.observation_count = observation_count
    # Attach deopt_count as a dynamic attribute for forward-compat testing.
    instr.deopt_count = deopt_count
    return instr


def make_function(name: str, instrs: list[IIRInstr], params: list | None = None) -> IIRFunction:
    """Build an IIRFunction from a list of instructions."""
    return IIRFunction(
        name=name,
        params=params or [("n", "any")],
        return_type="any",
        instructions=instrs,
    )


# ---------------------------------------------------------------------------
# Shared fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def guard_instr():
    """A type_assert instruction (GUARD) on an untyped register."""
    return make_instr(
        op="type_assert",
        dest=None,
        srcs=["%r0", "int"],
        type_hint="any",
        observed_type="int",
        observation_count=1_048_576,
    )


@pytest.fixture
def generic_call_instr():
    """A call_runtime with a generic_ callee (GENERIC_CALL)."""
    return make_instr(
        op="call_runtime",
        dest="%r1",
        srcs=["generic_add", "%r0", "%r2"],
        type_hint="any",
        observed_type="int",
        observation_count=500_000,
    )


@pytest.fixture
def typed_instr():
    """A typed add instruction — no overhead (NONE)."""
    return make_instr(
        op="add",
        dest="%r0",
        srcs=["%a", "%b"],
        type_hint="u8",
        observed_type="u8",
        observation_count=1_000,
    )


@pytest.fixture
def unobserved_instr():
    """An instruction the profiler never sampled."""
    return make_instr(
        op="add",
        dest="%r0",
        srcs=["%a"],
        type_hint="any",
        observation_count=0,
    )


@pytest.fixture
def deopt_instr():
    """An instruction with deopt_count > 0 — interpreter fallback occurred."""
    return make_instr(
        op="add",
        dest="%r0",
        srcs=["%r1"],
        type_hint="any",
        observed_type="int",
        observation_count=200,
        deopt_count=5,
    )


@pytest.fixture
def fibonacci_fn():
    """A synthetic fibonacci IIRFunction mirroring the spec's canonical example.

    Instruction layout:
      0: load_mem %r0 <- arg[0] : any  (parameter n — untyped)
      1: type_assert %r0, "int" : any  (GUARD — JIT inserted because it saw int)
      2: cmp_lt %r1 <- %r0, 2 : any   (comparison for base case)
      3: type_assert %r1, "bool" : any (GUARD on comparison result)
      4: jmp_if_true "base_case" : any
      5: ret %r0 : any                 (base case return — simplified)
    """
    instrs = [
        make_instr("load_mem", "%r0", ["arg[0]"], "any",
                   observed_type="int", observation_count=1_048_576),
        make_instr("type_assert", None, ["%r0", "int"], "any",
                   observed_type="int", observation_count=1_048_576),
        make_instr("cmp_lt", "%r1", ["%r0", 2], "any",
                   observed_type="bool", observation_count=1_048_576),
        make_instr("type_assert", None, ["%r1", "bool"], "any",
                   observed_type="bool", observation_count=1_048_576),
        make_instr("jmp_if_true", None, ["base_case"], "any",
                   observation_count=1_048_576),
        make_instr("ret", None, ["%r0"], "any", observation_count=2),
    ]
    return make_function("fibonacci", instrs, params=[("n", "any")])


@pytest.fixture
def main_fn():
    """A synthetic main function that calls fibonacci via generic dispatch."""
    instrs = [
        make_instr("const", "%r0", [10], "any",
                   observed_type="int", observation_count=3),
        make_instr("call_runtime", "%result", ["generic_call", "%r0"], "any",
                   observed_type="int", observation_count=3),
        make_instr("ret", None, ["%result"], "any", observation_count=3),
    ]
    return make_function("main", instrs, params=[])
