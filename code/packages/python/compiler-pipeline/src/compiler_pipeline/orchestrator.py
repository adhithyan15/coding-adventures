"""Pipeline Orchestrator — Wiring the Full Computing Stack.

==========================================================
Chapter 1: What Is a Pipeline?
==========================================================

Imagine a factory assembly line. Raw steel enters at one end and a finished
car rolls out at the other. Between those two points, a dozen stations each
do one specific job: stamping, welding, painting, assembly. No single station
builds the whole car — each one transforms its input and passes the result
downstream.

A **compiler pipeline** works the same way. Raw source code enters at one
end, and executable results come out the other. Our pipeline has four
stations:

    Source code  →  Lexer  →  Parser  →  Compiler  →  VM

1. **Lexer** (tokenizer): Reads raw characters and groups them into
   meaningful tokens — identifiers, numbers, operators, keywords.
   Input: ``"x = 1 + 2"``  Output: ``[NAME("x"), EQUALS, NUMBER(1), PLUS, NUMBER(2)]``

2. **Parser**: Takes the flat token stream and builds a tree structure
   (the Abstract Syntax Tree) that encodes precedence and grouping.
   Input: token list  Output: ``Assignment(Name("x"), BinaryOp(1, "+", 2))``

3. **Compiler**: Walks the AST and emits flat bytecode instructions for
   a stack machine. This is where tree structure becomes linear execution.
   Input: AST  Output: ``[LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0]``

4. **Virtual Machine**: Executes the bytecode instructions one by one,
   maintaining a stack, variables, and captured output.
   Input: bytecode  Output: ``{x: 3}``

==========================================================
Chapter 2: Why Capture Traces?
==========================================================

The pipeline doesn't just *run* code — it **records** what happened at every
stage. This is critical for the HTML visualizer, which lets a learner see
exactly how ``"x = 1 + 2"`` transforms at each step:

- The lexer stage shows which characters became which tokens.
- The parser stage shows the tree structure (why ``*`` binds tighter than ``+``).
- The compiler stage shows the flat bytecode instructions.
- The VM stage shows the stack evolving as each instruction executes.

Each stage captures its output into a dedicated dataclass (``LexerStage``,
``ParserStage``, ``CompilerStage``, ``VMStage``), and the complete result
is bundled into a ``PipelineResult``. The visualizer can then iterate over
these stages and render each one.

==========================================================
Chapter 3: Implementation
==========================================================
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from lexer import Lexer, LexerConfig, Token
from lang_parser import (
    Assignment,
    BinaryOp,
    Name,
    NumberLiteral,
    Program,
    StringLiteral,
)
from bytecode_compiler import BytecodeCompiler
from virtual_machine import CodeObject, OpCode, VirtualMachine, VMTrace


# ---------------------------------------------------------------------------
# Stage dataclasses — one per pipeline stage
# ---------------------------------------------------------------------------
#
# Each dataclass captures the output of one stage in a format that is both
# programmatically useful (the raw objects) and visualization-friendly
# (JSON-serializable summaries, human-readable text).


@dataclass
class LexerStage:
    """Captured output from the lexer stage.

    The lexer is the first station on the assembly line. It reads raw
    source characters and produces a list of *tokens* — the smallest
    meaningful units of the language.

    Attributes
    ----------
    tokens : list[Token]
        The complete token stream, including the final EOF token.
    token_count : int
        How many tokens were produced (a quick summary stat).
    source : str
        The original source code that was tokenized.
    """

    tokens: list[Token]
    token_count: int
    source: str


@dataclass
class ParserStage:
    """Captured output from the parser stage.

    The parser takes the flat token list and builds a tree (the AST)
    that captures the grammatical structure of the source code —
    operator precedence, grouping, statement boundaries.

    Attributes
    ----------
    ast : Program
        The root of the Abstract Syntax Tree.
    ast_dict : dict
        A JSON-serializable representation of the AST, suitable for
        rendering in the HTML visualizer as an interactive tree diagram.
    """

    ast: Program
    ast_dict: dict  # JSON-serializable representation of the AST


@dataclass
class CompilerStage:
    """Captured output from the compiler stage.

    The compiler walks the AST and emits a flat list of bytecode
    instructions for the stack-based VM. This is where tree structure
    becomes linear execution order.

    Attributes
    ----------
    code : CodeObject
        The compiled bytecode — instructions, constants, and names.
    instructions_text : list[str]
        Human-readable instruction listing (e.g., ``"LOAD_CONST 0 (42)"``).
        This is what the visualizer displays in the bytecode panel.
    constants : list[Any]
        The constant pool — literal values referenced by instructions.
    names : list[str]
        The name pool — variable names referenced by instructions.
    """

    code: CodeObject
    instructions_text: list[str]  # Human-readable instruction listing
    constants: list[Any]
    names: list[str]


@dataclass
class VMStage:
    """Captured output from the VM execution stage.

    The VM executes bytecode instructions one at a time, recording a
    trace snapshot after each step. This gives the visualizer a complete
    replay of the execution — stack states, variable changes, and output.

    Attributes
    ----------
    traces : list[VMTrace]
        One trace per executed instruction, showing the VM state before
        and after each step.
    final_variables : dict[str, Any]
        The variable bindings after execution completes (e.g., ``{"x": 3}``).
    output : list[str]
        Any captured print output from the program.
    """

    traces: list[VMTrace]
    final_variables: dict[str, Any]
    output: list[str]  # Captured print output


@dataclass
class PipelineResult:
    """The complete result of running source code through all stages.

    This is the top-level container that bundles every stage's output
    into a single object. The HTML visualizer receives one of these and
    renders each stage in its own panel.

    Attributes
    ----------
    source : str
        The original source code that entered the pipeline.
    lexer_stage : LexerStage
        Tokens produced by the lexer.
    parser_stage : ParserStage
        AST produced by the parser.
    compiler_stage : CompilerStage
        Bytecode produced by the compiler.
    vm_stage : VMStage
        Execution traces and final state from the VM.
    """

    source: str
    lexer_stage: LexerStage
    parser_stage: ParserStage
    compiler_stage: CompilerStage
    vm_stage: VMStage


# ---------------------------------------------------------------------------
# AST-to-dictionary conversion
# ---------------------------------------------------------------------------
#
# The HTML visualizer needs a JSON-serializable representation of the AST
# so it can render the tree as an interactive diagram. Python dataclasses
# aren't directly JSON-serializable, so we convert each node type manually.
#
# This is a classic "visitor" pattern: we inspect the type of each node and
# recursively convert its children. The result is a nested dictionary that
# mirrors the tree structure.


def ast_to_dict(node: Any) -> dict | str | int:
    """Convert an AST node to a JSON-serializable dictionary.

    This function walks the AST recursively, converting each node into a
    plain dictionary with a ``"type"`` key and type-specific fields. The
    HTML visualizer uses these dictionaries to render the AST as an
    interactive tree.

    Parameters
    ----------
    node : Any
        An AST node (Program, Assignment, BinaryOp, NumberLiteral,
        StringLiteral, Name) or any other object.

    Returns
    -------
    dict | str | int
        A JSON-serializable representation of the node.

    Examples
    --------
    >>> from lang_parser import NumberLiteral
    >>> ast_to_dict(NumberLiteral(42))
    {'type': 'NumberLiteral', 'value': 42}

    >>> from lang_parser import BinaryOp, NumberLiteral
    >>> ast_to_dict(BinaryOp(NumberLiteral(1), "+", NumberLiteral(2)))
    {'type': 'BinaryOp', 'op': '+', 'left': {'type': 'NumberLiteral', 'value': 1}, ...}
    """
    # --- Program: the root node containing all statements ---
    if isinstance(node, Program):
        return {
            "type": "Program",
            "statements": [ast_to_dict(s) for s in node.statements],
        }

    # --- Assignment: ``target = value`` ---
    elif isinstance(node, Assignment):
        return {
            "type": "Assignment",
            "target": ast_to_dict(node.target),
            "value": ast_to_dict(node.value),
        }

    # --- BinaryOp: ``left op right`` (e.g., ``1 + 2``) ---
    elif isinstance(node, BinaryOp):
        return {
            "type": "BinaryOp",
            "op": node.op,
            "left": ast_to_dict(node.left),
            "right": ast_to_dict(node.right),
        }

    # --- NumberLiteral: a numeric value like ``42`` ---
    elif isinstance(node, NumberLiteral):
        return {"type": "NumberLiteral", "value": node.value}

    # --- StringLiteral: a string value like ``"hello"`` ---
    elif isinstance(node, StringLiteral):
        return {"type": "StringLiteral", "value": node.value}

    # --- Name: a variable reference like ``x`` ---
    elif isinstance(node, Name):
        return {"type": "Name", "name": node.name}

    # --- Fallback for unknown node types ---
    # If the parser adds new node types in the future, they'll be caught
    # here with a reasonable representation rather than crashing.
    else:
        return {"type": type(node).__name__, "repr": repr(node)}


# ---------------------------------------------------------------------------
# Instruction-to-text conversion
# ---------------------------------------------------------------------------
#
# The visualizer shows human-readable bytecode like:
#
#     LOAD_CONST 0 (42)
#     LOAD_CONST 1 (2)
#     ADD
#     STORE_NAME 0 ('x')
#
# This helper resolves operand indices to their actual values from the
# constant and name pools, making the output much more readable than
# raw indices.


def instruction_to_text(instr: Any, code: CodeObject) -> str:
    """Convert a bytecode instruction to human-readable text.

    For instructions with operands, this resolves the operand index to the
    actual value from the constant or name pool. For example:

    - ``LOAD_CONST 0`` becomes ``LOAD_CONST 0 (42)`` if constants[0] is 42.
    - ``STORE_NAME 0`` becomes ``STORE_NAME 0 ('x')`` if names[0] is "x".
    - ``ADD`` stays ``ADD`` (no operand to resolve).

    Parameters
    ----------
    instr : Any
        An ``Instruction`` object with ``opcode`` and ``operand`` fields.
    code : CodeObject
        The compiled code object (provides the constant and name pools).

    Returns
    -------
    str
        A human-readable string representation of the instruction.
    """
    opcode_name = instr.opcode.name

    if instr.operand is not None:
        # For LOAD_CONST instructions, show the actual constant value
        # alongside the index. This is like showing "MOV R1, #42" instead
        # of just "MOV R1, [const_pool+0]" — much easier to understand.
        if instr.opcode in (OpCode.LOAD_CONST,) and isinstance(
            instr.operand, int
        ):
            if 0 <= instr.operand < len(code.constants):
                return (
                    f"{opcode_name} {instr.operand}"
                    f" ({code.constants[instr.operand]!r})"
                )

        # For STORE_NAME and LOAD_NAME, show the actual variable name.
        # This turns "STORE_NAME 0" into "STORE_NAME 0 ('x')" — you can
        # read the bytecode like pseudocode.
        elif instr.opcode in (
            OpCode.STORE_NAME,
            OpCode.LOAD_NAME,
        ) and isinstance(instr.operand, int):
            if 0 <= instr.operand < len(code.names):
                return (
                    f"{opcode_name} {instr.operand}"
                    f" ({code.names[instr.operand]!r})"
                )

        # For any other instruction with an operand, just show the raw value.
        return f"{opcode_name} {instr.operand}"

    # Instructions without operands (ADD, SUB, MUL, DIV, POP, HALT, etc.)
    # are self-describing — just the opcode name is enough.
    return opcode_name


# ---------------------------------------------------------------------------
# The Pipeline class — the assembly line itself
# ---------------------------------------------------------------------------


class Pipeline:
    """The main pipeline orchestrator.

    Chains: Source -> Lexer -> Parser -> Compiler -> VM

    This class is the assembly line foreman. It doesn't do any of the
    actual work (tokenizing, parsing, compiling, executing) — that's
    handled by the specialized packages. Instead, it coordinates them:

    1. Creates a lexer, feeds it the source, collects tokens.
    2. Creates a parser, feeds it the tokens, collects the AST.
    3. Creates a compiler, feeds it the AST, collects bytecode.
    4. Creates a VM, feeds it the bytecode, collects execution traces.

    At each step, it captures the output into a stage dataclass so the
    HTML visualizer can show exactly what happened.

    Why a class and not a bare function?
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    Right now, ``Pipeline`` has no instance state — ``run()`` could easily
    be a module-level function. We use a class because:

    1. Future configuration (e.g., choosing between hand-written and
       grammar-driven parsers) will be instance attributes.
    2. It's conventional for orchestrators to be classes (think of
       ``unittest.TestRunner``, ``concurrent.futures.Executor``).
    3. It makes the API consistent: ``Pipeline().run(source)``.
    """

    def run(
        self,
        source: str,
        keywords: list[str] | None = None,
    ) -> PipelineResult:
        """Run source code through the full pipeline.

        This is the main entry point. Given source code like ``"x = 1 + 2"``,
        it runs all four stages and returns a ``PipelineResult`` containing
        captured data from every stage.

        Parameters
        ----------
        source : str
            The source code to execute (e.g., ``"x = 1 + 2"``).
        keywords : list[str] | None, optional
            Language-specific keywords to pass to the lexer. If ``None``,
            the lexer uses its default configuration with no keywords.

        Returns
        -------
        PipelineResult
            A complete record of what happened at every stage — tokens,
            AST, bytecode, execution traces, and final variable state.

        Examples
        --------
        Basic usage::

            result = Pipeline().run("x = 1 + 2")
            assert result.vm_stage.final_variables == {"x": 3}

        Multiple statements::

            result = Pipeline().run("a = 10\\nb = 20\\nc = a + b")
            assert result.vm_stage.final_variables == {"a": 10, "b": 20, "c": 30}

        With custom keywords::

            result = Pipeline().run("if x = 1", keywords=["if", "else"])
        """
        # ---------------------------------------------------------------
        # Stage 1: Lexing — characters to tokens
        # ---------------------------------------------------------------
        # The lexer reads the source string character by character and
        # groups characters into tokens. This is like the first station
        # on an assembly line that cuts raw material into standard pieces.

        config = LexerConfig(keywords=keywords) if keywords else None
        lexer = Lexer(source, config)
        tokens = lexer.tokenize()

        lexer_stage = LexerStage(
            tokens=tokens,
            token_count=len(tokens),
            source=source,
        )

        # ---------------------------------------------------------------
        # Stage 2: Parsing — tokens to AST
        # ---------------------------------------------------------------
        # The parser takes the flat token stream and builds a tree that
        # represents the grammatical structure. This is where operator
        # precedence is encoded: ``1 + 2 * 3`` becomes a tree where
        # ``*`` is deeper than ``+``, ensuring it's evaluated first.

        parser = __import__("lang_parser").Parser(tokens)
        ast = parser.parse()

        parser_stage = ParserStage(
            ast=ast,
            ast_dict=ast_to_dict(ast),
        )

        # ---------------------------------------------------------------
        # Stage 3: Compilation — AST to bytecode
        # ---------------------------------------------------------------
        # The compiler walks the AST in post-order and emits a flat
        # sequence of stack-machine instructions. Tree structure becomes
        # linear execution order. This is the same transformation that
        # javac, csc, and cpython perform.

        compiler = BytecodeCompiler()
        code = compiler.compile(ast)

        compiler_stage = CompilerStage(
            code=code,
            instructions_text=[
                instruction_to_text(instr, code)
                for instr in code.instructions
            ],
            constants=list(code.constants),
            names=list(code.names),
        )

        # ---------------------------------------------------------------
        # Stage 4: VM Execution — bytecode to results
        # ---------------------------------------------------------------
        # The VM interprets the bytecode instructions one by one,
        # maintaining a stack, variables, and captured output. It records
        # a trace after each instruction so the visualizer can replay
        # the entire execution step by step.

        vm = VirtualMachine()
        traces = vm.execute(code)

        vm_stage = VMStage(
            traces=traces,
            final_variables=dict(vm.variables),
            output=list(vm.output),
        )

        # ---------------------------------------------------------------
        # Bundle everything into a PipelineResult
        # ---------------------------------------------------------------

        return PipelineResult(
            source=source,
            lexer_stage=lexer_stage,
            parser_stage=parser_stage,
            compiler_stage=compiler_stage,
            vm_stage=vm_stage,
        )
