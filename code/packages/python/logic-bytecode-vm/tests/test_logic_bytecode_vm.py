"""Tests for the bytecode-native logic VM."""

import pytest
from logic_bytecode import (
    LogicBytecodeInstruction,
    LogicBytecodeOp,
    LogicBytecodeProgram,
    compile_program,
)
from logic_engine import State, atom, conj, defer, eq, fresh, relation, term, var
from logic_instructions import (
    InstructionProgram,
    defdynamic,
    defrel,
    fact,
    instruction_program,
    query,
    rule,
)

from logic_bytecode_vm import (
    LogicBytecodeVM,
    LogicBytecodeVMError,
    LogicBytecodeVMValidationError,
    UnknownLogicBytecodeOpcodeError,
    __version__,
    compile_and_execute,
    compile_and_execute_all,
    create_logic_bytecode_vm,
    execute,
    execute_all,
)


def _ancestor_program() -> tuple[InstructionProgram, LogicBytecodeProgram]:
    """Build one shared recursive program in both instruction and bytecode form."""

    parent = relation("parent", 2)
    ancestor = relation("ancestor", 2)
    x = var("X")
    y = var("Y")
    z = var("Z")
    who = var("Who")

    instructions = instruction_program(
        defrel(parent),
        defrel(ancestor),
        fact(parent("homer", "bart")),
        fact(parent("homer", "lisa")),
        rule(ancestor(x, y), parent(x, y)),
        rule(ancestor(x, y), conj(parent(x, z), ancestor(z, y))),
        query(ancestor("homer", who), outputs=(who,)),
    )
    return instructions, compile_program(instructions)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestLogicBytecodeVM:
    """The VM should load loader bytecode and execute queries correctly."""

    def test_can_run_recursive_program_end_to_end(self) -> None:
        _instructions, bytecode = _ancestor_program()

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)
        trace = vm.run()

        assert len(trace) == 8
        assert trace[-1].opcode is LogicBytecodeOp.HALT
        assert trace[-1].query_count == 1
        assert vm.run_query() == [atom("bart"), atom("lisa")]

    def test_execute_helper_runs_one_query(self) -> None:
        _instructions, bytecode = _ancestor_program()

        assert execute(bytecode) == [atom("bart"), atom("lisa")]

    def test_compile_and_execute_matches_direct_bytecode_execution(self) -> None:
        instructions, bytecode = _ancestor_program()

        assert compile_and_execute(instructions) == execute(bytecode)

    def test_dynamic_relation_declarations_survive_bytecode_loading(self) -> None:
        memo = relation("memo", 1)
        value = var("Value")
        bytecode = compile_program(
            instruction_program(
                defdynamic(memo),
                fact(memo("cached")),
                query(memo(value), outputs=(value,)),
            ),
        )

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)
        vm.run()

        assert memo.key() in vm.state.dynamic_relations
        assert memo.key() in vm.assembled_program().dynamic_relations
        assert vm.run_query() == [atom("cached")]

    def test_run_query_from_uses_existing_state_and_reifies_results(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        marker = var("Marker")
        bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
                query(conj(eq(marker, "seen"), parent("homer", who)), outputs=(who,)),
            ),
        )

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)
        vm.run()

        state = next(vm.solve_query_from(State()), None)

        assert state is not None
        assert vm.run_query_from(State()) == [atom("bart")]

    def test_execute_all_and_compile_and_execute_all_run_multiple_queries(self) -> None:
        parent = relation("parent", 2)
        child = relation("child", 2)
        parent_name = var("ParentName")
        child_name = var("ChildName")

        instructions = instruction_program(
            defrel(parent),
            defrel(child),
            fact(parent("homer", "bart")),
            fact(child("bart", "homer")),
            query(parent(parent_name, "bart"), outputs=(parent_name,)),
            query(child("bart", child_name), outputs=(child_name,)),
        )
        bytecode = compile_program(instructions)

        expected = [[atom("homer")], [atom("homer")]]
        assert execute_all(bytecode) == expected
        assert compile_and_execute_all(instructions) == expected

    def test_step_reports_trace_counts_after_each_instruction(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
                query(parent("homer", who)),
            ),
        )

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)

        first = vm.step()
        second = vm.step()
        third = vm.step()
        fourth = vm.step()

        assert first.instruction_index == 0
        assert first.opcode is LogicBytecodeOp.EMIT_RELATION
        assert first.relation_count == 1
        assert first.clause_count == 0
        assert first.query_count == 0

        assert second.instruction_index == 1
        assert second.clause_count == 1

        assert third.instruction_index == 2
        assert third.query_count == 1

        assert fourth.instruction_index == 3
        assert fourth.opcode is LogicBytecodeOp.HALT
        assert vm.state.halted is True

    def test_run_query_requires_finished_loading(self) -> None:
        _instructions, bytecode = _ancestor_program()

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)

        with pytest.raises(LogicBytecodeVMError, match="finish loading"):
            vm.run_query()

    def test_duplicate_relation_declarations_fail_at_runtime(self) -> None:
        parent = relation("parent", 2)
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(parent,),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)
        vm.step()

        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="declared more than once",
        ):
            vm.step()

    def test_fact_requires_declared_relation(self) -> None:
        parent = relation("parent", 2)
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_FACT, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(fact(parent("homer", "bart")),),
            rule_pool=(),
            query_pool=(),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)

        with pytest.raises(LogicBytecodeVMValidationError, match="undeclared relation"):
            vm.step()

    def test_fact_rejects_variables_in_the_head(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_FACT, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(parent,),
            fact_pool=(fact(parent(term("box", child), "bart")),),
            rule_pool=(),
            query_pool=(),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)
        vm.step()

        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="facts must be ground",
        ):
            vm.step()

    def test_rule_body_requires_declared_relations(self) -> None:
        parent = relation("parent", 2)
        ancestor = relation("ancestor", 2)
        x = var("X")
        y = var("Y")
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RULE, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(ancestor, parent),
            fact_pool=(),
            rule_pool=(rule(ancestor(x, y), parent(x, y)),),
            query_pool=(),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)
        vm.step()

        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="rule body references undeclared",
        ):
            vm.step()

    def test_query_requires_declared_relations(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_QUERY, 0),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(parent,),
            fact_pool=(),
            rule_pool=(),
            query_pool=(query(parent("homer", who)),),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)

        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="query references undeclared",
        ):
            vm.step()

    def test_run_query_infers_outputs(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
                query(eq(term("box", who), term("box", "bart"))),
            ),
        )

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)
        vm.run()

        assert vm.run_query() == [atom("bart")]

    def test_run_query_infers_outputs_from_deferred_and_fresh_goals(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        inner = var("Inner")

        fresh_bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
                query(
                    conj(
                        parent("homer", who),
                        fresh(1, lambda local: parent("homer", local)),
                    ),
                ),
            ),
        )
        deferred_bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
                query(defer(lambda child: parent("homer", child), inner)),
            ),
        )

        fresh_vm = create_logic_bytecode_vm()
        fresh_vm.load(fresh_bytecode)
        fresh_vm.run()

        deferred_vm = create_logic_bytecode_vm()
        deferred_vm.load(deferred_bytecode)
        deferred_vm.run()

        assert fresh_vm.run_query() == [atom("bart")]
        assert deferred_vm.run_query() == [atom("bart")]

    def test_assembled_program_exposes_loaded_clauses(self) -> None:
        parent = relation("parent", 2)
        bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
            ),
        )

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)
        vm.run()

        assembled = vm.assembled_program()
        assert len(assembled.clauses) == 1

    def test_run_query_rejects_out_of_range_query_indices(self) -> None:
        parent = relation("parent", 2)
        bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
            ),
        )

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)
        vm.run()

        with pytest.raises(LogicBytecodeVMError, match="out of range"):
            vm.run_query()

    def test_run_query_supports_limits(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")
        bytecode = compile_program(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
                fact(parent("homer", "lisa")),
                query(parent("homer", who)),
            ),
        )

        vm = create_logic_bytecode_vm()
        vm.load(bytecode)
        vm.run()

        assert vm.run_query(limit=1) == [atom("bart")]

    def test_stepping_without_a_program_or_after_halt_fails(self) -> None:
        vm = create_logic_bytecode_vm()

        with pytest.raises(LogicBytecodeVMError, match="no loaded bytecode program"):
            vm.step()

        bytecode = LogicBytecodeProgram(
            instructions=(LogicBytecodeInstruction(LogicBytecodeOp.HALT),),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )
        vm.load(bytecode)
        vm.step()

        with pytest.raises(LogicBytecodeVMError, match="halted"):
            vm.step()

    def test_register_rejects_duplicate_handlers(self) -> None:
        vm = LogicBytecodeVM()
        vm.register(LogicBytecodeOp.EMIT_RELATION, lambda _vm, _instruction: None)

        with pytest.raises(LogicBytecodeVMError, match="already registered"):
            vm.register(LogicBytecodeOp.EMIT_RELATION, lambda _vm, _instruction: None)

    def test_known_opcode_without_registered_handler_raises_runtime_error(self) -> None:
        parent = relation("parent", 2)
        vm = LogicBytecodeVM()
        vm.load(
            LogicBytecodeProgram(
                instructions=(
                    LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION, 0),
                ),
                relation_pool=(parent,),
                fact_pool=(),
                rule_pool=(),
                query_pool=(),
            ),
        )

        with pytest.raises(
            LogicBytecodeVMError,
            match="no handler registered for bytecode opcode",
        ):
            vm.step()

    def test_unknown_opcodes_raise_a_runtime_error(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(0x99),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)

        with pytest.raises(
            UnknownLogicBytecodeOpcodeError,
            match="unknown logic bytecode opcode",
        ):
            vm.step()

    def test_missing_operands_raise_a_validation_error(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(relation("parent", 2),),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)

        with pytest.raises(LogicBytecodeVMValidationError, match="requires an operand"):
            vm.step()

    def test_empty_program_rejects_missing_halt_before_fetch(self) -> None:
        vm = create_logic_bytecode_vm()
        vm.load(
            LogicBytecodeProgram(
                instructions=(),
                relation_pool=(),
                fact_pool=(),
                rule_pool=(),
                query_pool=(),
            ),
        )

        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="missing a final HALT",
        ):
            vm.step()

    def test_negative_and_out_of_range_pool_indexes_raise_validation_errors(
        self,
    ) -> None:
        parent = relation("parent", 2)
        negative = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION, -1),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(parent,),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )
        out_of_range = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.EMIT_FACT, 7),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        negative_vm = create_logic_bytecode_vm()
        negative_vm.load(negative)
        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="relation pool index -1",
        ):
            negative_vm.step()

        out_of_range_vm = create_logic_bytecode_vm()
        out_of_range_vm.load(out_of_range)
        with pytest.raises(LogicBytecodeVMValidationError, match="fact pool index 7"):
            out_of_range_vm.step()

    def test_halt_must_be_the_final_instruction(self) -> None:
        malformed = LogicBytecodeProgram(
            instructions=(
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
                LogicBytecodeInstruction(LogicBytecodeOp.HALT),
            ),
            relation_pool=(),
            fact_pool=(),
            rule_pool=(),
            query_pool=(),
        )

        vm = create_logic_bytecode_vm()
        vm.load(malformed)

        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="HALT must be the final",
        ):
            vm.step()

    def test_running_off_the_end_without_halt_raises_validation_error(self) -> None:
        parent = relation("parent", 2)
        vm = create_logic_bytecode_vm()
        vm.load(
            LogicBytecodeProgram(
                instructions=(
                    LogicBytecodeInstruction(LogicBytecodeOp.EMIT_RELATION, 0),
                ),
                relation_pool=(parent,),
                fact_pool=(),
                rule_pool=(),
                query_pool=(),
            ),
        )

        with pytest.raises(
            LogicBytecodeVMValidationError,
            match="missing a final HALT",
        ):
            vm.step()
