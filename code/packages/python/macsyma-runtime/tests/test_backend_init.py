"""MacsymaBackend wiring — flags, default history, handler installation."""

from __future__ import annotations

import math

from symbolic_ir import IRFloat

from macsyma_runtime import DISPLAY, EV, KILL, SUPPRESS, History, MacsymaBackend


def test_default_history_is_fresh() -> None:
    b = MacsymaBackend()
    assert b.history.next_input_index() == 1


def test_explicit_history_is_used() -> None:
    h = History()
    b = MacsymaBackend(history=h)
    assert b.history is h


def test_default_flags() -> None:
    b = MacsymaBackend()
    assert b.numer is False
    assert b.simp is True


def test_runtime_handlers_installed() -> None:
    b = MacsymaBackend()
    handlers = b.handlers()
    assert DISPLAY.name in handlers
    assert SUPPRESS.name in handlers
    assert KILL.name in handlers
    assert EV.name in handlers


def test_inherited_arithmetic_handlers_still_present() -> None:
    """Subclassing must not drop the parent's handler table."""
    b = MacsymaBackend()
    handlers = b.handlers()
    # SymbolicBackend installs Add, Mul, D, Integrate (among others).
    assert "Add" in handlers
    assert "Mul" in handlers
    assert "D" in handlers
    assert "Integrate" in handlers


def test_cas_handlers_inherited_from_symbolic_backend() -> None:
    """Factor, Solve, Simplify, Length, Determinant, Limit, … live in
    SymbolicBackend so every CAS frontend inherits them automatically.
    MacsymaBackend must not shadow or lose any of them.
    """
    b = MacsymaBackend()
    handlers = b.handlers()
    expected = [
        "Factor", "Solve", "Simplify", "Expand", "Subst",
        "Length", "First", "Rest", "Last", "Append", "Reverse",
        "Range", "Map", "Apply", "Select", "Sort", "Part", "Flatten", "Join",
        "Matrix", "Transpose", "Determinant", "Inverse",
        "Limit", "Taylor",
        "Abs", "Floor", "Ceiling", "Mod", "Gcd", "Lcm",
    ]
    for name in expected:
        assert name in handlers, f"Missing CAS handler: {name!r}"


def test_pi_constant_pre_bound() -> None:
    """%pi is pre-bound to IRFloat(math.pi) so users never see it as a
    free symbol.  lookup() takes a str (the symbol name), not an IRSymbol.
    """
    b = MacsymaBackend()
    val = b.lookup("%pi")
    assert isinstance(val, IRFloat)
    assert abs(val.value - math.pi) < 1e-12


def test_e_constant_pre_bound() -> None:
    """%e is pre-bound to IRFloat(math.e)."""
    b = MacsymaBackend()
    val = b.lookup("%e")
    assert isinstance(val, IRFloat)
    assert abs(val.value - math.e) < 1e-12
