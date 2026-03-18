"""Bytecode Compiler — The bridge between parsing and execution.

=================================================================
Chapter 4a: From Trees to Instructions
=================================================================

In the previous layers, we built a lexer (Layer 2) that turns source code into
tokens, and a parser (Layer 3) that arranges those tokens into an Abstract
Syntax Tree (AST). Now we face the next question: how do we turn a *tree* into
something a machine can actually execute?

The answer is **compilation** — walking the tree and emitting a flat sequence
of stack-machine instructions. This is exactly what real compilers do:

    javac    : Java source  -->  JVM bytecode  (.class files)
    csc      : C# source    -->  CLR IL        (.dll files)
    cpython  : Python source -->  Python bytecode (.pyc files)
    Our compiler: AST       -->  CodeObject    (for our VM)

The key insight is that a tree-structured program can always be "flattened"
into a sequence of stack operations. Consider the expression ``1 + 2 * 3``.
The AST looks like this::

        +
       / \\
      1   *
         / \\
        2   3

To evaluate this on a stack machine, we do a **post-order traversal** (visit
children before the parent):

    1. Visit the left child of ``+``:  emit LOAD_CONST 1
    2. Visit the right child of ``+`` (which is ``*``):
       a. Visit left child of ``*``:   emit LOAD_CONST 2
       b. Visit right child of ``*``:  emit LOAD_CONST 3
       c. Visit ``*`` itself:          emit MUL
    3. Visit ``+`` itself:             emit ADD

The result is: ``LOAD_CONST 1, LOAD_CONST 2, LOAD_CONST 3, MUL, ADD``

This is called **Reverse Polish Notation** (RPN), and it's the natural output
format for a stack-machine compiler. The stack does all the bookkeeping that
parentheses and precedence rules handle in the source code.

Terminology
-----------
- **Emit**: To append an instruction to the output list.
- **Constant pool**: A list of literal values (numbers, strings) that
  instructions reference by index, not by value.
- **Name pool**: A list of variable names, similarly referenced by index.
- **CodeObject**: The final compiled artifact — instructions + pools.
"""

from __future__ import annotations

from lang_parser import (
    Assignment,
    BinaryOp,
    Expression,
    Name,
    NumberLiteral,
    Program,
    Statement,
    StringLiteral,
)
from virtual_machine import CodeObject, Instruction, OpCode


# ---------------------------------------------------------------------------
# Operator-to-opcode mapping
# ---------------------------------------------------------------------------

OPERATOR_MAP: dict[str, OpCode] = {
    "+": OpCode.ADD,
    "-": OpCode.SUB,
    "*": OpCode.MUL,
    "/": OpCode.DIV,
}
"""Maps the source-level operator symbols to their corresponding VM opcodes.

Each arithmetic operator in the source language has a direct counterpart in the
VM instruction set. The compiler uses this table during expression compilation
to translate ``BinaryOp.op`` strings into the correct ``OpCode``.

Why a dictionary and not a chain of if/elif? Because:
  1. It's easier to extend — adding ``%`` just means one new entry.
  2. It separates data (the mapping) from logic (the compilation).
  3. It's faster for large operator sets (O(1) lookup vs. O(n) branches).
"""


class BytecodeCompiler:
    """Compiles an AST into a CodeObject for the virtual machine.

    This is the bridge between the parser (which understands language syntax)
    and the VM (which executes instructions). The compiler's job is to
    translate tree-structured code into a flat sequence of stack operations.

    This is analogous to:
    - ``javac``:  compiles Java source  -> JVM bytecode (.class files)
    - ``csc``:    compiles C# source    -> CLR IL bytecode (.dll files)
    - Our compiler: compiles AST        -> CodeObject (for our VM)

    How it works
    ------------
    The compiler maintains three pieces of state as it walks the AST:

    1. **instructions** — The growing list of bytecode instructions. Each call
       to ``_compile_expression`` or ``_compile_statement`` appends one or more
       instructions to this list.

    2. **constants** — The constant pool. When the compiler encounters a literal
       value like ``42`` or ``"hello"``, it adds it here (if not already present)
       and emits a ``LOAD_CONST <index>`` instruction referencing its position.

    3. **names** — The name pool. Variable names like ``x`` or ``total`` go here,
       and ``STORE_NAME <index>`` / ``LOAD_NAME <index>`` reference them.

    Example walkthrough
    -------------------
    Compiling ``x = 1 + 2``::

        AST:
            Assignment(
                target=Name("x"),
                value=BinaryOp(NumberLiteral(1), "+", NumberLiteral(2))
            )

        Step 1: _compile_assignment is called.
        Step 2: It calls _compile_expression on the BinaryOp.
        Step 3: _compile_expression recurses:
            - Left:  NumberLiteral(1) -> adds 1 to constants[0], emits LOAD_CONST 0
            - Right: NumberLiteral(2) -> adds 2 to constants[1], emits LOAD_CONST 1
            - Op "+":                 -> emits ADD
        Step 4: Back in _compile_assignment:
            - Adds "x" to names[0], emits STORE_NAME 0

        Result:
            instructions = [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0]
            constants    = [1, 2]
            names        = ["x"]
    """

    def __init__(self) -> None:
        """Initialize an empty compiler with no instructions or pools.

        Each ``BytecodeCompiler`` instance compiles exactly one ``Program``.
        If you need to compile another program, create a new instance.
        This keeps the state clean and avoids accidental cross-contamination
        between compilation units.
        """
        self.instructions: list[Instruction] = []
        """The bytecode instructions emitted so far, in order."""

        self.constants: list[int | str] = []
        """The constant pool — literal values referenced by LOAD_CONST."""

        self.names: list[str] = []
        """The name pool — variable names referenced by STORE_NAME / LOAD_NAME."""

    # -------------------------------------------------------------------
    # Public API
    # -------------------------------------------------------------------

    def compile(self, program: Program) -> CodeObject:
        """Compile a full program AST into a CodeObject.

        This is the main entry point. It iterates over every statement in the
        program, compiles each one, then appends a final ``HALT`` instruction
        to tell the VM that execution is complete.

        Parameters
        ----------
        program : Program
            The root AST node, as produced by ``Parser.parse()``.

        Returns
        -------
        CodeObject
            A self-contained unit of bytecode ready for the VM to execute.
            Contains the instruction list, constant pool, and name pool.

        Example
        -------
        ::

            from lang_parser import Parser, Program
            from lexer import Lexer

            tokens = Lexer("x = 42").tokenize()
            ast = Parser(tokens).parse()

            compiler = BytecodeCompiler()
            code = compiler.compile(ast)
            # code.instructions = [LOAD_CONST 0, STORE_NAME 0, HALT]
            # code.constants = [42]
            # code.names = ["x"]
        """
        for statement in program.statements:
            self._compile_statement(statement)

        # Every program ends with HALT so the VM knows to stop.
        # Without this, the VM would try to read past the end of the
        # instruction array — just like a CPU needs a HLT instruction.
        self.instructions.append(Instruction(OpCode.HALT))

        return CodeObject(
            instructions=self.instructions,
            constants=self.constants,
            names=self.names,
        )

    # -------------------------------------------------------------------
    # Statement compilation
    # -------------------------------------------------------------------

    def _compile_statement(self, stmt: Statement) -> None:
        """Compile a single statement.

        There are two kinds of statements in our language:

        1. **Assignment** (``x = expr``) — Evaluate the expression, then store
           the result in a named variable. The value stays bound to that name
           for the rest of the program.

        2. **Expression statement** (just ``expr`` on its own) — Evaluate the
           expression for its side effects (there are none yet, but there will
           be when we add function calls and print). Since no one captures the
           result, we emit a ``POP`` to discard it and keep the stack clean.

        Why POP for expression statements?
        ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
        The stack machine's invariant is: after each statement completes, the
        stack should be in the same state as before. An expression like ``1 + 2``
        would leave the result ``3`` on the stack. If we didn't pop it, the
        stack would grow by one element for every expression statement, and
        subsequent operations would find unexpected values.

        Parameters
        ----------
        stmt : Statement
            An AST node that is either an ``Assignment`` or an ``Expression``.
        """
        if isinstance(stmt, Assignment):
            self._compile_assignment(stmt)
        else:
            # Expression statement — compile the expression, then throw away
            # the result. The expression's value is computed and pushed onto
            # the stack, but since nobody assigned it to a variable, we
            # discard it with POP.
            self._compile_expression(stmt)
            self.instructions.append(Instruction(OpCode.POP))

    def _compile_assignment(self, node: Assignment) -> None:
        """Compile a variable assignment: ``name = expression``.

        The compilation strategy is straightforward:
        1. Compile the right-hand side expression (pushes its value onto the stack).
        2. Emit ``STORE_NAME <index>`` to pop the value and bind it to the name.

        This mirrors how CPython compiles assignments: evaluate the value first,
        then store it. The order matters — we need the value on the stack before
        we can store it.

        Parameters
        ----------
        node : Assignment
            An AST node with ``target`` (a Name) and ``value`` (an Expression).

        Example
        -------
        ``x = 42`` compiles to::

            LOAD_CONST 0    # Push 42 onto the stack
            STORE_NAME 0    # Pop it and bind to "x"
        """
        # First, evaluate the right-hand side. After this, the value sits
        # on top of the stack, waiting to be stored.
        self._compile_expression(node.value)

        # Now store the top-of-stack value into the named variable.
        # _add_name handles deduplication: if "x" was already used, we reuse
        # its index rather than adding a duplicate entry to the name pool.
        name_index = self._add_name(node.target.name)
        self.instructions.append(Instruction(OpCode.STORE_NAME, name_index))

    # -------------------------------------------------------------------
    # Expression compilation — the recursive heart
    # -------------------------------------------------------------------

    def _compile_expression(self, node: Expression) -> None:
        """Compile an expression — the recursive heart of the compiler.

        Every expression, no matter how complex, ultimately compiles down to a
        sequence of instructions that leaves exactly one value on the stack.
        This is the fundamental contract of expression compilation:

            **Before**: stack has N items.
            **After**:  stack has N + 1 items (the expression's value on top).

        The compiler handles each expression type differently:

        - **NumberLiteral** / **StringLiteral**: Add the value to the constant
          pool, emit ``LOAD_CONST <index>`` to push it onto the stack.

        - **Name**: Add the variable name to the name pool, emit
          ``LOAD_NAME <index>`` to look up and push its current value.

        - **BinaryOp**: Recursively compile left and right operands (each pushes
          one value), then emit the appropriate arithmetic instruction (ADD, SUB,
          etc.) which pops both values and pushes the result.

        The recursion is what makes this work for arbitrarily nested expressions.
        ``1 + 2 * 3`` has a BinaryOp at the top, whose right child is another
        BinaryOp. The compiler just keeps recursing until it hits leaf nodes
        (literals or names), then the instructions "unwind" in the correct order.

        Parameters
        ----------
        node : Expression
            Any AST expression node (NumberLiteral, StringLiteral, Name, BinaryOp).

        Raises
        ------
        TypeError
            If ``node`` is an unrecognized expression type. This should never
            happen if the parser is correct, but it's good defensive programming.
        """
        if isinstance(node, NumberLiteral):
            # A number literal like 42. We store the value in the constant pool
            # and emit an instruction to push it onto the stack at runtime.
            const_index = self._add_constant(node.value)
            self.instructions.append(Instruction(OpCode.LOAD_CONST, const_index))

        elif isinstance(node, StringLiteral):
            # A string literal like "hello". Same strategy as numbers — store
            # in the constant pool, emit LOAD_CONST.
            const_index = self._add_constant(node.value)
            self.instructions.append(Instruction(OpCode.LOAD_CONST, const_index))

        elif isinstance(node, Name):
            # A variable reference like x. We store the name string in the name
            # pool and emit LOAD_NAME, which tells the VM to look up the current
            # value of that variable and push it onto the stack.
            name_index = self._add_name(node.name)
            self.instructions.append(Instruction(OpCode.LOAD_NAME, name_index))

        elif isinstance(node, BinaryOp):
            # A binary operation like 1 + 2. The compilation order is critical:
            #
            #   1. Compile left operand  -> pushes left value onto stack
            #   2. Compile right operand -> pushes right value onto stack
            #   3. Emit the operator     -> pops both, pushes result
            #
            # This is a post-order traversal of the expression tree, which
            # naturally produces Reverse Polish Notation (RPN). The stack
            # handles all the intermediate storage that explicit temporary
            # variables would handle in a register machine.
            self._compile_expression(node.left)
            self._compile_expression(node.right)
            opcode = OPERATOR_MAP[node.op]
            self.instructions.append(Instruction(opcode))

        else:
            raise TypeError(
                f"Unknown expression type: {type(node).__name__}. "
                f"The compiler doesn't know how to handle this AST node."
            )

    # -------------------------------------------------------------------
    # Pool management — constants and names
    # -------------------------------------------------------------------

    def _add_constant(self, value: int | str) -> int:
        """Add a constant to the pool, returning its index. Deduplicates.

        The constant pool is a list of literal values that appear in the source
        code. Instead of embedding the value directly in each instruction, we
        store it once in the pool and reference it by index. This has two
        benefits:

        1. **Space efficiency**: If the program uses ``42`` in ten places, we
           store it once and emit ``LOAD_CONST 0`` ten times, rather than
           embedding the number in each instruction.

        2. **Simplicity**: Instructions have a uniform format (opcode + integer
           operand), regardless of whether the constant is a small number or a
           long string.

        Deduplication means we check if the value is already in the pool before
        adding it. If ``42`` is already at index 0, we just return 0.

        Parameters
        ----------
        value : int | str
            The literal value to add (a number or string).

        Returns
        -------
        int
            The index of the value in the constant pool.
        """
        if value in self.constants:
            return self.constants.index(value)
        self.constants.append(value)
        return len(self.constants) - 1

    def _add_name(self, name: str) -> int:
        """Add a variable name to the name pool, returning its index. Deduplicates.

        The name pool works exactly like the constant pool, but for variable
        names instead of literal values. When the compiler sees ``x = 42``,
        it adds ``"x"`` to the name pool and emits ``STORE_NAME <index>``.
        Later, when it sees ``x`` used in an expression, it reuses the same
        index and emits ``LOAD_NAME <index>``.

        Deduplication is especially important for names because the same variable
        is typically used many times. Without deduplication, each reference to
        ``x`` would create a new entry, wasting space and potentially confusing
        the VM (which identifies variables by their name-pool index).

        Parameters
        ----------
        name : str
            The variable name string (e.g., "x", "total", "my_var").

        Returns
        -------
        int
            The index of the name in the name pool.
        """
        if name in self.names:
            return self.names.index(name)
        self.names.append(name)
        return len(self.names) - 1


# ---------------------------------------------------------------------------
# Convenience function for end-to-end compilation
# ---------------------------------------------------------------------------


def compile_source(
    source: str, keywords: list[str] | None = None
) -> CodeObject:
    """Convenience: source code string -> CodeObject in one call.

    This chains the entire front-end pipeline:

        Source code  ->  Lexer  ->  Tokens  ->  Parser  ->  AST  ->  Compiler  ->  CodeObject

    It's the simplest way to go from human-readable code to VM-executable
    bytecode. Under the hood, it creates a fresh Lexer, Parser, and
    BytecodeCompiler for each call.

    Parameters
    ----------
    source : str
        The source code to compile (e.g., ``"x = 1 + 2"``).
    keywords : list[str] | None, optional
        Language-specific keywords to pass to the lexer (e.g., ``["if", "while"]``).
        If ``None``, the lexer uses its default configuration.

    Returns
    -------
    CodeObject
        A compiled bytecode object ready for the VM to execute.

    Example
    -------
    ::

        from bytecode_compiler import compile_source
        from virtual_machine import VirtualMachine

        code = compile_source("x = 1 + 2")
        vm = VirtualMachine()
        vm.execute(code)
        print(vm.globals["x"])  # 3

    Notes
    -----
    For production use, you'd typically keep the lexer/parser/compiler separate
    so you can inspect or transform the intermediate representations. This
    function is for quick experiments and tests.
    """
    from lexer import Lexer, LexerConfig
    from lang_parser import Parser

    config = LexerConfig(keywords=keywords) if keywords else None
    tokens = Lexer(source, config).tokenize()
    ast = Parser(tokens).parse()
    return BytecodeCompiler().compile(ast)
