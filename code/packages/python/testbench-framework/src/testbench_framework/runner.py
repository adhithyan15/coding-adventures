"""Testbench runner: a Pythonic harness around HardwareVM.

v0.1.0 scope: Python-native API. Keeps things simple — no async/await yet
since hardware-vm is currently combinational-only. When sequential simulation
arrives in v0.2.0, an async layer will land alongside.
"""

from __future__ import annotations

import time as _wallclock
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from hardware_vm import HardwareVM
from hdl_ir import HIR

# ---------------------------------------------------------------------------
# Test discovery + registry
# ---------------------------------------------------------------------------


@dataclass
class TestCase:
    """One named test against a DUT (HIR)."""

    name: str
    func: Callable[[DUTHandle], None]
    timeout_s: float = 5.0
    should_fail: bool = False


_REGISTRY: list[TestCase] = []


def test(
    func: Callable[[DUTHandle], None] | None = None,
    *,
    name: str | None = None,
    timeout_s: float = 5.0,
    should_fail: bool = False,
) -> Any:
    """Decorator: register a function as a testbench test.

    Usage:
        @test
        def my_test(dut):
            dut.a.value = 1
            assert dut.y.value == 1

        @test(name="custom_name", timeout_s=10)
        def another(dut):
            ...
    """

    def _wrap(f: Callable[[DUTHandle], None]) -> Callable[[DUTHandle], None]:
        tc = TestCase(name=name or f.__name__, func=f, timeout_s=timeout_s, should_fail=should_fail)
        _REGISTRY.append(tc)
        return f

    if func is None:
        return _wrap
    return _wrap(func)


def discover() -> list[TestCase]:
    """Return all registered tests since the last clear."""
    return list(_REGISTRY)


def clear_registry() -> None:
    """Clear the global test registry. Useful between runs."""
    _REGISTRY.clear()


# ---------------------------------------------------------------------------
# DUT handle
# ---------------------------------------------------------------------------


class SignalHandle:
    """Attribute access to one signal on the DUT."""

    def __init__(self, vm: HardwareVM, name: str) -> None:
        object.__setattr__(self, "_vm", vm)
        object.__setattr__(self, "_name", name)

    @property
    def value(self) -> int:
        return self._vm.read(self._name)  # type: ignore[attr-defined]

    @value.setter
    def value(self, v: int) -> None:
        self._vm.set_input(self._name, v)  # type: ignore[attr-defined]

    def __repr__(self) -> str:
        return f"SignalHandle({self._name!r}, value={self.value})"  # type: ignore[attr-defined]


class DUTHandle:
    """Handle for a Device Under Test. Attribute access maps to signals."""

    def __init__(self, vm: HardwareVM) -> None:
        object.__setattr__(self, "_vm", vm)

    def __getattr__(self, name: str) -> SignalHandle:
        if name.startswith("_"):
            raise AttributeError(name)
        return SignalHandle(self._vm, name)  # type: ignore[attr-defined]


# ---------------------------------------------------------------------------
# Test runner
# ---------------------------------------------------------------------------


@dataclass
class TestReport:
    passed: list[str] = field(default_factory=list)
    failed: list[tuple[str, str]] = field(default_factory=list)  # (name, message)
    skipped: list[str] = field(default_factory=list)
    duration_s: float = 0.0

    @property
    def all_passed(self) -> bool:
        return not self.failed

    def summary(self) -> str:
        return (
            f"{len(self.passed)} passed, "
            f"{len(self.failed)} failed, "
            f"{len(self.skipped)} skipped "
            f"in {self.duration_s:.3f}s"
        )


def run(hir: HIR, tests: list[TestCase] | None = None) -> TestReport:
    """Run all registered (or explicitly given) tests against the HIR."""
    if tests is None:
        tests = discover()

    report = TestReport()
    start = _wallclock.perf_counter()

    for tc in tests:
        # Each test gets a fresh VM so state doesn't leak.
        vm = HardwareVM(hir)
        dut = DUTHandle(vm)
        try:
            tc.func(dut)
            if tc.should_fail:
                report.failed.append((tc.name, "expected failure but test passed"))
            else:
                report.passed.append(tc.name)
        except AssertionError as e:
            if tc.should_fail:
                report.passed.append(tc.name)
            else:
                report.failed.append((tc.name, f"assertion failed: {e}"))
        except Exception as e:
            if tc.should_fail:
                report.passed.append(tc.name)
            else:
                report.failed.append((tc.name, f"{type(e).__name__}: {e}"))

    report.duration_s = _wallclock.perf_counter() - start
    return report


# ---------------------------------------------------------------------------
# Stimulus helpers
# ---------------------------------------------------------------------------


def exhaustive(
    dut: DUTHandle,
    inputs: dict[str, int],
    *,
    on_step: Callable[[DUTHandle], None] | None = None,
) -> None:
    """Drive each input through 0..(2^width - 1) and call ``on_step`` after
    each combination. Useful for small designs (≤ ~20 input bits).

    ``inputs`` maps signal name -> bit width.
    """
    names = list(inputs.keys())
    widths = [inputs[n] for n in names]
    total_bits = sum(widths)
    if total_bits > 20:
        raise ValueError(
            f"exhaustive over {total_bits} bits would take 2^{total_bits} iterations"
        )

    n_combinations = 1 << total_bits
    for combination in range(n_combinations):
        # Decompose combination into per-input values.
        offset = 0
        for name, width in zip(names, widths, strict=False):
            val = (combination >> offset) & ((1 << width) - 1)
            getattr(dut, name).value = val
            offset += width

        if on_step is not None:
            on_step(dut)


def random_stimulus(
    dut: DUTHandle,
    inputs: dict[str, int],
    iterations: int,
    *,
    seed: int = 42,
    on_step: Callable[[DUTHandle], None] | None = None,
) -> None:
    """Drive each input with random values for ``iterations`` cycles.

    ``inputs`` maps signal name -> bit width. ``seed`` makes runs reproducible."""
    import random

    rng = random.Random(seed)
    for _ in range(iterations):
        for name, width in inputs.items():
            getattr(dut, name).value = rng.randint(0, (1 << width) - 1)
        if on_step is not None:
            on_step(dut)
