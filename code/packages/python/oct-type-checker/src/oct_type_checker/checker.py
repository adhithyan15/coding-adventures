"""Oct type checker — walks an untyped AST and produces a typed AST.

=============================================================================
OVERVIEW
=============================================================================

    The type checker is the third stage of the Oct compiler pipeline:

    Source text
        → Oct Lexer      (characters → tokens)
        → Oct Parser     (tokens → untyped ASTNode tree)
        → Type Checker   (untyped AST → typed AST)   ← this module
        → IR Compiler    (typed AST → IR)
        → Intel 8008 IR Validator  (IR → validated IR)
        → IR-to-Intel-8008 Compiler (validated IR → 8008 assembly)

The ``OctTypeChecker`` class implements the ``TypeChecker[ASTNode, ASTNode]``
protocol from ``type_checker_protocol``. Calling ``check(ast)`` performs two
passes over the AST:

  **Pass 1 — Signature Collection**
    Walk the top-level declarations and record:
    - ``static`` names and their types in the global scope.
    - Function names, parameter types, and return types.

  **Pass 2 — Body Type-Checking**
    Walk each function body with the complete global scope available.
    Check every statement, every expression, every assignment for type
    correctness. Annotate each expression node with ``._oct_type``.

The two-pass structure means functions can be called before they are declared
in the source file — you do not need prototypes or forward declarations.

=============================================================================
OCT TYPE SYSTEM
=============================================================================

Oct has exactly two value types:

``u8`` — unsigned 8-bit integer, range 0–255.
    This is the native word size of the Intel 8008. All arithmetic wraps
    modulo 256. There is no overflow trap — use ``carry()`` after addition
    or subtraction to detect overflow manually.

``bool`` — boolean, values ``true`` (1) and ``false`` (0).
    Stored as a u8 containing 0 or 1. All conditionals (``if``, ``while``)
    require a ``bool``.

**Compatibility rule**: ``bool`` may be used wherever ``u8`` is expected
(implicit coercion), but a ``u8`` may NOT be used where ``bool`` is expected
without an explicit comparison. This prevents the classic C bug of writing
``if (x)`` when you meant ``if (x != 0)``.

Examples::

    let x: u8 = true;      # ok — bool coerces to u8
    let y: bool = carry();  # ok — carry() returns bool
    let z: bool = x;        # ERROR — u8 cannot coerce to bool

=============================================================================
LANGUAGE INVARIANTS ENFORCED
=============================================================================

This checker enforces *language-level* invariants only. It is completely
independent of any compilation target. The same checker runs whether you
are targeting the Intel 8008 or a future backend.

Invariants enforced:

1. **All names declared before use**: Variables must be declared with
   ``let`` or ``static`` before they appear in expressions or on the
   left-hand side of assignments.

2. **Expression types correct bottom-up**: The type of a composite
   expression (e.g., ``a + b``) is determined from the types of its
   operands.

   - Arithmetic ``+``, ``-`` and bitwise ``&``, ``|``, ``^``: both operands
     must be ``u8``-compatible; result is ``u8``.
   - Bitwise NOT ``~``: operand must be ``u8``-compatible; result is ``u8``.
   - Logical ``&&``, ``||``: both operands must be ``bool``; result is
     ``bool``.
   - Logical NOT ``!``: operand must be ``bool``; result is ``bool``.
   - Comparisons ``==``, ``!=``, ``<``, ``>``, ``<=``, ``>=``: both operands
     must be ``u8``-compatible; result is ``bool``.

3. **Assignment type compatibility**: The RHS type must be compatible with
   the LHS declared type (``bool`` → ``u8`` allowed; ``u8`` → ``bool`` not).

4. **Function call argument types match parameter types**.

5. **Intrinsic argument types**: ``adc``, ``sbb``, ``rlc``, ``rrc``,
   ``ral``, ``rar``, ``parity`` require ``u8`` arguments. ``out``'s value
   argument must be ``u8``-compatible.

6. **Port arguments are compile-time literals**: The first argument of
   ``in(PORT)`` and ``out(PORT, val)`` must be an integer literal (INT_LIT,
   HEX_LIT, or BIN_LIT), not a computed value. The Intel 8008 encodes port
   numbers directly in the instruction opcode — there is no variable-port
   addressing mode.

7. **``if`` and ``while`` conditions must be ``bool``**: Unlike C, integers
   are not implicitly truthy. This prevents ``if (x)`` where ``if (x != 0)``
   was intended.

8. **Return statements match the declared return type**.

9. **Integer literals in range 0–255**.

10. **``main`` function exists** with no parameters and no return type.

11. **No undefined functions called**.

=============================================================================
WHAT IS NOT CHECKED HERE
=============================================================================

- **Max 4 locals per function**: Hardware register constraint (B, C, D, E
  only). Belongs in the Intel 8008 IR validator, which runs after IR codegen.

- **Max 7 call depth**: Hardware push-down stack limit. Backend concern.

- **Port range 0–7 (input), 0–23 (output)**: ISA-specific limit. Backend
  concern.

- **Program size ≤ 16 KB**: 8008 address space limit. Backend concern.

Keeping hardware constraints out of the type checker makes the design
composable. The same ``OctTypeChecker`` can in principle target the 8080
(a strict superset of the 8008 ISA) without any modifications.

=============================================================================
ANNOTATION STRATEGY
=============================================================================

Rather than building a parallel tree, we annotate in place by setting a
``._oct_type`` attribute (either ``"u8"`` or ``"bool"``) on each ASTNode
that corresponds to an expression. This preserves source location info and
keeps the output structurally identical to the input.

The annotated root node is returned as ``TypeCheckResult.typed_ast``.

=============================================================================
AST STRUCTURE (from oct.grammar)
=============================================================================

The oct-parser produces the following ASTNode shapes (children listed in
grammar order, square brackets denote optional):

  program       → { top_decl }
  top_decl      → static_decl | fn_decl
  static_decl   → Token("static") Token(NAME) Token(COLON) ASTNode(type)
                   Token(EQ) ASTNode(expr) Token(SEMICOLON)
  fn_decl       → Token("fn") Token(NAME) Token(LPAREN) [ASTNode(param_list)]
                   Token(RPAREN) [Token(ARROW) ASTNode(type)] ASTNode(block)
  param_list    → ASTNode(param) { Token(COMMA) ASTNode(param) }
  param         → Token(NAME) Token(COLON) ASTNode(type)
  block         → Token(LBRACE) { ASTNode(stmt) } Token(RBRACE)
  stmt          → let_stmt | static_decl | assign_stmt | return_stmt
                | if_stmt | while_stmt | loop_stmt | break_stmt | expr_stmt
  let_stmt      → Token("let") Token(NAME) Token(COLON) ASTNode(type)
                   Token(EQ) ASTNode(expr) Token(SEMICOLON)
  assign_stmt   → Token(NAME) Token(EQ) ASTNode(expr) Token(SEMICOLON)
  return_stmt   → Token("return") [ASTNode(expr)] Token(SEMICOLON)
  if_stmt       → Token("if") ASTNode(expr) ASTNode(block)
                   [Token("else") ASTNode(block)]
  while_stmt    → Token("while") ASTNode(expr) ASTNode(block)
  loop_stmt     → Token("loop") ASTNode(block)
  break_stmt    → Token("break") Token(SEMICOLON)
  expr_stmt     → ASTNode(expr) Token(SEMICOLON)

  Expressions — binary nodes have flat children (left, op, right, op, right, …)
  or single child (pass-through when there is no operator at this level):

  expr          → ASTNode(or_expr)   [alias: pass-through]
  or_expr       → ASTNode(and_expr) [Token(LOR) ASTNode(and_expr) …]
  and_expr      → ASTNode(eq_expr)  [Token(LAND) ASTNode(eq_expr) …]
  eq_expr       → ASTNode(cmp_expr) [Token(EQ_EQ|NEQ) ASTNode(cmp_expr) …]
  cmp_expr      → ASTNode(add_expr) [Token(LT|GT|LEQ|GEQ) ASTNode(add_expr) …]
  add_expr      → ASTNode(bitwise_expr) [Token(PLUS|MINUS) ASTNode(bitwise_expr) …]
  bitwise_expr  → ASTNode(unary_expr) [Token(AMP|PIPE|CARET) ASTNode(unary_expr) …]
  unary_expr    → Token(BANG|TILDE) ASTNode(unary_expr) | ASTNode(primary)
  primary       → ASTNode(intrinsic_call) | ASTNode(call_expr)
                | Token(INT_LIT) | Token(HEX_LIT) | Token(BIN_LIT)
                | Token("true") | Token("false") | Token(NAME)
                | Token(LPAREN) ASTNode(expr) Token(RPAREN)
  call_expr     → Token(NAME) Token(LPAREN) [ASTNode(arg_list)] Token(RPAREN)
  arg_list      → ASTNode(expr) { Token(COMMA) ASTNode(expr) }
  intrinsic_call→ Token("in"|"out"|…) Token(LPAREN) … Token(RPAREN)
"""

from __future__ import annotations

from dataclasses import dataclass

from lang_parser import ASTNode
from lexer import Token
from type_checker_protocol import (
    GenericTypeChecker,
    TypeCheckResult,
    TypeErrorDiagnostic,
)

# ---------------------------------------------------------------------------
# Oct type representation
# ---------------------------------------------------------------------------

# Oct has exactly two value types.  We represent them as plain strings so
# that the IR compiler and other downstream consumers can read them without
# importing this package.
#
#   "u8"  — unsigned 8-bit integer (range 0–255)
#   "bool" — boolean (true=1, false=0); stored as u8 but type-checked separately
#
# "void" is used internally to represent functions with no return value.
# It never appears as a variable type.
OctType = str  # "u8" | "bool" | "void"

_VALID_TYPES: frozenset[str] = frozenset({"u8", "bool"})

# Arithmetic and bitwise operators that require u8-compatible operands.
_U8_OPS: frozenset[str] = frozenset({"PLUS", "MINUS", "AMP", "PIPE", "CARET"})

# Comparison operators: require u8-compatible operands, return bool.
_CMP_OPS: frozenset[str] = frozenset({"EQ_EQ", "NEQ", "LT", "GT", "LEQ", "GEQ"})

# Logical operators: require bool operands.
_BOOL_OPS: frozenset[str] = frozenset({"LAND", "LOR"})


def _is_u8_compatible(typ: OctType | None) -> bool:
    """Return True if ``typ`` can be used in a u8 context.

    Both ``u8`` and ``bool`` are u8-compatible. ``bool`` is stored as 0 or 1
    in a u8 register, so it freely coerces to u8 in arithmetic.

    Examples::

        _is_u8_compatible("u8")   # True
        _is_u8_compatible("bool") # True — bool coerces to u8
        _is_u8_compatible("void") # False
        _is_u8_compatible(None)   # False — unknown/error type
    """
    return typ in ("u8", "bool")


def _assignable(src: OctType | None, dst: OctType | None) -> bool:
    """Return True if a value of type ``src`` can be assigned to ``dst``.

    Rules:
    - ``u8``  → ``u8``   : ok
    - ``bool``→ ``u8``   : ok (bool coerces implicitly to u8)
    - ``bool``→ ``bool`` : ok
    - ``u8``  → ``bool`` : ERROR (must use explicit comparison)
    - anything → None  : not applicable
    - None   → anything : unknown type (already reported), silently ok

    The asymmetry reflects hardware reality: on the 8008, a bool result
    (comparison, carry(), parity()) is just a byte containing 0 or 1 —
    storing it in a u8 variable is perfectly sensible.  But treating an
    arbitrary u8 as a boolean condition risks the ``if (x = 0)`` class
    of bugs, so we require an explicit ``x != 0`` comparison.
    """
    if src is None:
        # Propagated error — already reported; don't double-report.
        return True
    if dst is None:
        return True
    if src == dst:
        return True
    # bool coerces to u8 — the only allowed implicit coercion.
    return src == "bool" and dst == "u8"


# ---------------------------------------------------------------------------
# Function signature
# ---------------------------------------------------------------------------


@dataclass
class _FnInfo:
    """Collected function signature (from Pass 1).

    Attributes
    ----------
    name:
        The function's declared name.
    params:
        List of (param_name, param_type) pairs in declaration order.
    return_type:
        The declared return type string (``"u8"`` or ``"bool"``), or
        ``None`` for void functions.
    node:
        The ``fn_decl`` ASTNode, used to report location in diagnostics.
    """

    name: str
    params: list[tuple[str, OctType]]
    return_type: OctType | None
    node: ASTNode


# ---------------------------------------------------------------------------
# AST traversal helpers
# ---------------------------------------------------------------------------


def _tok_type_name(child: ASTNode | Token) -> str:
    """Return the token type as a string, normalising enum → name.

    After keyword promotion in ``tokenize_oct``, keyword tokens have their
    ``type`` field set to the keyword string (e.g. ``"fn"``, ``"carry"``).
    All other tokens carry a ``TokenType`` enum value.  This helper
    normalises both so callers can compare uniformly.
    """
    if isinstance(child, Token):
        t = child.type
        return t if isinstance(t, str) else t.name
    # ASTNode leaf: delegate to the embedded token.
    if child.is_leaf and child.token is not None:
        t = child.token.type
        return t if isinstance(t, str) else t.name
    return ""


def _tok_value(child: ASTNode | Token) -> str:
    """Return the token value string from a Token or leaf ASTNode."""
    if isinstance(child, Token):
        return child.value
    if child.is_leaf and child.token is not None:
        return child.token.value
    return ""


def _first_token(node: ASTNode | Token) -> Token | None:
    """Return the first token reachable from *node* (depth-first)."""
    if isinstance(node, Token):
        return node
    for child in node.children:
        tok = _first_token(child)
        if tok is not None:
            return tok
    return None


def _loc(node: ASTNode | Token) -> tuple[int, int]:
    """Return (line, column) for error messages (1-based, defaults to 1,1)."""
    if isinstance(node, Token):
        return (node.line, node.column)
    tok = _first_token(node)
    if tok is not None:
        return (tok.line, tok.column)
    if node.start_line is not None and node.start_column is not None:
        return (node.start_line, node.start_column)
    return (1, 1)


def _direct_token_of_type(node: ASTNode, type_name: str) -> Token | None:
    """Return the first direct Token child with the given type name."""
    for child in node.children:
        if isinstance(child, Token) and _tok_type_name(child) == type_name:
            return child
    return None


def _direct_node_of_rule(node: ASTNode, rule_name: str) -> ASTNode | None:
    """Return the first direct ASTNode child with the given rule_name."""
    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == rule_name:
            return child
    return None


def _all_direct_nodes(node: ASTNode) -> list[ASTNode]:
    """Return all direct ASTNode children (excluding Token children)."""
    return [c for c in node.children if isinstance(c, ASTNode)]


# ---------------------------------------------------------------------------
# Type resolution helpers
# ---------------------------------------------------------------------------


def _resolve_type(type_node: ASTNode) -> OctType | None:
    """Resolve a ``type`` grammar node to an OctType string.

    The grammar rule is::

        type = NAME ;

    So the ``type`` node has one NAME token child whose value is the type
    name (``"u8"`` or ``"bool"``).

    Returns ``None`` if the type name is unrecognised.
    """
    name_tok = _direct_token_of_type(type_node, "NAME")
    if name_tok is None:
        return None
    typ = name_tok.value
    return typ if typ in _VALID_TYPES else None


def _extract_params(fn_decl: ASTNode) -> list[tuple[str, OctType]]:
    """Extract parameter (name, type) pairs from a fn_decl node.

    The grammar is::

        fn_decl   = "fn" NAME LPAREN [ param_list ] RPAREN …
        param_list = param { COMMA param }
        param      = NAME COLON type

    Returns an empty list for zero-parameter functions.
    """
    param_list = _direct_node_of_rule(fn_decl, "param_list")
    if param_list is None:
        return []
    params: list[tuple[str, OctType]] = []
    for child in param_list.children:
        if isinstance(child, ASTNode) and child.rule_name == "param":
            name_tok = _direct_token_of_type(child, "NAME")
            type_node = _direct_node_of_rule(child, "type")
            if name_tok is not None and type_node is not None:
                resolved = _resolve_type(type_node)
                params.append((name_tok.value, resolved or "u8"))
    return params


def _extract_return_type(fn_decl: ASTNode) -> OctType | None:
    """Extract the declared return type from a fn_decl, or None for void.

    The grammar is::

        fn_decl = "fn" NAME LPAREN … RPAREN [ ARROW type ] block

    If the ARROW token is present, the following ``type`` node holds the
    return type.  If absent the function is void.
    """
    saw_arrow = False
    for child in fn_decl.children:
        if isinstance(child, Token) and _tok_type_name(child) == "ARROW":
            saw_arrow = True
        elif (
            saw_arrow
            and isinstance(child, ASTNode)
            and child.rule_name == "type"
        ):
            return _resolve_type(child)
    return None


# ---------------------------------------------------------------------------
# Literal value helpers
# ---------------------------------------------------------------------------


def _parse_literal_value(tok: Token) -> int | None:
    """Return the integer value of an INT_LIT, HEX_LIT, or BIN_LIT token.

    Returns ``None`` if the token is not a recognised literal kind.
    """
    kind = _tok_type_name(tok)
    if kind == "INT_LIT":
        try:
            return int(tok.value)
        except ValueError:
            return None
    if kind == "HEX_LIT":
        try:
            return int(tok.value, 16)
        except ValueError:
            return None
    if kind == "BIN_LIT":
        try:
            return int(tok.value, 2)
        except ValueError:
            return None
    return None


def _is_literal_token(child: ASTNode | Token) -> bool:
    """Return True if ``child`` is an integer/hex/binary literal token."""
    kind = _tok_type_name(child)
    return kind in ("INT_LIT", "HEX_LIT", "BIN_LIT")


def _is_literal_expr(node: ASTNode | Token) -> bool:
    """Return True if the entire expression is a bare integer literal.

    Used to verify that port arguments to ``in`` and ``out`` are
    compile-time constants, not computed values.  A bare primary containing
    only an INT_LIT/HEX_LIT/BIN_LIT token qualifies.

    Only literal tokens pass — not NAME tokens, not compound expressions.
    """
    if isinstance(node, Token):
        return _is_literal_token(node)
    if node.rule_name in (
        "expr",
        "or_expr",
        "and_expr",
        "eq_expr",
        "cmp_expr",
        "add_expr",
        "bitwise_expr",
    ):
        # These rules pass through when they have exactly one child and no
        # operator tokens.
        non_op_children = [
            c
            for c in node.children
            if not (
                isinstance(c, Token)
                and _tok_type_name(c)
                in (
                    "LOR",
                    "LAND",
                    "EQ_EQ",
                    "NEQ",
                    "LT",
                    "GT",
                    "LEQ",
                    "GEQ",
                    "PLUS",
                    "MINUS",
                    "AMP",
                    "PIPE",
                    "CARET",
                )
            )
        ]
        if len(non_op_children) == 1:
            return _is_literal_expr(non_op_children[0])
        return False
    if node.rule_name == "unary_expr":
        # unary_expr = (BANG | TILDE) unary_expr | primary
        # A bare primary with no unary operator is a single child.
        children = list(node.children)
        if len(children) == 1:
            return _is_literal_expr(children[0])
        return False
    if node.rule_name == "primary":
        # primary may wrap a single Token or a parenthesised expr.
        # We only accept bare literals, not parens.
        children = list(node.children)
        if len(children) == 1 and isinstance(children[0], Token):
            return _is_literal_token(children[0])
        return False
    return False


# ---------------------------------------------------------------------------
# OctTypeChecker
# ---------------------------------------------------------------------------


class OctTypeChecker(GenericTypeChecker[ASTNode]):
    """Type checker for the Oct language.

    Implements the ``TypeChecker[ASTNode, ASTNode]`` protocol from
    ``type_checker_protocol``.

    Usage::

        from oct_type_checker import OctTypeChecker
        checker = OctTypeChecker()
        result = checker.check(ast)
        if result.ok:
            ...  # pass result.typed_ast to oct-ir-compiler
        else:
            for err in result.errors:
                print(f"{err.line}:{err.column}: {err.message}")

    The checker performs two passes:

    1. **Signature collection** — gather all ``static`` and ``fn`` names so
       that forward calls are allowed (functions need not be declared in
       source order before being called).

    2. **Body checking** — walk each function body, resolving names,
       inferring expression types, and annotating every expression node with
       ``._oct_type``.
    """

    def __init__(self) -> None:
        super().__init__()
        # Global variables: static declarations (name → type)
        self._statics: dict[str, OctType] = {}
        # Function signatures collected in Pass 1
        self._functions: dict[str, _FnInfo] = {}

    # ------------------------------------------------------------------
    # GenericTypeChecker entry point
    # ------------------------------------------------------------------

    def run(self, ast: ASTNode) -> None:
        """Execute both passes over the AST.

        This is called by ``GenericTypeChecker.check()`` after resetting the
        error list.  Do not call ``run`` directly — call ``check`` instead.
        """
        self._pass1_collect(ast)
        self._verify_main()
        self._pass2_check(ast)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _err(self, loc: tuple[int, int] | ASTNode | Token, msg: str) -> None:
        """Append a type error at the given source location.

        ``loc`` can be a ``(line, col)`` tuple, a ``Token``, or an
        ``ASTNode`` — the function extracts the position automatically.
        """
        if isinstance(loc, tuple):
            line, col = loc
        else:
            line, col = _loc(loc)
        self._errors.append(TypeErrorDiagnostic(message=msg, line=line, column=col))

    def _resolve_name(
        self,
        name: str,
        local_scope: dict[str, OctType],
    ) -> OctType | None:
        """Look up a variable name in local scope, then statics.

        Returns the type string, or ``None`` if undeclared.
        """
        if name in local_scope:
            return local_scope[name]
        if name in self._statics:
            return self._statics[name]
        return None

    # ------------------------------------------------------------------
    # Pass 1: signature collection
    # ------------------------------------------------------------------

    def _pass1_collect(self, program: ASTNode) -> None:
        """Walk top-level declarations and record statics and fn signatures."""
        for child in program.children:
            if isinstance(child, ASTNode) and child.rule_name == "top_decl":
                for inner in child.children:
                    if isinstance(inner, ASTNode):
                        if inner.rule_name == "static_decl":
                            self._collect_static(inner)
                        elif inner.rule_name == "fn_decl":
                            self._collect_fn(inner)

    def _collect_static(self, node: ASTNode) -> None:
        """Register a top-level static declaration in the global scope.

        Grammar: ``static NAME COLON type EQ expr SEMICOLON``
        """
        name_tok = _direct_token_of_type(node, "NAME")
        type_node = _direct_node_of_rule(node, "type")
        if name_tok is None:
            self._err(node, "internal: static_decl has no NAME token")
            return
        name = name_tok.value
        if type_node is None:
            self._err(node, f"static '{name}': missing type annotation")
            return
        resolved = _resolve_type(type_node)
        if resolved is None:
            type_name = _tok_value(_direct_token_of_type(type_node, "NAME") or name_tok)
            self._err(type_node, f"unknown type '{type_name}' in static '{name}'")
            return
        if name in self._statics:
            self._err(name_tok, f"static '{name}' is already declared")
            return
        self._statics[name] = resolved

    def _collect_fn(self, node: ASTNode) -> None:
        """Register a function signature in the global function table.

        Grammar: ``fn NAME LPAREN [param_list] RPAREN [ARROW type] block``
        """
        name_tok = _direct_token_of_type(node, "NAME")
        if name_tok is None:
            self._err(node, "internal: fn_decl has no NAME token")
            return
        name = name_tok.value
        params = _extract_params(node)
        return_type = _extract_return_type(node)
        if name in self._functions:
            self._err(name_tok, f"function '{name}' is already defined")
            return
        self._functions[name] = _FnInfo(
            name=name,
            params=params,
            return_type=return_type,
            node=node,
        )

    def _verify_main(self) -> None:
        """Verify that a valid ``main`` function exists.

        Oct programs must have exactly one function named ``main``, with no
        parameters and no return type.  This is the hardware entry point:
        the generated code emits ``CAL main`` at address 0 followed by ``HLT``.
        """
        if "main" not in self._functions:
            self._err((1, 1), "program must define a 'main' function")
            return
        main = self._functions["main"]
        if main.params:
            self._err(main.node, "'main' must take no parameters")
        if main.return_type is not None:
            self._err(main.node, "'main' must have no return type (void)")

    # ------------------------------------------------------------------
    # Pass 2: body type-checking
    # ------------------------------------------------------------------

    def _pass2_check(self, program: ASTNode) -> None:
        """Type-check all function bodies using the collected signatures."""
        for child in program.children:
            if isinstance(child, ASTNode) and child.rule_name == "top_decl":
                for inner in child.children:
                    if isinstance(inner, ASTNode) and inner.rule_name == "fn_decl":
                        self._check_fn_body(inner)

    def _check_fn_body(self, fn_decl: ASTNode) -> None:
        """Type-check a single function body.

        Builds a local scope from the function's parameters, then checks
        the body block.
        """
        name_tok = _direct_token_of_type(fn_decl, "NAME")
        if name_tok is None:
            return
        fn_name = name_tok.value
        fn_info = self._functions.get(fn_name)
        if fn_info is None:
            return
        # Seed the local scope with parameter names and types.
        local_scope: dict[str, OctType] = {
            pname: ptype for pname, ptype in fn_info.params
        }
        block = _direct_node_of_rule(fn_decl, "block")
        if block is None:
            return
        self._check_block(block, local_scope, fn_info.return_type)

    def _check_block(
        self,
        block: ASTNode,
        local_scope: dict[str, OctType],
        return_type: OctType | None,
    ) -> None:
        """Type-check all statements inside a ``{ … }`` block.

        ``local_scope`` is mutated as new ``let`` declarations are processed.
        ``return_type`` is the enclosing function's declared return type (None
        for void).
        """
        for child in block.children:
            if isinstance(child, ASTNode) and child.rule_name == "stmt":
                self._check_stmt(child, local_scope, return_type)

    def _check_stmt(
        self,
        stmt: ASTNode,
        local_scope: dict[str, OctType],
        return_type: OctType | None,
    ) -> None:
        """Dispatch a statement node to the appropriate checker."""
        for inner in stmt.children:
            if isinstance(inner, ASTNode):
                match inner.rule_name:
                    case "let_stmt":
                        self._check_let_stmt(inner, local_scope)
                    case "static_decl":
                        # static declarations are valid inside function bodies
                        # (they refer to global statics already collected in
                        # Pass 1, so we just re-validate for local context).
                        self._check_static_in_body(inner)
                    case "assign_stmt":
                        self._check_assign_stmt(inner, local_scope)
                    case "return_stmt":
                        self._check_return_stmt(inner, local_scope, return_type)
                    case "if_stmt":
                        self._check_if_stmt(inner, local_scope, return_type)
                    case "while_stmt":
                        self._check_while_stmt(inner, local_scope, return_type)
                    case "loop_stmt":
                        self._check_loop_stmt(inner, local_scope, return_type)
                    case "break_stmt":
                        pass  # syntactically valid; no type check needed
                    case "expr_stmt":
                        self._check_expr_stmt(inner, local_scope)

    # ------------------------------------------------------------------
    # Statement checkers
    # ------------------------------------------------------------------

    def _check_let_stmt(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> None:
        """Check ``let NAME COLON type EQ expr SEMICOLON``.

        Adds the declared name to ``local_scope`` so later statements in the
        same block can reference it.
        """
        name_tok = _direct_token_of_type(node, "NAME")
        type_node = _direct_node_of_rule(node, "type")
        # The expr child is the ASTNode that is NOT the "type" node.
        expr_node: ASTNode | None = None
        past_eq = False
        for child in node.children:
            if isinstance(child, Token) and _tok_type_name(child) == "EQ":
                past_eq = True
            elif (
                past_eq
                and isinstance(child, ASTNode)
                and child.rule_name != "type"
            ):
                expr_node = child
                break

        if name_tok is None:
            self._err(node, "internal: let_stmt has no NAME token")
            return
        name = name_tok.value
        if type_node is None:
            self._err(node, f"let '{name}': missing type annotation")
            return
        declared_type = _resolve_type(type_node)
        if declared_type is None:
            raw = _tok_value(type_node) or "?"
            self._err(type_node, f"unknown type '{raw}' in let '{name}'")
            return
        # Type-check the initialiser expression.
        actual_type: OctType | None = None
        if expr_node is not None:
            actual_type = self._check_expr(expr_node, local_scope)
        if actual_type is not None and not _assignable(actual_type, declared_type):
            self._err(
                expr_node or node,
                f"cannot assign '{actual_type}' to '{declared_type}' variable '{name}'",
            )
        # Register the variable regardless of the error so later references
        # don't cascade into spurious "undeclared" errors.
        local_scope[name] = declared_type

    def _check_static_in_body(self, node: ASTNode) -> None:
        """Validate a static declaration appearing inside a function body.

        Static declarations inside function bodies are legal Oct syntax — they
        reference the program-level static (already collected in Pass 1).
        We just need to ensure the static name actually exists.
        """
        name_tok = _direct_token_of_type(node, "NAME")
        if name_tok is None:
            return
        # Already collected in Pass 1; nothing further to do here.

    def _check_assign_stmt(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> None:
        """Check ``NAME EQ expr SEMICOLON``.

        The variable must already be declared (either as a local or a static).
        The RHS type must be compatible with the declared type.
        """
        name_tok = _direct_token_of_type(node, "NAME")
        expr_node: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode):
                expr_node = child
                break

        if name_tok is None:
            self._err(node, "internal: assign_stmt has no NAME token")
            return
        name = name_tok.value
        declared_type = self._resolve_name(name, local_scope)
        if declared_type is None:
            self._err(name_tok, f"assignment to undeclared variable '{name}'")
            return
        if expr_node is not None:
            actual_type = self._check_expr(expr_node, local_scope)
            if actual_type is not None and not _assignable(actual_type, declared_type):
                self._err(
                    expr_node,
                    f"cannot assign '{actual_type}' to"
                    f" '{declared_type}' variable '{name}'",
                )

    def _check_return_stmt(
        self,
        node: ASTNode,
        local_scope: dict[str, OctType],
        return_type: OctType | None,
    ) -> None:
        """Check ``return [expr] SEMICOLON``.

        A bare ``return`` is valid only in a void function.  A ``return expr``
        must match the declared return type.
        """
        # Find the optional expr child.
        expr_node: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode):
                expr_node = child
                break

        if expr_node is None:
            # Bare return.
            if return_type is not None:
                self._err(
                    node,
                    f"'return' with no value in function"
                    f" returning '{return_type}'",
                )
        else:
            actual = self._check_expr(expr_node, local_scope)
            if return_type is None:
                self._err(expr_node, "void function must not return a value")
            elif actual is not None and not _assignable(actual, return_type):
                self._err(
                    expr_node,
                    f"'return' type mismatch: expected '{return_type}', got '{actual}'",
                )

    def _check_if_stmt(
        self,
        node: ASTNode,
        local_scope: dict[str, OctType],
        return_type: OctType | None,
    ) -> None:
        """Check ``if expr block [else block]``.

        The condition must be of type ``bool``.  Both branches are checked
        with a **copy** of the current local scope so that variables declared
        inside ``if``/``else`` bodies do not leak out to the enclosing scope.
        """
        # Children: Token("if"), ASTNode(expr), ASTNode(block),
        #           [Token("else"), ASTNode(block)]
        expr_node: ASTNode | None = None
        blocks: list[ASTNode] = []
        for child in node.children:
            if isinstance(child, ASTNode):
                if child.rule_name == "block":
                    blocks.append(child)
                elif expr_node is None:
                    expr_node = child

        if expr_node is not None:
            cond_type = self._check_expr(expr_node, local_scope)
            if cond_type is not None and cond_type != "bool":
                self._err(
                    expr_node,
                    f"'if' condition must be 'bool', got '{cond_type}' — "
                    f"use an explicit comparison (e.g. x != 0)",
                )
        for blk in blocks:
            # Each branch gets a *copy* of the current scope so that inner
            # declarations do not escape the block.
            self._check_block(blk, dict(local_scope), return_type)

    def _check_while_stmt(
        self,
        node: ASTNode,
        local_scope: dict[str, OctType],
        return_type: OctType | None,
    ) -> None:
        """Check ``while expr block``.

        The condition must be of type ``bool``.
        """
        expr_node: ASTNode | None = None
        block_node: ASTNode | None = None
        for child in node.children:
            if isinstance(child, ASTNode):
                if child.rule_name == "block":
                    block_node = child
                elif expr_node is None:
                    expr_node = child

        if expr_node is not None:
            cond_type = self._check_expr(expr_node, local_scope)
            if cond_type is not None and cond_type != "bool":
                self._err(
                    expr_node,
                    f"'while' condition must be 'bool', got '{cond_type}' — "
                    f"use an explicit comparison (e.g. n != 255)",
                )
        if block_node is not None:
            self._check_block(block_node, dict(local_scope), return_type)

    def _check_loop_stmt(
        self,
        node: ASTNode,
        local_scope: dict[str, OctType],
        return_type: OctType | None,
    ) -> None:
        """Check ``loop block`` (infinite loop, exited via break)."""
        block = _direct_node_of_rule(node, "block")
        if block is not None:
            self._check_block(block, dict(local_scope), return_type)

    def _check_expr_stmt(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> None:
        """Check ``expr SEMICOLON``.

        The expression is type-checked for side effects (e.g. ``out(1, x);``).
        Its result type is discarded.
        """
        for child in node.children:
            if isinstance(child, ASTNode):
                self._check_expr(child, local_scope)
                break

    # ------------------------------------------------------------------
    # Expression type inference
    # ------------------------------------------------------------------

    def _check_expr(
        self,
        node: ASTNode | Token,
        local_scope: dict[str, OctType],
    ) -> OctType | None:
        """Infer and check the type of an expression node.

        Returns the inferred type (``"u8"`` or ``"bool"``), or ``None`` if
        an error was reported (callers should tolerate ``None`` gracefully).

        Annotates the ``node`` with ``._oct_type`` on success.
        """
        if isinstance(node, Token):
            typ = self._check_token_primary(node, local_scope)
            if typ is not None:
                node._oct_type = typ  # type: ignore[attr-defined]
            return typ

        result: OctType | None = None
        match node.rule_name:
            case "expr":
                # Transparent alias: expr = or_expr
                for child in node.children:
                    result = self._check_expr(child, local_scope)
                    break
            case (
                "or_expr" | "and_expr" | "eq_expr"
                | "cmp_expr" | "add_expr" | "bitwise_expr"
            ):
                result = self._check_binary_expr(node, local_scope)
            case "unary_expr":
                result = self._check_unary_expr(node, local_scope)
            case "primary":
                result = self._check_primary(node, local_scope)
            case _:
                # Unknown wrapper node — recurse into its first ASTNode child.
                for child in node.children:
                    if isinstance(child, ASTNode):
                        result = self._check_expr(child, local_scope)
                        break

        if result is not None:
            node._oct_type = result  # type: ignore[attr-defined]
        return result

    def _check_token_primary(
        self, tok: Token, local_scope: dict[str, OctType]
    ) -> OctType | None:
        """Infer the type of a bare Token used as a primary expression.

        Handles ``INT_LIT``, ``HEX_LIT``, ``BIN_LIT``, ``true``, ``false``,
        and ``NAME`` tokens.
        """
        kind = _tok_type_name(tok)
        match kind:
            case "INT_LIT" | "HEX_LIT" | "BIN_LIT":
                val = _parse_literal_value(tok)
                if val is None or not (0 <= val <= 255):
                    self._err(
                        tok,
                        f"integer literal {tok.value!r} is out of u8 range 0–255",
                    )
                return "u8"
            case "true" | "false":
                return "bool"
            case "NAME":
                name = tok.value
                typ = self._resolve_name(name, local_scope)
                if typ is None:
                    self._err(tok, f"undefined variable '{name}'")
                    return None
                return typ
            case _:
                return None

    def _check_binary_expr(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> OctType | None:
        """Type-check a binary expression node (or_expr, add_expr, etc.).

        When the node has a single child and no operator token, it is a
        pass-through from a higher-precedence rule and we simply recurse.
        When it has the flat ``left op right [op right …]`` structure we
        type-check each operand and operator in turn.

        Returns the result type of the whole expression.
        """
        children = node.children
        if not children:
            return None

        # Collect non-operator children (ASTNodes and NAME/literal Tokens)
        # to distinguish "single operand" from "binary expression".
        expr_children = [
            c
            for c in children
            if isinstance(c, ASTNode)
            or (
                isinstance(c, Token)
                and _tok_type_name(c)
                not in (
                    "LOR",
                    "LAND",
                    "EQ_EQ",
                    "NEQ",
                    "LT",
                    "GT",
                    "LEQ",
                    "GEQ",
                    "PLUS",
                    "MINUS",
                    "AMP",
                    "PIPE",
                    "CARET",
                )
            )
        ]

        if len(expr_children) == 1:
            # Pass-through — no operator at this level.
            return self._check_expr(children[0], local_scope)

        # Flat binary structure: left [op right] *
        left_type = self._check_expr(children[0], local_scope)
        i = 1
        while i < len(children) - 1:
            op_child = children[i]
            right_child = children[i + 1]
            op_name = _tok_type_name(op_child) if isinstance(op_child, Token) else ""
            right_type = self._check_expr(right_child, local_scope)

            # Determine result type from operator category.
            if op_name in _U8_OPS:
                # +, -, &, |, ^ require u8-compatible operands → u8
                if not _is_u8_compatible(left_type):
                    self._err(
                        op_child,
                        f"operator '{op_name}' requires 'u8'"
                        f" operand, got '{left_type}'",
                    )
                if not _is_u8_compatible(right_type):
                    self._err(
                        op_child,
                        f"operator '{op_name}' requires 'u8'"
                        f" operand, got '{right_type}'",
                    )
                left_type = "u8"
            elif op_name in _CMP_OPS:
                # ==, !=, <, >, <=, >= produce bool
                if not _is_u8_compatible(left_type):
                    self._err(
                        op_child,
                        f"comparison '{op_name}' requires 'u8'"
                        f" operand, got '{left_type}'",
                    )
                if not _is_u8_compatible(right_type):
                    self._err(
                        op_child,
                        f"comparison '{op_name}' requires 'u8'"
                        f" operand, got '{right_type}'",
                    )
                left_type = "bool"
            elif op_name in _BOOL_OPS:
                # &&, || require bool operands → bool
                if left_type is not None and left_type != "bool":
                    self._err(
                        op_child,
                        f"operator '{op_name}' requires 'bool'"
                        f" operand, got '{left_type}' — "
                        f"use an explicit comparison (e.g. x != 0)",
                    )
                if right_type is not None and right_type != "bool":
                    self._err(
                        op_child,
                        f"operator '{op_name}' requires 'bool'"
                        f" operand, got '{right_type}' — "
                        f"use an explicit comparison (e.g. y != 0)",
                    )
                left_type = "bool"
            else:
                # Unknown operator (shouldn't happen with valid grammar).
                left_type = None

            i += 2

        return left_type

    def _check_unary_expr(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> OctType | None:
        """Type-check ``(BANG | TILDE) unary_expr  |  primary``.

        - ``!x`` (logical NOT): ``x`` must be ``bool``; result is ``bool``.
        - ``~x`` (bitwise NOT): ``x`` must be ``u8``-compatible; result is ``u8``.
        - No operator: pass through to the single child (``primary``).
        """
        children = node.children
        if not children:
            return None

        # Check if the first child is an operator token.
        first = children[0]
        if isinstance(first, Token):
            op_name = _tok_type_name(first)
            if len(children) >= 2:
                operand_type = self._check_expr(children[1], local_scope)
                if op_name == "BANG":
                    if operand_type is not None and operand_type != "bool":
                        self._err(
                            first,
                            f"'!' (logical NOT) requires 'bool'"
                            f" operand, got '{operand_type}' — "
                            f"use an explicit comparison (e.g. x != 0)",
                        )
                    return "bool"
                if op_name == "TILDE":
                    if operand_type is not None and not _is_u8_compatible(operand_type):
                        self._err(
                            first,
                            f"'~' (bitwise NOT) requires 'u8'"
                            f" operand, got '{operand_type}'",
                        )
                    return "u8"
            return None

        # No unary operator — single child is a primary or deeper rule.
        return self._check_expr(first, local_scope)

    def _check_primary(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> OctType | None:
        """Type-check a primary expression.

        Dispatches to:
        - ``intrinsic_call`` — hardware intrinsic (``in``, ``out``, ``carry``, …)
        - ``call_expr``      — user-defined function call
        - bare Token         — literal (INT_LIT, HEX_LIT, BIN_LIT, true, false, NAME)
        - parenthesised expr — recurse into inner expression
        """
        for child in node.children:
            if isinstance(child, ASTNode):
                match child.rule_name:
                    case "intrinsic_call":
                        return self._check_intrinsic(child, local_scope)
                    case "call_expr":
                        return self._check_call_expr(child, local_scope)
                    case "expr":
                        # Parenthesised: LPAREN expr RPAREN
                        return self._check_expr(child, local_scope)
                    case _:
                        return self._check_expr(child, local_scope)
            elif isinstance(child, Token):
                kind = _tok_type_name(child)
                if kind in ("LPAREN", "RPAREN"):
                    continue  # skip parens themselves
                return self._check_token_primary(child, local_scope)
        return None

    def _check_call_expr(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> OctType | None:
        """Type-check a user-defined function call.

        Grammar: ``NAME LPAREN [arg_list] RPAREN``

        Checks:
        - Function must be declared.
        - Argument count must match parameter count.
        - Each argument type must be compatible with the parameter type.

        Returns the function's return type, or ``None`` on error.
        """
        name_tok = _direct_token_of_type(node, "NAME")
        if name_tok is None:
            return None
        fn_name = name_tok.value
        fn_info = self._functions.get(fn_name)
        if fn_info is None:
            self._err(name_tok, f"call to undefined function '{fn_name}'")
            return None

        # Collect argument expressions from arg_list.
        arg_list_node = _direct_node_of_rule(node, "arg_list")
        arg_exprs: list[ASTNode | Token] = []
        if arg_list_node is not None:
            for child in arg_list_node.children:
                is_arg = isinstance(child, ASTNode) or (
                    isinstance(child, Token)
                    and _tok_type_name(child) not in ("COMMA",)
                )
                if is_arg:
                    arg_exprs.append(child)

        expected = fn_info.params
        if len(arg_exprs) != len(expected):
            self._err(
                name_tok,
                f"function '{fn_name}' expects {len(expected)} argument(s), "
                f"got {len(arg_exprs)}",
            )
            # Still type-check the args we have to catch nested errors.
        for i, arg in enumerate(arg_exprs):
            arg_type = self._check_expr(arg, local_scope)
            if i < len(expected):
                _, param_type = expected[i]
                if arg_type is not None and not _assignable(arg_type, param_type):
                    self._err(
                        arg,
                        f"argument {i + 1} to '{fn_name}': expected '{param_type}', "
                        f"got '{arg_type}'",
                    )

        return fn_info.return_type

    def _check_intrinsic(
        self, node: ASTNode, local_scope: dict[str, OctType]
    ) -> OctType | None:
        """Type-check a hardware intrinsic call.

        The intrinsic name is always the first token child of the
        ``intrinsic_call`` node (e.g. Token("in"), Token("carry")).

        Intrinsic signatures::

            in(PORT)          → u8    PORT must be a literal
            out(PORT, val)    → void  PORT literal; val u8-compatible
            adc(a, b)         → u8    a, b must be u8-compatible
            sbb(a, b)         → u8    a, b must be u8-compatible
            rlc(a)            → u8    a must be u8-compatible
            rrc(a)            → u8    a must be u8-compatible
            ral(a)            → u8    a must be u8-compatible
            rar(a)            → u8    a must be u8-compatible
            carry()           → bool  no arguments
            parity(a)         → bool  a must be u8-compatible
        """
        # Find the intrinsic name token (first keyword token child).
        name_tok: Token | None = None
        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type_name(child)
                if kind in (
                    "in", "out", "adc", "sbb",
                    "rlc", "rrc", "ral", "rar",
                    "carry", "parity",
                ):
                    name_tok = child
                    break

        if name_tok is None:
            self._err(node, "internal: intrinsic_call has no intrinsic name token")
            return None

        intrinsic = name_tok.value  # "in", "out", "carry", …

        # Collect argument expression nodes (excluding LPAREN, RPAREN, COMMA).
        arg_nodes: list[ASTNode | Token] = []
        seen_lparen = False
        for child in node.children:
            if isinstance(child, Token):
                kind = _tok_type_name(child)
                if kind == "LPAREN":
                    seen_lparen = True
                    continue
                if kind in ("RPAREN", "COMMA"):
                    continue
                if seen_lparen and kind not in (
                    "in", "out", "adc", "sbb",
                    "rlc", "rrc", "ral", "rar",
                    "carry", "parity",
                ):
                    arg_nodes.append(child)
            elif isinstance(child, ASTNode) and seen_lparen:
                arg_nodes.append(child)

        match intrinsic:
            case "in":
                return self._check_intrinsic_in(node, name_tok, arg_nodes, local_scope)
            case "out":
                self._check_intrinsic_out(node, name_tok, arg_nodes, local_scope)
                return None  # out() is void
            case "adc" | "sbb":
                return self._check_intrinsic_two_u8(
                    intrinsic, node, name_tok, arg_nodes, local_scope
                )
            case "rlc" | "rrc" | "ral" | "rar":
                return self._check_intrinsic_one_u8(
                    intrinsic, node, name_tok, arg_nodes, local_scope
                )
            case "carry":
                if arg_nodes:
                    self._err(name_tok, "'carry()' takes no arguments")
                return "bool"
            case "parity":
                return self._check_intrinsic_parity(
                    node, name_tok, arg_nodes, local_scope
                )
            case _:
                return None

    def _check_intrinsic_in(
        self,
        node: ASTNode,
        name_tok: Token,
        args: list[ASTNode | Token],
        local_scope: dict[str, OctType],
    ) -> OctType | None:
        """Check ``in(PORT)`` — PORT must be a compile-time literal."""
        if len(args) != 1:
            self._err(name_tok, f"'in' expects 1 argument, got {len(args)}")
            return "u8"
        port_arg = args[0]
        if not _is_literal_expr(port_arg):
            self._err(
                port_arg,
                "'in' port must be a compile-time integer literal — "
                "the Intel 8008 encodes the port number in the instruction opcode",
            )
        else:
            self._check_expr(port_arg, local_scope)
        return "u8"

    def _check_intrinsic_out(
        self,
        node: ASTNode,
        name_tok: Token,
        args: list[ASTNode | Token],
        local_scope: dict[str, OctType],
    ) -> None:
        """Check ``out(PORT, val)`` — PORT literal, val u8-compatible."""
        if len(args) != 2:
            self._err(name_tok, f"'out' expects 2 arguments, got {len(args)}")
            return
        port_arg, val_arg = args[0], args[1]
        if not _is_literal_expr(port_arg):
            self._err(
                port_arg,
                "'out' port must be a compile-time integer literal — "
                "the Intel 8008 encodes the port number in the instruction opcode",
            )
        else:
            self._check_expr(port_arg, local_scope)
        val_type = self._check_expr(val_arg, local_scope)
        if val_type is not None and not _is_u8_compatible(val_type):
            self._err(
                val_arg,
                f"'out' value argument must be 'u8'-compatible, got '{val_type}'",
            )

    def _check_intrinsic_two_u8(
        self,
        name: str,
        node: ASTNode,
        name_tok: Token,
        args: list[ASTNode | Token],
        local_scope: dict[str, OctType],
    ) -> OctType | None:
        """Check ``adc(a, b)`` and ``sbb(a, b)`` — both args u8-compatible."""
        if len(args) != 2:
            self._err(name_tok, f"'{name}' expects 2 arguments, got {len(args)}")
            return "u8"
        for i, arg in enumerate(args):
            t = self._check_expr(arg, local_scope)
            if t is not None and not _is_u8_compatible(t):
                self._err(
                    arg,
                    f"'{name}' argument {i + 1} must be 'u8'-compatible, got '{t}'",
                )
        return "u8"

    def _check_intrinsic_one_u8(
        self,
        name: str,
        node: ASTNode,
        name_tok: Token,
        args: list[ASTNode | Token],
        local_scope: dict[str, OctType],
    ) -> OctType | None:
        """Check ``rlc(a)``, ``rrc(a)``, ``ral(a)``, ``rar(a)``."""
        if len(args) != 1:
            self._err(name_tok, f"'{name}' expects 1 argument, got {len(args)}")
            return "u8"
        t = self._check_expr(args[0], local_scope)
        if t is not None and not _is_u8_compatible(t):
            self._err(
                args[0],
                f"'{name}' argument must be 'u8'-compatible, got '{t}'",
            )
        return "u8"

    def _check_intrinsic_parity(
        self,
        node: ASTNode,
        name_tok: Token,
        args: list[ASTNode | Token],
        local_scope: dict[str, OctType],
    ) -> OctType | None:
        """Check ``parity(a)`` — a must be u8-compatible."""
        if len(args) != 1:
            self._err(name_tok, f"'parity' expects 1 argument, got {len(args)}")
            return "bool"
        t = self._check_expr(args[0], local_scope)
        if t is not None and not _is_u8_compatible(t):
            self._err(
                args[0],
                f"'parity' argument must be 'u8'-compatible, got '{t}'",
            )
        return "bool"

    # ------------------------------------------------------------------
    # node_kind / locate (GenericTypeChecker hooks)
    # ------------------------------------------------------------------

    def node_kind(self, node: ASTNode) -> str | None:  # type: ignore[override]
        """Return the rule_name of an ASTNode (used by dispatch hooks)."""
        if isinstance(node, ASTNode):
            return node.rule_name
        return None

    def locate(self, subject: object) -> tuple[int, int]:  # type: ignore[override]
        """Return (line, col) for any node/token (used by error messages)."""
        if isinstance(subject, (ASTNode, Token)):
            return _loc(subject)
        return (1, 1)


# ---------------------------------------------------------------------------
# Module-level convenience function
# ---------------------------------------------------------------------------


def check_oct(ast: ASTNode) -> TypeCheckResult[ASTNode]:
    """Type-check an Oct AST and return the result.

    This is the main entry point for the Oct type checker.  Pass in the
    root ``ASTNode`` produced by ``oct_parser.parse_oct()`` and get back
    a ``TypeCheckResult`` containing the annotated AST and any errors.

    Args:
        ast: The untyped root ASTNode from the Oct parser.

    Returns:
        A ``TypeCheckResult[ASTNode]`` where:
        - ``result.ok`` is ``True`` if there are no type errors.
        - ``result.typed_ast`` is the annotated AST (expression nodes have
          ``._oct_type`` set to ``"u8"`` or ``"bool"``).
        - ``result.errors`` is a list of ``TypeErrorDiagnostic`` objects.

    Example::

        from oct_parser import parse_oct
        from oct_type_checker import check_oct

        ast = parse_oct("fn main() { let x: u8 = 42; }")
        result = check_oct(ast)
        assert result.ok

    Example — type error::

        ast = parse_oct("fn main() { let x: bool = 42; }")
        result = check_oct(ast)
        assert not result.ok
        assert "cannot assign 'u8' to 'bool'" in result.errors[0].message
    """
    return OctTypeChecker().check(ast)
