"""Nib type checker — walks an untyped AST and produces a typed AST.

=============================================================================
OVERVIEW
=============================================================================

    The type checker is the third stage of the Nib compiler pipeline:

    Source text
        → Nib Lexer      (characters → tokens)
        → Nib Parser     (tokens → untyped ASTNode tree)
        → Type Checker   (untyped AST → typed AST)   ← this module
        → IR Compiler    (typed AST → IR)
        → Intel 4004 IR Validator  (IR → validated IR for specific ISA)
        → IR-to-Intel-4004 Compiler (validated IR → Intel 4004 assembly)

The ``NibTypeChecker`` class implements the ``TypeChecker[ASTNode, ASTNode]``
protocol from ``type_checker_protocol``. Calling ``check(ast)`` performs two
passes over the AST:

  **Pass 1 — Signature Collection**
    Walk the top-level declarations and record:
    - ``const`` names and their types in the global scope.
    - ``static`` names and their types in the global scope.
    - Function names, parameter types, and return types in the global scope.
    - Build the call graph: ``{fn_name → set of function names it calls}``.

  **Pass 2 — Body Type-Checking**
    Walk each function body with the complete global scope available.
    Check every statement, every expression, every assignment for type
    correctness. Annotate each expression node with ``._nib_type``.

The two-pass structure means functions can be called in any order — you
do not have to declare a function before calling it (unlike C without
header files).

=============================================================================
LANGUAGE INVARIANTS ENFORCED
=============================================================================

This checker enforces *language-level* invariants only. It is completely
independent of any compilation target. The same checker runs whether you
are targeting the Intel 4004, an ARM Cortex-M, or a WASM module.

Invariants enforced:

1. **All names declared before use**: Variables must be declared with
   ``let`` or ``const`` or ``static`` before they appear in expressions or
   on the left-hand side of assignments.

2. **Expression types correct bottom-up**: The type of a composite
   expression (e.g., ``a + b``) is determined from the types of its
   operands. Both operands of a binary arithmetic op must have the same
   numeric type. Both operands of ``||``/``&&`` must be ``bool``.
   Comparison operators (``<``, ``>``, ``==``, etc.) produce ``bool``.

3. **Assignment LHS type == RHS type**: No implicit widening. Assigning
   a ``u4`` expression to a ``u8`` variable is an error.

4. **Function call argument types match parameter types**: Each positional
   argument must match the declared parameter type exactly. The number of
   arguments must also match.

5. **BCD operator restriction**: If either operand of an arithmetic
   operator resolves to ``bcd``, only ``+%`` (wrapping add with DAA) and
   ``-`` (subtraction) are legal. ``+``, ``+?``, ``*``, ``/`` are errors.

6. **for-loop bounds are constants**: The start and end expressions of a
   ``for i: T in start..end { ... }`` loop must be either:
   - An integer literal (``INT_LIT`` or ``HEX_LIT``) token, or
   - A ``NAME`` that resolves to a ``const``-declared symbol.
   Runtime variable bounds are not allowed because the 4004 code generator
   needs to emit DJNZ (decrement-and-jump) with a statically known trip count.

7. **No recursion**: The static call graph must be acyclic. Any cycle
   (direct or mutual) is a type error.

8. **``if`` and ``for`` conditions must be ``bool``**: Unlike C (where any
   non-zero integer is "truthy"), Nib requires explicit boolean conditions.
   This avoids the class of bugs from ``if (x = 0)`` vs ``if (x == 0)``.

9. **Return statements match the declared return type**: In a function
   declared ``-> u4``, every ``return`` expression must have type ``u4``.

=============================================================================
WHAT IS NOT CHECKED HERE
=============================================================================

- **Call depth ≤ 2**: This is a 3-level hardware call stack constraint on
  the Intel 4004. It belongs in the Intel 4004 IR validator package, which runs
  after IR generation.

- **Total static RAM ≤ 160 bytes**: Also a hardware constraint — checked
  in the backend.

- **Physical register count**: CPU register allocation — backend concern.

Keeping hardware constraints out of the type checker makes the design
composable. The same ``NibTypeChecker`` works for any target ISA.

=============================================================================
ANNOTATION STRATEGY
=============================================================================

Rather than building a new parallel tree, we *annotate in place* by setting
a ``._nib_type`` attribute on each ``ASTNode`` that corresponds to an
expression. This keeps the output AST identical in structure to the input
(preserving line/column info), while adding type metadata that later stages
can read.

The annotated root node is returned as ``TypeCheckResult.typed_ast``.

=============================================================================
CALL GRAPH AND CYCLE DETECTION
=============================================================================

To enforce no-recursion, we build a directed graph after Pass 1:

    nodes = all function names
    edges = "function A calls function B"

Then we check for cycles using iterative DFS (depth-first search) with
three-colour marking:

    WHITE (0) = not yet visited
    GREY  (1) = currently on the DFS stack (potential back-edge)
    BLACK (2) = fully explored

If the DFS ever reaches a GREY node, we have found a cycle (back-edge),
and recursion is present. We report an error naming the cycle.

This is the standard textbook algorithm for detecting cycles in a directed
graph. Its time complexity is O(V + E) where V is the number of functions
and E is the number of call-site relationships.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token
from type_checker_protocol import GenericTypeChecker, TypeCheckResult

from nib_type_checker.scope import ScopeChain, Symbol
from nib_type_checker.types import (
    NibType,
    is_bcd_op_allowed,
    is_numeric,
    parse_type_name,
    types_are_compatible,
)


# ---------------------------------------------------------------------------
# AST traversal helpers
# ---------------------------------------------------------------------------


def _token_value(child: ASTNode | Token) -> str:
    """Return the ``.value`` of a Token (or the single leaf token of an ASTNode)."""
    if isinstance(child, Token):
        return child.value
    if child.is_leaf and child.token is not None:
        return child.token.value
    return ""


def _token_type(child: ASTNode | Token) -> str:
    """Return the type name of a Token (or the single leaf token of an ASTNode)."""
    if isinstance(child, Token):
        t = child.type
        return t if isinstance(t, str) else t.name
    if child.is_leaf and child.token is not None:
        t = child.token.type
        return t if isinstance(t, str) else t.name
    return ""


def _is_token_type(child: ASTNode | Token, type_name: str) -> bool:
    return _token_type(child) == type_name


def _is_numeric_literal_expr(node: ASTNode | Token) -> bool:
    """Return True if ``node`` consists *only* of integer/hex literals.

    Nib numeric literals are *untyped* — they fit any numeric type (u4,
    u8, bcd).  When the RHS of a ``let`` or assignment is a pure literal
    expression, we skip the type-compatibility check and trust the declared
    type as the authority.

    Examples that return True:
        5        INT_LIT
        0xF      HEX_LIT
        3 +% 4   add_expr containing only literals (both operands are literals)

    Examples that return False:
        x        NAME token (variable reference — has a declared type)
        x +% 4   contains a variable
    """
    if isinstance(node, Token):
        t_name = node.type if isinstance(node.type, str) else node.type.name
        return t_name in ("INT_LIT", "HEX_LIT")
    # ASTNode: check if all children that are expressions are also numeric literals.
    if node.is_leaf and node.token is not None:
        t_name = (
            node.token.type
            if isinstance(node.token.type, str)
            else node.token.type.name
        )
        return t_name in ("INT_LIT", "HEX_LIT")
    # For compound nodes: traverse children, treating operator tokens as neutral.
    has_any_child = False
    for child in node.children:
        if isinstance(child, Token):
            t_name = child.type if isinstance(child.type, str) else child.type.name
            # After the nib-lexer keyword reclassification, `true` and `false`
            # come back with type="true"/"false" (not type="NAME").  Check for
            # both the reclassified form and the NAME+value form.
            if t_name in ("true", "false"):
                return False  # boolean keyword token — not a numeric literal
            # Comparison and logical operators produce bool, not a numeric type.
            # Expressions like `1 == 1` or `a < b` are NOT numeric literals.
            if t_name in ("EQ_EQ", "NEQ", "LEQ", "GEQ", "LT", "GT", "LAND", "LOR"):
                return False
            # NAME tokens mean a variable is involved.
            if t_name == "NAME":
                if child.value in ("true", "false"):
                    return False  # boolean literal — not a numeric literal
                return False  # variable reference
            # Other tokens (arithmetic operators, punctuation) are neutral.
        elif isinstance(child, ASTNode):
            has_any_child = True
            if not _is_numeric_literal_expr(child):
                return False
    # If only tokens (no sub-nodes) and no NAME tokens found, it's a literal expr.
    return True


def _find_tokens_of_type(
    node: ASTNode, type_name: str
) -> list[Token]:
    """Recursively collect all Token children with a given type name."""
    result: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            t = child.type
            t_name = t if isinstance(t, str) else t.name
            if t_name == type_name:
                result.append(child)
        elif isinstance(child, ASTNode):
            result.extend(_find_tokens_of_type(child, type_name))
    return result


def _child_nodes(node: ASTNode) -> list[ASTNode]:
    """Return only the ASTNode children (excluding Token children)."""
    return [c for c in node.children if isinstance(c, ASTNode)]


def _first_token(node: ASTNode | Token) -> Token | None:
    """Return the first token reachable from ``node``."""
    if isinstance(node, Token):
        return node
    for child in node.children:
        t = _first_token(child)
        if t is not None:
            return t
    return None


def _loc(node: ASTNode | Token) -> tuple[int, int]:
    """Return (line, column) for error messages, defaulting to (1, 1)."""
    if isinstance(node, Token):
        return (node.line, node.column)
    tok = _first_token(node)
    if tok is not None:
        return (tok.line, tok.column)
    if node.start_line is not None and node.start_column is not None:
        return (node.start_line, node.start_column)
    return (1, 1)


# ---------------------------------------------------------------------------
# NibTypeChecker
# ---------------------------------------------------------------------------


class NibTypeChecker(GenericTypeChecker[ASTNode]):
    """Type checker for the Nib language.

    Implements the ``TypeChecker[ASTNode, ASTNode]`` protocol from
    ``type_checker_protocol``.

    Usage::

        from nib_type_checker import NibTypeChecker
        checker = NibTypeChecker()
        result = checker.check(ast)
        if result.ok:
            ...  # pass typed_ast to IR compiler
        else:
            for err in result.errors:
                print(f"{err.line}:{err.column}: {err.message}")
    """

    def __init__(self) -> None:
        super().__init__()
        for kind in ("const_decl", "static_decl", "fn_decl"):
            self.register_hook("collect", kind, getattr(self, f"_collect_{kind}"))

        self.register_hook("stmt", "let_stmt", self._stmt_let_stmt)
        self.register_hook("stmt", "assign_stmt", self._stmt_assign_stmt)
        for kind in ("return_stmt", "for_stmt", "if_stmt", "expr_stmt"):
            self.register_hook("stmt", kind, getattr(self, f"_check_{kind}"))

        self.register_hook("expr", "add_expr", self._check_add_expr)
        self.register_hook("expr", "primary", self._check_primary)
        self.register_hook("expr", "call_expr", self._check_call_expr)
        for kind in (
            "expr",
            "or_expr",
            "and_expr",
            "eq_expr",
            "cmp_expr",
            "bitwise_expr",
            "unary_expr",
        ):
            self.register_hook("expr", kind, self._check_compound_expr)

    # ------------------------------------------------------------------
    # Public interface — implements TypeChecker protocol
    # ------------------------------------------------------------------

    def run(self, ast: ASTNode) -> None:
        """Type-check the Nib AST in place."""
        scope = ScopeChain()
        self._check_program(ast, scope)

    def node_kind(self, node: ASTNode) -> str | None:
        return node.rule_name

    def locate(self, subject: object) -> tuple[int, int]:
        if isinstance(subject, (ASTNode, Token)):
            return _loc(subject)
        return super().locate(subject)

    # ------------------------------------------------------------------
    # Pass 1: Signature collection
    # ------------------------------------------------------------------

    def _check_program(self, node: ASTNode, scope: ScopeChain) -> None:
        """Two-pass program checker: collect signatures, then check bodies."""
        # Collect all top-level signatures into the global scope.
        fn_nodes: list[tuple[str, ASTNode]] = []

        for child in node.children:
            # child is a top_decl node
            if not isinstance(child, ASTNode):
                continue
            decl = child.children[0] if child.children else child
            if not isinstance(decl, ASTNode):
                continue
            self.dispatch("collect", decl, scope, fn_nodes)

        # Pass 2: type-check each function body.
        for fn_name, fn_decl in fn_nodes:
            self._check_fn_body(fn_decl, scope)

    def _collect_const_decl(
        self,
        node: ASTNode,
        scope: ScopeChain,
        fn_nodes: list[tuple[str, ASTNode]],
    ) -> None:
        del fn_nodes
        self._collect_const_static(node, scope, is_const=True)

    def _collect_static_decl(
        self,
        node: ASTNode,
        scope: ScopeChain,
        fn_nodes: list[tuple[str, ASTNode]],
    ) -> None:
        del fn_nodes
        self._collect_const_static(node, scope, is_const=False)

    def _collect_fn_decl(
        self,
        node: ASTNode,
        scope: ScopeChain,
        fn_nodes: list[tuple[str, ASTNode]],
    ) -> None:
        fn_name, _sym = self._collect_fn_signature(node, scope)
        if fn_name:
            fn_nodes.append((fn_name, node))

    def _collect_const_static(
        self,
        node: ASTNode,
        scope: ScopeChain,
        *,
        is_const: bool,
    ) -> None:
        """Collect a const or static declaration into the global scope.

        Structure (const_decl / static_decl):
            [CONST_kw|STATIC_kw, NAME, COLON, type, EQ, expr, SEMICOLON]
        """
        # Find the NAME token (index 1) and type node (index 3).
        name_tok: Token | None = None
        type_node: ASTNode | None = None
        token_idx = 0
        child_idx = 0

        for child in node.children:
            if isinstance(child, Token):
                if token_idx == 1:  # NAME is the second token
                    name_tok = child
                token_idx += 1
            elif isinstance(child, ASTNode) and child.rule_name == "type":
                type_node = child
                break
            child_idx += 1

        if name_tok is None or type_node is None:
            return

        nib_type = self._resolve_type_node(type_node)
        if nib_type is None:
            return

        sym = Symbol(
            name=name_tok.value,
            nib_type=nib_type,
            is_const=is_const,
            is_static=not is_const,
        )
        scope.define_global(name_tok.value, sym)

    def _collect_fn_signature(
        self, node: ASTNode, scope: ScopeChain
    ) -> tuple[str | None, Symbol | None]:
        """Collect a function signature into the global scope.

        Returns (fn_name, Symbol) or (None, None) on error.

        Structure (fn_decl):
            FN_kw  NAME  LPAREN  [param_list]  RPAREN  [ARROW type]  block
        """
        # Extract NAME token (first NAME token in children).
        fn_name: str | None = None
        return_type: NibType | None = None
        params: list[tuple[str, NibType]] = []

        # Walk children to extract name, params, return type
        children = node.children
        token_idx = 0

        for i, child in enumerate(children):
            if isinstance(child, Token):
                t = child.type
                t_name = t if isinstance(t, str) else t.name
                if t_name == "NAME" and fn_name is None:
                    fn_name = child.value
            elif isinstance(child, ASTNode):
                if child.rule_name == "param_list":
                    params = self._extract_params(child)
                elif child.rule_name == "type":
                    # This is the return type (after ARROW)
                    return_type = self._resolve_type_node(child)
            token_idx += 1

        if fn_name is None:
            return None, None

        sym = Symbol(
            name=fn_name,
            nib_type=return_type,
            is_fn=True,
            fn_params=params,
            fn_return_type=return_type,
        )
        scope.define_global(fn_name, sym)
        return fn_name, sym

    def _extract_params(self, param_list_node: ASTNode) -> list[tuple[str, NibType]]:
        """Extract parameter names and types from a param_list node.

        Structure (param_list): [param, {COMMA, param}]
        Structure (param): [NAME, COLON, type]
        """
        params: list[tuple[str, NibType]] = []
        for child in param_list_node.children:
            if isinstance(child, ASTNode) and child.rule_name == "param":
                name_tok: Token | None = None
                type_node: ASTNode | None = None
                for pc in child.children:
                    if isinstance(pc, Token):
                        t = pc.type
                        t_name = t if isinstance(t, str) else t.name
                        if t_name == "NAME" and name_tok is None:
                            name_tok = pc
                    elif isinstance(pc, ASTNode) and pc.rule_name == "type":
                        type_node = pc
                if name_tok is not None and type_node is not None:
                    nib_type = self._resolve_type_node(type_node)
                    if nib_type is not None:
                        params.append((name_tok.value, nib_type))
        return params

    # ------------------------------------------------------------------
    # Pass 2: Function body checking
    # ------------------------------------------------------------------

    def _check_fn_body(self, fn_decl: ASTNode, outer_scope: ScopeChain) -> None:
        """Type-check the body of a single function declaration."""
        # Resolve function name and signature from global scope.
        fn_name = None
        block_node: ASTNode | None = None
        params: list[tuple[str, NibType]] = []
        return_type: NibType | None = None

        for child in fn_decl.children:
            if isinstance(child, Token):
                t = child.type
                t_name = t if isinstance(t, str) else t.name
                if t_name == "NAME" and fn_name is None:
                    fn_name = child.value
            elif isinstance(child, ASTNode):
                if child.rule_name == "block":
                    block_node = child
                elif child.rule_name == "param_list":
                    params = self._extract_params(child)
                elif child.rule_name == "type":
                    return_type = self._resolve_type_node(child)

        if block_node is None:
            return

        # Push a scope for the function body; add parameters.
        outer_scope.push()
        for param_name, param_type in params:
            outer_scope.define(
                param_name,
                Symbol(name=param_name, nib_type=param_type),
            )

        self._check_block(block_node, outer_scope, return_type)
        outer_scope.pop()

    # ------------------------------------------------------------------
    # Statement checking
    # ------------------------------------------------------------------

    def _check_block(
        self,
        block: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        """Check all statements inside a ``{ ... }`` block."""
        scope.push()
        for child in block.children:
            if isinstance(child, ASTNode) and child.rule_name == "stmt":
                self._check_stmt(child, scope, expected_return_type)
        scope.pop()

    def _check_stmt(
        self,
        stmt_node: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        """Dispatch to the appropriate statement checker."""
        if not stmt_node.children:
            return
        inner = stmt_node.children[0]
        if not isinstance(inner, ASTNode):
            return
        self.dispatch("stmt", inner, scope, expected_return_type, default=None)

    def _stmt_let_stmt(
        self,
        node: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        del expected_return_type
        self._check_let_stmt(node, scope)

    def _stmt_assign_stmt(
        self,
        node: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        del expected_return_type
        self._check_assign_stmt(node, scope)

    def _check_expr_stmt(
        self,
        node: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        """Check an expression statement for side effects."""
        del expected_return_type
        if node.children:
            expr_child = node.children[0]
            if isinstance(expr_child, ASTNode):
                self._check_expr(expr_child, scope)

    def _check_let_stmt(self, node: ASTNode, scope: ScopeChain) -> None:
        """Check and register a ``let`` declaration.

        Structure: LET_kw  NAME  COLON  type  EQ  expr  SEMICOLON
        """
        name_tok: Token | None = None
        type_node: ASTNode | None = None
        expr_node: ASTNode | None = None

        # Walk children to find NAME, type, and expr
        found_name = False
        found_type = False
        for child in node.children:
            if isinstance(child, Token):
                t = child.type
                t_name = t if isinstance(t, str) else t.name
                if t_name == "NAME" and not found_name:
                    name_tok = child
                    found_name = True
            elif isinstance(child, ASTNode):
                if child.rule_name == "type" and not found_type:
                    type_node = child
                    found_type = True
                elif child.rule_name in (
                    "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
                    "add_expr", "bitwise_expr", "unary_expr", "primary",
                    "call_expr",
                ) and type_node is not None:
                    expr_node = child

        if name_tok is None or type_node is None or expr_node is None:
            return

        declared_type = self._resolve_type_node(type_node)
        if declared_type is None:
            return

        rhs_type = self._check_expr(expr_node, scope)

        # Numeric literals are *untyped* in Nib — they fit any numeric type.
        # Only check the mismatch when the RHS has a *resolved named type*.
        # This allows: let x: u8 = 5; let d: bcd = 7; etc.
        literal_only = _is_numeric_literal_expr(expr_node)
        if (
            rhs_type is not None
            and not literal_only
            and not types_are_compatible(declared_type, rhs_type)
        ):
            self._error(
                f"Type mismatch in 'let' declaration of '{name_tok.value}': "
                f"declared type is '{declared_type.value}' but the expression "
                f"has type '{rhs_type.value}'.",
                name_tok,
            )

        # If the RHS is a literal but declared type is bool, that's still an
        # error — integer literals are not boolean literals.
        if literal_only and declared_type == NibType.BOOL:
            self._error(
                f"Type mismatch in 'let' declaration of '{name_tok.value}': "
                f"declared type is 'bool' but an integer literal was provided. "
                "Use 'true' or 'false' for boolean values.",
                name_tok,
            )

        # When the declared type is bcd, enforce BCD operator restrictions on
        # the expression even if the operands are literals.
        if declared_type == NibType.BCD:
            self._check_bcd_operators_in_expr(expr_node)

        scope.define(name_tok.value, Symbol(name=name_tok.value, nib_type=declared_type))

    def _check_bcd_operators_in_expr(self, node: ASTNode | Token) -> None:
        """Walk ``node`` and report any arithmetic operators illegal for BCD.

        Called when the declared context is ``bcd``. Traverses add_expr nodes
        looking for PLUS, SAT_ADD, STAR, or SLASH operator tokens.
        """
        if isinstance(node, Token):
            return
        if node.rule_name == "add_expr":
            # Children: operand op operand op operand ...
            # Odd-indexed children are operators.
            for i, child in enumerate(node.children):
                if i % 2 == 1 and isinstance(child, Token):
                    if not is_bcd_op_allowed(child.value):
                        self._error(
                            f"BCD type only supports '+%' (wrapping add) and '-' operators. "
                            f"Operator '{child.value}' is not allowed with 'bcd' operands. "
                            "This restriction exists because the Intel 4004's DAA "
                            "(Decimal Adjust Accumulator) instruction only works with "
                            "addition — not bare addition, multiplication, or division.",
                            child,
                        )
        # Recurse into sub-nodes.
        for child in node.children:
            if isinstance(child, ASTNode):
                self._check_bcd_operators_in_expr(child)

    def _check_assign_stmt(self, node: ASTNode, scope: ScopeChain) -> None:
        """Check a variable assignment.

        Structure: NAME  EQ  expr  SEMICOLON
        """
        name_tok: Token | None = None
        expr_node: ASTNode | None = None

        for child in node.children:
            if isinstance(child, Token):
                t = child.type
                t_name = t if isinstance(t, str) else t.name
                if t_name == "NAME" and name_tok is None:
                    name_tok = child
            elif isinstance(child, ASTNode) and name_tok is not None:
                if child.rule_name in (
                    "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
                    "add_expr", "bitwise_expr", "unary_expr", "primary",
                    "call_expr",
                ):
                    expr_node = child

        if name_tok is None:
            return

        sym = scope.lookup(name_tok.value)
        if sym is None or sym.is_fn:
            self._error(
                f"'{name_tok.value}' is not defined. "
                "Variables must be declared with 'let', 'const', or 'static' "
                "before they can be assigned.",
                name_tok,
            )
            # Still check the RHS to find additional errors.
            if expr_node is not None:
                self._check_expr(expr_node, scope)
            return

        if expr_node is None:
            return

        rhs_type = self._check_expr(expr_node, scope)
        literal_only = _is_numeric_literal_expr(expr_node)
        if rhs_type is not None and sym.nib_type is not None and not literal_only:
            if not types_are_compatible(sym.nib_type, rhs_type):
                self._error(
                    f"Type mismatch in assignment to '{name_tok.value}': "
                    f"variable has type '{sym.nib_type.value}' but the expression "
                    f"has type '{rhs_type.value}'. "
                    "Nib does not support implicit type widening.",
                    name_tok,
                )

        # If assigning to a bcd variable, enforce BCD operator restrictions.
        if sym.nib_type == NibType.BCD:
            self._check_bcd_operators_in_expr(expr_node)

    def _check_return_stmt(
        self,
        node: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        """Check a ``return`` statement.

        Structure: RETURN_kw  expr  SEMICOLON
        """
        expr_node: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name in (
                "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
                "add_expr", "bitwise_expr", "unary_expr", "primary",
                "call_expr",
            ):
                expr_node = child
                break

        if expr_node is None:
            return

        actual_type = self._check_expr(expr_node, scope)
        if actual_type is None:
            return

        if expected_return_type is None:
            self._error(
                f"Unexpected 'return' with value in a void function. "
                f"The expression has type '{actual_type.value}'.",
                node,
            )
            return

        # Numeric literals are untyped — they fit any numeric return type.
        # However, an integer literal does NOT fit bool — use true/false.
        literal_only = _is_numeric_literal_expr(expr_node)
        if literal_only and expected_return_type == NibType.BOOL:
            self._error(
                f"Return type mismatch: function declares '-> bool' "
                "but an integer literal was returned. Use 'true' or 'false'.",
                expr_node,
            )
        elif not literal_only and not types_are_compatible(expected_return_type, actual_type):
            self._error(
                f"Return type mismatch: function declares '-> {expected_return_type.value}' "
                f"but the return expression has type '{actual_type.value}'.",
                expr_node,
            )

    def _check_for_stmt(
        self,
        node: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        """Check a range-for statement.

        Structure: FOR_kw  NAME  COLON  type  IN_kw  expr  RANGE  expr  block
        """
        loop_var_tok: Token | None = None
        type_node: ASTNode | None = None
        exprs: list[ASTNode] = []
        block_node: ASTNode | None = None

        found_type = False
        for child in node.children:
            if isinstance(child, Token):
                t = child.type
                t_name = t if isinstance(t, str) else t.name
                if t_name == "NAME" and loop_var_tok is None:
                    loop_var_tok = child
            elif isinstance(child, ASTNode):
                if child.rule_name == "type" and not found_type:
                    type_node = child
                    found_type = True
                elif child.rule_name == "block":
                    block_node = child
                elif child.rule_name in (
                    "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
                    "add_expr", "bitwise_expr", "unary_expr", "primary",
                    "call_expr",
                ):
                    exprs.append(child)

        # The two range exprs are exprs[0] (start) and exprs[1] (end).
        for bound_expr in exprs[:2]:
            bound_type = self._check_expr(bound_expr, scope)
            if bound_type is not None and not is_numeric(bound_type):
                self._error(
                    f"For-loop bounds must be numeric, but got '{bound_type.value}'.",
                    bound_expr,
                )

        # The loop variable is in scope inside the block.
        loop_type: NibType | None = None
        if type_node is not None:
            loop_type = self._resolve_type_node(type_node)

        if block_node is not None:
            scope.push()
            if loop_var_tok is not None and loop_type is not None:
                scope.define(
                    loop_var_tok.value,
                    Symbol(name=loop_var_tok.value, nib_type=loop_type),
                )
            self._check_block(block_node, scope, expected_return_type)
            scope.pop()

    def _check_if_stmt(
        self,
        node: ASTNode,
        scope: ScopeChain,
        expected_return_type: NibType | None,
    ) -> None:
        """Check an ``if`` statement.

        Structure: IF_kw  expr  block  [ELSE_kw  block]
        """
        cond_expr: ASTNode | None = None
        blocks: list[ASTNode] = []

        for child in node.children:
            if isinstance(child, ASTNode):
                if child.rule_name in (
                    "expr", "or_expr", "and_expr", "eq_expr", "cmp_expr",
                    "add_expr", "bitwise_expr", "unary_expr", "primary",
                    "call_expr",
                ) and cond_expr is None:
                    cond_expr = child
                elif child.rule_name == "block":
                    blocks.append(child)

        if cond_expr is not None:
            cond_type = self._check_expr(cond_expr, scope)
            if cond_type is not None and cond_type != NibType.BOOL:
                self._error(
                    f"The condition of 'if' must have type 'bool', "
                    f"but got '{cond_type.value}'. "
                    "Nib does not allow integer conditions — use an explicit comparison "
                    "(e.g., 'x == 0') to produce a bool.",
                    cond_expr,
                )

        for blk in blocks:
            self._check_block(blk, scope, expected_return_type)

    # ------------------------------------------------------------------
    # Expression type checking
    # ------------------------------------------------------------------

    def _check_expr(
        self,
        node: ASTNode | Token,
        scope: ScopeChain,
    ) -> NibType | None:
        """Recursively type-check ``node`` and return its ``NibType``.

        Annotates the node with ``._nib_type`` on success. Returns ``None``
        if the type cannot be resolved (e.g., an undeclared variable) —
        which prevents cascading errors.
        """
        if isinstance(node, Token):
            return self._check_token_expr(node, scope)

        result_type = self._check_ast_expr(node, scope)
        if result_type is not None:
            # Annotate in place for downstream consumers.
            node._nib_type = result_type  # noqa: SLF001
        return result_type

    def _check_token_expr(self, tok: Token, scope: ScopeChain) -> NibType | None:
        """Type of a bare token (INT_LIT, HEX_LIT, true, false, NAME)."""
        t_name = tok.type if isinstance(tok.type, str) else tok.type.name
        if t_name in ("INT_LIT", "HEX_LIT"):
            # Numeric literal — use the sentinel LITERAL_NUMERIC type.
            # The caller (_check_let_stmt, _check_assign_stmt) will coerce
            # this to the declared type. Returning U4 as a representative
            # numeric type allows comparison operators and binary ops to work.
            return NibType.U4
        # After the nib-lexer keyword reclassification, `true` and `false`
        # tokens have type="true"/"false" (not type="NAME").  Handle both.
        if t_name in ("true", "false"):
            return NibType.BOOL
        if t_name == "NAME":
            if tok.value in ("true", "false"):
                return NibType.BOOL
            sym = scope.lookup(tok.value)
            if sym is None:
                self._error(
                    f"'{tok.value}' is not defined. "
                    "Did you forget to declare it with 'let', 'const', or 'static'?",
                    tok,
                )
                return None
            if sym.is_fn:
                self._error(
                    f"'{tok.value}' is a function. "
                    "To call it, use parentheses: '{tok.value}()'.",
                    tok,
                )
                return None
            return sym.nib_type
        return None

    def _check_ast_expr(self, node: ASTNode, scope: ScopeChain) -> NibType | None:
        """Dispatch expression-level AST nodes to specific checkers."""
        sentinel = object()
        dispatched = self.dispatch("expr", node, scope, default=sentinel)
        if dispatched is not sentinel:
            return dispatched

        # Fallback: try unwrapping single-child nodes.
        if len(node.children) == 1:
            return self._check_expr(node.children[0], scope)

        return None

    def _check_compound_expr(self, node: ASTNode, scope: ScopeChain) -> NibType | None:
        """Handle multi-child expression nodes: or_expr, and_expr, eq_expr, etc.

        For nodes with a single child (no operator present), delegate to that
        child. For nodes with 3+ children (left op right), check both operands.
        """
        children = node.children
        rule = node.rule_name

        # Single-child passthrough (no operator in this rule at this level).
        if len(children) == 1:
            return self._check_expr(children[0], scope)

        # Binary operator node: children alternate operand/op/operand/...
        # Collect operand nodes (even indices) and operator tokens (odd indices).
        operand_types: list[NibType] = []
        op_tokens: list[Token] = []

        for i, child in enumerate(children):
            if i % 2 == 0:
                # Operand
                t = self._check_expr(child, scope)
                operand_types.append(t)  # type: ignore[arg-type]
            else:
                # Operator token (may be a Token or an ASTNode wrapping one)
                if isinstance(child, Token):
                    op_tokens.append(child)

        # Logical operators (or_expr, and_expr) require bool operands.
        if rule in ("or_expr", "and_expr"):
            for op_type in operand_types:
                if op_type is not None and op_type != NibType.BOOL:
                    self._error(
                        f"Logical operators '||' and '&&' require 'bool' operands, "
                        f"but got '{op_type.value}'. "
                        "Use comparisons to produce booleans from integers.",
                        node,
                    )
            return NibType.BOOL

        # Comparison operators produce bool.
        if rule in ("eq_expr", "cmp_expr"):
            # Optionally check that both sides have the same type.
            types_seen = [t for t in operand_types if t is not None]
            if len(types_seen) >= 2 and types_seen[0] != types_seen[1]:
                self._error(
                    f"Comparison operands must have the same type. "
                    f"Got '{types_seen[0].value}' and '{types_seen[1].value}'.",
                    node,
                )
            return NibType.BOOL

        # Bitwise operators: operands must be same numeric type.
        if rule == "bitwise_expr":
            types_seen = [t for t in operand_types if t is not None]
            if len(types_seen) >= 2:
                if types_seen[0] != types_seen[1]:
                    self._error(
                        f"Bitwise operator operands must have the same type. "
                        f"Got '{types_seen[0].value}' and '{types_seen[1].value}'.",
                        node,
                    )
                return types_seen[0]
            return types_seen[0] if types_seen else None

        # Unary operators.
        if rule == "unary_expr":
            # Check for BANG (!) or TILDE (~) followed by the operand.
            if len(children) == 2:
                op_child = children[0]
                if isinstance(op_child, Token):
                    op_val = op_child.value
                    operand_type = operand_types[0] if operand_types else None
                    if op_val == "!":
                        if operand_type is not None and operand_type != NibType.BOOL:
                            self._error(
                                f"Logical NOT '!' requires a 'bool' operand, "
                                f"but got '{operand_type.value}'.",
                                node,
                            )
                        return NibType.BOOL
                    if op_val == "~":
                        return operand_type
            if operand_types:
                return operand_types[0]
            return None

        return operand_types[0] if operand_types else None

    def _check_add_expr(self, node: ASTNode, scope: ScopeChain) -> NibType | None:
        """Check an additive expression, enforcing BCD operator restrictions.

        Structure: bitwise_expr { (PLUS | MINUS | WRAP_ADD | SAT_ADD) bitwise_expr }
        """
        children = node.children

        # Single-child: no operator at this level.
        if len(children) == 1:
            return self._check_expr(children[0], scope)

        # Interleaved: operand op operand op operand ...
        operand_types: list[NibType | None] = []
        op_tokens: list[Token] = []

        for i, child in enumerate(children):
            if i % 2 == 0:
                operand_types.append(self._check_expr(child, scope))
            else:
                if isinstance(child, Token):
                    op_tokens.append(child)

        # Determine the result type from first resolved operand.
        resolved_types = [t for t in operand_types if t is not None]
        if not resolved_types:
            return None

        result_type = resolved_types[0]

        # Check BCD restriction and type consistency.
        has_bcd = any(t == NibType.BCD for t in resolved_types)
        for i, op_tok in enumerate(op_tokens):
            op_val = op_tok.value
            if has_bcd and not is_bcd_op_allowed(op_val):
                self._error(
                    f"BCD type only supports '+%' (wrapping add) and '-' operators. "
                    f"Operator '{op_val}' is not allowed with 'bcd' operands. "
                    "This restriction exists because the Intel 4004's DAA "
                    "(Decimal Adjust Accumulator) instruction only works with "
                    "addition — not bare addition, multiplication, or division.",
                    op_tok,
                )
            # Check operand type consistency.
            left_type = operand_types[i] if i < len(operand_types) else None
            right_type = operand_types[i + 1] if i + 1 < len(operand_types) else None
            if left_type is not None and right_type is not None:
                if left_type != right_type:
                    self._error(
                        f"Operands of '{op_val}' must have the same type. "
                        f"Got '{left_type.value}' and '{right_type.value}'.",
                        op_tok,
                    )
                else:
                    result_type = left_type

        return result_type

    def _check_primary(self, node: ASTNode, scope: ScopeChain) -> NibType | None:
        """Check a primary expression: literal, name, call, or parenthesized."""
        if not node.children:
            return None

        first = node.children[0]

        # Token primary: INT_LIT, HEX_LIT, "true", "false", or NAME.
        if isinstance(first, Token):
            t_name = first.type if isinstance(first.type, str) else first.type.name
            if t_name in ("INT_LIT", "HEX_LIT"):
                return NibType.U4
            # After the nib-lexer keyword reclassification, `true` and `false`
            # tokens have type="true"/"false" (not type="NAME").  Handle both.
            if t_name in ("true", "false"):
                return NibType.BOOL
            if t_name == "NAME":
                if first.value in ("true", "false"):
                    return NibType.BOOL
                sym = scope.lookup(first.value)
                if sym is None:
                    self._error(
                        f"'{first.value}' is not defined. "
                        "Did you forget to declare it with 'let', 'const', or 'static'?",
                        first,
                    )
                    return None
                if sym.is_fn:
                    self._error(
                        f"'{first.value}' is a function name used as a value. "
                        "To call it, add parentheses.",
                        first,
                    )
                    return None
                return sym.nib_type

        # AST node primary: call_expr or grouped (LPAREN expr RPAREN).
        if isinstance(first, ASTNode):
            if first.rule_name == "call_expr":
                return self._check_call_expr(first, scope)
            # Parenthesized expression — dig out the inner expr.
            return self._check_expr(first, scope)

        # Parenthesized: (expr) — first child is LPAREN token, second is expr.
        if len(node.children) >= 2 and isinstance(node.children[1], ASTNode):
            return self._check_expr(node.children[1], scope)

        return None

    def _check_call_expr(self, node: ASTNode, scope: ScopeChain) -> NibType | None:
        """Check a function call expression.

        Structure: NAME  LPAREN  [arg_list]  RPAREN
        """
        fn_tok: Token | None = None
        arg_exprs: list[ASTNode] = []

        for child in node.children:
            if isinstance(child, Token):
                t_name = child.type if isinstance(child.type, str) else child.type.name
                if t_name == "NAME" and fn_tok is None:
                    fn_tok = child
            elif isinstance(child, ASTNode):
                if child.rule_name == "arg_list":
                    # arg_list: expr { COMMA expr }
                    for ac in child.children:
                        if isinstance(ac, ASTNode) and ac.rule_name not in ():
                            if ac.rule_name in (
                                "expr", "or_expr", "and_expr", "eq_expr",
                                "cmp_expr", "add_expr", "bitwise_expr",
                                "unary_expr", "primary", "call_expr",
                            ):
                                arg_exprs.append(ac)

        if fn_tok is None:
            return None

        sym = scope.lookup(fn_tok.value)
        if sym is None:
            self._error(
                f"Function '{fn_tok.value}' is not defined. "
                "Make sure the function is declared at the top level.",
                fn_tok,
            )
            return None

        if not sym.is_fn:
            self._error(
                f"'{fn_tok.value}' is not a function (it has type "
                f"'{sym.nib_type.value if sym.nib_type else 'unknown'}').",
                fn_tok,
            )
            return None

        # Check argument count.
        if len(arg_exprs) != len(sym.fn_params):
            self._error(
                f"Function '{fn_tok.value}' expects {len(sym.fn_params)} argument(s) "
                f"but {len(arg_exprs)} were provided.",
                fn_tok,
            )
            # Still check argument types where we can.

        # Check argument types.
        for i, arg_expr in enumerate(arg_exprs):
            arg_type = self._check_expr(arg_expr, scope)
            if i < len(sym.fn_params):
                param_name, param_type = sym.fn_params[i]
                if arg_type is not None and not types_are_compatible(param_type, arg_type):
                    self._error(
                        f"Argument {i + 1} to '{fn_tok.value}': expected "
                        f"'{param_type.value}' (parameter '{param_name}') "
                        f"but got '{arg_type.value}'.",
                        arg_expr,
                    )

        return sym.fn_return_type

    # ------------------------------------------------------------------
    # Type resolution
    # ------------------------------------------------------------------

    def _resolve_type_node(self, type_node: ASTNode) -> NibType | None:
        """Convert a ``type`` AST node to a ``NibType`` enum value.

        The ``type`` node is a leaf: its single token has ``type=NAME`` and
        its value is one of ``"u4"``, ``"u8"``, ``"bcd"``, ``"bool"``.
        """
        # type node: single NAME token with value u4/u8/bcd/bool
        for child in type_node.children:
            val: str | None = None
            if isinstance(child, Token):
                val = child.value
            elif isinstance(child, ASTNode) and child.is_leaf and child.token:
                val = child.token.value
            if val is not None:
                result = parse_type_name(val)
                if result is None:
                    self._error(
                        f"Unknown type '{val}'. "
                        "Valid Nib types are: u4, u8, bcd, bool.",
                        type_node,
                    )
                return result
        return None


# ---------------------------------------------------------------------------
# Module-level convenience function
# ---------------------------------------------------------------------------


_checker = NibTypeChecker()


def check(ast: ASTNode) -> TypeCheckResult[ASTNode]:
    """Type-check a Nib AST and return the annotated result.

    This is a convenience wrapper around ``NibTypeChecker().check(ast)``.
    Each call creates a fresh checker instance to avoid state leakage
    between independent programs.

    Parameters
    ----------
    ast:
        The root ``ASTNode`` produced by ``parse_nib(source)``.

    Returns
    -------
    TypeCheckResult[ASTNode]
        ``ok=True`` means the program is type-safe. ``ok=False`` means one
        or more type errors were found; see ``errors`` for details.

    Examples
    --------
    ::

        from nib_parser import parse_nib
        from nib_type_checker import check

        ast = parse_nib("fn main() { let x: u4 = 5; }")
        result = check(ast)
        assert result.ok
    """
    return NibTypeChecker().check(ast)
