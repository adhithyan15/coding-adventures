"""Tests for the stateful Prolog VM query runtime."""

from __future__ import annotations

from pathlib import Path

import pytest
from logic_builtins import (
    PrologEvaluationError,
    PrologInstantiationError,
    PrologTypeError,
)
from logic_engine import Disequality, LogicVar, atom

from prolog_vm_compiler import (
    compile_swi_prolog_project_from_files,
    compile_swi_prolog_source,
    create_prolog_bytecode_vm_runtime,
    create_prolog_vm_runtime,
    create_swi_prolog_file_bytecode_vm_runtime,
    create_swi_prolog_file_runtime,
    create_swi_prolog_project_bytecode_vm_runtime,
    create_swi_prolog_project_file_bytecode_vm_runtime,
    create_swi_prolog_project_file_runtime,
    create_swi_prolog_project_runtime,
    create_swi_prolog_vm_runtime,
    run_compiled_prolog_query_answers,
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

    def test_runtime_bounds_repeat_with_limits_and_cut(self) -> None:
        runtime = create_swi_prolog_vm_runtime("")

        assert runtime.query_values(
            "repeat, member(Item, [tea, cake]).",
            limit=5,
        ) == [
            atom("tea"),
            atom("cake"),
            atom("tea"),
            atom("cake"),
            atom("tea"),
        ]
        assert runtime.query_values(
            "repeat, member(Item, [tea, cake]), !.",
            limit=5,
        ) == [atom("tea")]

    def test_runtime_answers_expose_residual_constraints(self) -> None:
        runtime = create_swi_prolog_vm_runtime("")

        answers = runtime.query("dif(X, tea).")

        assert len(answers) == 1
        binding = answers[0].as_dict()["X"]
        assert isinstance(binding, LogicVar)
        assert answers[0].residual_constraints == (
            Disequality(left=binding, right=atom("tea")),
        )

    def test_runtime_raises_prolog_arithmetic_errors(self) -> None:
        runtime = create_swi_prolog_vm_runtime("")

        with pytest.raises(PrologInstantiationError) as instantiation:
            runtime.query("X is Y + 1.")
        assert instantiation.value.kind == "instantiation_error"

        with pytest.raises(PrologTypeError) as type_error:
            runtime.query("X is tea + 1.")
        assert type_error.value.expected == "evaluable"

        with pytest.raises(PrologEvaluationError) as evaluation:
            runtime.query("X is 1 / 0.")
        assert evaluation.value.evaluation_error == "zero_divisor"

        with pytest.raises(PrologInstantiationError):
            runtime.query("Y < 3.")
        with pytest.raises(PrologTypeError):
            runtime.query("tea < 3.")

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

    def test_bytecode_runtime_can_be_created_from_existing_compiled_program(
        self,
    ) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            """,
        )
        runtime = create_prolog_bytecode_vm_runtime(compiled)

        assert [answer.as_dict() for answer in runtime.query("parent(homer, Who)")] == [
            {"Who": atom("bart")},
        ]

    def test_file_runtime_loads_includes_and_answers_ad_hoc_queries(
        self,
        tmp_path: Path,
    ) -> None:
        facts_path = tmp_path / "facts.pl"
        facts_path.write_text(
            "parent(homer, bart).\nparent(homer, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- include('facts.pl').\n"
            "ancestor(X, Y) :- parent(X, Y).\n",
            encoding="utf-8",
        )

        runtime = create_swi_prolog_file_runtime(app_path)

        answers = runtime.query("ancestor(homer, Who)")

        assert [answer.as_dict() for answer in answers] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_bytecode_file_runtime_loads_includes_and_answers_queries(
        self,
        tmp_path: Path,
    ) -> None:
        facts_path = tmp_path / "facts.pl"
        facts_path.write_text(
            "parent(homer, bart).\nparent(homer, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- include('facts.pl').\n"
            "ancestor(X, Y) :- parent(X, Y).\n",
            encoding="utf-8",
        )

        runtime = create_swi_prolog_file_bytecode_vm_runtime(app_path)

        answers = runtime.query("ancestor(homer, Who)")

        assert [answer.as_dict() for answer in answers] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_project_file_compiler_runs_linked_module_source_queries(
        self,
        tmp_path: Path,
    ) -> None:
        family_path = tmp_path / "family.pl"
        family_path.write_text(
            ":- module(family, [ancestor/2]).\n"
            "ancestor(homer, bart).\n"
            "ancestor(homer, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, [run/1]).\n"
            ":- use_module(family, [ancestor/2]).\n"
            "run(Who) :- ancestor(homer, Who).\n"
            "?- run(Who).\n",
            encoding="utf-8",
        )

        compiled = compile_swi_prolog_project_from_files(app_path)

        assert [
            answer.as_dict() for answer in run_compiled_prolog_query_answers(compiled)
        ] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_project_runtime_answers_global_ad_hoc_queries(self) -> None:
        runtime = create_swi_prolog_project_runtime(
            """
            parent(homer, bart).
            """,
            """
            parent(homer, lisa).
            """,
        )

        assert [answer.as_dict() for answer in runtime.query("parent(homer, Who)")] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_project_runtime_resolves_ad_hoc_queries_in_module_context(self) -> None:
        runtime = create_swi_prolog_project_runtime(
            """
            :- module(family, [ancestor/2]).
            ancestor(homer, bart).
            ancestor(homer, lisa).
            """,
            """
            :- module(app, []).
            :- use_module(family, [ancestor/2]).
            """,
            query_module="app",
        )

        answers = runtime.query("ancestor(homer, Who)")

        assert [answer.as_dict() for answer in answers] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_bytecode_project_runtime_resolves_module_context_queries(self) -> None:
        runtime = create_swi_prolog_project_bytecode_vm_runtime(
            """
            :- module(family, [ancestor/2]).
            ancestor(homer, bart).
            ancestor(homer, lisa).
            """,
            """
            :- module(app, []).
            :- use_module(family, [ancestor/2]).
            """,
            query_module="app",
        )

        answers = runtime.query("ancestor(homer, Who)")

        assert [answer.as_dict() for answer in answers] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_project_file_runtime_answers_consulted_global_queries(
        self,
        tmp_path: Path,
    ) -> None:
        facts_path = tmp_path / "facts.pl"
        facts_path.write_text(
            "parent(homer, bart).\nparent(homer, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(":- consult(facts).\n", encoding="utf-8")

        runtime = create_swi_prolog_project_file_runtime(app_path)

        assert [answer.as_dict() for answer in runtime.query("parent(homer, Who)")] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_bytecode_project_file_runtime_answers_consulted_queries(
        self,
        tmp_path: Path,
    ) -> None:
        facts_path = tmp_path / "facts.pl"
        facts_path.write_text(
            "parent(homer, bart).\nparent(homer, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(":- consult(facts).\n", encoding="utf-8")

        runtime = create_swi_prolog_project_file_bytecode_vm_runtime(app_path)

        assert [answer.as_dict() for answer in runtime.query("parent(homer, Who)")] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_project_file_runtime_resolves_ad_hoc_queries_in_module_context(
        self,
        tmp_path: Path,
    ) -> None:
        family_path = tmp_path / "family.pl"
        family_path.write_text(
            ":- module(family, [ancestor/2]).\n"
            "ancestor(homer, bart).\n"
            "ancestor(homer, lisa).\n",
            encoding="utf-8",
        )
        app_path = tmp_path / "app.pl"
        app_path.write_text(
            ":- module(app, []).\n"
            ":- use_module(family, [ancestor/2]).\n",
            encoding="utf-8",
        )

        runtime = create_swi_prolog_project_file_runtime(
            app_path,
            query_module="app",
        )

        answers = runtime.query("ancestor(homer, Who)")

        assert [answer.as_dict() for answer in answers] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

    def test_runtime_rejects_empty_and_negative_limited_queries(self) -> None:
        runtime = create_swi_prolog_vm_runtime("fact(ok).")

        with pytest.raises(ValueError, match="must not be empty"):
            runtime.query("")
        with pytest.raises(ValueError, match="non-negative"):
            runtime.query("fact(ok)", limit=-1)
