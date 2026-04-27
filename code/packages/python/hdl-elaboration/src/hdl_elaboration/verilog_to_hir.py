"""Verilog AST -> HIR.

Walks an ASTNode tree produced by ``verilog-parser`` and emits HIR.

The Verilog grammar wraps every expression in a chain of precedence rules
(``expression > ternary_expr > or_expr > and_expr > bit_or_expr > bit_xor_expr
> bit_and_expr > equality_expr > relational_expr > shift_expr > additive_expr
> multiplicative_expr > power_expr > unary_expr > primary``). Most chain links
have one child each because the input expression doesn't use that precedence
level; we walk down through these single-child chains until we hit a node
that actually has a binary operation or terminates in a primary.
"""

from __future__ import annotations

from dataclasses import dataclass

from hdl_ir import (
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    Lit,
    Module,
    NetRef,
    PortRef,
    Provenance,
    Slice,
    SourceLang,
    SourceLocation,
    TyLogic,
    TyVector,
)
from hdl_ir.expr import Expr
from hdl_ir.types import Ty
from lang_parser import ASTNode
from lexer import Token

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _is_token(x: object) -> bool:
    return not hasattr(x, "rule_name") and hasattr(x, "value")


def _children(node: ASTNode) -> list[object]:
    return list(node.children)


def _ast_children(node: ASTNode) -> list[ASTNode]:
    return [c for c in node.children if hasattr(c, "rule_name")]


def _token_children(node: ASTNode) -> list[Token]:
    return [c for c in node.children if _is_token(c)]


def _token_text(node: ASTNode | Token) -> str:
    if _is_token(node):
        return node.value  # type: ignore[union-attr]
    raise TypeError(f"expected Token, got {node!r}")


def _find_child(node: ASTNode, rule_name: str) -> ASTNode | None:
    for c in node.children:
        if hasattr(c, "rule_name") and c.rule_name == rule_name:
            return c  # type: ignore[return-value]
    return None


def _find_token(node: ASTNode, value: str | None = None, kind: str | None = None) -> Token | None:
    for c in node.children:
        if _is_token(c):
            if value is not None and c.value == value:  # type: ignore[union-attr]
                return c  # type: ignore[return-value]
            if kind is not None and str(c.type) == kind:  # type: ignore[union-attr]
                return c  # type: ignore[return-value]
            if value is None and kind is None:
                return c  # type: ignore[return-value]
    return None


def _provenance(node: ASTNode | Token, file: str = "<unknown>") -> Provenance:
    line = getattr(node, "start_line", None) or getattr(node, "line", None) or 1
    col = getattr(node, "start_column", None) or getattr(node, "column", None) or 1
    return Provenance(SourceLang.VERILOG, SourceLocation(file, max(1, line), max(1, col)))


# ---------------------------------------------------------------------------
# Constant evaluation (used for port ranges and literal expressions)
# ---------------------------------------------------------------------------


def _eval_constant(node: ASTNode) -> int:
    """Evaluate an expression node that should be a compile-time constant.

    Used for port range bounds. Walks the precedence chain looking for the
    underlying constant integer."""
    cur: ASTNode | Token = node
    while True:
        if _is_token(cur):
            text = cur.value  # type: ignore[union-attr]
            return _parse_number(text)
        ast_kids = _ast_children(cur)  # type: ignore[arg-type]
        tokens = _token_children(cur)  # type: ignore[arg-type]
        if not ast_kids and tokens:
            return _parse_number(tokens[0].value)
        if len(ast_kids) == 1 and not tokens:
            cur = ast_kids[0]
            continue
        # Hit a complex node (binary op, etc.) — try to walk into a primary
        for c in ast_kids:
            if c.rule_name in ("primary",):
                cur = c
                break
            if c.rule_name == "expression":
                cur = c
                break
        else:
            # Fallback: try first child
            if ast_kids:
                cur = ast_kids[0]
                continue
            raise ValueError(f"cannot evaluate constant from {node.rule_name}")


def _parse_number(text: str) -> int:
    """Parse a Verilog number literal to int. Handles 4'b1010, 8'hFF, 32'd42, etc."""
    text = text.replace("_", "")
    if "'" in text:
        # sized literal: [size]'[s]?[bdhoBDHO]digits
        size_part, _, base_digits = text.partition("'")
        # strip optional 's'
        if base_digits.lower().startswith("s"):
            base_digits = base_digits[1:]
        base = base_digits[0].lower()
        digits = base_digits[1:]
        if base == "b":
            return int(digits, 2)
        if base == "o":
            return int(digits, 8)
        if base == "h":
            return int(digits, 16)
        if base == "d":
            return int(digits, 10)
        raise ValueError(f"unknown number base: {base}")
    return int(text, 10)


# ---------------------------------------------------------------------------
# Type extraction from port declaration / range
# ---------------------------------------------------------------------------


def _extract_type_from_range(range_node: ASTNode | None) -> Ty:
    """Compute Ty from an optional Verilog ``range`` node ``[msb:lsb]``."""
    if range_node is None:
        return TyLogic()
    exprs = [c for c in range_node.children if hasattr(c, "rule_name") and c.rule_name == "expression"]
    if len(exprs) != 2:
        return TyLogic()
    msb = _eval_constant(exprs[0])
    lsb = _eval_constant(exprs[1])
    width = abs(msb - lsb) + 1
    if width == 1:
        return TyLogic()
    return TyVector(TyLogic(), width, msb_first=msb >= lsb)


# ---------------------------------------------------------------------------
# Expression elaboration
# ---------------------------------------------------------------------------


_VERILOG_BINOP_MAP = {
    "+": "+",
    "-": "-",
    "*": "*",
    "/": "/",
    "%": "%",
    "&&": "&&",
    "||": "||",
    "==": "==",
    "!=": "!=",
    "===": "===",
    "!==": "!==",
    "<": "<",
    ">": ">",
    "<=": "<=",
    ">=": ">=",
    "&": "&",
    "|": "|",
    "^": "^",
    "<<": "<<",
    ">>": ">>",
    "**": "**",
}


_VERILOG_UNARY_MAP = {
    "-": "NEG",
    "+": "POS",
    "!": "LOGIC_NOT",
    "~": "NOT",
    "&": "AND_RED",
    "|": "OR_RED",
    "^": "XOR_RED",
}


@dataclass
class ExprElaborator:
    """Walks Verilog expression nodes -> HIR Expr."""

    file: str
    port_names: set[str]
    net_names: set[str]

    def elaborate(self, node: ASTNode) -> Expr:
        """Entry point: takes any expression-precedence node, returns Expr."""
        return self._walk(node)

    def _walk(self, node: ASTNode) -> Expr:
        # Walk single-child chains.
        ast_kids = _ast_children(node)
        tokens = _token_children(node)
        if not ast_kids and tokens:
            # Leaf: a name or number directly under this node.
            return self._token_to_expr(tokens[0])

        if len(ast_kids) == 1 and not tokens:
            return self._walk(ast_kids[0])

        # Binary expression: e.g. additive_expr -> [multiplicative_expr, '+', multiplicative_expr]
        if len(ast_kids) >= 2 and tokens:
            return self._build_binop(node, ast_kids, tokens)

        # primary node with parens or concat
        if node.rule_name == "primary":
            return self._walk_primary(node)

        # concatenation / replication handled at primary level normally
        if node.rule_name == "concatenation":
            return self._walk_concat(node)

        # Fallback
        if ast_kids:
            return self._walk(ast_kids[0])
        raise ValueError(f"cannot elaborate {node.rule_name}")

    def _build_binop(
        self, node: ASTNode, ast_kids: list[ASTNode], tokens: list[Token]
    ) -> Expr:
        # Left-associative chains: walk left-to-right collecting operands and ops.
        # Verilog grammar: e.g.   additive_expr = multiplicative_expr { ('+'|'-') multiplicative_expr }
        # The children are interleaved sub-exprs and operator tokens.
        seq: list[object] = list(node.children)
        # Filter to keep only operands and operator tokens
        result: Expr | None = None
        i = 0
        while i < len(seq):
            item = seq[i]
            if hasattr(item, "rule_name"):
                operand = self._walk(item)  # type: ignore[arg-type]
                if result is None:
                    result = operand
                else:
                    raise ValueError(
                        "binop sequence missing operator before operand"
                    )
            else:
                # operator token
                op_text = item.value  # type: ignore[union-attr]
                if op_text not in _VERILOG_BINOP_MAP:
                    # Skip non-operator punctuation (e.g., parens, commas)
                    i += 1
                    continue
                # Need a right operand next.
                i += 1
                while i < len(seq) and not hasattr(seq[i], "rule_name"):
                    i += 1
                if i >= len(seq):
                    raise ValueError("binop missing right operand")
                right = self._walk(seq[i])  # type: ignore[arg-type]
                if result is None:
                    raise ValueError("binop missing left operand")
                result = BinaryOp(
                    op=_VERILOG_BINOP_MAP[op_text],
                    lhs=result,
                    rhs=right,
                    provenance=_provenance(node, self.file),
                )
            i += 1
        if result is None:
            raise ValueError(f"empty binop in {node.rule_name}")
        return result

    def _walk_primary(self, node: ASTNode) -> Expr:
        # primary = NUMBER | NAME | (expression) | concatenation | replication | system_call | etc.
        # Walk child structure.
        for c in node.children:
            if hasattr(c, "rule_name"):
                if c.rule_name == "expression":
                    return self._walk(c)  # type: ignore[arg-type]
                if c.rule_name == "concatenation":
                    return self._walk_concat(c)  # type: ignore[arg-type]
                # range_select: primary[h:l]
                if c.rule_name == "range_select":
                    base_token = _find_token(node)
                    if base_token is not None:
                        base = self._token_to_expr(base_token)
                        return self._apply_range_select(base, c)
                # Fallback: walk into any other AST child
                return self._walk(c)  # type: ignore[arg-type]
        # All-token primary: NUMBER or NAME
        for c in node.children:
            if _is_token(c):
                return self._token_to_expr(c)  # type: ignore[arg-type]
        raise ValueError(f"empty primary {node.rule_name}")

    def _walk_concat(self, node: ASTNode) -> Expr:
        parts: list[Expr] = []
        for c in node.children:
            if hasattr(c, "rule_name") and c.rule_name == "expression":
                parts.append(self._walk(c))  # type: ignore[arg-type]
        return Concat(
            parts=tuple(parts),
            provenance=_provenance(node, self.file),
        )

    def _apply_range_select(self, base: Expr, range_select_node: ASTNode) -> Expr:
        exprs = [
            c for c in range_select_node.children
            if hasattr(c, "rule_name") and c.rule_name == "expression"
        ]
        if len(exprs) == 1:
            # single bit
            idx = _eval_constant(exprs[0])  # type: ignore[arg-type]
            return Slice(base=base, msb=idx, lsb=idx)
        if len(exprs) == 2:
            msb = _eval_constant(exprs[0])  # type: ignore[arg-type]
            lsb = _eval_constant(exprs[1])  # type: ignore[arg-type]
            return Slice(base=base, msb=msb, lsb=lsb)
        raise ValueError("invalid range_select")

    def _token_to_expr(self, token: Token) -> Expr:
        text = token.value  # type: ignore[union-attr]
        kind = str(token.type)  # type: ignore[union-attr]

        if "NUMBER" in kind or "REAL" in kind:
            return Lit(value=_parse_number(text), type=TyLogic())

        if "NAME" in kind:
            # Resolve to PortRef or NetRef based on scope.
            if text in self.port_names:
                return PortRef(name=text)
            if text in self.net_names:
                return NetRef(name=text)
            # Default to NetRef; validator will catch
            return NetRef(name=text)

        # Unknown token kind
        return Lit(value=text, type=TyLogic())


# ---------------------------------------------------------------------------
# Module elaboration
# ---------------------------------------------------------------------------


def elaborate_module_decl(node: ASTNode, file: str = "<source>") -> Module:
    """Elaborate a single ``module_declaration`` AST node into a HIR Module."""
    if node.rule_name != "module_declaration":
        raise ValueError(f"expected module_declaration, got {node.rule_name}")

    # First NAME token is the module name.
    name_token = _find_token(node, kind="TokenType.NAME")
    if name_token is None:
        raise ValueError("module_declaration missing module name")
    mod_name = name_token.value

    module = Module(
        name=mod_name,
        provenance=_provenance(node, file),
    )

    # Parse the port_list (Verilog style: ports declared inline).
    port_list = _find_child(node, "port_list")
    if port_list is not None:
        for port_node in [c for c in port_list.children if hasattr(c, "rule_name") and c.rule_name == "port"]:
            module.ports.append(_elaborate_port(port_node, file))

    # Build set of port names for expression elaborator.
    port_names = {p.name for p in module.ports}
    net_names: set[str] = set()
    elaborator = ExprElaborator(file=file, port_names=port_names, net_names=net_names)

    # Walk module_items.
    for module_item in [c for c in node.children if hasattr(c, "rule_name") and c.rule_name == "module_item"]:
        _elaborate_module_item(module_item, module, elaborator, file)

    return module


def _elaborate_port(port_node: ASTNode, file: str) -> object:
    """Elaborate a Verilog port node -> hdl_ir.Port."""
    from hdl_ir import Port

    direction = Direction.IN
    dir_node = _find_child(port_node, "port_direction")
    if dir_node is not None:
        dir_token = _find_token(dir_node, kind="TokenType.KEYWORD")
        if dir_token is not None:
            v = dir_token.value
            if v == "input":
                direction = Direction.IN
            elif v == "output":
                direction = Direction.OUT
            elif v == "inout":
                direction = Direction.INOUT

    range_node = _find_child(port_node, "range")
    ty = _extract_type_from_range(range_node)

    name_token = _find_token(port_node, kind="TokenType.NAME")
    if name_token is None:
        raise ValueError("port missing name")
    return Port(
        name=name_token.value,
        direction=direction,
        type=ty,
        provenance=_provenance(port_node, file),
    )


def _elaborate_module_item(
    item: ASTNode, module: Module, elab: ExprElaborator, file: str
) -> None:
    # module_item -> continuous_assign | net_declaration | always_construct | ...
    for c in item.children:
        if not hasattr(c, "rule_name"):
            continue
        if c.rule_name == "continuous_assign":
            _elaborate_continuous_assign(c, module, elab, file)
        # Future: net_declaration, always_construct, etc.


def _elaborate_continuous_assign(
    node: ASTNode, module: Module, elab: ExprElaborator, file: str
) -> None:
    for c in node.children:
        if hasattr(c, "rule_name") and c.rule_name == "assignment":
            target = _elaborate_lvalue(c, elab, file)
            expr_node = _find_child(c, "expression")
            if expr_node is None:
                continue
            rhs = elab.elaborate(expr_node)
            module.cont_assigns.append(
                ContAssign(
                    target=target,
                    rhs=rhs,
                    provenance=_provenance(node, file),
                )
            )


def _elaborate_lvalue(
    assignment: ASTNode, elab: ExprElaborator, file: str
) -> Expr:
    lvalue = _find_child(assignment, "lvalue")
    if lvalue is None:
        raise ValueError("assignment missing lvalue")
    # An lvalue is either a NAME (possibly with a range_select) or a concatenation.
    for c in lvalue.children:
        if hasattr(c, "rule_name"):
            if c.rule_name == "concatenation":
                return elab._walk_concat(c)  # type: ignore[arg-type]
            if c.rule_name == "range_select":
                # Sibling NAME token in lvalue.
                name_token = _find_token(lvalue)
                if name_token is None:
                    raise ValueError("range_select lvalue missing name")
                base = elab._token_to_expr(name_token)
                return elab._apply_range_select(base, c)  # type: ignore[arg-type]
    # Plain name lvalue.
    name_token = _find_token(lvalue, kind="TokenType.NAME")
    if name_token is None:
        raise ValueError("lvalue missing name")
    return elab._token_to_expr(name_token)
