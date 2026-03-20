"""Starlark Compiler — Compiles Starlark ASTs to bytecode.

==========================================================================
Chapter 1: The Starlark Compilation Pipeline
==========================================================================

The full pipeline from source code to execution is::

    Starlark source code
        ↓ (starlark_lexer)
    Token stream
        ↓ (starlark_parser)
    AST (ASTNode tree)
        ↓ (THIS MODULE)
    CodeObject (bytecode)
        ↓ (starlark_vm)
    Execution result

This module handles the AST → CodeObject step. It registers handlers for
all Starlark grammar rules with the ``GenericCompiler`` framework, then
provides a ``compile_starlark()`` convenience function that does the
full source → bytecode path.

==========================================================================
Chapter 2: How Rule Handlers Work
==========================================================================

Each Starlark grammar rule (``file``, ``assign_stmt``, ``if_stmt``, etc.)
gets a corresponding handler function. The handler receives the compiler
and the AST node, then:

1. Inspects the node's children to understand the source construct.
2. Calls ``compiler.compile_node(child)`` to recursively compile sub-expressions.
3. Calls ``compiler.emit(opcode)`` to emit bytecode instructions.

For example, the ``assign_stmt`` handler for ``x = 1 + 2``:

1. Compiles the RHS expression (``1 + 2``) → emits LOAD_CONST, LOAD_CONST, ADD
2. Emits STORE_NAME for the LHS (``x``)

==========================================================================
Chapter 3: Grammar Rules Reference
==========================================================================

The Starlark grammar defines 55 rules. Not all need dedicated handlers —
many are pass-through rules (single child, no semantics). Here's the
breakdown:

**Rules with handlers** (do real work):
    file, simple_stmt, assign_stmt, return_stmt, break_stmt, continue_stmt,
    pass_stmt, load_stmt, if_stmt, for_stmt, def_stmt, suite,
    expression, expression_list, or_expr, and_expr, not_expr, comparison,
    arith, term, factor, power, primary, atom, list_expr, dict_expr,
    paren_expr, lambda_expr, arguments, argument

**Pass-through rules** (single child, handled automatically):
    statement, compound_stmt, small_stmt, bitwise_or, bitwise_xor,
    bitwise_and, shift
"""

from __future__ import annotations

from typing import Any

from bytecode_compiler import GenericCompiler
from lang_parser import ASTNode
from lexer import Token

from starlark_compiler.opcodes import (
    AUGMENTED_ASSIGN_MAP,
    BINARY_OP_MAP,
    COMPARE_OP_MAP,
    Op,
)


# =========================================================================
# Helper: extracting children by type
# =========================================================================


def _tokens(node: ASTNode) -> list[Token]:
    """Extract all Token children from an ASTNode."""
    return [c for c in node.children if isinstance(c, Token)]


def _nodes(node: ASTNode) -> list[ASTNode]:
    """Extract all ASTNode children from an ASTNode."""
    return [c for c in node.children if isinstance(c, ASTNode)]


def _token_values(node: ASTNode, token_type: str) -> list[str]:
    """Extract values of all tokens of a specific type."""
    return [
        c.value for c in node.children
        if isinstance(c, Token) and _type_name(c) == token_type
    ]


def _type_name(token: Token) -> str:
    """Get the type name from a token, handling both enum and string types."""
    t = token.type
    if isinstance(t, str):
        return t
    return t.name if hasattr(t, "name") else str(t)


def _has_token(node: ASTNode, value: str) -> bool:
    """Check if any child token has the given value."""
    return any(
        isinstance(c, Token) and c.value == value
        for c in node.children
    )


def _find_token_index(node: ASTNode, value: str) -> int | None:
    """Find the index of the first child token with the given value."""
    for i, c in enumerate(node.children):
        if isinstance(c, Token) and c.value == value:
            return i
    return None


# =========================================================================
# Rule Handlers — Top-Level Structure
# =========================================================================


def compile_file(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a Starlark file — a sequence of statements.

    Grammar: file = { NEWLINE | statement } ;

    The file rule is the root of every Starlark AST. Its children are
    a mix of NEWLINE tokens (which we skip) and statement nodes (which
    we compile).
    """
    for child in node.children:
        if isinstance(child, ASTNode):
            compiler.compile_node(child)
        # Skip NEWLINE tokens — they're structural, not semantic


def compile_simple_stmt(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a simple statement line.

    Grammar: simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;

    A simple statement line can contain multiple small statements separated
    by semicolons. We compile each small_stmt child.
    """
    for child in node.children:
        if isinstance(child, ASTNode):
            compiler.compile_node(child)
        # Skip SEMICOLON and NEWLINE tokens


def compile_assign_stmt(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile an assignment or expression statement.

    Grammar: assign_stmt = expression_list
                           [ ( assign_op | augmented_assign_op ) expression_list ] ;

    Three cases:
    1. Expression statement: ``f(x)`` — compile expr, emit POP
    2. Simple assignment: ``x = expr`` — compile RHS, emit STORE_NAME
    3. Augmented assignment: ``x += expr`` — load, compile RHS, op, store
    4. Tuple unpacking: ``a, b = 1, 2`` — compile RHS, emit UNPACK_SEQUENCE
    """
    sub_nodes = _nodes(node)

    if len(sub_nodes) == 1:
        # Case 1: Expression statement (no assignment operator)
        # Compile the expression and discard the result
        compiler.compile_node(sub_nodes[0])
        compiler.emit(Op.POP)
        return

    # Cases 2-4: assignment
    # sub_nodes[0] = LHS (expression_list), sub_nodes[-1] = RHS (expression_list)
    # Between them is the assignment operator (assign_op or augmented_assign_op)
    lhs = sub_nodes[0]
    rhs = sub_nodes[-1]

    # Find the operator
    op_node = None
    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name in ("assign_op", "augmented_assign_op"):
            op_node = child
            break

    if op_node is not None and op_node.rule_name == "augmented_assign_op":
        # Augmented assignment: x += expr → LOAD x, compile expr, ADD, STORE x
        op_token = op_node.children[0]
        assert isinstance(op_token, Token)
        arith_op = AUGMENTED_ASSIGN_MAP.get(op_token.value)

        # Load the current value of the target
        _compile_load_target(compiler, lhs)
        # Compile the RHS
        compiler.compile_node(rhs)
        # Emit the arithmetic operation
        if arith_op is not None:
            compiler.emit(arith_op)
        # Store back to the target
        _compile_store_target(compiler, lhs)
    else:
        # Simple assignment: x = expr or a, b = expr
        # Compile RHS first
        compiler.compile_node(rhs)

        # Check for tuple unpacking (multiple names on LHS)
        lhs_exprs = _nodes(lhs)
        if len(lhs_exprs) > 1:
            # Tuple unpacking: a, b = ...
            compiler.emit(Op.UNPACK_SEQUENCE, len(lhs_exprs))
            for expr in lhs_exprs:
                _compile_store_target(compiler, expr)
        else:
            # Single assignment: x = ...
            _compile_store_target(compiler, lhs)


def _compile_load_target(compiler: GenericCompiler, target: ASTNode) -> None:
    """Emit load instructions for an assignment target (for augmented assign)."""
    name = _extract_simple_name(target)
    if name is not None:
        idx = compiler.add_name(name)
        if compiler.scope and compiler.scope.get_local(name) is not None:
            compiler.emit(Op.LOAD_LOCAL, compiler.scope.get_local(name))
        else:
            compiler.emit(Op.LOAD_NAME, idx)
        return
    # For subscript/attribute targets, compile the object and key
    compiler.compile_node(target)


def _compile_store_target(compiler: GenericCompiler, target: ASTNode) -> None:
    """Emit store instructions for an assignment target.

    Handles three kinds of targets:
    - Simple name: ``x`` → STORE_NAME
    - Subscript: ``obj[key]`` → STORE_SUBSCRIPT
    - Attribute: ``obj.attr`` → STORE_ATTR
    """
    name = _extract_simple_name(target)
    if name is not None:
        if compiler.scope and compiler.scope.get_local(name) is not None:
            slot = compiler.scope.get_local(name)
            compiler.emit(Op.STORE_LOCAL, slot)
        else:
            idx = compiler.add_name(name)
            compiler.emit(Op.STORE_NAME, idx)
        return

    # Not a simple name — could be subscript or attribute
    # For now, raise an error for complex targets
    # TODO: handle obj[key] and obj.attr assignments
    raise NotImplementedError(
        f"Complex assignment targets not yet supported: {target.rule_name}"
    )


def _extract_simple_name(node: ASTNode) -> str | None:
    """Try to extract a simple variable name from an expression node.

    Unwraps pass-through nodes until we find a NAME token.
    Returns None if the expression is not a simple name.
    """
    # Unwrap single-child wrappers
    current: ASTNode | Token = node
    while isinstance(current, ASTNode) and len(current.children) == 1:
        current = current.children[0]

    if isinstance(current, Token) and _type_name(current) == "NAME":
        return current.value
    return None


# =========================================================================
# Rule Handlers — Simple Statements
# =========================================================================


def compile_return_stmt(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a return statement.

    Grammar: return_stmt = "return" [ expression ] ;

    If there's no expression, push None as the return value.
    """
    sub_nodes = _nodes(node)
    if sub_nodes:
        compiler.compile_node(sub_nodes[0])
    else:
        compiler.emit(Op.LOAD_NONE)
    compiler.emit(Op.RETURN)


def compile_break_stmt(compiler: GenericCompiler, _node: ASTNode) -> None:
    """Compile a break statement.

    Grammar: break_stmt = "break" ;

    Break is compiled as a JUMP to the end of the enclosing for-loop.
    We emit a placeholder jump that gets patched when the for-loop
    compilation completes.

    We use a convention: the compiler stores pending break jumps in a
    list on the compiler instance (set up by compile_for_stmt).
    """
    # Emit a placeholder jump — patched by the enclosing for-loop handler
    if hasattr(compiler, "_break_jumps") and compiler._break_jumps:  # type: ignore[attr-defined]
        jump_idx = compiler.emit_jump(Op.JUMP)
        compiler._break_jumps[-1].append(jump_idx)  # type: ignore[attr-defined]
    else:
        raise SyntaxError("'break' outside of a for loop")


def compile_continue_stmt(compiler: GenericCompiler, _node: ASTNode) -> None:
    """Compile a continue statement.

    Grammar: continue_stmt = "continue" ;

    Continue jumps to the top of the for-loop (back to FOR_ITER).
    """
    if hasattr(compiler, "_continue_targets") and compiler._continue_targets:  # type: ignore[attr-defined]
        compiler.emit(Op.JUMP, compiler._continue_targets[-1])  # type: ignore[attr-defined]
    else:
        raise SyntaxError("'continue' outside of a for loop")


def compile_pass_stmt(compiler: GenericCompiler, _node: ASTNode) -> None:
    """Compile a pass statement.

    Grammar: pass_stmt = "pass" ;

    Pass is a no-op — we emit nothing. This is exactly what CPython does.
    """
    pass  # No-op


def compile_load_stmt(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a load statement.

    Grammar: load_stmt = "load" LPAREN STRING { COMMA load_arg } [ COMMA ] RPAREN ;

    load("module.star", "symbol1", alias = "symbol2")

    This imports symbols from another Starlark module.
    """
    # Extract the module path (first STRING token)
    tokens = _tokens(node)
    module_path = None
    for t in tokens:
        if _type_name(t) == "STRING":
            module_path = t.value.strip('"').strip("'")
            break

    if module_path is None:
        raise SyntaxError("load() requires a module path string")

    # Emit LOAD_MODULE
    module_idx = compiler.add_name(module_path)
    compiler.emit(Op.LOAD_MODULE, module_idx)

    # Process load_arg children
    load_args = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "load_arg"]
    for arg in load_args:
        _compile_load_arg(compiler, arg)

    # Pop the module object (we've extracted what we need)
    compiler.emit(Op.POP)


def _compile_load_arg(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a single load argument.

    Grammar: load_arg = NAME EQUALS STRING | STRING ;

    - ``"symbol"`` → imports symbol with its original name
    - ``alias = "symbol"`` → imports symbol with a local alias
    """
    tokens = _tokens(node)
    sub_nodes = _nodes(node)

    if _has_token(node, "="):
        # alias = "symbol"
        alias = None
        symbol = None
        for t in tokens:
            if _type_name(t) == "NAME":
                alias = t.value
            elif _type_name(t) == "STRING":
                symbol = t.value.strip('"').strip("'")
        if alias and symbol:
            sym_idx = compiler.add_name(symbol)
            compiler.emit(Op.DUP)  # keep module on stack
            compiler.emit(Op.IMPORT_FROM, sym_idx)
            alias_idx = compiler.add_name(alias)
            compiler.emit(Op.STORE_NAME, alias_idx)
    else:
        # "symbol" → import with original name
        for t in tokens:
            if _type_name(t) == "STRING":
                symbol = t.value.strip('"').strip("'")
                sym_idx = compiler.add_name(symbol)
                compiler.emit(Op.DUP)  # keep module on stack
                compiler.emit(Op.IMPORT_FROM, sym_idx)
                compiler.emit(Op.STORE_NAME, sym_idx)
                break


# =========================================================================
# Rule Handlers — Compound Statements
# =========================================================================


def compile_if_stmt(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile an if/elif/else statement.

    Grammar: if_stmt = "if" expression COLON suite
                       { "elif" expression COLON suite }
                       [ "else" COLON suite ] ;

    Compilation pattern::

        compile condition
        JUMP_IF_FALSE → elif_or_else
        compile if-body
        JUMP → end
        elif_or_else:
        compile elif-condition (if any)
        JUMP_IF_FALSE → next_elif_or_else
        compile elif-body
        JUMP → end
        ...
        else:
        compile else-body (if any)
        end:
    """
    # Collect sections: (condition_node, suite_node) pairs, plus optional else_suite
    sections: list[tuple[ASTNode | None, ASTNode]] = []
    i = 0
    children = node.children

    while i < len(children):
        child = children[i]
        if isinstance(child, Token) and child.value in ("if", "elif"):
            # Next child is the condition, then COLON, then suite
            cond = children[i + 1]
            assert isinstance(cond, ASTNode)
            # Find the suite after the COLON
            suite = None
            for j in range(i + 2, len(children)):
                if isinstance(children[j], ASTNode) and children[j].rule_name == "suite":
                    suite = children[j]
                    i = j + 1
                    break
            assert suite is not None
            sections.append((cond, suite))
        elif isinstance(child, Token) and child.value == "else":
            # Find the suite after the COLON
            for j in range(i + 1, len(children)):
                if isinstance(children[j], ASTNode) and children[j].rule_name == "suite":
                    sections.append((None, children[j]))
                    i = j + 1
                    break
            break
        else:
            i += 1

    # Now compile: condition → JUMP_IF_FALSE → body → JUMP → end
    end_jumps: list[int] = []

    for cond, suite in sections:
        if cond is not None:
            # Conditional branch
            compiler.compile_node(cond)
            false_jump = compiler.emit_jump(Op.JUMP_IF_FALSE)
            _compile_suite(compiler, suite)
            end_jump = compiler.emit_jump(Op.JUMP)
            end_jumps.append(end_jump)
            compiler.patch_jump(false_jump)
        else:
            # Else branch — no condition
            _compile_suite(compiler, suite)

    # Patch all end-jumps to point here
    for j in end_jumps:
        compiler.patch_jump(j)


def compile_for_stmt(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a for loop.

    Grammar: for_stmt = "for" loop_vars "in" expression COLON suite ;

    Compilation pattern (same as CPython)::

        compile iterable
        GET_ITER
        loop_top:
        FOR_ITER → loop_end    (jump past body when iterator exhausted)
        store loop variable(s)
        compile body
        JUMP → loop_top
        loop_end:
    """
    # Extract children
    loop_vars_node = None
    iterable_node = None
    suite_node = None

    found_in = False
    for child in node.children:
        if isinstance(child, Token) and child.value == "in":
            found_in = True
            continue
        if isinstance(child, ASTNode):
            if child.rule_name == "loop_vars" and not found_in:
                loop_vars_node = child
            elif child.rule_name == "suite":
                suite_node = child
            elif found_in and iterable_node is None:
                iterable_node = child

    assert loop_vars_node is not None
    assert iterable_node is not None
    assert suite_node is not None

    # Initialize break/continue tracking
    if not hasattr(compiler, "_break_jumps"):
        compiler._break_jumps = []  # type: ignore[attr-defined]
    if not hasattr(compiler, "_continue_targets"):
        compiler._continue_targets = []  # type: ignore[attr-defined]

    # Compile the iterable and get its iterator
    compiler.compile_node(iterable_node)
    compiler.emit(Op.GET_ITER)

    # loop_top: FOR_ITER → loop_end
    loop_top = compiler.current_offset
    compiler._continue_targets.append(loop_top)  # type: ignore[attr-defined]
    compiler._break_jumps.append([])  # type: ignore[attr-defined]

    for_iter_jump = compiler.emit_jump(Op.FOR_ITER)

    # Store loop variable(s)
    _compile_loop_vars_store(compiler, loop_vars_node)

    # Compile body
    _compile_suite(compiler, suite_node)

    # Jump back to top
    compiler.emit(Op.JUMP, loop_top)

    # loop_end:
    compiler.patch_jump(for_iter_jump)

    # Patch break jumps
    for break_jump in compiler._break_jumps.pop():  # type: ignore[attr-defined]
        compiler.patch_jump(break_jump)
    compiler._continue_targets.pop()  # type: ignore[attr-defined]


def _compile_loop_vars_store(compiler: GenericCompiler, node: ASTNode) -> None:
    """Store loop variables after FOR_ITER pushes the next value.

    Grammar: loop_vars = NAME { COMMA NAME } ;

    Single variable: ``for x in ...`` → just STORE_NAME x
    Multiple: ``for x, y in ...`` → UNPACK_SEQUENCE 2, STORE_NAME x, STORE_NAME y
    """
    names = [c for c in node.children if isinstance(c, Token) and _type_name(c) == "NAME"]

    if len(names) > 1:
        compiler.emit(Op.UNPACK_SEQUENCE, len(names))

    for name_token in names:
        idx = compiler.add_name(name_token.value)
        if compiler.scope and compiler.scope.get_local(name_token.value) is not None:
            compiler.emit(Op.STORE_LOCAL, compiler.scope.get_local(name_token.value))
        else:
            compiler.emit(Op.STORE_NAME, idx)


def compile_def_stmt(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a function definition.

    Grammar: def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;

    Compilation pattern:
    1. Compile default parameter values (pushed onto stack)
    2. Compile the function body as a nested CodeObject
    3. Emit MAKE_FUNCTION to create the function object
    4. Emit STORE_NAME to bind the function to its name
    """
    # Extract name, parameters, and suite
    func_name = None
    params_node = None
    suite_node = None

    for child in node.children:
        if isinstance(child, Token) and _type_name(child) == "NAME" and func_name is None:
            func_name = child.value
        elif isinstance(child, ASTNode) and child.rule_name == "parameters":
            params_node = child
        elif isinstance(child, ASTNode) and child.rule_name == "suite":
            suite_node = child

    assert func_name is not None
    assert suite_node is not None

    # Parse parameters
    param_names: list[str] = []
    default_count = 0
    has_varargs = False
    has_kwargs = False

    if params_node is not None:
        for param_child in params_node.children:
            if isinstance(param_child, ASTNode) and param_child.rule_name == "parameter":
                param_info = _parse_parameter(param_child)
                if param_info["kind"] == "varargs":
                    has_varargs = True
                    param_names.append("*" + param_info["name"])
                elif param_info["kind"] == "kwargs":
                    has_kwargs = True
                    param_names.append("**" + param_info["name"])
                else:
                    param_names.append(param_info["name"])
                    if param_info["default"] is not None:
                        compiler.compile_node(param_info["default"])
                        default_count += 1

    # Compile the function body as a nested CodeObject
    # Enter a new scope for the function
    clean_param_names = [n.lstrip("*") for n in param_names]
    compiler.enter_scope(clean_param_names)

    body_code = compiler.compile_nested(suite_node)

    compiler.exit_scope()

    # Push the body CodeObject as a constant
    code_idx = compiler.add_constant(body_code)
    compiler.emit(Op.LOAD_CONST, code_idx)

    # Emit MAKE_FUNCTION
    # Flags encode: bit 0 = has defaults, bit 1 = has varargs, bit 2 = has kwargs
    flags = 0
    if default_count > 0:
        flags |= 0x01
    if has_varargs:
        flags |= 0x02
    if has_kwargs:
        flags |= 0x04
    compiler.emit(Op.MAKE_FUNCTION, flags)

    # Store as a named function
    name_idx = compiler.add_name(func_name)
    compiler.emit(Op.STORE_NAME, name_idx)


def _parse_parameter(node: ASTNode) -> dict[str, Any]:
    """Parse a parameter node into its components.

    Grammar: parameter = DOUBLE_STAR NAME | STAR NAME | NAME EQUALS expression | NAME ;
    """
    tokens = _tokens(node)
    sub_nodes = _nodes(node)

    if _has_token(node, "**"):
        name = [t.value for t in tokens if _type_name(t) == "NAME"][0]
        return {"name": name, "kind": "kwargs", "default": None}
    elif _has_token(node, "*"):
        name = [t.value for t in tokens if _type_name(t) == "NAME"][0]
        return {"name": name, "kind": "varargs", "default": None}
    elif _has_token(node, "="):
        name = [t.value for t in tokens if _type_name(t) == "NAME"][0]
        default = sub_nodes[0] if sub_nodes else None
        return {"name": name, "kind": "default", "default": default}
    else:
        name = tokens[0].value
        return {"name": name, "kind": "positional", "default": None}


def _compile_suite(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a suite (function/if/for body).

    Grammar: suite = simple_stmt | NEWLINE INDENT { statement } DEDENT ;

    A suite is either:
    1. A single simple_stmt on the same line: ``if True: pass``
    2. An indented block: ``if True:\\n    x = 1\\n    y = 2``
    """
    for child in node.children:
        if isinstance(child, ASTNode):
            compiler.compile_node(child)
        # Skip NEWLINE, INDENT, DEDENT tokens


# =========================================================================
# Rule Handlers — Expressions
# =========================================================================


def compile_expression(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile an expression (possibly with ternary if/else).

    Grammar: expression = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;

    Ternary: ``value_if_true if condition else value_if_false``

    Compilation pattern::
        compile condition
        JUMP_IF_FALSE → else_branch
        compile value_if_true
        JUMP → end
        else_branch:
        compile value_if_false
        end:
    """
    sub_nodes = _nodes(node)

    if len(sub_nodes) == 1:
        # No ternary — pass through
        compiler.compile_node(sub_nodes[0])
        return

    # Check for ternary: value "if" condition "else" value
    if _has_token(node, "if") and _has_token(node, "else"):
        # Children order: or_expr "if" or_expr "else" expression
        # sub_nodes[0] = value_if_true (first or_expr)
        # sub_nodes[1] = condition (second or_expr)
        # sub_nodes[2] = value_if_false (expression after "else")
        value_true = sub_nodes[0]
        condition = sub_nodes[1]
        value_false = sub_nodes[2]

        compiler.compile_node(condition)
        false_jump = compiler.emit_jump(Op.JUMP_IF_FALSE)
        compiler.compile_node(value_true)
        end_jump = compiler.emit_jump(Op.JUMP)
        compiler.patch_jump(false_jump)
        compiler.compile_node(value_false)
        compiler.patch_jump(end_jump)
    else:
        # Shouldn't happen for well-formed Starlark
        compiler.compile_node(sub_nodes[0])


def compile_expression_list(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile an expression list (possible tuple creation).

    Grammar: expression_list = expression { COMMA expression } [ COMMA ] ;

    If there's just one expression (no commas), compile it directly.
    If there are multiple, build a tuple.
    """
    exprs = _nodes(node)

    if len(exprs) == 1:
        # Check for trailing comma → single-element tuple
        has_trailing_comma = any(
            isinstance(c, Token) and c.value == ","
            for c in node.children
        )
        if has_trailing_comma:
            compiler.compile_node(exprs[0])
            compiler.emit(Op.BUILD_TUPLE, 1)
        else:
            compiler.compile_node(exprs[0])
    else:
        for expr in exprs:
            compiler.compile_node(expr)
        compiler.emit(Op.BUILD_TUPLE, len(exprs))


def compile_or_expr(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a boolean OR expression with short-circuit evaluation.

    Grammar: or_expr = and_expr { "or" and_expr } ;

    Short-circuit: ``a or b`` → if a is truthy, result is a (don't eval b).

    Compilation pattern::
        compile a
        JUMP_IF_TRUE_OR_POP → end   (if truthy, keep a on stack)
        compile b
        end:

    JUMP_IF_TRUE_OR_POP is special: if the top of stack is truthy, it
    leaves the value on the stack and jumps. Otherwise, it pops the value
    and falls through to evaluate the next operand.
    """
    sub_nodes = _nodes(node)

    if len(sub_nodes) == 1:
        compiler.compile_node(sub_nodes[0])
        return

    # Compile first operand
    compiler.compile_node(sub_nodes[0])

    # For each subsequent operand: short-circuit if truthy
    end_jumps: list[int] = []
    for i in range(1, len(sub_nodes)):
        jump = compiler.emit_jump(Op.JUMP_IF_TRUE_OR_POP)
        end_jumps.append(jump)
        compiler.compile_node(sub_nodes[i])

    for j in end_jumps:
        compiler.patch_jump(j)


def compile_and_expr(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a boolean AND expression with short-circuit evaluation.

    Grammar: and_expr = not_expr { "and" not_expr } ;

    Short-circuit: ``a and b`` → if a is falsy, result is a (don't eval b).

    Compilation pattern::
        compile a
        JUMP_IF_FALSE_OR_POP → end   (if falsy, keep a on stack)
        compile b
        end:
    """
    sub_nodes = _nodes(node)

    if len(sub_nodes) == 1:
        compiler.compile_node(sub_nodes[0])
        return

    compiler.compile_node(sub_nodes[0])

    end_jumps: list[int] = []
    for i in range(1, len(sub_nodes)):
        jump = compiler.emit_jump(Op.JUMP_IF_FALSE_OR_POP)
        end_jumps.append(jump)
        compiler.compile_node(sub_nodes[i])

    for j in end_jumps:
        compiler.patch_jump(j)


def compile_not_expr(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a boolean NOT expression.

    Grammar: not_expr = "not" not_expr | comparison ;

    ``not x`` → compile x, emit NOT
    """
    if _has_token(node, "not"):
        sub_nodes = _nodes(node)
        compiler.compile_node(sub_nodes[0])
        compiler.emit(Op.NOT)
    else:
        sub_nodes = _nodes(node)
        compiler.compile_node(sub_nodes[0])


def compile_comparison(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a comparison expression.

    Grammar: comparison = bitwise_or { comp_op bitwise_or } ;

    ``a == b`` → compile a, compile b, CMP_EQ
    ``a in b`` → compile a, compile b, CMP_IN
    ``a not in b`` → compile a, compile b, CMP_NOT_IN
    """
    sub_nodes = _nodes(node)

    if len(sub_nodes) == 1:
        compiler.compile_node(sub_nodes[0])
        return

    # sub_nodes alternates: operand, comp_op, operand, comp_op, operand, ...
    # The grammar produces: bitwise_or comp_op bitwise_or
    # where comp_op is a sub-node containing the operator token(s)

    # Separate operands and operators
    operands: list[ASTNode] = []
    operators: list[ASTNode] = []
    for sn in sub_nodes:
        if sn.rule_name == "comp_op":
            operators.append(sn)
        else:
            operands.append(sn)

    # Compile first operand
    compiler.compile_node(operands[0])

    for i, op_node in enumerate(operators):
        compiler.compile_node(operands[i + 1])
        op_str = _extract_comp_op(op_node)
        opcode = COMPARE_OP_MAP.get(op_str)
        if opcode is not None:
            compiler.emit(opcode)
        else:
            raise SyntaxError(f"Unknown comparison operator: {op_str}")


def _extract_comp_op(node: ASTNode) -> str:
    """Extract the comparison operator string from a comp_op node.

    Grammar: comp_op = EQUALS_EQUALS | NOT_EQUALS | ... | "in" | "not" "in" ;
    """
    tokens = _tokens(node)
    if len(tokens) == 2 and tokens[0].value == "not" and tokens[1].value == "in":
        return "not in"
    if tokens:
        return tokens[0].value
    return ""


def compile_binary_op(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a binary operation (arith, term, shift, bitwise_*).

    Grammar patterns:
        arith = term { ( PLUS | MINUS ) term } ;
        term  = factor { ( STAR | SLASH | FLOOR_DIV | PERCENT ) factor } ;
        shift = arith { ( LEFT_SHIFT | RIGHT_SHIFT ) arith } ;
        bitwise_or  = bitwise_xor { PIPE bitwise_xor } ;
        bitwise_xor = bitwise_and { CARET bitwise_and } ;
        bitwise_and = shift { AMP shift } ;

    All these follow the same pattern: left-associative binary operations.
    Compile left operand, then for each (operator, right operand) pair:
    compile right, emit the operation.
    """
    children = node.children

    # First child is always an operand
    compiler.compile_node(children[0])

    # Process pairs: (operator_token, operand)
    i = 1
    while i < len(children):
        child = children[i]
        if isinstance(child, Token):
            op_value = child.value
            # Next child should be the right operand
            if i + 1 < len(children):
                compiler.compile_node(children[i + 1])
                opcode = BINARY_OP_MAP.get(op_value)
                if opcode is not None:
                    compiler.emit(opcode)
                else:
                    raise SyntaxError(f"Unknown binary operator: {op_value}")
                i += 2
            else:
                i += 1
        else:
            # Shouldn't happen in well-formed AST
            compiler.compile_node(child)
            i += 1


def compile_factor(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a unary factor expression.

    Grammar: factor = ( PLUS | MINUS | TILDE ) factor | power ;

    ``-x`` → compile x, emit NEGATE
    ``~x`` → compile x, emit BIT_NOT
    ``+x`` → compile x (no-op, but validates it's numeric)
    """
    children = node.children

    if len(children) == 2 and isinstance(children[0], Token):
        op = children[0].value
        compiler.compile_node(children[1])
        if op == "-":
            compiler.emit(Op.NEGATE)
        elif op == "~":
            compiler.emit(Op.BIT_NOT)
        # unary + is a no-op
    elif len(children) == 1:
        compiler.compile_node(children[0])
    else:
        compiler.compile_node(children[0])


def compile_power(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile an exponentiation expression.

    Grammar: power = primary [ DOUBLE_STAR factor ] ;

    ``a ** b`` → compile a, compile b, emit POWER
    """
    sub_nodes = _nodes(node)

    if len(sub_nodes) == 1:
        compiler.compile_node(sub_nodes[0])
        return

    # a ** b
    compiler.compile_node(sub_nodes[0])
    compiler.compile_node(sub_nodes[1])
    compiler.emit(Op.POWER)


def compile_primary(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a primary expression (atom with suffixes).

    Grammar: primary = atom { suffix } ;

    Suffixes are:
    - ``.attr`` → LOAD_ATTR
    - ``[key]`` → LOAD_SUBSCRIPT
    - ``(args)`` → CALL_FUNCTION
    """
    children = node.children

    # Compile the atom (first child)
    compiler.compile_node(children[0])

    # Apply each suffix
    for i in range(1, len(children)):
        child = children[i]
        if isinstance(child, ASTNode) and child.rule_name == "suffix":
            _compile_suffix(compiler, child)


def _compile_suffix(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a single suffix (attribute, subscript, or call).

    Grammar: suffix = DOT NAME
                    | LBRACKET subscript RBRACKET
                    | LPAREN [ arguments ] RPAREN ;
    """
    children = node.children

    if _has_token(node, "."):
        # Attribute access: obj.attr
        for child in children:
            if isinstance(child, Token) and _type_name(child) == "NAME":
                attr_idx = compiler.add_name(child.value)
                compiler.emit(Op.LOAD_ATTR, attr_idx)
                break

    elif _has_token(node, "["):
        # Subscript/slice: obj[key] or obj[start:stop:step]
        subscript_nodes = [c for c in children if isinstance(c, ASTNode) and c.rule_name == "subscript"]
        if subscript_nodes:
            _compile_subscript(compiler, subscript_nodes[0])
        else:
            # Simple subscript with expression
            for c in children:
                if isinstance(c, ASTNode):
                    compiler.compile_node(c)
                    compiler.emit(Op.LOAD_SUBSCRIPT)
                    break

    elif _has_token(node, "("):
        # Function call: f(args)
        arg_nodes = [c for c in children if isinstance(c, ASTNode) and c.rule_name == "arguments"]
        if arg_nodes:
            argc, has_kw = _compile_arguments(compiler, arg_nodes[0])
            if has_kw:
                compiler.emit(Op.CALL_FUNCTION_KW, argc)
            else:
                compiler.emit(Op.CALL_FUNCTION, argc)
        else:
            # No arguments: f()
            compiler.emit(Op.CALL_FUNCTION, 0)


def _compile_subscript(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a subscript expression.

    Grammar: subscript = expression
                       | [ expression ] COLON [ expression ] [ COLON [ expression ] ] ;
    """
    if _has_token(node, ":"):
        # Slice: [start:stop:step]
        # Count how many colon-separated parts we have
        parts: list[ASTNode | None] = []
        current_exprs: list[ASTNode] = []
        colon_count = 0

        for child in node.children:
            if isinstance(child, Token) and child.value == ":":
                parts.append(current_exprs[0] if current_exprs else None)
                current_exprs = []
                colon_count += 1
            elif isinstance(child, ASTNode):
                current_exprs.append(child)
        # Last part
        parts.append(current_exprs[0] if current_exprs else None)

        # Pad to 3 elements: [start, stop, step]
        while len(parts) < 3:
            parts.append(None)

        # Compile each part (push None for missing parts)
        flags = 0
        for i, part in enumerate(parts[:3]):
            if part is not None:
                compiler.compile_node(part)
                flags |= (1 << i)
            else:
                compiler.emit(Op.LOAD_NONE)

        compiler.emit(Op.LOAD_SLICE, flags)
    else:
        # Simple index: [expr]
        sub_nodes = _nodes(node)
        if sub_nodes:
            compiler.compile_node(sub_nodes[0])
            compiler.emit(Op.LOAD_SUBSCRIPT)


def _compile_arguments(compiler: GenericCompiler, node: ASTNode) -> tuple[int, bool]:
    """Compile function call arguments.

    Grammar: arguments = argument { COMMA argument } [ COMMA ] ;

    Returns (arg_count, has_keyword_args).
    """
    arg_nodes = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "argument"]
    argc = 0
    has_kw = False

    for arg in arg_nodes:
        kw = _compile_argument(compiler, arg)
        if kw:
            has_kw = True
        argc += 1

    return argc, has_kw


def _compile_argument(compiler: GenericCompiler, node: ASTNode) -> bool:
    """Compile a single function call argument.

    Grammar: argument = DOUBLE_STAR expression | STAR expression
                      | NAME EQUALS expression | expression ;

    Returns True if this is a keyword argument.
    """
    if _has_token(node, "**"):
        # **kwargs unpacking
        sub_nodes = _nodes(node)
        if sub_nodes:
            compiler.compile_node(sub_nodes[0])
        return True
    elif _has_token(node, "*"):
        # *args unpacking
        sub_nodes = _nodes(node)
        if sub_nodes:
            compiler.compile_node(sub_nodes[0])
        return False
    elif _has_token(node, "="):
        # Keyword argument: name=value
        tokens = _tokens(node)
        sub_nodes = _nodes(node)
        name = None
        for t in tokens:
            if _type_name(t) == "NAME":
                name = t.value
                break
        if name and sub_nodes:
            # Push the keyword name as a constant, then the value
            name_idx = compiler.add_constant(name)
            compiler.emit(Op.LOAD_CONST, name_idx)
            compiler.compile_node(sub_nodes[0])
        return True
    else:
        # Positional argument
        sub_nodes = _nodes(node)
        if sub_nodes:
            compiler.compile_node(sub_nodes[0])
        else:
            # Token-based expression
            for c in node.children:
                if isinstance(c, Token):
                    compiler.compile_node(c)
                    break
        return False


# =========================================================================
# Rule Handlers — Atoms
# =========================================================================


def compile_atom(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile an atom — the leaf-level expression.

    Grammar: atom = INT | FLOAT | STRING { STRING } | NAME
                  | "True" | "False" | "None"
                  | list_expr | dict_expr | paren_expr ;
    """
    children = node.children

    if len(children) == 1:
        child = children[0]

        if isinstance(child, Token):
            ttype = _type_name(child)

            if ttype == "INT":
                value = int(child.value)
                idx = compiler.add_constant(value)
                compiler.emit(Op.LOAD_CONST, idx)

            elif ttype == "FLOAT":
                value = float(child.value)
                idx = compiler.add_constant(value)
                compiler.emit(Op.LOAD_CONST, idx)

            elif ttype == "STRING":
                # Strip quotes
                value = _parse_string_literal(child.value)
                idx = compiler.add_constant(value)
                compiler.emit(Op.LOAD_CONST, idx)

            elif ttype == "NAME":
                # Check for keywords: True, False, None
                if child.value == "True":
                    compiler.emit(Op.LOAD_TRUE)
                elif child.value == "False":
                    compiler.emit(Op.LOAD_FALSE)
                elif child.value == "None":
                    compiler.emit(Op.LOAD_NONE)
                else:
                    # Variable reference
                    if compiler.scope and compiler.scope.get_local(child.value) is not None:
                        slot = compiler.scope.get_local(child.value)
                        compiler.emit(Op.LOAD_LOCAL, slot)
                    else:
                        idx = compiler.add_name(child.value)
                        compiler.emit(Op.LOAD_NAME, idx)

            else:
                # Check for keyword literals
                if child.value == "True":
                    compiler.emit(Op.LOAD_TRUE)
                elif child.value == "False":
                    compiler.emit(Op.LOAD_FALSE)
                elif child.value == "None":
                    compiler.emit(Op.LOAD_NONE)
                else:
                    raise SyntaxError(f"Unexpected token in atom: {child}")

        elif isinstance(child, ASTNode):
            # list_expr, dict_expr, or paren_expr
            compiler.compile_node(child)

    elif len(children) >= 2:
        # Adjacent string concatenation: "hello" "world"
        all_strings = all(
            isinstance(c, Token) and _type_name(c) == "STRING"
            for c in children
        )
        if all_strings:
            # Concatenate at compile time (just like Python)
            concatenated = "".join(
                _parse_string_literal(c.value)  # type: ignore[union-attr]
                for c in children
            )
            idx = compiler.add_constant(concatenated)
            compiler.emit(Op.LOAD_CONST, idx)
        else:
            # Shouldn't happen in well-formed Starlark
            for c in children:
                if isinstance(c, ASTNode):
                    compiler.compile_node(c)


def _parse_string_literal(s: str) -> str:
    """Parse a string literal, stripping quotes and handling escapes."""
    # Strip outer quotes (single, double, or triple-quoted)
    if s.startswith('"""') or s.startswith("'''"):
        s = s[3:-3]
    elif s.startswith('"') or s.startswith("'"):
        s = s[1:-1]

    # Handle basic escape sequences
    result = []
    i = 0
    while i < len(s):
        if s[i] == "\\" and i + 1 < len(s):
            c = s[i + 1]
            if c == "n":
                result.append("\n")
            elif c == "t":
                result.append("\t")
            elif c == "\\":
                result.append("\\")
            elif c == '"':
                result.append('"')
            elif c == "'":
                result.append("'")
            elif c == "r":
                result.append("\r")
            elif c == "0":
                result.append("\0")
            else:
                result.append("\\")
                result.append(c)
            i += 2
        else:
            result.append(s[i])
            i += 1

    return "".join(result)


# =========================================================================
# Rule Handlers — Collection Literals
# =========================================================================


def compile_list_expr(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a list literal or list comprehension.

    Grammar: list_expr = LBRACKET [ list_body ] RBRACKET ;
    """
    body_nodes = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "list_body"]

    if not body_nodes:
        # Empty list: []
        compiler.emit(Op.BUILD_LIST, 0)
        return

    _compile_list_body(compiler, body_nodes[0])


def _compile_list_body(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile list body — either literal elements or comprehension.

    Grammar: list_body = expression comp_clause
                       | expression { COMMA expression } [ COMMA ] ;
    """
    sub_nodes = _nodes(node)

    # Check for comprehension
    has_comp = any(sn.rule_name == "comp_clause" for sn in sub_nodes)

    if has_comp:
        # List comprehension: [expr for x in iterable if cond]
        _compile_list_comprehension(compiler, node)
    else:
        # List literal: [expr, expr, ...]
        exprs = [sn for sn in sub_nodes if sn.rule_name != "comp_clause"]
        for expr in exprs:
            compiler.compile_node(expr)
        compiler.emit(Op.BUILD_LIST, len(exprs))


def _compile_list_comprehension(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a list comprehension.

    [expr for x in iterable if cond]

    Compilation pattern::
        BUILD_LIST 0           # empty accumulator list
        compile iterable
        GET_ITER
        loop:
        FOR_ITER → end
        store x
        compile condition (if any)
        JUMP_IF_FALSE → loop_continue
        compile expr
        LIST_APPEND
        loop_continue:
        JUMP → loop
        end:
    """
    sub_nodes = _nodes(node)
    expr_node = sub_nodes[0]
    comp_clause = [sn for sn in sub_nodes if sn.rule_name == "comp_clause"][0]

    # Create empty list
    compiler.emit(Op.BUILD_LIST, 0)

    # Compile the comprehension clause(s)
    _compile_comp_clause(compiler, comp_clause, expr_node, is_list=True)


def _compile_comp_clause(
    compiler: GenericCompiler,
    node: ASTNode,
    expr_node: ASTNode,
    is_list: bool = True,
) -> None:
    """Compile comprehension for/if clauses.

    Grammar: comp_clause = comp_for { comp_for | comp_if } ;
    """
    sub_nodes = _nodes(node)

    # First must be comp_for
    if not sub_nodes or sub_nodes[0].rule_name != "comp_for":
        return

    # Compile the first for clause, nesting subsequent clauses
    _compile_comp_for(compiler, sub_nodes, 0, expr_node, is_list)


def _compile_comp_for(
    compiler: GenericCompiler,
    clauses: list[ASTNode],
    clause_idx: int,
    expr_node: ASTNode,
    is_list: bool,
) -> None:
    """Compile a single for clause in a comprehension, with nested clauses."""
    if clause_idx >= len(clauses):
        # Base case: compile the expression and append
        compiler.compile_node(expr_node)
        if is_list:
            compiler.emit(Op.LIST_APPEND)
        else:
            compiler.emit(Op.DICT_SET)
        return

    clause = clauses[clause_idx]

    if clause.rule_name == "comp_for":
        # for loop_vars in iterable
        # Extract loop_vars and iterable
        loop_vars_node = None
        iterable_node = None
        found_in = False

        for child in clause.children:
            if isinstance(child, Token) and child.value == "in":
                found_in = True
                continue
            if isinstance(child, ASTNode):
                if not found_in:
                    loop_vars_node = child
                else:
                    iterable_node = child

        assert loop_vars_node is not None
        assert iterable_node is not None

        compiler.compile_node(iterable_node)
        compiler.emit(Op.GET_ITER)
        loop_top = compiler.current_offset
        for_iter_jump = compiler.emit_jump(Op.FOR_ITER)
        _compile_loop_vars_store(compiler, loop_vars_node)

        # Compile remaining clauses recursively
        _compile_comp_for(compiler, clauses, clause_idx + 1, expr_node, is_list)

        compiler.emit(Op.JUMP, loop_top)
        compiler.patch_jump(for_iter_jump)

    elif clause.rule_name == "comp_if":
        # if condition — filter
        sub_nodes = _nodes(clause)
        if sub_nodes:
            compiler.compile_node(sub_nodes[0])
            skip_jump = compiler.emit_jump(Op.JUMP_IF_FALSE)
            # Compile remaining clauses
            _compile_comp_for(compiler, clauses, clause_idx + 1, expr_node, is_list)
            compiler.patch_jump(skip_jump)
        else:
            _compile_comp_for(compiler, clauses, clause_idx + 1, expr_node, is_list)


def compile_dict_expr(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a dict literal or dict comprehension.

    Grammar: dict_expr = LBRACE [ dict_body ] RBRACE ;
    """
    body_nodes = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "dict_body"]

    if not body_nodes:
        # Empty dict: {}
        compiler.emit(Op.BUILD_DICT, 0)
        return

    _compile_dict_body(compiler, body_nodes[0])


def _compile_dict_body(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile dict body — either literal entries or comprehension.

    Grammar: dict_body = dict_entry comp_clause
                       | dict_entry { COMMA dict_entry } [ COMMA ] ;
    """
    sub_nodes = _nodes(node)

    has_comp = any(sn.rule_name == "comp_clause" for sn in sub_nodes)

    if has_comp:
        # Dict comprehension
        _compile_dict_comprehension(compiler, node)
    else:
        # Dict literal: {key: val, ...}
        entries = [sn for sn in sub_nodes if sn.rule_name == "dict_entry"]
        for entry in entries:
            _compile_dict_entry(compiler, entry)
        compiler.emit(Op.BUILD_DICT, len(entries))


def _compile_dict_entry(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a single dict entry (key: value).

    Grammar: dict_entry = expression COLON expression ;
    """
    sub_nodes = _nodes(node)
    # First expression is key, second is value
    compiler.compile_node(sub_nodes[0])
    compiler.compile_node(sub_nodes[1])


def _compile_dict_comprehension(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a dict comprehension.

    {key: value for x in iterable if cond}
    """
    sub_nodes = _nodes(node)
    entry_node = [sn for sn in sub_nodes if sn.rule_name == "dict_entry"][0]
    comp_clause = [sn for sn in sub_nodes if sn.rule_name == "comp_clause"][0]

    # Create empty dict
    compiler.emit(Op.BUILD_DICT, 0)

    # The entry_node has key and value expressions
    # We need to compile both and emit DICT_SET for each iteration
    # For now, compile as a special case of comprehension
    _compile_comp_clause(compiler, comp_clause, entry_node, is_list=False)


def compile_paren_expr(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a parenthesized expression or tuple.

    Grammar: paren_expr = LPAREN [ paren_body ] RPAREN ;

    - ``()`` → empty tuple
    - ``(x)`` → just x (parenthesized, not a tuple)
    - ``(x,)`` → single-element tuple
    - ``(x, y)`` → two-element tuple
    """
    body_nodes = [c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "paren_body"]

    if not body_nodes:
        # Empty tuple: ()
        compiler.emit(Op.BUILD_TUPLE, 0)
        return

    _compile_paren_body(compiler, body_nodes[0])


def _compile_paren_body(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile parenthesized expression body.

    Grammar: paren_body = expression comp_clause
                        | expression COMMA [ expression { COMMA expression } [ COMMA ] ]
                        | expression ;
    """
    sub_nodes = _nodes(node)

    # Check for comprehension (generator expression)
    has_comp = any(sn.rule_name == "comp_clause" for sn in sub_nodes)
    if has_comp:
        # Generator expression — compile as list for now
        # TODO: proper generator support
        _compile_list_comprehension(compiler, node)
        return

    # Check for commas (tuple)
    has_comma = any(
        isinstance(c, Token) and c.value == ","
        for c in node.children
    )

    if has_comma:
        # Tuple
        exprs = [sn for sn in sub_nodes if sn.rule_name != "comp_clause"]
        for expr in exprs:
            compiler.compile_node(expr)
        compiler.emit(Op.BUILD_TUPLE, len(exprs))
    else:
        # Parenthesized expression — just compile the inner expression
        compiler.compile_node(sub_nodes[0])


def compile_lambda_expr(compiler: GenericCompiler, node: ASTNode) -> None:
    """Compile a lambda expression.

    Grammar: lambda_expr = "lambda" [ lambda_params ] COLON expression ;

    Lambda is compiled just like a function definition, but anonymous.
    """
    # Extract params and body expression
    params_node = None
    body_node = None

    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == "lambda_params":
            params_node = child
        elif isinstance(child, ASTNode) and body_node is None and child.rule_name != "lambda_params":
            body_node = child

    assert body_node is not None

    # Parse parameters
    param_names: list[str] = []
    default_count = 0

    if params_node is not None:
        for param_child in params_node.children:
            if isinstance(param_child, ASTNode) and param_child.rule_name == "lambda_param":
                info = _parse_lambda_param(param_child)
                param_names.append(info["name"])
                if info["default"] is not None:
                    compiler.compile_node(info["default"])
                    default_count += 1

    # Compile body as nested CodeObject
    compiler.enter_scope(param_names)
    body_code = compiler.compile_nested(body_node)
    compiler.exit_scope()

    code_idx = compiler.add_constant(body_code)
    compiler.emit(Op.LOAD_CONST, code_idx)

    flags = 0
    if default_count > 0:
        flags |= 0x01
    compiler.emit(Op.MAKE_FUNCTION, flags)


def _parse_lambda_param(node: ASTNode) -> dict[str, Any]:
    """Parse a lambda parameter.

    Grammar: lambda_param = NAME [ EQUALS expression ] | STAR NAME | DOUBLE_STAR NAME ;
    """
    tokens = _tokens(node)
    sub_nodes = _nodes(node)

    if _has_token(node, "*"):
        name = [t.value for t in tokens if _type_name(t) == "NAME"][0]
        return {"name": name, "kind": "varargs", "default": None}
    elif _has_token(node, "**"):
        name = [t.value for t in tokens if _type_name(t) == "NAME"][0]
        return {"name": name, "kind": "kwargs", "default": None}
    elif _has_token(node, "="):
        name = [t.value for t in tokens if _type_name(t) == "NAME"][0]
        return {"name": name, "kind": "default", "default": sub_nodes[0] if sub_nodes else None}
    else:
        return {"name": tokens[0].value, "kind": "positional", "default": None}


# =========================================================================
# Registration — creates a configured GenericCompiler
# =========================================================================


def create_starlark_compiler() -> GenericCompiler:
    """Create a ``GenericCompiler`` configured with all Starlark rule handlers.

    This is the main factory function. It creates a fresh GenericCompiler
    and registers handlers for all Starlark grammar rules.

    Returns
    -------
    GenericCompiler
        A compiler ready to compile Starlark ASTs.

    Example::

        compiler = create_starlark_compiler()
        ast = parse_starlark("x = 1 + 2\\n")
        code = compiler.compile(ast)
    """
    compiler = GenericCompiler()

    # -- Top-level structure --
    compiler.register_rule("file", compile_file)
    compiler.register_rule("simple_stmt", compile_simple_stmt)

    # -- Simple statements --
    compiler.register_rule("assign_stmt", compile_assign_stmt)
    compiler.register_rule("return_stmt", compile_return_stmt)
    compiler.register_rule("break_stmt", compile_break_stmt)
    compiler.register_rule("continue_stmt", compile_continue_stmt)
    compiler.register_rule("pass_stmt", compile_pass_stmt)
    compiler.register_rule("load_stmt", compile_load_stmt)

    # -- Compound statements --
    compiler.register_rule("if_stmt", compile_if_stmt)
    compiler.register_rule("for_stmt", compile_for_stmt)
    compiler.register_rule("def_stmt", compile_def_stmt)
    compiler.register_rule("suite", _compile_suite)

    # -- Expressions --
    compiler.register_rule("expression", compile_expression)
    compiler.register_rule("expression_list", compile_expression_list)
    compiler.register_rule("or_expr", compile_or_expr)
    compiler.register_rule("and_expr", compile_and_expr)
    compiler.register_rule("not_expr", compile_not_expr)
    compiler.register_rule("comparison", compile_comparison)

    # Binary operations — all follow the same pattern
    compiler.register_rule("arith", compile_binary_op)
    compiler.register_rule("term", compile_binary_op)
    compiler.register_rule("shift", compile_binary_op)
    compiler.register_rule("bitwise_or", compile_binary_op)
    compiler.register_rule("bitwise_xor", compile_binary_op)
    compiler.register_rule("bitwise_and", compile_binary_op)

    # Unary and power
    compiler.register_rule("factor", compile_factor)
    compiler.register_rule("power", compile_power)

    # Primary expressions (atom + suffixes)
    compiler.register_rule("primary", compile_primary)

    # Atoms
    compiler.register_rule("atom", compile_atom)

    # Collection literals
    compiler.register_rule("list_expr", compile_list_expr)
    compiler.register_rule("dict_expr", compile_dict_expr)
    compiler.register_rule("paren_expr", compile_paren_expr)

    # Lambda
    compiler.register_rule("lambda_expr", compile_lambda_expr)

    # -- Pass-through rules (handled by GenericCompiler automatically) --
    # statement, compound_stmt, small_stmt — all have single child

    return compiler


# =========================================================================
# Convenience Function
# =========================================================================


def compile_starlark(source: str) -> CodeObject:
    """Compile Starlark source code to a CodeObject in one call.

    This chains the full pipeline: source → tokens → AST → bytecode.

    Parameters
    ----------
    source : str
        Starlark source code. Should end with a newline.

    Returns
    -------
    CodeObject
        Compiled bytecode ready for the Starlark VM.

    Example::

        code = compile_starlark("x = 1 + 2\\n")
    """
    from starlark_parser import parse_starlark

    ast = parse_starlark(source)
    compiler = create_starlark_compiler()
    return compiler.compile(ast, halt_opcode=Op.HALT)
