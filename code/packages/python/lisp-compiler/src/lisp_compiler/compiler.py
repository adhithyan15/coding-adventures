"""Lisp Compiler — Transforms Lisp ASTs into GenericVM bytecode.

==========================================================================
Chapter 1: How Lisp Compilation Differs from Starlark
==========================================================================

Starlark has many grammar rules (50+) because it has statements, expressions,
assignments, comprehensions, etc. The compiler registers a handler for each rule.

Lisp is radically simpler. There are only 6 grammar rules::

    program   = { sexpr } ;
    sexpr     = atom | list | quoted ;
    atom      = NUMBER | SYMBOL | STRING ;
    list      = LPAREN list_body RPAREN ;
    list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;
    quoted    = QUOTE sexpr ;

The grammar doesn't distinguish ``(define x 1)`` from ``(+ 1 2)`` from
``(lambda (n) n)``. They're all just "list" nodes. The *compiler* inspects
the first element of each list to decide what to do:

- If the first element is ``define`` → compile as a definition
- If the first element is ``lambda`` → compile as a function
- If the first element is ``cond`` → compile as conditional
- If the first element is ``+`` → compile as arithmetic
- Otherwise → compile as a function call

This is why Lisp is called "homoiconic" — code and data share the same
structure. The compiler's job is to assign *meaning* to that structure.

==========================================================================
Chapter 2: Special Forms vs. Function Calls
==========================================================================

"Special forms" are syntactic constructs that the compiler handles directly.
They cannot be implemented as functions because they control evaluation order.

For example, ``(cond (p1 e1) (p2 e2))`` should NOT evaluate all branches —
only the one whose predicate is true. A regular function call would evaluate
all arguments before calling. So ``cond`` must be a special form.

McCarthy's original 7 special forms plus our additions:

    quote   — Return data without evaluating
    atom    — Test if value is an atom (not a cons cell)
    eq      — Test equality
    car     — First element of cons cell
    cdr     — Second element of cons cell
    cons    — Create a cons cell
    cond    — Conditional branching

    lambda  — Create a function (closure)
    define  — Bind a name to a value
    +,-,*,/ — Arithmetic (could be functions, but inlining is faster)
    <, >, = — Comparison operators
    print   — Output a value

Everything else is compiled as a function call.

==========================================================================
Chapter 3: Tail Call Optimization
==========================================================================

A call is in "tail position" when it's the last thing a function does
before returning. The compiler tracks tail position and emits TAIL_CALL
instead of CALL_FUNCTION for calls in tail position.

Tail positions:
- The body (last expression) of a lambda
- The consequent expression in each ``cond`` branch
- NOT: arguments to other calls
- NOT: predicate positions in ``cond``
- NOT: the top level of a program

This is a *compiler* concern — the VM's TAIL_CALL handler is generic.

==========================================================================
Chapter 4: Compiling Quoted Data
==========================================================================

``(quote x)`` or ``'x`` returns x as data, not code. This means the
compiler must emit instructions to *construct* the data at runtime:

- ``(quote 42)``     → LOAD_CONST 42
- ``(quote foo)``    → MAKE_SYMBOL "foo"
- ``(quote (1 2 3))``→ Build a cons chain: LOAD_NIL, LOAD_CONST 3, CONS,
                        LOAD_CONST 2, CONS, LOAD_CONST 1, CONS

The last case is interesting: to build ``(1 2 3)`` we need to construct
the cons chain from right to left:

    (1 . (2 . (3 . NIL)))

So we push NIL, then 3, CONS, then 2, CONS, then 1, CONS.
"""

from __future__ import annotations

from bytecode_compiler import GenericCompiler
from lang_parser import ASTNode
from lexer import Token
from lisp_parser import parse_lisp
from lisp_vm import NIL, LispOp, create_lisp_vm
from virtual_machine import CodeObject

# =========================================================================
# AST Navigation Helpers
# =========================================================================
#
# The grammar-driven parser produces generic ASTNode objects. These helpers
# make it easier to extract the parts we care about.
# =========================================================================


def _get_rule(node: ASTNode | Token) -> str | None:  # type: ignore[arg-type]
    """Get the grammar rule name from a node, or None for tokens."""
    return node.rule_name if hasattr(node, "rule_name") else None


def _get_children(node: ASTNode | Token) -> list[ASTNode | Token]:  # type: ignore[arg-type]
    """Get children from a node, or empty list for tokens."""
    return node.children if hasattr(node, "children") else []


def _type_name(token: Token) -> str:
    """Get token type as a string, handling both str and enum types."""
    if isinstance(token.type, str):
        return token.type
    return token.type.name


def _unwrap_sexpr(node: ASTNode) -> ASTNode:
    """Unwrap a sexpr node to its child (atom, list, or quoted).

    The grammar wraps everything in sexpr nodes. Most of the time
    we want to look at the inner node directly.
    """
    if _get_rule(node) == "sexpr":
        children = _get_children(node)
        if len(children) == 1:
            return children[0]
    return node


def _is_atom(node: ASTNode) -> bool:
    """Check if a node is an atom (after unwrapping sexpr)."""
    inner = _unwrap_sexpr(node)
    return _get_rule(inner) == "atom"


def _is_list(node: ASTNode) -> bool:
    """Check if a node is a list (after unwrapping sexpr)."""
    inner = _unwrap_sexpr(node)
    return _get_rule(inner) == "list"


def _is_quoted(node: ASTNode) -> bool:
    """Check if a node is a quoted form (after unwrapping sexpr)."""
    inner = _unwrap_sexpr(node)
    return _get_rule(inner) == "quoted"


def _atom_token(node: ASTNode) -> Token:
    """Extract the token from an atom node (unwrapping sexpr if needed)."""
    inner = _unwrap_sexpr(node)
    assert _get_rule(inner) == "atom", f"Expected atom, got {_get_rule(inner)}"
    return _get_children(inner)[0]


def _atom_value(node: ASTNode) -> str:
    """Get the string value from an atom node."""
    return _atom_token(node).value


def _atom_type(node: ASTNode) -> str:
    """Get the type name (NUMBER, SYMBOL, STRING) from an atom node."""
    return _type_name(_atom_token(node))


def _list_elements(node: ASTNode) -> list[ASTNode]:
    """Get the s-expression children of a list node.

    Strips LPAREN, RPAREN tokens and unwraps the list_body node,
    returning only the sexpr children.
    """
    inner = _unwrap_sexpr(node)
    assert _get_rule(inner) == "list", f"Expected list, got {_get_rule(inner)}"
    children = _get_children(inner)
    # Find the list_body child
    for child in children:
        if _get_rule(child) == "list_body":
            # list_body's children are sexprs (and possibly DOT + sexpr)
            return [c for c in _get_children(child) if _get_rule(c) == "sexpr"]
    return []


def _first_symbol(node: ASTNode) -> str | None:
    """If node is a list whose first element is a symbol, return that symbol.

    This is how we identify special forms: ``(define ...)``, ``(lambda ...)``, etc.
    """
    if not _is_list(node):
        return None
    elements = _list_elements(node)
    if not elements:
        return None
    first = elements[0]
    if _is_atom(first) and _atom_type(first) == "SYMBOL":
        return _atom_value(first)
    return None


# =========================================================================
# The Compiler
# =========================================================================

# Arithmetic and comparison operator maps — maps symbol names to opcodes
ARITHMETIC_OPS: dict[str, int] = {
    "+": LispOp.ADD,
    "-": LispOp.SUB,
    "*": LispOp.MUL,
    "/": LispOp.DIV,
}

COMPARISON_OPS: dict[str, int] = {
    "=": LispOp.CMP_EQ,
    "<": LispOp.CMP_LT,
    ">": LispOp.CMP_GT,
}


class LispCompiler:
    """Compiles Lisp ASTs into bytecode for the GenericVM.

    This compiler processes the grammar-driven AST and emits LispOp
    instructions. It handles special forms, function calls, quoted data,
    and tail call optimization.

    Usage::

        compiler = LispCompiler()
        code = compiler.compile(ast)
    """

    def __init__(self) -> None:
        self._compiler = GenericCompiler()
        self._tail_position = False
        self._in_function = False
        # Register grammar rule handlers
        self._compiler.register_rule("program", self._compile_program)
        self._compiler.register_rule("sexpr", self._compile_sexpr)
        self._compiler.register_rule("atom", self._compile_atom)
        self._compiler.register_rule("list", self._compile_list)
        self._compiler.register_rule("quoted", self._compile_quoted)
        self._compiler.register_rule("list_body", self._compile_list_body)

    def compile(self, ast: ASTNode) -> CodeObject:
        """Compile a Lisp AST into a CodeObject.

        Args:
            ast: The root AST node from the parser.

        Returns:
            A CodeObject with instructions, constants, and names.
        """
        return self._compiler.compile(ast, halt_opcode=LispOp.HALT)

    # -----------------------------------------------------------------
    # Grammar Rule Handlers
    # -----------------------------------------------------------------

    def _compile_program(
        self, compiler: GenericCompiler, node: ASTNode,
    ) -> None:
        """Compile a program — a sequence of top-level s-expressions.

        Grammar: program = { sexpr } ;

        Each s-expression is compiled in order. The result of the last
        expression is left on the stack (all others are popped).
        """
        children = [c for c in _get_children(node) if _get_rule(c) == "sexpr"]
        for i, child in enumerate(children):
            self._compile_sexpr(compiler, child)
            # Pop intermediate results, keep the last one
            if i < len(children) - 1:
                compiler.emit(LispOp.POP)

    def _compile_sexpr(
        self, compiler: GenericCompiler, node: ASTNode,
    ) -> None:
        """Compile an s-expression — dispatch to atom, list, or quoted.

        Grammar: sexpr = atom | list | quoted ;

        This is just a pass-through wrapper. The real work happens in the
        specific rule handlers.
        """
        children = _get_children(node)
        if len(children) == 1:
            compiler.compile_node(children[0])

    def _compile_atom(
        self, compiler: GenericCompiler, node: ASTNode,
    ) -> None:
        """Compile an atom — a number, symbol, or string literal.

        Grammar: atom = NUMBER | SYMBOL | STRING ;

        Numbers become LOAD_CONST instructions. Symbols become LOAD_NAME
        (variable lookups). The special symbols ``nil`` and ``t`` are
        handled specially.
        """
        token = _get_children(node)[0]
        ttype = _type_name(token)

        if ttype == "NUMBER":
            idx = compiler.add_constant(int(token.value))
            compiler.emit(LispOp.LOAD_CONST, idx)
        elif ttype == "STRING":
            # The lexer already strips surrounding quotes
            idx = compiler.add_constant(token.value)
            compiler.emit(LispOp.LOAD_CONST, idx)
        elif ttype == "SYMBOL":
            name = token.value
            if name == "nil":
                compiler.emit(LispOp.LOAD_NIL)
            elif name == "t":
                compiler.emit(LispOp.LOAD_TRUE)
            else:
                # Look up in local scope first, then global
                if compiler.scope and compiler.scope.get_local(name) is not None:
                    slot = compiler.scope.get_local(name)
                    compiler.emit(LispOp.LOAD_LOCAL, slot)
                else:
                    idx = compiler.add_name(name)
                    compiler.emit(LispOp.LOAD_NAME, idx)

    def _compile_list(
        self, compiler: GenericCompiler, node: ASTNode,
    ) -> None:
        """Compile a list — the workhorse of Lisp compilation.

        Grammar: list = LPAREN list_body RPAREN ;

        A list could be:
        - A special form: (define ...), (lambda ...), (cond ...), etc.
        - An arithmetic op: (+ 1 2), (* 3 4)
        - A comparison: (eq x y), (< a b)
        - A function call: (f x y)
        - An empty list: ()

        We inspect the first element to decide which case applies.
        """
        elements = _list_elements(node)

        if not elements:
            # Empty list () → NIL
            compiler.emit(LispOp.LOAD_NIL)
            return

        first_sym = _first_symbol(node)

        if first_sym is not None:
            # Dispatch to special form handlers
            if first_sym == "define":
                self._compile_define(compiler, elements)
            elif first_sym == "lambda":
                self._compile_lambda(compiler, elements)
            elif first_sym == "cond":
                self._compile_cond(compiler, elements)
            elif first_sym == "quote":
                self._compile_quote_form(compiler, elements)
            elif first_sym == "cons":
                self._compile_cons(compiler, elements)
            elif first_sym == "car":
                self._compile_unary_op(compiler, elements, LispOp.CAR)
            elif first_sym == "cdr":
                self._compile_unary_op(compiler, elements, LispOp.CDR)
            elif first_sym == "atom":
                self._compile_unary_op(compiler, elements, LispOp.IS_ATOM)
            elif first_sym == "eq":
                self._compile_binary_op(compiler, elements, LispOp.CMP_EQ)
            elif first_sym == "print":
                self._compile_unary_op(compiler, elements, LispOp.PRINT)
            elif first_sym in ARITHMETIC_OPS:
                self._compile_binary_op(
                    compiler, elements, ARITHMETIC_OPS[first_sym],
                )
            elif first_sym in COMPARISON_OPS:
                self._compile_binary_op(
                    compiler, elements, COMPARISON_OPS[first_sym],
                )
            elif first_sym == "is-nil":
                self._compile_unary_op(compiler, elements, LispOp.IS_NIL)
            else:
                # Not a special form — compile as function call
                self._compile_call(compiler, elements)
        else:
            # First element is not a symbol — must be a computed call
            # e.g., ((lambda (x) x) 42)
            self._compile_call(compiler, elements)

    def _compile_list_body(
        self, compiler: GenericCompiler, node: ASTNode,
    ) -> None:
        """Compile list_body — should not be called directly.

        Grammar: list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;

        The list handler extracts elements from list_body directly.
        This handler exists as a fallback but shouldn't be reached.
        """
        # This is handled by _compile_list via _list_elements
        for child in _get_children(node):
            if _get_rule(child) == "sexpr":
                compiler.compile_node(child)

    def _compile_quoted(
        self, compiler: GenericCompiler, node: ASTNode,
    ) -> None:
        """Compile a quoted form: 'x → (quote x).

        Grammar: quoted = QUOTE sexpr ;

        The QUOTE token is syntactic sugar. We extract the sexpr and
        compile it as quoted data (not code).
        """
        children = _get_children(node)
        # children[0] is the QUOTE token, children[1] is the sexpr
        sexpr = children[1]
        self._compile_quoted_datum(compiler, sexpr)

    # -----------------------------------------------------------------
    # Special Form Compilation
    # -----------------------------------------------------------------

    def _compile_define(
        self, compiler: GenericCompiler, elements: list[ASTNode],
    ) -> None:
        """Compile (define name expr).

        Evaluates expr and binds the result to name in the global scope.

        Example::

            (define x 42)       → LOAD_CONST 42, STORE_NAME "x"
            (define f (lambda (n) n)) → compile lambda, STORE_NAME "f"
        """
        n_args = len(elements) - 1
        assert n_args == 2, f"define expects 2 arguments, got {n_args}"
        name_node = elements[1]
        assert _is_atom(name_node), "define name must be a symbol"
        name = _atom_value(name_node)

        # Compile the value expression
        value_node = elements[2]
        saved_tail = self._tail_position
        self._tail_position = False
        self._compile_sexpr(compiler, value_node)
        self._tail_position = saved_tail

        # Store in global scope
        idx = compiler.add_name(name)
        compiler.emit(LispOp.STORE_NAME, idx)

        # define returns NIL (the stored value is a side effect)
        compiler.emit(LispOp.LOAD_NIL)

    def _compile_lambda(
        self, compiler: GenericCompiler, elements: list[ASTNode],
    ) -> None:
        """Compile (lambda (params...) body).

        Creates a closure: compile the body as a nested CodeObject,
        push it as a constant, then emit MAKE_CLOSURE.

        The body is compiled in tail position — any call at the end
        of the body will use TAIL_CALL instead of CALL_FUNCTION.

        Example::

            (lambda (x y) (+ x y))

        Compiles body as:
            LOAD_LOCAL 0     ; x
            LOAD_LOCAL 1     ; y
            ADD
            RETURN

        Then at the call site:
            LOAD_CONST <body CodeObject>
            MAKE_CLOSURE <param_count>
        """
        assert len(elements) >= 3, "lambda needs params and body"
        param_node = elements[1]
        assert _is_list(param_node), "lambda params must be a list"

        # Extract parameter names
        param_elements = _list_elements(param_node)
        params = []
        for p in param_elements:
            assert _is_atom(p) and _atom_type(p) == "SYMBOL", \
                f"lambda parameter must be a symbol, got {p}"
            params.append(_atom_value(p))

        # Compile body as a nested CodeObject
        # The body is the last element (or all elements after params)
        body_nodes = elements[2:]

        # Enter a new scope with parameter names
        compiler.enter_scope(params)

        # Save and set state for function body
        saved_tail = self._tail_position
        saved_in_func = self._in_function
        self._in_function = True

        # Save current compiler state
        saved_instructions = compiler.instructions[:]
        saved_constants = compiler.constants[:]
        saved_names = compiler.names[:]
        compiler.instructions.clear()
        compiler.constants.clear()
        compiler.names.clear()

        # We'll store param names as a tuple constant in the body
        # CodeObject so MAKE_CLOSURE can extract real names for
        # variable binding (needed for closure capture).

        # Compile body expressions — last one is in tail position
        for i, body_expr in enumerate(body_nodes):
            is_last = i == len(body_nodes) - 1
            self._tail_position = is_last
            self._compile_sexpr(compiler, body_expr)
            if not is_last:
                compiler.emit(LispOp.POP)

        # Emit RETURN at the end of the function body
        compiler.emit(LispOp.RETURN)

        # Store param names as a tuple in the constants pool.
        # MAKE_CLOSURE extracts this to get real parameter names
        # for variable binding (needed so inner closures can capture
        # outer lambda parameters).
        if params:
            compiler.constants.append(tuple(params))

        # Capture the compiled body
        body_code = CodeObject(
            instructions=compiler.instructions[:],
            constants=compiler.constants[:],
            names=compiler.names[:],
        )

        # Restore compiler state
        compiler.instructions.clear()
        compiler.instructions.extend(saved_instructions)
        compiler.constants.clear()
        compiler.constants.extend(saved_constants)
        compiler.names.clear()
        compiler.names.extend(saved_names)

        # Restore tail position state
        self._tail_position = saved_tail
        self._in_function = saved_in_func

        # Exit the scope
        compiler.exit_scope()

        # Push the body CodeObject as a constant
        idx = compiler.add_constant(body_code)
        compiler.emit(LispOp.LOAD_CONST, idx)

        # Emit MAKE_CLOSURE with param count
        compiler.emit(LispOp.MAKE_CLOSURE, len(params))

    def _compile_cond(
        self, compiler: GenericCompiler, elements: list[ASTNode],
    ) -> None:
        """Compile (cond (pred1 expr1) (pred2 expr2) ...).

        Each clause is a list (predicate expression). We compile:

            compile pred1
            JUMP_IF_FALSE → next_clause
            compile expr1
            JUMP → end
        next_clause:
            compile pred2
            JUMP_IF_FALSE → next_clause2
            compile expr2
            JUMP → end
        ...
        end:

        The special symbol ``t`` as a predicate means "always true" — the
        else clause.

        Expressions in cond branches inherit the current tail position.
        """
        clauses = elements[1:]  # Skip 'cond' itself

        # Collect jumps-to-end for patching
        end_jumps: list[int] = []

        for clause in clauses:
            assert _is_list(clause), "cond clause must be a list"
            clause_parts = _list_elements(clause)
            assert len(clause_parts) >= 2, "cond clause needs predicate and expression"

            predicate = clause_parts[0]
            # The expression is the last element (could have multiple)
            expression = clause_parts[-1]

            # Check if predicate is 't' (always true — the else clause)
            is_else = _is_atom(predicate) and _atom_value(predicate) == "t"

            if is_else:
                # No conditional jump needed — just compile the expression
                saved_tail = self._tail_position
                # Expression inherits tail position from enclosing context
                self._compile_sexpr(compiler, expression)
                self._tail_position = saved_tail
            else:
                # Compile predicate (never in tail position)
                saved_tail = self._tail_position
                self._tail_position = False
                self._compile_sexpr(compiler, predicate)
                self._tail_position = saved_tail

                # Jump past this clause if predicate is false
                false_jump = compiler.emit_jump(LispOp.JUMP_IF_FALSE)

                # Compile expression (inherits tail position)
                saved_tail2 = self._tail_position
                self._compile_sexpr(compiler, expression)
                self._tail_position = saved_tail2

                # Jump to end after executing this branch
                end_jump = compiler.emit_jump(LispOp.JUMP)
                end_jumps.append(end_jump)

                # Patch the false jump to here (next clause)
                compiler.patch_jump(false_jump)

        # If no else clause, push NIL as default
        if not clauses or not (
            _is_atom(
                _list_elements(clauses[-1])[0],
            )
            and _atom_value(_list_elements(clauses[-1])[0]) == "t"
        ):
            compiler.emit(LispOp.LOAD_NIL)

        # Patch all end jumps to here
        for j in end_jumps:
            compiler.patch_jump(j)

    def _compile_quote_form(
        self, compiler: GenericCompiler, elements: list[ASTNode],
    ) -> None:
        """Compile (quote datum).

        Emits instructions to construct the datum as data, not code.
        """
        assert len(elements) == 2, "quote takes exactly 1 argument"
        datum = elements[1]
        self._compile_quoted_datum(compiler, datum)

    def _compile_quoted_datum(
        self, compiler: GenericCompiler, node: ASTNode,
    ) -> None:
        """Compile a datum as quoted data (not code).

        - Numbers → LOAD_CONST
        - Symbols → MAKE_SYMBOL
        - Lists → build a cons chain from right to left
        - Nested quotes → recursive

        This is where we build data structures at runtime. For example,
        ``(quote (1 2 3))`` builds the cons chain:

            (1 . (2 . (3 . NIL)))

        By emitting: LOAD_NIL, LOAD_CONST 3, CONS, LOAD_CONST 2, CONS,
                     LOAD_CONST 1, CONS
        """
        if _is_atom(node):
            token = _atom_token(node)
            ttype = _type_name(token)
            if ttype == "NUMBER":
                idx = compiler.add_constant(int(token.value))
                compiler.emit(LispOp.LOAD_CONST, idx)
            elif ttype == "STRING":
                idx = compiler.add_constant(token.value)
                compiler.emit(LispOp.LOAD_CONST, idx)
            elif ttype == "SYMBOL":
                name = token.value
                if name == "nil":
                    compiler.emit(LispOp.LOAD_NIL)
                else:
                    # Intern the symbol
                    idx = compiler.add_constant(name)
                    compiler.emit(LispOp.MAKE_SYMBOL, idx)
        elif _is_list(node):
            elements = _list_elements(node)
            if not elements:
                compiler.emit(LispOp.LOAD_NIL)
            else:
                # Build cons chain from right to left:
                # (a b c) → (a . (b . (c . NIL)))
                # Push NIL, then c, CONS, then b, CONS, then a, CONS
                compiler.emit(LispOp.LOAD_NIL)
                for elem in reversed(elements):
                    self._compile_quoted_datum(compiler, elem)
                    compiler.emit(LispOp.CONS)
        elif _is_quoted(node):
            # Nested quote: ''x
            inner = _unwrap_sexpr(node)
            children = _get_children(inner)
            self._compile_quoted_datum(compiler, children[1])

    def _compile_cons(
        self, compiler: GenericCompiler, elements: list[ASTNode],
    ) -> None:
        """Compile (cons car cdr).

        Evaluates both arguments and creates a cons cell.
        The GC allocates the cell on the heap.

        Stack effect: pushes cdr first, then car, then CONS pops both
        and pushes the heap address.
        """
        assert len(elements) == 3, "cons takes exactly 2 arguments"
        saved_tail = self._tail_position
        self._tail_position = False

        # Push cdr first (it goes below car on the stack)
        self._compile_sexpr(compiler, elements[2])
        # Then car
        self._compile_sexpr(compiler, elements[1])

        self._tail_position = saved_tail
        compiler.emit(LispOp.CONS)

    def _compile_unary_op(
        self,
        compiler: GenericCompiler,
        elements: list[ASTNode],
        opcode: int,
    ) -> None:
        """Compile a unary operation: (op arg).

        Used for car, cdr, atom, is-nil, print.
        """
        n = len(elements) - 1
        assert n == 1, f"Unary op expects 1 argument, got {n}"
        saved_tail = self._tail_position
        self._tail_position = False
        self._compile_sexpr(compiler, elements[1])
        self._tail_position = saved_tail
        compiler.emit(opcode)

    def _compile_binary_op(
        self,
        compiler: GenericCompiler,
        elements: list[ASTNode],
        opcode: int,
    ) -> None:
        """Compile a binary operation: (op left right).

        Used for arithmetic (+, -, *, /), comparison (eq, <, >).
        """
        n = len(elements) - 1
        assert n == 2, f"Binary op expects 2 arguments, got {n}"
        saved_tail = self._tail_position
        self._tail_position = False
        self._compile_sexpr(compiler, elements[1])
        self._compile_sexpr(compiler, elements[2])
        self._tail_position = saved_tail
        compiler.emit(opcode)

    def _compile_call(
        self, compiler: GenericCompiler, elements: list[ASTNode],
    ) -> None:
        """Compile a function call: (func arg1 arg2 ...).

        Pushes arguments left-to-right, then the function, then emits
        CALL_FUNCTION (or TAIL_CALL if in tail position).

        Stack layout before call: [arg1, arg2, ..., argN, func]
        """
        func_node = elements[0]
        arg_nodes = elements[1:]

        # Compile arguments left-to-right (never in tail position)
        saved_tail = self._tail_position
        self._tail_position = False
        for arg in arg_nodes:
            self._compile_sexpr(compiler, arg)

        # Compile the function expression
        self._compile_sexpr(compiler, func_node)
        self._tail_position = saved_tail

        # Emit CALL_FUNCTION or TAIL_CALL
        if self._tail_position and self._in_function:
            compiler.emit(LispOp.TAIL_CALL, len(arg_nodes))
        else:
            compiler.emit(LispOp.CALL_FUNCTION, len(arg_nodes))


# =========================================================================
# Public API
# =========================================================================


def create_lisp_compiler() -> LispCompiler:
    """Create a new Lisp compiler instance.

    Returns:
        A ``LispCompiler`` ready to compile Lisp ASTs.
    """
    return LispCompiler()


def compile_lisp(source: str) -> CodeObject:
    """Compile Lisp source code into bytecode.

    This is the main entry point. Parses the source, then compiles
    the AST into a CodeObject.

    Args:
        source: Lisp source code as a string.

    Returns:
        A ``CodeObject`` containing instructions, constants, and names.

    Example::

        code = compile_lisp("(+ 1 2)")
    """
    ast = parse_lisp(source)
    compiler = create_lisp_compiler()
    return compiler.compile(ast)


def run_lisp(source: str) -> object:
    """Compile and execute Lisp source code, returning the result.

    This is a convenience function that compiles the source, creates
    a Lisp VM, executes the bytecode, and returns the top of stack.

    Args:
        source: Lisp source code as a string.

    Returns:
        The result of evaluating the last expression, or NIL if the
        stack is empty.

    Example::

        result = run_lisp("(+ 1 2)")  # => 3
        result = run_lisp("(car (cons 1 2))")  # => 1
    """
    code = compile_lisp(source)
    vm = create_lisp_vm()
    vm.execute(code)
    if vm.stack:
        return vm.stack[-1]
    return NIL
