"""History — input/output table tests."""

from __future__ import annotations

from symbolic_ir import IRInteger, IRSymbol

from macsyma_runtime import History


def test_initial_state() -> None:
    h = History()
    assert h.next_input_index() == 1
    assert h.last_output() is None
    assert h.get_input(1) is None
    assert h.get_output(1) is None


def test_record_and_retrieve() -> None:
    h = History()
    n1 = h.record_input(IRInteger(1))
    n2 = h.record_output(IRInteger(2))
    assert n1 == 1 and n2 == 1
    assert h.get_input(1) == IRInteger(1)
    assert h.get_output(1) == IRInteger(2)


def test_indices_advance() -> None:
    h = History()
    h.record_input(IRInteger(1))
    h.record_input(IRInteger(2))
    h.record_input(IRInteger(3))
    assert h.next_input_index() == 4


def test_last_output() -> None:
    h = History()
    h.record_output(IRInteger(1))
    h.record_output(IRInteger(2))
    assert h.last_output() == IRInteger(2)


def test_resolve_percent_shorthand() -> None:
    h = History()
    h.record_output(IRInteger(42))
    assert h.resolve_history_symbol("%") == IRInteger(42)


def test_resolve_percent_iN() -> None:
    h = History()
    x = IRSymbol("x")
    h.record_input(x)
    assert h.resolve_history_symbol("%i1") == x


def test_resolve_percent_oN() -> None:
    h = History()
    h.record_output(IRInteger(7))
    assert h.resolve_history_symbol("%o1") == IRInteger(7)


def test_resolve_unknown_returns_none() -> None:
    h = History()
    assert h.resolve_history_symbol("xyz") is None
    assert h.resolve_history_symbol("%foo") is None
    assert h.resolve_history_symbol("%i999") is None


def test_resolve_with_no_history_returns_none() -> None:
    h = History()
    assert h.resolve_history_symbol("%") is None
    assert h.resolve_history_symbol("%i1") is None
    assert h.resolve_history_symbol("%o1") is None


def test_reset() -> None:
    h = History()
    h.record_input(IRInteger(1))
    h.record_output(IRInteger(2))
    h.reset()
    assert h.next_input_index() == 1
    assert h.last_output() is None
