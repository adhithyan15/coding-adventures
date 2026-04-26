"""MacsymaBackend wiring — flags, default history, handler installation."""

from __future__ import annotations

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
