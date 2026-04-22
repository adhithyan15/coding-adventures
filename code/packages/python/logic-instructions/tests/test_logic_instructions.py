"""Tests for the standardized logic instruction layer."""

import pytest
from logic_engine import (
    atom,
    conj,
    fresh,
    num,
    relation,
    var,
)
from symbol_core import sym

from logic_instructions import (
    __version__,
    assemble,
    defdynamic,
    defrel,
    fact,
    instruction_program,
    query,
    rule,
    run_all_queries,
    run_query,
    validate,
)


class TestVersion:
    """Verify the package is importable and versioned."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.2.0"


class TestInstructionPrograms:
    """The instruction layer should validate and execute real logic programs."""

    def test_can_run_declared_facts_rules_and_queries_end_to_end(self) -> None:
        parent = relation("parent", 2)
        ancestor = relation("ancestor", 2)

        x = var("X")
        y = var("Y")
        z = var("Z")
        who = var("Who")

        program_value = instruction_program(
            defrel(parent),
            defrel(ancestor),
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            rule(ancestor(x, y), parent(x, y)),
            rule(ancestor(x, y), conj(parent(x, z), ancestor(z, y))),
            query(ancestor("homer", who), outputs=(who,)),
        )

        assert run_query(program_value) == [atom("bart"), atom("lisa")]

    def test_validate_rejects_undeclared_relation_use(self) -> None:
        parent = relation("parent", 2)
        ancestor = relation("ancestor", 2)

        x = var("X")
        y = var("Y")

        program_value = instruction_program(
            defrel(parent),
            rule(ancestor(x, y), parent(x, y)),
        )

        try:
            validate(program_value)
        except ValueError as error:
            assert "undeclared relation" in str(error)
        else:
            msg = "validate() should reject undeclared relations"
            raise AssertionError(msg)

    def test_validate_rejects_duplicate_relation_declarations(self) -> None:
        parent = relation("parent", 2)

        with pytest.raises(ValueError, match="declared more than once"):
            validate(
                instruction_program(
                    defrel(parent),
                    defrel(parent),
                ),
            )

    def test_validate_rejects_undeclared_relation_in_rule_body(self) -> None:
        parent = relation("parent", 2)
        ancestor = relation("ancestor", 2)

        x = var("X")
        y = var("Y")

        with pytest.raises(ValueError, match="rule body references undeclared"):
            validate(
                instruction_program(
                    defrel(ancestor),
                    rule(ancestor(x, y), parent(x, y)),
                ),
            )

    def test_validate_rejects_undeclared_relation_in_query(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")

        with pytest.raises(ValueError, match="query references undeclared"):
            validate(instruction_program(query(parent("homer", who))))

    def test_validate_rejects_facts_with_variables(self) -> None:
        parent = relation("parent", 2)
        child = var("Child")

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", child)),
        )

        try:
            validate(program_value)
        except ValueError as error:
            assert "facts must be ground" in str(error)
        else:
            msg = "validate() should reject non-ground facts"
            raise AssertionError(msg)

    def test_query_outputs_can_be_inferred_from_goal_variables(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            query(parent("homer", who)),
        )

        assert run_query(program_value) == [atom("bart"), atom("lisa")]

    def test_query_output_inference_ignores_fresh_local_variables(self) -> None:
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

        assert run_query(program_value) == [atom("bart")]

    def test_multiple_queries_can_run_from_one_instruction_stream(self) -> None:
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

        assert run_all_queries(program_value) == [
            [atom("homer")],
            [atom("homer")],
        ]

    def test_zero_output_queries_report_success_tuples(self) -> None:
        parent = relation("parent", 2)

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            query(parent("homer", "bart"), outputs=()),
        )

        assert run_query(program_value) == [()]

    def test_run_query_supports_limits(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            fact(parent("homer", "lisa")),
            query(parent("homer", who)),
        )

        assert run_query(program_value, limit=1) == [atom("bart")]

    def test_run_query_rejects_out_of_range_indices(self) -> None:
        parent = relation("parent", 2)

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
        )

        with pytest.raises(IndexError, match="out of range"):
            run_query(program_value)

    def test_assemble_exposes_relations_program_and_queries(self) -> None:
        parent = relation("parent", 2)
        who = var("Who")

        assembled = assemble(
            instruction_program(
                defrel(parent),
                fact(parent("homer", "bart")),
                query(parent("homer", who)),
            ),
        )

        assert assembled.relations == (parent,)
        assert len(assembled.program.clauses) == 1
        assert len(assembled.queries) == 1

    def test_dynamic_relation_declarations_lower_into_program_metadata(self) -> None:
        memo = relation("memo", 1)
        item = var("Item")

        assembled = assemble(
            instruction_program(
                defdynamic(memo),
                fact(memo("cached")),
                query(memo(item), outputs=(item,)),
            ),
        )

        assert assembled.relations == (memo,)
        assert assembled.program.dynamic_relations == frozenset({memo.key()})
        assert run_query(
            instruction_program(
                defdynamic(memo),
                fact(memo("cached")),
                query(memo(item), outputs=(item,)),
            ),
        ) == [atom("cached")]

    def test_defrel_accepts_symbols_and_rejects_ambiguous_arguments(self) -> None:
        parent_decl = defrel(sym("parent"), 2)

        assert parent_decl.relation == relation("parent", 2)

        with pytest.raises(ValueError, match="requires arity"):
            defrel("parent")

        with pytest.raises(ValueError, match="does not accept arity"):
            defrel(relation("parent", 2), arity=2)

    def test_defdynamic_accepts_symbols_and_rejects_ambiguous_arguments(self) -> None:
        parent_decl = defdynamic(sym("parent"), 2)

        assert parent_decl.relation == relation("parent", 2)

        with pytest.raises(ValueError, match="requires arity"):
            defdynamic("parent")

        with pytest.raises(ValueError, match="does not accept arity"):
            defdynamic(relation("parent", 2), arity=2)

    def test_fact_rule_and_query_reject_invalid_shapes(self) -> None:
        parent = relation("parent", 2)

        with pytest.raises(TypeError, match="relation call"):
            fact("not-a-call")  # type: ignore[arg-type]

        with pytest.raises(TypeError, match="relation call"):
            rule("not-a-call", parent("homer", "bart"))  # type: ignore[arg-type]

        with pytest.raises(TypeError, match="goal expression"):
            query("not-a-goal")  # type: ignore[arg-type]

    def test_query_outputs_coerce_terms_and_reject_bool_values(self) -> None:
        parent = relation("parent", 2)

        program_value = instruction_program(
            defrel(parent),
            fact(parent("homer", "bart")),
            query(parent("homer", "bart"), outputs=(sym("ok"), 7)),
        )

        assert run_query(program_value) == [(atom("ok"), num(7))]

        with pytest.raises(TypeError, match="bool values are ambiguous"):
            query(parent("homer", "bart"), outputs=(True,))

    def test_instruction_program_rejects_unknown_instruction_entries(self) -> None:
        with pytest.raises(TypeError, match="InstructionProgram entries"):
            instruction_program("not-an-instruction")  # type: ignore[arg-type]
