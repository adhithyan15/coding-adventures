"""Generic Compiler — A Pluggable AST-to-Bytecode Compiler Framework.

==========================================================================
Chapter 1: Why a *Generic* Compiler?
==========================================================================

The original ``BytecodeCompiler`` (in ``compiler.py``) compiles a specific
AST structure — ``Program``, ``Assignment``, ``BinaryOp``, etc. — produced
by the hand-written parser. But the grammar-driven parser produces a
different AST: ``ASTNode`` objects where each node's ``rule_name`` tells
you which grammar rule created it.

We need a compiler that can handle *any* grammar's AST, not just one
specific language. Enter the ``GenericCompiler``:

- It walks an ``ASTNode`` tree, dispatching on each node's ``rule_name``.
- Languages register ``rule_name → handler`` mappings via ``register_rule()``.
- The compiler provides universal helpers: emit instructions, manage constant
  and name pools, patch jumps, track labels, and compile nested scopes.

This is the compiler equivalent of the ``GenericVM``: the same chassis
(tree walking, instruction emission, pool management) with pluggable
language-specific behavior (what to do for ``assign_stmt``, ``if_stmt``, etc.).

==========================================================================
Chapter 2: How Compilation Works
==========================================================================

Compilation is a **tree walk**. The compiler starts at the root ``ASTNode``
(typically ``rule_name="file"``), looks up the handler for that rule name,
and calls it. The handler processes the node's children — some are tokens
(leaf values like ``42`` or ``x``), others are sub-nodes (like ``expression``
or ``assign_stmt``). For sub-nodes, the handler calls ``compile_node()``
recursively.

The result is a flat list of ``Instruction`` objects — the bytecode — plus
constant and name pools. Together these form a ``CodeObject`` that the VM
can execute.

**Example: compiling ``x = 1 + 2``**

The AST looks like::

    ASTNode(rule_name="file", children=[
        ASTNode(rule_name="statement", children=[
            ASTNode(rule_name="simple_stmt", children=[
                ASTNode(rule_name="small_stmt", children=[
                    ASTNode(rule_name="assign_stmt", children=[
                        ASTNode(rule_name="expression_list", children=[
                            ASTNode(rule_name="expression", children=[
                                ASTNode(rule_name="atom", children=[
                                    Token(type="NAME", value="x")
                                ])
                            ])
                        ]),
                        Token(type="EQUALS", value="="),
                        ASTNode(rule_name="expression_list", children=[
                            ASTNode(rule_name="arith", children=[
                                ASTNode(rule_name="atom", children=[
                                    Token(type="INT", value="1")
                                ]),
                                Token(type="PLUS", value="+"),
                                ASTNode(rule_name="atom", children=[
                                    Token(type="INT", value="2")
                                ])
                            ])
                        ])
                    ])
                ]),
                Token(type="NEWLINE", value="\\n")
            ])
        ])
    ])

The compiler dispatches on each ``rule_name``:
1. ``"file"`` handler → compiles each child statement
2. ``"statement"`` → pass-through (single child)
3. ``"simple_stmt"`` handler → compiles child small_stmts
4. ``"assign_stmt"`` handler → compiles RHS, emits STORE_NAME
5. ``"arith"`` handler → compiles left, compiles right, emits ADD
6. ``"atom"`` handler → emits LOAD_CONST for ``1`` and ``2``

==========================================================================
Chapter 3: Pass-Through Rules
==========================================================================

Many grammar rules exist only to encode operator precedence. For example,
the precedence chain for Starlark expressions is::

    expression → or_expr → and_expr → not_expr → comparison →
    bitwise_or → bitwise_xor → bitwise_and → shift → arith →
    term → factor → power → primary → atom

When parsing ``42``, the result is deeply nested::

    expression(or_expr(and_expr(not_expr(comparison(bitwise_or(
    bitwise_xor(bitwise_and(shift(arith(term(factor(power(
    primary(atom(Token("42")))))))))))))))

Most of these nodes have exactly one child and carry no additional semantics.
The compiler handles this with a **pass-through rule**: if a node has one
child and no registered handler, the compiler just recurses into that child.

This means you only need to register handlers for rules that actually *do*
something (``assign_stmt``, ``arith``, ``if_stmt``), not for every
precedence-encoding wrapper.

==========================================================================
Chapter 4: Jump Patching
==========================================================================

Control flow (if/else, for loops, short-circuit booleans) requires **jumps**
— instructions that change the program counter. But when you emit a jump,
you don't yet know the target address (because you haven't compiled the
code that comes after the jump body).

The solution is **backpatching**:

1. Emit a placeholder jump with operand ``0``.
2. Record the instruction's index.
3. Compile the body.
4. Now you know the target. **Patch** the placeholder with the real target.

Example for ``if x: body``::

    emit LOAD_NAME x
    emit JUMP_IF_FALSE 0     ← placeholder (index 2)
    compile body              ← emits instructions at indices 3, 4, 5
    patch instruction[2] with target=6  ← past the body

The ``emit_jump()`` and ``patch_jump()`` methods handle this pattern.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Protocol

from lang_parser import ASTNode
from lexer import Token
from virtual_machine import CodeObject, Instruction


# =========================================================================
# CompileHandler Protocol
# =========================================================================


class CompileHandler(Protocol):
    """Protocol for AST rule compilation handlers.

    Every rule handler must conform to this signature. It receives:

    - **compiler** — The GenericCompiler instance. Use ``compiler.emit()``,
      ``compiler.add_constant()``, etc. to emit bytecode.
    - **node** — The ASTNode to compile (its ``rule_name`` matched this handler).

    The handler should emit instructions by calling ``compiler.emit()`` and
    recurse into children by calling ``compiler.compile_node(child)``.

    Example::

        def compile_arith(compiler, node):
            # arith = term { ( PLUS | MINUS ) term }
            compiler.compile_node(node.children[0])  # first term
            i = 1
            while i < len(node.children):
                op_token = node.children[i]
                compiler.compile_node(node.children[i + 1])
                if op_token.value == "+":
                    compiler.emit(0x20)  # ADD
                else:
                    compiler.emit(0x21)  # SUB
                i += 2
    """

    def __call__(
        self,
        compiler: GenericCompiler,
        node: ASTNode,
    ) -> None: ...


# =========================================================================
# Compile Errors
# =========================================================================


class CompilerError(Exception):
    """Base class for all compilation errors.

    Raised when the compiler encounters an AST structure it can't handle,
    or when the generated bytecode would be invalid.
    """


class UnhandledRuleError(CompilerError):
    """Raised when no handler is registered for an AST rule.

    This happens when the compiler encounters a rule_name it doesn't know
    how to compile. Unlike pass-through rules (which have exactly one child),
    multi-child rules without a handler indicate a missing compilation rule.
    """


# =========================================================================
# Scope Tracking
# =========================================================================


@dataclass
class CompilerScope:
    """Tracks local variable names within a function scope.

    When compiling a function body, local variables are assigned numbered
    slots for fast access. The scope tracks which names have been assigned
    which slots, and provides the total count of locals needed.

    This mirrors what CPython does: each function has a co_varnames tuple
    listing its local variable names, and LOAD_FAST/STORE_FAST reference
    them by index.

    Attributes:
        locals: Mapping from variable name to slot index.
        parent: The enclosing scope (for closures), or None for module scope.
    """

    locals: dict[str, int] = field(default_factory=dict)
    parent: CompilerScope | None = None

    def add_local(self, name: str) -> int:
        """Register a local variable and return its slot index.

        If the variable is already registered, returns its existing index.
        """
        if name in self.locals:
            return self.locals[name]
        index = len(self.locals)
        self.locals[name] = index
        return index

    def get_local(self, name: str) -> int | None:
        """Look up a local variable's slot index. Returns None if not found."""
        return self.locals.get(name)

    @property
    def num_locals(self) -> int:
        """Total number of local variables in this scope."""
        return len(self.locals)


# =========================================================================
# The Generic Compiler
# =========================================================================


class GenericCompiler:
    """A pluggable AST-to-bytecode compiler framework.

    This is the **compiler chassis** — it provides universal compilation
    primitives that every bytecode compiler needs. Languages register their
    AST rule handlers via ``register_rule()`` to teach the compiler how to
    handle language-specific constructs.

    **What GenericCompiler provides (universal):**

    - Instruction emission (``emit()``)
    - Constant pool management (``add_constant()``)
    - Name pool management (``add_name()``)
    - Jump patching (``emit_jump()``, ``patch_jump()``)
    - Current instruction offset (``current_offset``)
    - AST dispatch (``compile_node()``)
    - Pass-through for single-child nodes
    - Scope tracking for local variables
    - Nested CodeObject compilation (for function bodies)

    **What plugins provide (language-specific):**

    - Rule handlers for each grammar rule (``register_rule()``)
    - Opcode numbers and their semantics

    Usage::

        compiler = GenericCompiler()

        # Register rule handlers
        compiler.register_rule("file", compile_file)
        compiler.register_rule("assign_stmt", compile_assign)
        compiler.register_rule("arith", compile_binary_op)

        # Compile an AST
        ast = parse_starlark("x = 1 + 2\\n")
        code = compiler.compile(ast)
    """

    def __init__(self) -> None:
        """Initialize an empty compiler with no instructions or handlers."""
        # -- Bytecode output ---------------------------------------------------
        self.instructions: list[Instruction] = []
        """The bytecode instructions emitted so far, in order."""

        self.constants: list[Any] = []
        """The constant pool — literal values referenced by LOAD_CONST."""

        self.names: list[str] = []
        """The name pool — variable/function names referenced by index."""

        # -- Plugin registry ---------------------------------------------------
        self._dispatch: dict[str, CompileHandler] = {}
        """Rule name → handler mapping. Languages register handlers here."""

        # -- Scope tracking ----------------------------------------------------
        self.scope: CompilerScope | None = None
        """Current compilation scope. None for module-level code."""

        # -- Nested code objects -----------------------------------------------
        self._code_objects: list[CodeObject] = []
        """Compiled CodeObjects for nested functions/lambdas."""

    # =====================================================================
    # Plugin Registration
    # =====================================================================

    def register_rule(self, rule_name: str, handler: CompileHandler) -> None:
        """Register a compilation handler for a grammar rule.

        Parameters
        ----------
        rule_name : str
            The grammar rule name (e.g., "assign_stmt", "if_stmt", "arith").
        handler : CompileHandler
            A callable ``(compiler, node) -> None`` that compiles AST nodes
            with this rule name.

        Example::

            def compile_assign(compiler, node):
                # Compile RHS, emit STORE_NAME for LHS
                ...

            compiler.register_rule("assign_stmt", compile_assign)
        """
        self._dispatch[rule_name] = handler

    # =====================================================================
    # Instruction Emission
    # =====================================================================

    def emit(self, opcode: int, operand: int | str | None = None) -> int:
        """Emit a single bytecode instruction.

        Parameters
        ----------
        opcode : int
            The opcode number (e.g., 0x01 for LOAD_CONST, 0x20 for ADD).
        operand : int or str or None
            Optional operand for the instruction.

        Returns
        -------
        int
            The index of the emitted instruction (useful for jump patching).
        """
        index = len(self.instructions)
        self.instructions.append(Instruction(opcode, operand))
        return index

    def emit_jump(self, opcode: int) -> int:
        """Emit a jump instruction with a placeholder target of 0.

        The returned index is used later with ``patch_jump()`` to fill in
        the real target once you know it.

        Parameters
        ----------
        opcode : int
            The jump opcode (JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE, etc.).

        Returns
        -------
        int
            The index of the emitted jump instruction (for patching).

        Example::

            # Emit a conditional jump (target unknown)
            jump_idx = compiler.emit_jump(JUMP_IF_FALSE)
            # ... compile the body ...
            # Now patch the jump to skip past the body
            compiler.patch_jump(jump_idx)
        """
        return self.emit(opcode, 0)  # placeholder target

    def patch_jump(self, index: int, target: int | None = None) -> None:
        """Patch a previously emitted jump with its real target.

        Parameters
        ----------
        index : int
            The instruction index returned by ``emit_jump()``.
        target : int or None
            The target instruction index. If None, patches to the current
            instruction offset (the most common case — "jump to here").
        """
        if target is None:
            target = self.current_offset
        self.instructions[index] = Instruction(
            self.instructions[index].opcode, target
        )

    @property
    def current_offset(self) -> int:
        """The index where the next emitted instruction will go.

        This is the current length of the instruction list. Used for
        jump targets — "the instruction I'm about to emit will be at
        this offset."
        """
        return len(self.instructions)

    # =====================================================================
    # Constant and Name Pool Management
    # =====================================================================

    def add_constant(self, value: Any) -> int:
        """Add a value to the constant pool, returning its index.

        Deduplicates: if the value already exists in the pool, returns
        the existing index rather than adding a duplicate.

        Parameters
        ----------
        value : Any
            The literal value to add (int, float, str, bool, None, etc.).

        Returns
        -------
        int
            The index of the value in the constant pool.
        """
        # We check both value and type to avoid deduplication bugs like
        # True == 1 in Python (we want them to be separate constants).
        for i, existing in enumerate(self.constants):
            if existing is value or (existing == value and type(existing) is type(value)):
                return i
        self.constants.append(value)
        return len(self.constants) - 1

    def add_name(self, name: str) -> int:
        """Add a variable/function name to the name pool, returning its index.

        Deduplicates: reuses existing entries.

        Parameters
        ----------
        name : str
            The identifier string.

        Returns
        -------
        int
            The index of the name in the name pool.
        """
        if name in self.names:
            return self.names.index(name)
        self.names.append(name)
        return len(self.names) - 1

    # =====================================================================
    # Scope Management
    # =====================================================================

    def enter_scope(self, params: list[str] | None = None) -> CompilerScope:
        """Enter a new local scope (for function bodies).

        Creates a new CompilerScope and pushes it. If parameter names are
        provided, they're registered as the first local slots.

        Parameters
        ----------
        params : list[str] or None
            Function parameter names to pre-register as locals.

        Returns
        -------
        CompilerScope
            The new scope.
        """
        scope = CompilerScope(parent=self.scope)
        if params:
            for name in params:
                scope.add_local(name)
        self.scope = scope
        return scope

    def exit_scope(self) -> CompilerScope:
        """Exit the current scope and return to the parent.

        Returns
        -------
        CompilerScope
            The scope that was exited (in case you need its locals info).
        """
        if self.scope is None:
            raise CompilerError("Cannot exit scope — not in any scope")
        old = self.scope
        self.scope = self.scope.parent
        return old

    # =====================================================================
    # Nested Code Object Compilation
    # =====================================================================

    def compile_nested(self, node: ASTNode) -> CodeObject:
        """Compile a sub-tree into a separate CodeObject (for function bodies).

        This saves the current compiler state, compiles the node into a
        fresh instruction list, builds a CodeObject, and restores the
        original state.

        Parameters
        ----------
        node : ASTNode
            The AST subtree to compile as a separate CodeObject.

        Returns
        -------
        CodeObject
            The compiled CodeObject for the nested function/lambda.
        """
        # Save current state
        saved_instructions = self.instructions
        saved_constants = self.constants
        saved_names = self.names

        # Fresh state for the nested compilation
        self.instructions = []
        self.constants = []
        self.names = []

        # Compile the nested code
        self.compile_node(node)

        # Build the nested CodeObject
        nested = CodeObject(
            instructions=self.instructions,
            constants=self.constants,
            names=self.names,
        )
        self._code_objects.append(nested)

        # Restore parent state
        self.instructions = saved_instructions
        self.constants = saved_constants
        self.names = saved_names

        return nested

    # =====================================================================
    # AST Dispatch — the recursive core
    # =====================================================================

    def compile_node(self, node: ASTNode | Token) -> None:
        """Compile an AST node or token by dispatching to the registered handler.

        This is the heart of the compiler. For each node:

        1. If it's a ``Token``, call ``compile_token()`` (which can be
           overridden by registering a token handler).
        2. If it's an ``ASTNode`` with a registered handler, call that handler.
        3. If it's an ``ASTNode`` with exactly one child and no handler,
           pass through to the child (precedence-encoding nodes).
        4. Otherwise, raise ``UnhandledRuleError``.

        Parameters
        ----------
        node : ASTNode or Token
            The AST element to compile.

        Raises
        ------
        UnhandledRuleError
            If no handler is registered and the node has multiple children.
        """
        if isinstance(node, Token):
            self.compile_token(node)
            return

        handler = self._dispatch.get(node.rule_name)
        if handler is not None:
            handler(self, node)
        elif len(node.children) == 1:
            # Pass-through: single-child node with no handler → recurse
            self.compile_node(node.children[0])
        else:
            raise UnhandledRuleError(
                f"No handler registered for rule '{node.rule_name}' "
                f"and it has {len(node.children)} children (not a pass-through). "
                f"Register a handler with compiler.register_rule('{node.rule_name}', handler)."
            )

    def compile_token(self, token: Token) -> None:
        """Compile a bare token (not wrapped in an ASTNode).

        By default, this is a no-op — tokens that appear directly in the
        tree (like NEWLINE, SEMICOLON, INDENT, DEDENT) are usually structural
        and don't need compilation. Language-specific handlers handle
        meaningful tokens (like INT, STRING, NAME) when they appear as
        children of specific rules.

        Override this in a subclass or handle tokens within rule handlers.
        """
        # Structural tokens (NEWLINE, INDENT, DEDENT, etc.) are ignored.
        # Meaningful tokens are handled by their parent rule's handler.
        pass

    # =====================================================================
    # Top-Level Compile API
    # =====================================================================

    def compile(self, ast: ASTNode, halt_opcode: int = 0xFF) -> CodeObject:
        """Compile an AST into a CodeObject.

        This is the main entry point. It compiles the root AST node into
        bytecode instructions, appends a HALT instruction, and returns
        a CodeObject ready for the VM to execute.

        Parameters
        ----------
        ast : ASTNode
            The root AST node (e.g., ``rule_name="file"``).
        halt_opcode : int
            The opcode for the HALT instruction (default: 0xFF).

        Returns
        -------
        CodeObject
            The compiled bytecode, ready for the VM.
        """
        self.compile_node(ast)
        self.emit(halt_opcode)

        return CodeObject(
            instructions=self.instructions,
            constants=self.constants,
            names=self.names,
        )
