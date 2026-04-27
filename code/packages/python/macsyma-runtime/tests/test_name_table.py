"""Name-table extensions — coverage check."""

from __future__ import annotations

from symbolic_ir import IRSymbol

from macsyma_runtime import MACSYMA_NAME_TABLE, extend_compiler_name_table


def test_table_is_a_dict() -> None:
    assert isinstance(MACSYMA_NAME_TABLE, dict)


def test_subst_routes_to_subst_head() -> None:
    assert MACSYMA_NAME_TABLE["subst"] == IRSymbol("Subst")


def test_factor_routes_to_factor_head() -> None:
    assert MACSYMA_NAME_TABLE["factor"] == IRSymbol("Factor")


def test_solve_routes_to_solve_head() -> None:
    assert MACSYMA_NAME_TABLE["solve"] == IRSymbol("Solve")


def test_kill_routes_to_kill_head() -> None:
    assert MACSYMA_NAME_TABLE["kill"] == IRSymbol("Kill")


def test_extend_compiler_name_table_merges() -> None:
    target: dict[str, IRSymbol] = {}
    extend_compiler_name_table(target)
    assert "factor" in target
    assert target["factor"] == IRSymbol("Factor")


def test_extend_is_idempotent() -> None:
    target: dict[str, IRSymbol] = {}
    extend_compiler_name_table(target)
    snapshot = dict(target)
    extend_compiler_name_table(target)
    assert target == snapshot
