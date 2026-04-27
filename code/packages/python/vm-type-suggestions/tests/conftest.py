"""Shared test fixtures for vm-type-suggestions."""

from __future__ import annotations

import pytest

from interpreter_ir import IIRFunction, IIRInstr


def make_load_mem(arg_index: int, *, observed_type: str | None = None, count: int = 0) -> IIRInstr:
    """Build a load_mem instruction for arg[N] with optional profiler data."""
    instr = IIRInstr(
        op="load_mem",
        dest=f"%r{arg_index}",
        srcs=[f"arg[{arg_index}]"],
        type_hint="any",
    )
    instr.observed_type = observed_type
    instr.observation_count = count
    return instr


def make_typed_load_mem(arg_index: int, type_hint: str) -> IIRInstr:
    """Build a load_mem for a typed parameter (should be skipped)."""
    return IIRInstr(
        op="load_mem",
        dest=f"%r{arg_index}",
        srcs=[f"arg[{arg_index}]"],
        type_hint=type_hint,
    )


def make_function(
    name: str,
    params: list[tuple[str, str]],
    instrs: list[IIRInstr],
) -> IIRFunction:
    return IIRFunction(
        name=name,
        params=params,
        return_type="any",
        instructions=instrs,
    )


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture
def add_fn():
    """add(a, b) — both params always u8, 1,000,000 calls each."""
    return make_function(
        "add",
        params=[("a", "any"), ("b", "any")],
        instrs=[
            make_load_mem(0, observed_type="u8", count=1_000_000),
            make_load_mem(1, observed_type="u8", count=1_000_000),
            IIRInstr("add", "%r2", ["%r0", "%r1"], "any"),
            IIRInstr("ret", None, ["%r2"], "any"),
        ],
    )


@pytest.fixture
def fibonacci_fn():
    """fibonacci(n) — n always u8, 1,048,576 calls."""
    return make_function(
        "fibonacci",
        params=[("n", "any")],
        instrs=[
            make_load_mem(0, observed_type="u8", count=1_048_576),
            IIRInstr("cmp_lt", "%r1", ["%r0", 2], "any"),
            IIRInstr("ret", None, ["%r0"], "any"),
        ],
    )


@pytest.fixture
def mixed_fn():
    """format_value(s) — s is polymorphic (u8 + str)."""
    return make_function(
        "format_value",
        params=[("s", "any")],
        instrs=[
            make_load_mem(0, observed_type="polymorphic", count=3),
        ],
    )


@pytest.fixture
def never_called_fn():
    """A function that was defined but never executed."""
    return make_function(
        "unused",
        params=[("x", "any")],
        instrs=[
            make_load_mem(0, observed_type=None, count=0),
        ],
    )


@pytest.fixture
def typed_fn():
    """A function whose parameters are already typed."""
    return make_function(
        "typed_add",
        params=[("a", "u8"), ("b", "u8")],
        instrs=[
            make_typed_load_mem(0, "u8"),
            make_typed_load_mem(1, "u8"),
            IIRInstr("add", "%r2", ["%r0", "%r1"], "u8"),
        ],
    )


@pytest.fixture
def no_loader_fn():
    """A function with untyped params but no load_mem [arg[N]] instructions."""
    return make_function(
        "no_loader",
        params=[("x", "any")],
        instrs=[
            IIRInstr("const", "%r0", [42], "any"),
            IIRInstr("ret", None, ["%r0"], "any"),
        ],
    )
