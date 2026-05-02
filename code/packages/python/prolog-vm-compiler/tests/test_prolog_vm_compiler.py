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
    compile_iso_prolog_source,
    compile_prolog_source,
    compile_prolog_to_bytecode,
    compile_swi_prolog_project,
    compile_swi_prolog_source,
    create_iso_prolog_vm_runtime,
    create_prolog_source_bytecode_vm_runtime,
    create_prolog_source_vm_runtime,
    create_swi_prolog_bytecode_vm_runtime,
    create_swi_prolog_vm_runtime,
    load_compiled_prolog_backend_vm,
    load_compiled_prolog_bytecode_vm,
    load_compiled_prolog_vm,
    run_compiled_prolog_bytecode_queries,
    run_compiled_prolog_bytecode_query,
    run_compiled_prolog_queries,
    run_compiled_prolog_query,
    run_compiled_prolog_query_answers,
    run_initialized_compiled_prolog_bytecode_query_answers,
    run_initialized_compiled_prolog_query_answers,
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

    def test_compiles_generic_iso_and_swi_dialect_sources(self) -> None:
        iso_compiled = compile_prolog_source(
            """
            parent(homer, bart).
            ?- parent(homer, Who).
            """,
            dialect="iso",
        )
        swi_compiled = compile_prolog_source(
            """
            :- op(450, xfx, <=>).
            ?- current_op(P, Type, '<=>').
            """,
            dialect="swi",
        )

        assert iso_compiled.dialect_profile is not None
        assert iso_compiled.dialect_profile.name == "iso"
        assert run_compiled_prolog_query(iso_compiled) == [atom("bart")]
        assert swi_compiled.dialect_profile is not None
        assert swi_compiled.dialect_profile.name == "swi"
        assert run_compiled_prolog_query(swi_compiled) == [
            (num(450), atom("xfx")),
        ]

    def test_iso_wrapper_compiles_through_same_vm_path(self) -> None:
        compiled = compile_iso_prolog_source(
            """
            parent(homer, bart).
            ?- parent(homer, Who).
            """,
        )

        assert run_compiled_prolog_query(compiled) == [atom("bart")]

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

    def test_compiled_program_runs_through_logic_bytecode_vm(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(homer, lisa).

            ?- parent(homer, Who).
            """,
        )

        bytecode = compile_prolog_to_bytecode(compiled)
        vm = load_compiled_prolog_bytecode_vm(compiled)

        assert bytecode.query_pool
        assert run_compiled_prolog_bytecode_query(compiled) == [
            atom("bart"),
            atom("lisa"),
        ]
        assert vm.run_query() == [atom("bart"), atom("lisa")]

    def test_backend_selector_loads_structured_or_bytecode_vms(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).

            ?- parent(homer, Who).
            """,
        )

        structured = load_compiled_prolog_backend_vm(
            compiled,
            backend="structured",
        )
        bytecode = load_compiled_prolog_backend_vm(compiled, backend="bytecode")

        assert structured.run_query() == [atom("bart")]
        assert bytecode.run_query() == [atom("bart")]

    def test_backend_selector_runs_source_queries_and_named_answers(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            parent(homer, bart).
            parent(homer, lisa).

            ?- parent(homer, Who).
            """,
        )

        assert run_compiled_prolog_query(compiled, backend="structured") == [
            atom("bart"),
            atom("lisa"),
        ]
        assert run_compiled_prolog_query(compiled, backend="bytecode") == [
            atom("bart"),
            atom("lisa"),
        ]
        assert [
            answer.as_dict()
            for answer in run_compiled_prolog_query_answers(
                compiled,
                backend="bytecode",
            )
        ] == [
            {"Who": atom("bart")},
            {"Who": atom("lisa")},
        ]

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

    def test_bytecode_vm_preserves_dynamic_declarations(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- dynamic(memo/1).
            memo(cached).

            ?- memo(Value).
            """,
        )
        vm = load_compiled_prolog_bytecode_vm(compiled)

        assert relation("memo", 1).key() in vm.state.dynamic_relations
        assert run_compiled_prolog_bytecode_query(compiled) == [atom("cached")]

    def test_bytecode_vm_runs_initializations_before_source_queries(self) -> None:
        compiled = compile_swi_prolog_source(
            """
            :- initialization(dynamic(seen/1)).
            :- initialization(assertz(seen(alpha))).

            ?- seen(Name).
            """,
        )

        assert [
            answer.as_dict()
            for answer in run_initialized_compiled_prolog_bytecode_query_answers(
                compiled,
            )
        ] == [{"Name": atom("alpha")}]
        assert [
            answer.as_dict()
            for answer in run_initialized_compiled_prolog_query_answers(
                compiled,
                backend="bytecode",
            )
        ] == [{"Name": atom("alpha")}]

    def test_bytecode_runtime_accepts_stateful_ad_hoc_queries(self) -> None:
        runtime = create_swi_prolog_bytecode_vm_runtime(
            """
            :- dynamic(memo/1).
            parent(homer, bart).
            """,
        )

        runtime.query("assertz(memo(saved))", commit=True)

        assert runtime.query_values("parent(homer, Who)") == [atom("bart")]
        assert runtime.query_values("memo(Value)") == [atom("saved")]

    def test_generic_bytecode_runtime_uses_selected_dialect_parser(self) -> None:
        runtime = create_prolog_source_bytecode_vm_runtime(
            """
            parent(homer, bart).
            parent(homer, lisa).
            """,
            dialect="iso",
        )

        assert runtime.query_values("parent(homer, Who)") == [
            atom("bart"),
            atom("lisa"),
        ]


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

    def test_generic_runtime_uses_selected_dialect_query_parser(self) -> None:
        runtime = create_prolog_source_vm_runtime(
            """
            parent(homer, bart).
            parent(homer, lisa).
            """,
            dialect="iso",
        )

        assert runtime.query_values("parent(homer, Who)") == [
            atom("bart"),
            atom("lisa"),
        ]

    def test_iso_runtime_wrapper_queries_loaded_program(self) -> None:
        runtime = create_iso_prolog_vm_runtime(
            """
            parent(homer, bart).
            """,
        )

        assert runtime.query_values("parent(homer, Who)") == [atom("bart")]

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
        assert run_compiled_prolog_bytecode_queries(compiled) == [
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
