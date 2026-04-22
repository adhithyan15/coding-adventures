"""Tests for the logic VM runtime."""

import pytest
from logic_engine import atom, conj, defer, eq, fresh, relation, term, var
from logic_instructions import (
    InstructionProgram,
    defdynamic,
    defrel,
    fact,
    instruction_program,
    query,
    rule,
)

from logic_vm import (
    LogicVM,
    LogicVMError,
    LogicVMValidationError,
    UnknownInstructionOpcodeError,
    __version__,
    create_logic_vm,
    execute,
    execute_all,
)
from logic_vm.vm import (
    _handle_defrel,
    _handle_dynamic_rel,
    _handle_fact,
    _handle_query,
    _handle_rule,
)


def _ancestor_program() -> tuple[InstructionProgram, object, object, object]:
    """Build one shared recursive program used across the end-to-end tests."""

    parent = relation("parent", 2)
    ancestor = relation("ancestor", 2)

    x = var("X")
    y = var("Y")
    z = var("Z")
    who = var("Who")

    return (
        instruction_program(
            defrel(parent),
            defrel(ancestor),
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            rule(ancestor(x, y), parent(x, y)),
            rule(ancestor(x, y), conj(parent(x, z), ancestor(z, y))),
            query(ancestor("homer", who), outputs=(who,)),
        ),
        parent,
        ancestor,
        who,
    )


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.2.0"


class TestLogicVM:
    """The VM should load LP07 instructions incrementally and run queries."""

    def test_can_run_recursive_program_end_to_end(self) -> None:
        program_value, _parent, _ancestor, _who = _ancestor_program()

        vm = create_logic_vm()
        vm.load(program_value)
        trace = vm.run()

        assert len(trace) == 7
        assert trace[-1].query_count == 1
        assert vm.run_query() == [atom("bart"), atom("lisa")]

    def test_execute_helper_runs_one_query(self) -> None:
        program_value, _parent, _ancestor, _who = _ancestor_program()

        assert execute(program_value) == [atom("bart"), atom("lisa")]

    def test_execute_all_runs_multiple_stored_queries(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        parent_name = var("ParentName")
        child_name = var("ChildName")

        program_value = instruction_program(
            defrel(parent),
            defrel(child),
            fact(parent("homer", "bart")),
            fact(child("bart", "homer")),
            query(parent(parent_name, "bart"), outputs=(parent_name,)),
            query(child("bart", child_name), outputs=(child_name,)),
        )

        assert execute_all(program_value) == [
            [atom("homer")],
            [atom("homer")],
        ]

    def test_step_reports_trace_counts_after_each_instruction(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            query(parent("homer", who)),
        )

        vm = create_logic_vm()
        vm.load(program_value)

        first = vm.step()
        second = vm.step()
        third = vm.step()

        assert first.instruction_index == 0
        assert first.relation_count == 1
        assert first.clause_count == 0
        assert first.query_count == 0

        assert second.instruction_index == 1
        assert second.relation_count == 1
        assert second.clause_count == 1
        assert second.query_count == 0

        assert third.instruction_index == 2
        assert third.relation_count == 1
        assert third.clause_count == 1
        assert third.query_count == 1
        assert vm.state.halted is True

    def test_run_query_requires_finished_loading(self) -> None:
        program_value, _parent, _ancestor, _who = _ancestor_program()

        vm = create_logic_vm()
        vm.load(program_value)

        with pytest.raises(LogicVMError, match="finish loading"):
            vm.run_query()

    def test_duplicate_relation_declarations_fail_at_runtime(self) -> None:
        parent = relation("parent", 2)
        program_value = instruction_program(defrel(parent), defrel(parent))

        vm = create_logic_vm()
        vm.load(program_value)
        vm.step()

        with pytest.raises(LogicVMValidationError, match="declared more than once"):
            vm.step()

    def test_fact_requires_declared_relation(self) -> None:
        parent = relation("parent", 2)
        program_value = instruction_program(fact(parent("homer", "bart")))

        vm = create_logic_vm()
        vm.load(program_value)

        with pytest.raises(LogicVMValidationError, match="undeclared relation"):
            vm.step()

    def test_fact_rejects_variables_in_the_head(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        program_value = instruction_program(
            defrel(parent),
            fact(parent(term("box", child), "bart")),
        )

        vm = create_logic_vm()
        vm.load(program_value)
        vm.step()

        with pytest.raises(LogicVMValidationError, match="facts must be ground"):
            vm.step()

    def test_rule_body_requires_declared_relations(self) -> None:
        parent = relation("parent", 2)
        ancestor = relation("ancestor", 2)
        x = var("X")
        y = var("Y")

        program_value = instruction_program(
            defrel(ancestor),
            rule(ancestor(x, y), parent(x, y)),
        )

        vm = create_logic_vm()
        vm.load(program_value)
        vm.step()

        with pytest.raises(
            LogicVMValidationError,
            match="rule body references undeclared",
        ):
            vm.step()

    def test_query_requires_declared_relations(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        program_value = instruction_program(query(parent("homer", who)))

        vm = create_logic_vm()
        vm.load(program_value)

        with pytest.raises(
            LogicVMValidationError,
            match="query references undeclared",
        ):
            vm.step()

    def test_reset_clears_runtime_state(self) -> None:
        program_value, _parent, _ancestor, _who = _ancestor_program()

        vm = create_logic_vm()
        vm.load(program_value)
        vm.run()
        vm.reset()

        assert vm.state.program is None
        assert vm.state.instruction_pointer == 0
        assert vm.state.halted is True
        assert vm.state.relations == {}
        assert vm.state.dynamic_relations == {}
        assert vm.state.clauses == []
        assert vm.state.queries == []

    def test_run_query_infers_outputs_and_ignores_fresh_local_variables(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            query(
                conj(
                    parent("homer", who),
                    fresh(1, lambda inner: parent("homer", inner)),
                ),
            ),
        )

        vm = create_logic_vm()
        vm.load(program_value)
        vm.run()

        assert vm.run_query() == [atom("bart")]

    def test_run_query_infers_outputs_from_eq_and_deferred_goals(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        inner = var("Inner")

        eq_program = instruction_program(
            query(eq(term("box", who), term("box", "bart"))),
        )
        deferred_program = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            query(defer(lambda child: parent("homer", child), inner)),
        )

        eq_vm = create_logic_vm()
        eq_vm.load(eq_program)
        eq_vm.run()

        deferred_vm = create_logic_vm()
        deferred_vm.load(deferred_program)
        deferred_vm.run()

        assert eq_vm.run_query() == [atom("bart")]
        assert deferred_vm.run_query() == [atom("bart")]

    def test_assembled_program_exposes_loaded_clauses(self) -> None:
        parent = relation("parent", 2)
        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
        )

        vm = create_logic_vm()
        vm.load(program_value)
        vm.run()

        assembled = vm.assembled_program()
        assert len(assembled.clauses) == 1

    def test_dynamic_relation_declarations_reach_assembled_program(self) -> None:
        memo = relation("memo", 1)
        item = var("Item")
        program_value = instruction_program(
            defdynamic(memo),
            fact(memo("cached")),
            query(memo(item), outputs=(item,)),
        )

        vm = create_logic_vm()
        vm.load(program_value)
        trace = vm.run()

        assert trace[0].relation_count == 1
        assert vm.assembled_program().dynamic_relations == frozenset({memo.key()})
        assert vm.run_query() == [atom("cached")]

    def test_run_query_rejects_out_of_range_query_indices(self) -> None:
        parent = relation("parent", 2)
        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
        )

        vm = create_logic_vm()
        vm.load(program_value)
        vm.run()

        with pytest.raises(LogicVMError, match="out of range"):
            vm.run_query()

    def test_run_query_supports_limits(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            query(parent("homer", who)),
        )

        vm = create_logic_vm()
        vm.load(program_value)
        vm.run()

        assert vm.run_query(limit=1) == [atom("bart")]

    def test_stepping_without_a_program_or_after_halt_fails(self) -> None:
        vm = create_logic_vm()

        with pytest.raises(LogicVMError, match="no loaded instruction program"):
            vm.step()

        parent = relation("parent", 2)
        program_value = instruction_program(defrel(parent))
        vm.load(program_value)
        vm.step()

        with pytest.raises(LogicVMError, match="halted"):
            vm.step()

    def test_register_rejects_duplicate_handlers(self) -> None:
        vm = LogicVM()
        vm.register(  # type: ignore[arg-type]
            defrel(relation("parent", 2)).opcode,
            lambda _vm, _instruction: None,
        )

        with pytest.raises(LogicVMError, match="already registered"):
            vm.register(  # type: ignore[arg-type]
                defrel(relation("parent", 2)).opcode,
                lambda _vm, _instruction: None,
            )

    def test_internal_handlers_reject_wrong_instruction_types(self) -> None:
        vm = create_logic_vm()
        parent = relation("parent", 2)
        declared = defrel(parent)
        loaded_fact = fact(parent("homer", "bart"))
        loaded_rule = rule(parent("homer", "bart"), eq("ok", "ok"))
        loaded_query = query(eq("ok", "ok"))

        with pytest.raises(TypeError, match="DEF_REL handler"):
            _handle_defrel(vm, loaded_fact)
        with pytest.raises(TypeError, match="DYNAMIC_REL handler"):
            _handle_dynamic_rel(vm, loaded_fact)
        with pytest.raises(TypeError, match="FACT handler"):
            _handle_fact(vm, declared)
        with pytest.raises(TypeError, match="RULE handler"):
            _handle_rule(vm, loaded_query)
        with pytest.raises(TypeError, match="QUERY handler"):
            _handle_query(vm, loaded_rule)

    def test_unknown_opcodes_raise_a_runtime_error(self) -> None:
        vm = LogicVM()
        parent = relation("parent", 2)
        program_value = instruction_program(defrel(parent))
        vm.load(program_value)

        with pytest.raises(
            UnknownInstructionOpcodeError,
            match="no handler registered",
        ):
            vm.step()
