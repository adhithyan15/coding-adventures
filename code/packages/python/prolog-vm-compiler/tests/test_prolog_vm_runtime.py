"""Tests for the stateful Prolog VM query runtime."""

from __future__ import annotations

import pytest
from logic_engine import atom

from prolog_vm_compiler import (
    compile_swi_prolog_source,
    create_prolog_vm_runtime,
    create_swi_prolog_vm_runtime,
)


class TestPrologVMRuntime:
    """A runtime should support repeated ad-hoc queries over one VM load."""

    def test_runtime_answers_ad_hoc_query_strings_with_named_bindings(self) -> None:
        runtime = create_swi_prolog_vm_runtime(
            """
            parent(homer, bart).
            parent(homer, lisa).
            """,
        )

        answers = runtime.query("parent(homer, Who)")

        assert [answer.as_dict() for answer in answers] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_runtime_runs_initialization_once_before_queries(self) -> None:
        runtime = create_swi_prolog_vm_runtime(
            """
            :- initialization(dynamic(seen/1)).
            :- initialization(assertz(seen(alpha))).

            ?- seen(Name).
            """,
        )

        assert [answer.as_dict() for answer in runtime.query("seen(Name).")] == [
            {"Name": atom("alpha")},
        ]

    def test_runtime_commit_persists_dynamic_database_effects(self) -> None:
        runtime = create_swi_prolog_vm_runtime(
            """
            :- dynamic(memo/1).
            """,
        )

        assert [
            answer.as_dict()
            for answer in runtime.query("assertz(memo(saved))", commit=False)
        ] == [{}]
        assert runtime.query("memo(Value)") == []

        assert [
            answer.as_dict()
            for answer in runtime.query("assertz(memo(saved))", commit=True)
        ] == [{}]
        assert [answer.as_dict() for answer in runtime.query("memo(Value)")] == [
            {"Value": atom("saved")},
        ]

    def test_runtime_supports_limits_and_raw_values(self) -> None:
        runtime = create_swi_prolog_vm_runtime(
            """
            pick(first).
            pick(second).
            """,
        )

        assert runtime.query_values("pick(Value)", limit=1) == [atom("first")]

    def test_runtime_can_be_created_from_an_existing_compiled_program(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            """,
        )
        runtime = create_prolog_vm_runtime(compiled)

        assert [answer.as_dict() for answer in runtime.query("parent(homer, Who)")] == [
            {"Who": atom("bart")},
        ]

    def test_runtime_rejects_empty_and_negative_limited_queries(self) -> None:
        runtime = create_swi_prolog_vm_runtime("fact(ok).")

        with pytest.raises(ValueError, match="must not be empty"):
            runtime.query("")
        with pytest.raises(ValueError, match="non-negative"):
            runtime.query("fact(ok)", limit=-1)
