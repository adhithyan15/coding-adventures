"""Type checking for the first compiled ALGOL 60 subset."""

from __future__ import annotations

from dataclasses import dataclass, field

from lang_parser import ASTNode
from lexer import Token

INTEGER = "integer"
BOOLEAN = "boolean"
ERROR = "error"
FRAME_HEADER_SIZE = 20
FRAME_WORD_SIZE = 4


@dataclass(frozen=True)
class Diagnostic:
    """A stage-friendly type-checking diagnostic."""

    message: str
    line: int
    column: int


@dataclass
class Symbol:
    """A declared source name and the type associated with it."""

    name: str
    type_name: str
    line: int
    column: int
    symbol_id: int = -1
    kind: str = "scalar"
    storage_class: str = "frame"
    declaring_block_id: int = -1
    slot_offset: int | None = None
    slot_size: int | None = None


@dataclass(frozen=True)
class FrameSlot:
    """A concrete scalar storage cell within an ALGOL activation frame."""

    symbol_id: int
    name: str
    type_name: str
    offset: int
    size: int


@dataclass
class FrameLayout:
    """The planned memory footprint for one lexical block activation.

    ALGOL's nested scopes eventually become WASM linear-memory frames.  Phase 1
    does not emit those loads and stores yet; it records the header and slot
    layout so the later lowering pass can walk static links and address locals
    without re-resolving names.
    """

    block_id: int
    depth: int
    static_parent_id: int | None
    header_size: int = FRAME_HEADER_SIZE
    word_size: int = FRAME_WORD_SIZE
    slots: list[FrameSlot] = field(default_factory=list)

    @property
    def frame_size(self) -> int:
        return self.header_size + sum(slot.size for slot in self.slots)

    def allocate_scalar(self, symbol: Symbol) -> FrameSlot:
        slot = FrameSlot(
            symbol_id=symbol.symbol_id,
            name=symbol.name,
            type_name=symbol.type_name,
            offset=self.header_size + (len(self.slots) * self.word_size),
            size=self.word_size,
        )
        self.slots.append(slot)
        symbol.slot_offset = slot.offset
        symbol.slot_size = slot.size
        return slot


@dataclass
class Scope:
    """A lexical block scope.

    ALGOL 60 made nested lexical scopes mainstream. The checker mirrors that
    directly: every `begin ... end` block owns a scope, and lookups walk through
    parents when an inner block references an outer variable.
    """

    parent: Scope | None = None
    block_id: int = -1
    depth: int = -1
    frame_layout: FrameLayout | None = None
    symbols: dict[str, Symbol] = field(default_factory=dict)
    children: list[Scope] = field(default_factory=list)

    def declare(self, symbol: Symbol) -> bool:
        if symbol.name in self.symbols:
            return False
        self.symbols[symbol.name] = symbol
        return True

    def resolve(self, name: str) -> Symbol | None:
        resolved = self.resolve_with_scope(name)
        return resolved[0] if resolved is not None else None

    def resolve_with_scope(self, name: str) -> tuple[Symbol, Scope, int] | None:
        scope: Scope | None = self
        lexical_depth_delta = 0
        while scope is not None:
            found = scope.symbols.get(name)
            if found is not None:
                return found, scope, lexical_depth_delta
            scope = scope.parent
            lexical_depth_delta += 1
        return None


@dataclass(frozen=True)
class SemanticBlock:
    """A lexical ALGOL block enriched with static-parent and frame metadata."""

    block_id: int
    parent_block_id: int | None
    depth: int
    scope: Scope
    frame_layout: FrameLayout


@dataclass(frozen=True)
class ResolvedReference:
    """A variable occurrence after lexical lookup has selected one symbol."""

    token_id: int
    name: str
    role: str
    symbol_id: int
    type_name: str
    use_block_id: int
    declaration_block_id: int
    lexical_depth_delta: int
    slot_offset: int
    line: int
    column: int


@dataclass
class SemanticProgram:
    """Typed semantic facts produced before IR lowering."""

    ast: ASTNode
    root_block: SemanticBlock | None
    blocks: list[SemanticBlock]
    symbols: list[Symbol]
    references: list[ResolvedReference]
    diagnostics: list[Diagnostic] = field(default_factory=list)


@dataclass
class TypeCheckResult:
    """The typed surface consumed by the ALGOL IR compiler."""

    ast: ASTNode
    root_scope: Scope
    expression_types: dict[int, str]
    diagnostics: list[Diagnostic] = field(default_factory=list)
    semantic: SemanticProgram | None = None

    @property
    def ok(self) -> bool:
        return not self.diagnostics


class TypeCheckError(Exception):
    """Raised by callers that prefer exception-style checking."""


class AlgolTypeChecker:
    """Validate the first ALGOL 60 compiler subset."""

    def __init__(self) -> None:
        self.diagnostics: list[Diagnostic] = []
        self.expression_types: dict[int, str] = {}
        self.semantic_blocks: list[SemanticBlock] = []
        self.semantic_symbols: list[Symbol] = []
        self.resolved_references: list[ResolvedReference] = []
        self._next_block_id = 0
        self._next_symbol_id = 0

    def check(self, ast: ASTNode) -> TypeCheckResult:
        self.diagnostics = []
        self.expression_types = {}
        self.semantic_blocks = []
        self.semantic_symbols = []
        self.resolved_references = []
        self._next_block_id = 0
        self._next_symbol_id = 0
        root_scope = Scope()
        block = _first_node(ast, "block")
        if block is None:
            self._error(ast, "ALGOL program must contain a block")
        else:
            self._check_block(block, root_scope)
        semantic = SemanticProgram(
            ast=ast,
            root_block=self.semantic_blocks[0] if self.semantic_blocks else None,
            blocks=list(self.semantic_blocks),
            symbols=list(self.semantic_symbols),
            references=list(self.resolved_references),
            diagnostics=list(self.diagnostics),
        )
        return TypeCheckResult(
            ast=ast,
            root_scope=root_scope,
            expression_types=dict(self.expression_types),
            diagnostics=list(self.diagnostics),
            semantic=semantic,
        )

    def _check_block(self, block: ASTNode, parent: Scope) -> Scope:
        scope = self._new_block_scope(parent)

        for child in _node_children(block):
            if child.rule_name == "declaration":
                self._check_declaration(child, scope)

        for child in _node_children(block):
            if child.rule_name == "statement":
                self._check_statement(child, scope)

        return scope

    def _new_block_scope(self, parent: Scope) -> Scope:
        block_id = self._next_block_id
        self._next_block_id += 1
        parent_block_id = parent.block_id if parent.block_id >= 0 else None
        depth = parent.depth + 1 if parent.depth >= 0 else 0
        frame_layout = FrameLayout(
            block_id=block_id,
            depth=depth,
            static_parent_id=parent_block_id,
        )
        scope = Scope(
            parent=parent,
            block_id=block_id,
            depth=depth,
            frame_layout=frame_layout,
        )
        parent.children.append(scope)
        self.semantic_blocks.append(
            SemanticBlock(
                block_id=block_id,
                parent_block_id=parent_block_id,
                depth=depth,
                scope=scope,
                frame_layout=frame_layout,
            )
        )
        return scope

    def _check_declaration(self, declaration: ASTNode, scope: Scope) -> None:
        inner = _first_ast_child(declaration)
        if inner is None:
            return
        if inner.rule_name != "type_decl":
            self._error(inner, f"{inner.rule_name} declarations are not supported yet")
            return

        type_node = _first_node(inner, "type")
        declared_type = _first_keyword_value(type_node) if type_node is not None else ""
        if declared_type != INTEGER:
            self._error(
                inner, f"{declared_type or 'unknown'} variables are not supported yet"
            )
            return

        ident_list = _first_node(inner, "ident_list")
        for name_token in _tokens(ident_list):
            if name_token.type_name != "NAME":
                continue
            symbol = Symbol(
                name=name_token.value,
                type_name=declared_type,
                line=name_token.line,
                column=name_token.column,
                symbol_id=self._next_symbol_id,
                declaring_block_id=scope.block_id,
            )
            if not scope.declare(symbol):
                self._error(
                    name_token,
                    f"{name_token.value!r} is already declared in this scope",
                )
                continue
            self._next_symbol_id += 1
            if scope.frame_layout is not None:
                scope.frame_layout.allocate_scalar(symbol)
            self.semantic_symbols.append(symbol)

    def _check_statement(self, statement: ASTNode, scope: Scope) -> None:
        inner = _first_ast_child(statement)
        if inner is None:
            return
        if inner.rule_name == "unlabeled_stmt":
            self._check_unlabeled(inner, scope)
        elif inner.rule_name == "cond_stmt":
            self._check_cond(inner, scope)
        else:
            self._error(inner, f"{inner.rule_name} is not supported yet")

    def _check_unlabeled(self, node: ASTNode, scope: Scope) -> None:
        inner = _first_ast_child(node)
        if inner is None:
            return
        if inner.rule_name == "assign_stmt":
            self._check_assignment(inner, scope)
        elif inner.rule_name == "for_stmt":
            self._check_for(inner, scope)
        elif inner.rule_name == "compound_stmt":
            for statement in _direct_nodes(inner, "statement"):
                self._check_statement(statement, scope)
        elif inner.rule_name == "block":
            self._check_block(inner, scope)
        else:
            self._error(inner, f"{inner.rule_name} is not supported yet")

    def _check_assignment(self, assign: ASTNode, scope: Scope) -> None:
        left_parts = _direct_nodes(assign, "left_part")
        if len(left_parts) != 1:
            self._error(assign, "chained assignment is not supported yet")
            return

        name = _variable_name(left_parts[0])
        if name is None:
            self._error(left_parts[0], "only scalar variable assignment is supported")
            return

        symbol = self._resolve_name(name, scope, role="write")
        target_type = ERROR if symbol is None else symbol.type_name

        expr = _first_direct_node(assign, "expression")
        value_type = self._infer_expr(expr, scope) if expr is not None else ERROR
        if target_type != ERROR and value_type != ERROR and target_type != value_type:
            self._error(
                name,
                f"cannot assign {value_type} to {target_type} variable {name.value!r}",
            )

    def _check_cond(self, cond: ASTNode, scope: Scope) -> None:
        bool_expr = _first_direct_node(cond, "bool_expr")
        cond_type = (
            self._infer_expr(bool_expr, scope) if bool_expr is not None else ERROR
        )
        if cond_type != ERROR and cond_type != BOOLEAN:
            self._error(cond, "if condition must be boolean")

        seen_then = False
        for child in cond.children:
            if isinstance(child, Token) and child.value == "then":
                seen_then = True
            elif (
                isinstance(child, ASTNode)
                and child.rule_name == "unlabeled_stmt"
                and seen_then
            ):
                self._check_unlabeled(child, scope)
            elif isinstance(child, ASTNode) and child.rule_name == "statement":
                self._check_statement(child, scope)

    def _check_for(self, node: ASTNode, scope: Scope) -> None:
        loop_name = next(
            (tok for tok in _direct_tokens(node) if tok.type_name == "NAME"), None
        )
        if loop_name is None:
            self._error(node, "for loop is missing its control variable")
            return
        symbol = self._resolve_name(loop_name, scope, role="control")
        if symbol is not None and symbol.type_name != INTEGER:
            self._error(loop_name, "for loop control variable must be integer")

        for elem in _direct_nodes(_first_direct_node(node, "for_list"), "for_elem"):
            arith_nodes = _direct_nodes(elem, "arith_expr")
            if len(arith_nodes) != 3:
                self._error(elem, "only step/until for-elements are supported")
                continue
            for arith_node in arith_nodes:
                expr_type = self._infer_expr(arith_node, scope)
                if expr_type != ERROR and expr_type != INTEGER:
                    self._error(arith_node, "for loop bounds must be integer")

        body = _first_direct_node(node, "statement")
        if body is not None:
            self._check_statement(body, scope)

    def _infer_expr(self, expr: ASTNode | Token | None, scope: Scope) -> str:
        if expr is None:
            return ERROR
        if isinstance(expr, Token):
            inferred = self._infer_token(expr, scope)
            self.expression_types[id(expr)] = inferred
            return inferred

        if expr.rule_name == "variable":
            name = _variable_name(expr)
            if name is None:
                self._error(expr, "array subscripts are not supported yet")
                inferred = ERROR
            else:
                symbol = self._resolve_name(name, scope, role="read")
                inferred = ERROR if symbol is None else symbol.type_name
            self.expression_types[id(expr)] = inferred
            return inferred

        inferred = self._infer_ast_expr(expr, scope)
        self.expression_types[id(expr)] = inferred
        return inferred

    def _infer_ast_expr(self, expr: ASTNode, scope: Scope) -> str:
        meaningful = _meaningful_children(expr)
        if not meaningful:
            return ERROR

        if len(meaningful) == 1:
            return self._infer_expr(meaningful[0], scope)

        if expr.rule_name in {"expr_not", "bool_factor", "bool_secondary"}:
            first = meaningful[0]
            if isinstance(first, Token) and first.value == "not":
                return self._require_unary(expr, meaningful[1], scope, BOOLEAN, BOOLEAN)

        if expr.rule_name in {"expr_add", "simple_arith"} and isinstance(
            meaningful[0], Token
        ):
            operator = meaningful[0].value
            if operator in {"+", "-"}:
                return self._require_unary(expr, meaningful[1], scope, INTEGER, INTEGER)

        if expr.rule_name in {
            "expr_eqv",
            "expr_impl",
            "expr_or",
            "expr_and",
            "simple_bool",
            "implication",
            "bool_term",
        }:
            return self._fold_binary(expr, meaningful, scope, BOOLEAN, BOOLEAN)

        if expr.rule_name in {"expr_cmp", "relation"}:
            if any(
                isinstance(child, Token)
                and child.value in {"=", "!=", "<", "<=", ">", ">="}
                for child in meaningful
            ):
                return self._fold_binary(expr, meaningful, scope, INTEGER, BOOLEAN)
            return self._infer_expr(meaningful[0], scope)

        if expr.rule_name in {"expr_add", "simple_arith", "expr_mul", "term"}:
            if any(
                isinstance(child, Token) and child.value == "/" for child in meaningful
            ):
                self._error(
                    expr,
                    "real division is not supported yet; use div for integer division",
                )
                return ERROR
            return self._fold_binary(expr, meaningful, scope, INTEGER, INTEGER)

        if expr.rule_name in {"expr_pow", "factor"}:
            if any(
                isinstance(child, Token) and child.value in {"**", "^"}
                for child in meaningful
            ):
                self._error(expr, "exponentiation is not supported yet")
                return ERROR
            return self._infer_expr(meaningful[0], scope)

        if expr.rule_name in {"expr_atom", "primary", "bool_primary"}:
            if any(
                isinstance(child, Token) and child.value == "(" for child in meaningful
            ):
                nested = next(
                    (child for child in meaningful if isinstance(child, ASTNode)), None
                )
                return self._infer_expr(nested, scope)
            first_token = next(
                (child for child in meaningful if isinstance(child, Token)), None
            )
            if first_token is not None:
                return self._infer_token(first_token, scope)
            return self._infer_expr(meaningful[0], scope)

        if expr.rule_name in {"expression", "arith_expr", "bool_expr"}:
            return self._infer_expr(meaningful[0], scope)

        return self._infer_expr(meaningful[0], scope)

    def _infer_token(self, token: Token, scope: Scope) -> str:
        if token.type_name == "INTEGER_LIT":
            return INTEGER
        if token.value in {"true", "false"}:
            return BOOLEAN
        if token.type_name == "NAME":
            symbol = self._resolve_name(token, scope, role="read")
            if symbol is None:
                return ERROR
            return symbol.type_name
        self._error(token, f"unsupported expression token {token.value!r}")
        return ERROR

    def _resolve_name(
        self,
        token: Token,
        scope: Scope,
        *,
        role: str,
    ) -> Symbol | None:
        resolved = scope.resolve_with_scope(token.value)
        if resolved is None:
            self._error(
                token,
                f"{token.value!r} is not declared in block {scope.block_id} "
                "or its lexical parents",
            )
            return None

        symbol, declaring_scope, lexical_depth_delta = resolved
        if symbol.slot_offset is None:
            self._error(token, f"{token.value!r} has no planned frame slot")
            return symbol

        self.resolved_references.append(
            ResolvedReference(
                token_id=id(token),
                name=token.value,
                role=role,
                symbol_id=symbol.symbol_id,
                type_name=symbol.type_name,
                use_block_id=scope.block_id,
                declaration_block_id=declaring_scope.block_id,
                lexical_depth_delta=lexical_depth_delta,
                slot_offset=symbol.slot_offset,
                line=token.line,
                column=token.column,
            )
        )
        return symbol

    def _require_unary(
        self,
        node: ASTNode,
        operand: ASTNode | Token,
        scope: Scope,
        operand_type: str,
        result_type: str,
    ) -> str:
        actual = self._infer_expr(operand, scope)
        if actual != ERROR and actual != operand_type:
            self._error(node, f"operator requires {operand_type}, got {actual}")
            return ERROR
        return result_type

    def _fold_binary(
        self,
        node: ASTNode,
        children: list[ASTNode | Token],
        scope: Scope,
        operand_type: str,
        result_type: str,
    ) -> str:
        saw_operator = False
        for child in children:
            if isinstance(child, Token):
                saw_operator = True
                continue
            actual = self._infer_expr(child, scope)
            if actual != ERROR and actual != operand_type:
                self._error(node, f"operator requires {operand_type}, got {actual}")
                return ERROR
        return result_type if saw_operator else self._infer_expr(children[0], scope)

    def _error(self, obj: ASTNode | Token, message: str) -> None:
        self.diagnostics.append(Diagnostic(message, *_position(obj)))


def check_algol(ast: ASTNode) -> TypeCheckResult:
    return AlgolTypeChecker().check(ast)


def check(ast: ASTNode) -> TypeCheckResult:
    return check_algol(ast)


def assert_algol_typed(ast: ASTNode) -> TypeCheckResult:
    result = check_algol(ast)
    if not result.ok:
        details = "\n".join(
            f"Line {diag.line}, Col {diag.column}: {diag.message}"
            for diag in result.diagnostics
        )
        raise TypeCheckError(details)
    return result


def _position(obj: ASTNode | Token) -> tuple[int, int]:
    if isinstance(obj, Token):
        return obj.line, obj.column
    return obj.start_line or 1, obj.start_column or 1


def _node_children(node: ASTNode | None) -> list[ASTNode]:
    if node is None:
        return []
    return [child for child in node.children if isinstance(child, ASTNode)]


def _direct_nodes(node: ASTNode | None, rule_name: str) -> list[ASTNode]:
    return [child for child in _node_children(node) if child.rule_name == rule_name]


def _first_direct_node(node: ASTNode | None, rule_name: str) -> ASTNode | None:
    return next(iter(_direct_nodes(node, rule_name)), None)


def _first_ast_child(node: ASTNode) -> ASTNode | None:
    return next((child for child in node.children if isinstance(child, ASTNode)), None)


def _direct_tokens(node: ASTNode | None) -> list[Token]:
    if node is None:
        return []
    return [child for child in node.children if isinstance(child, Token)]


def _tokens(node: ASTNode | None) -> list[Token]:
    if node is None:
        return []
    found: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            found.append(child)
        else:
            found.extend(_tokens(child))
    return found


def _meaningful_children(node: ASTNode) -> list[ASTNode | Token]:
    return [
        child
        for child in node.children
        if not (isinstance(child, Token) and child.value in {"(", ")"})
    ]


def _first_node(node: ASTNode, rule_name: str) -> ASTNode | None:
    if node.rule_name == rule_name:
        return node
    for child in _node_children(node):
        found = _first_node(child, rule_name)
        if found is not None:
            return found
    return None


def _first_keyword_value(node: ASTNode | None) -> str | None:
    return next(
        (token.value for token in _tokens(node) if token.type_name == "KEYWORD"), None
    )


def _variable_name(node: ASTNode) -> Token | None:
    variable = node if node.rule_name == "variable" else _first_node(node, "variable")
    if variable is None:
        return None
    names = [token for token in _tokens(variable) if token.type_name == "NAME"]
    return names[0] if len(names) == 1 else None
