"""Tests for compiling loaded Prolog artifacts into Logic VM programs."""

from __future__ import annotations

import pytest
from logic_engine import atom, num, relation
from logic_instructions import (
    DynamicRelationDefInstruction,
    FactInstruction,
    RuleInstruction,
)
from logic_vm import execute

from prolog_vm_compiler import (
    CompiledPrologVMProgram,
    __version__,
    compile_swi_prolog_project,
    compile_swi_prolog_source,
    create_swi_prolog_vm_runtime,
    load_compiled_prolog_vm,
    run_compiled_prolog_queries,
    run_compiled_prolog_query,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestPrologVMCompiler:
    """Prolog loader artifacts should lower into executable VM instructions."""

    def test_compiles_recursive_source_queries_into_vm_program(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(homer, lisa).
            ancestor(X, Y) :- parent(X, Y).
            ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

            ?- ancestor(homer, Who).
            """,
        )

        assert isinstance(compiled, CompiledPrologVMProgram)
        assert compiled.initialization_query_count == 0
        assert compiled.source_query_count == 1
        assert run_compiled_prolog_query(compiled) == [atom("bart"), atom("lisa")]

    def test_compiled_instruction_program_runs_directly_through_logic_vm(
        self,
    ) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(homer, lisa).

            ?- parent(homer, Who).
            """,
        )

        assert execute(compiled.instructions) == [atom("bart"), atom("lisa")]

    def test_compiles_linked_module_project_before_vm_execution(self) -> None:
        compiled = compile_swi_prolog_project(
            """
            :- module(family, [parent/2]).
            parent(homer, bart).
            parent(homer, lisa).
            """,
            """
            :- use_module(family, [parent/2]).
            ?- parent(homer, Who).
            """,
        )

        assert run_compiled_prolog_query(compiled) == [atom("bart"), atom("lisa")]

    def test_compiles_dynamic_declarations_to_dynamic_relation_ops(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- dynamic(memo/1).
            memo(cached).

            ?- memo(Value).
            """,
        )

        assert any(
            isinstance(instruction, DynamicRelationDefInstruction)
            and instruction.relation == relation("memo", 1)
            for instruction in compiled.instructions.instructions
        )
        vm = load_compiled_prolog_vm(compiled)

        assert relation("memo", 1).key() in vm.state.dynamic_relations
        assert run_compiled_prolog_query(compiled) == [atom("cached")]

    def test_compiles_variable_facts_as_rules(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            same(X, X).

            ?- same(a, a).
            """,
        )

        assert any(
            isinstance(instruction, RuleInstruction)
            and instruction.head.relation == relation("same", 2)
            for instruction in compiled.instructions.instructions
        )
        assert not any(
            isinstance(instruction, FactInstruction)
            and instruction.head.relation == relation("same", 2)
            for instruction in compiled.instructions.instructions
        )
        assert run_compiled_prolog_query(compiled) == [()]

    def test_compiles_supported_prolog_builtins_before_validation(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            ?- not(missing).
            """,
        )

        assert run_compiled_prolog_query(compiled) == [()]

    def test_compiles_current_op_against_loaded_operator_table(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- op(500, yfx, ++).
            ?- current_op(P, Type, '++').
            """,
        )

        assert run_compiled_prolog_query(compiled) == [
            (num(500), atom("yfx")),
        ]

    def test_runtime_current_op_uses_ad_hoc_query_operator_table(self) -> None:
        runtime = create_swi_prolog_vm_runtime(
            """
            :- op(450, xfx, <=>).
            """,
        )

        assert runtime.query_values("?- current_op(P, Type, '<=>').") == [
            (num(450), atom("xfx")),
        ]

    def test_preserves_initialization_queries_before_source_queries(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- initialization(started).
            started.

            ?- started.
            """,
        )
        vm = load_compiled_prolog_vm(compiled)

        assert compiled.initialization_query_count == 1
        assert compiled.source_query_count == 1
        assert vm.state.queries[0].label == "initialization:1"
        assert vm.state.queries[1].label == "query:1"
        assert run_compiled_prolog_query(compiled) == [()]

    def test_runs_all_source_queries_without_running_initialization_slots(
        self,
    ) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(marge, lisa).

            ?- parent(homer, Child).
            ?- parent(Parent, lisa).
            """,
        )

        assert run_compiled_prolog_queries(compiled) == [
            [atom("bart")],
            [atom("marge")],
        ]

    def test_rejects_out_of_range_source_query_indexes(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            fact(ok).
            """,
        )

        with pytest.raises(IndexError, match="out of range"):
            compiled.source_query_vm_index(0)
