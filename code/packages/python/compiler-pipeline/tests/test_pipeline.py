"""Tests for the pipeline orchestrator.

These tests verify that the full pipeline — lexer, parser, compiler, VM —
works end-to-end. Each test feeds source code into the pipeline and checks
that every stage produced the expected output.

The tests are organized into three groups:

1. **TestPipelineBasic** — Simple programs that exercise the happy path.
2. **TestPipelineComplex** — More involved programs: multiple statements,
   operator precedence, parentheses, strings.
3. **TestAstToDict** — Unit tests for the AST-to-dictionary converter.
4. **TestInstructionToText** — Unit tests for human-readable bytecode.
5. **TestStageDataclasses** — Verify the structure of stage dataclasses.
"""

from __future__ import annotations

from compiler_pipeline import (
    CompilerStage,
    LexerStage,
    ParserStage,
    Pipeline,
    PipelineResult,
    VMStage,
    ast_to_dict,
)
from compiler_pipeline.orchestrator import instruction_to_text


# =========================================================================
# Group 1: Basic pipeline tests
# =========================================================================


class TestPipelineBasic:
    """Test the pipeline with simple single-statement programs."""

    def test_simple_assignment_returns_pipeline_result(self) -> None:
        """Running ``x = 1 + 2`` should return a PipelineResult."""
        result = Pipeline().run("x = 1 + 2")
        assert isinstance(result, PipelineResult)

    def test_source_is_preserved(self) -> None:
        """The original source code should be captured in the result."""
        result = Pipeline().run("x = 1 + 2")
        assert result.source == "x = 1 + 2"

    def test_lexer_stage_has_tokens(self) -> None:
        """The lexer stage should produce at least 6 tokens:
        NAME, EQUALS, NUMBER, PLUS, NUMBER, EOF."""
        result = Pipeline().run("x = 1 + 2")
        assert isinstance(result.lexer_stage, LexerStage)
        assert result.lexer_stage.token_count >= 6

    def test_lexer_stage_source(self) -> None:
        """The lexer stage should capture the original source."""
        result = Pipeline().run("x = 1 + 2")
        assert result.lexer_stage.source == "x = 1 + 2"

    def test_parser_stage_has_ast(self) -> None:
        """The parser stage should produce a Program with one statement."""
        result = Pipeline().run("x = 1 + 2")
        assert isinstance(result.parser_stage, ParserStage)
        ast_dict = result.parser_stage.ast_dict
        assert ast_dict["type"] == "Program"
        assert len(ast_dict["statements"]) == 1

    def test_parser_stage_assignment(self) -> None:
        """The AST should contain an Assignment to Name('x')."""
        result = Pipeline().run("x = 1 + 2")
        stmt = result.parser_stage.ast_dict["statements"][0]
        assert stmt["type"] == "Assignment"
        assert stmt["target"] == {"type": "Name", "name": "x"}

    def test_compiler_stage_has_instructions(self) -> None:
        """The compiler stage should produce at least one instruction."""
        result = Pipeline().run("x = 1 + 2")
        assert isinstance(result.compiler_stage, CompilerStage)
        assert len(result.compiler_stage.instructions_text) > 0

    def test_compiler_stage_constants(self) -> None:
        """The compiler should capture constants 1 and 2."""
        result = Pipeline().run("x = 1 + 2")
        assert result.compiler_stage.constants == [1, 2]

    def test_compiler_stage_names(self) -> None:
        """The compiler should capture name 'x'."""
        result = Pipeline().run("x = 1 + 2")
        assert result.compiler_stage.names == ["x"]

    def test_vm_stage_final_variables(self) -> None:
        """The VM should compute x = 3."""
        result = Pipeline().run("x = 1 + 2")
        assert isinstance(result.vm_stage, VMStage)
        assert result.vm_stage.final_variables == {"x": 3}

    def test_vm_stage_has_traces(self) -> None:
        """The VM should produce execution traces."""
        result = Pipeline().run("x = 1 + 2")
        assert len(result.vm_stage.traces) > 0

    def test_vm_stage_output_is_list(self) -> None:
        """The VM output should be a list (possibly empty)."""
        result = Pipeline().run("x = 1 + 2")
        assert isinstance(result.vm_stage.output, list)


# =========================================================================
# Group 2: Complex pipeline tests
# =========================================================================


class TestPipelineComplex:
    """Test the pipeline with multi-statement and complex programs."""

    def test_multiple_assignments(self) -> None:
        """Multiple assignments should all be captured."""
        source = "a = 10\nb = 20\nc = a + b"
        result = Pipeline().run(source)
        assert result.vm_stage.final_variables == {
            "a": 10,
            "b": 20,
            "c": 30,
        }

    def test_operator_precedence(self) -> None:
        """Multiplication should bind tighter than addition."""
        result = Pipeline().run("x = 1 + 2 * 3")
        assert result.vm_stage.final_variables == {"x": 7}

    def test_parentheses(self) -> None:
        """Parentheses should override default precedence."""
        result = Pipeline().run("x = (1 + 2) * 3")
        assert result.vm_stage.final_variables == {"x": 9}

    def test_string_assignment(self) -> None:
        """String literals should be handled correctly."""
        result = Pipeline().run('x = "hello"')
        assert result.vm_stage.final_variables == {"x": "hello"}

    def test_subtraction(self) -> None:
        """Subtraction should work correctly."""
        result = Pipeline().run("x = 10 - 3")
        assert result.vm_stage.final_variables == {"x": 7}

    def test_division(self) -> None:
        """Division should work correctly."""
        result = Pipeline().run("x = 10 / 2")
        assert result.vm_stage.final_variables == {"x": 5.0}

    def test_complex_expression(self) -> None:
        """A complex nested expression should evaluate correctly."""
        result = Pipeline().run("x = (10 + 20) * (3 - 1)")
        assert result.vm_stage.final_variables == {"x": 60}

    def test_variable_reuse(self) -> None:
        """Variables should be reusable across statements."""
        source = "x = 5\ny = x * 2"
        result = Pipeline().run(source)
        assert result.vm_stage.final_variables == {"x": 5, "y": 10}

    def test_multiple_statements_have_multiple_ast_nodes(self) -> None:
        """Multiple statements should produce multiple AST nodes."""
        source = "a = 1\nb = 2"
        result = Pipeline().run(source)
        assert len(result.parser_stage.ast_dict["statements"]) == 2

    def test_traces_count_increases_with_complexity(self) -> None:
        """More instructions should produce more traces."""
        simple = Pipeline().run("x = 1")
        complex_ = Pipeline().run("x = 1 + 2 * 3")
        assert len(complex_.vm_stage.traces) > len(simple.vm_stage.traces)


# =========================================================================
# Group 3: AST-to-dict conversion tests
# =========================================================================


class TestAstToDict:
    """Test the ast_to_dict helper function."""

    def test_number_literal(self) -> None:
        """A NumberLiteral should convert to a dict with type and value."""
        from lang_parser import NumberLiteral

        assert ast_to_dict(NumberLiteral(42)) == {
            "type": "NumberLiteral",
            "value": 42,
        }

    def test_string_literal(self) -> None:
        """A StringLiteral should convert to a dict with type and value."""
        from lang_parser import StringLiteral

        assert ast_to_dict(StringLiteral("hello")) == {
            "type": "StringLiteral",
            "value": "hello",
        }

    def test_name(self) -> None:
        """A Name should convert to a dict with type and name."""
        from lang_parser import Name

        assert ast_to_dict(Name("x")) == {"type": "Name", "name": "x"}

    def test_binary_op(self) -> None:
        """A BinaryOp should convert recursively."""
        from lang_parser import BinaryOp, NumberLiteral

        node = BinaryOp(NumberLiteral(1), "+", NumberLiteral(2))
        d = ast_to_dict(node)
        assert d["type"] == "BinaryOp"
        assert d["op"] == "+"
        assert d["left"] == {"type": "NumberLiteral", "value": 1}
        assert d["right"] == {"type": "NumberLiteral", "value": 2}

    def test_assignment(self) -> None:
        """An Assignment should convert with target and value."""
        from lang_parser import Assignment, Name, NumberLiteral

        node = Assignment(Name("x"), NumberLiteral(42))
        d = ast_to_dict(node)
        assert d["type"] == "Assignment"
        assert d["target"] == {"type": "Name", "name": "x"}
        assert d["value"] == {"type": "NumberLiteral", "value": 42}

    def test_program(self) -> None:
        """A Program should convert with a statements list."""
        from lang_parser import Assignment, Name, NumberLiteral, Program

        stmt = Assignment(Name("x"), NumberLiteral(1))
        prog = Program([stmt])
        d = ast_to_dict(prog)
        assert d["type"] == "Program"
        assert len(d["statements"]) == 1

    def test_unknown_type_fallback(self) -> None:
        """Unknown types should get a fallback dict with type and repr."""
        d = ast_to_dict("something else")
        assert d["type"] == "str"
        assert "repr" in d


# =========================================================================
# Group 4: Instruction-to-text conversion tests
# =========================================================================


class TestInstructionToText:
    """Test the instruction_to_text helper function."""

    def test_load_const_with_resolution(self) -> None:
        """LOAD_CONST should resolve to the actual constant value."""
        from virtual_machine import CodeObject, Instruction, OpCode

        code = CodeObject(
            instructions=[Instruction(OpCode.LOAD_CONST, 0)],
            constants=[42],
            names=[],
        )
        text = instruction_to_text(code.instructions[0], code)
        assert text == "LOAD_CONST 0 (42)"

    def test_store_name_with_resolution(self) -> None:
        """STORE_NAME should resolve to the actual variable name."""
        from virtual_machine import CodeObject, Instruction, OpCode

        code = CodeObject(
            instructions=[Instruction(OpCode.STORE_NAME, 0)],
            constants=[],
            names=["x"],
        )
        text = instruction_to_text(code.instructions[0], code)
        assert text == "STORE_NAME 0 ('x')"

    def test_load_name_with_resolution(self) -> None:
        """LOAD_NAME should resolve to the actual variable name."""
        from virtual_machine import CodeObject, Instruction, OpCode

        code = CodeObject(
            instructions=[Instruction(OpCode.LOAD_NAME, 0)],
            constants=[],
            names=["y"],
        )
        text = instruction_to_text(code.instructions[0], code)
        assert text == "LOAD_NAME 0 ('y')"

    def test_add_no_operand(self) -> None:
        """ADD should just show the opcode name."""
        from virtual_machine import CodeObject, Instruction, OpCode

        code = CodeObject(instructions=[Instruction(OpCode.ADD)], constants=[], names=[])
        text = instruction_to_text(code.instructions[0], code)
        assert text == "ADD"

    def test_halt_no_operand(self) -> None:
        """HALT should just show the opcode name."""
        from virtual_machine import CodeObject, Instruction, OpCode

        code = CodeObject(
            instructions=[Instruction(OpCode.HALT)], constants=[], names=[]
        )
        text = instruction_to_text(code.instructions[0], code)
        assert text == "HALT"

    def test_out_of_bounds_operand(self) -> None:
        """Out-of-bounds operand should fall back to raw display."""
        from virtual_machine import CodeObject, Instruction, OpCode

        code = CodeObject(
            instructions=[Instruction(OpCode.LOAD_CONST, 99)],
            constants=[42],
            names=[],
        )
        text = instruction_to_text(code.instructions[0], code)
        assert text == "LOAD_CONST 99"


# =========================================================================
# Group 5: Stage dataclass structure tests
# =========================================================================


class TestStageDataclasses:
    """Verify the structure and types of stage dataclass fields."""

    def test_lexer_stage_tokens_are_list(self) -> None:
        """Tokens should be a list."""
        result = Pipeline().run("x = 1")
        assert isinstance(result.lexer_stage.tokens, list)

    def test_parser_stage_ast_dict_is_dict(self) -> None:
        """AST dict should be a dictionary."""
        result = Pipeline().run("x = 1")
        assert isinstance(result.parser_stage.ast_dict, dict)

    def test_compiler_stage_code_has_instructions(self) -> None:
        """The CodeObject should have an instructions list."""
        result = Pipeline().run("x = 1")
        assert hasattr(result.compiler_stage.code, "instructions")

    def test_vm_stage_traces_are_list(self) -> None:
        """Traces should be a list."""
        result = Pipeline().run("x = 1")
        assert isinstance(result.vm_stage.traces, list)

    def test_vm_stage_final_variables_is_dict(self) -> None:
        """Final variables should be a dict."""
        result = Pipeline().run("x = 1")
        assert isinstance(result.vm_stage.final_variables, dict)
