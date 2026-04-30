"""Type checking for the first compiled ALGOL 60 subset."""

from __future__ import annotations

from dataclasses import dataclass, field

from lang_parser import ASTNode
from lexer import Token

INTEGER = "integer"
BOOLEAN = "boolean"
REAL = "real"
STRING = "string"
ERROR = "error"
FRAME_HEADER_SIZE = 20
FRAME_WORD_SIZE = 4
FRAME_REAL_SIZE = 8
VALUE = "value"
NAME = "name"
ARRAY = "array"
LABEL = "label"
SWITCH = "switch"
STATIC = "static"
PARAMETER_STORAGE = "parameter"
MAX_ARRAY_DIMENSIONS = 4
_OUTPUT_BUILTINS = {"print", "output"}
_FIXED_RETURN_NUMERIC_BUILTINS = {"entier": INTEGER, "sign": INTEGER, "sqrt": REAL}
_NUMERIC_BUILTINS = {"abs"} | set(_FIXED_RETURN_NUMERIC_BUILTINS)
_READ_ONLY_BUILTINS = _OUTPUT_BUILTINS | _NUMERIC_BUILTINS


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
    procedure_id: int | None = None
    array_id: int | None = None
    switch_id: int | None = None
    parameter_mode: str | None = None


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
        slot_size = FRAME_REAL_SIZE if symbol.type_name == REAL else self.word_size
        slot = FrameSlot(
            symbol_id=symbol.symbol_id,
            name=symbol.name,
            type_name=symbol.type_name,
            offset=self.header_size + sum(existing.size for existing in self.slots),
            size=slot_size,
        )
        self.slots.append(slot)
        symbol.slot_offset = slot.offset
        symbol.slot_size = slot.size
        return slot

    def allocate_descriptor(self, symbol: Symbol) -> FrameSlot:
        slot = FrameSlot(
            symbol_id=symbol.symbol_id,
            name=symbol.name,
            type_name=symbol.type_name,
            offset=self.header_size + sum(existing.size for existing in self.slots),
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
    owner_procedure_id: int | None = None
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
    ast_node_id: int | None = None
    owner_procedure_id: int | None = None


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
    storage_class: str
    slot_offset: int
    line: int
    column: int


@dataclass(frozen=True)
class ProcedureFormalCallShape:
    """A call shape required by a formal procedure parameter."""

    role: str
    argument_types: tuple[str, ...]
    argument_kinds: tuple[str, ...] = ()
    argument_assignable: tuple[bool, ...] = ()
    procedure_argument_ids: tuple[int | None, ...] = ()
    return_type: str | None = None


@dataclass(frozen=True)
class ProcedureParameter:
    """A procedure parameter planned as a frame slot in an activation."""

    name: str
    type_name: str
    mode: str
    symbol_id: int
    slot_offset: int
    kind: str = "scalar"
    may_write: bool = False
    write_reason: str | None = None
    procedure_call_shapes: tuple[ProcedureFormalCallShape, ...] = ()


@dataclass(frozen=True)
class _ParameterSpec:
    """Combined kind/type information for one formal parameter."""

    kind: str = "scalar"
    type_name: str | None = None


@dataclass(frozen=True)
class ProcedureDescriptor:
    """Semantic facts needed to lower a direct ALGOL procedure call."""

    procedure_id: int
    name: str
    label: str
    declaring_block_id: int
    body_block_id: int
    body_node_id: int
    return_type: str | None
    parameters: tuple[ProcedureParameter, ...]
    line: int
    column: int
    parameter_symbol_id: int | None = None


@dataclass(frozen=True)
class ResolvedProcedureCall:
    """A procedure occurrence after lexical lookup has selected a descriptor."""

    token_id: int
    name: str
    role: str
    procedure_id: int
    label: str
    use_block_id: int
    declaration_block_id: int
    lexical_depth_delta: int
    argument_count: int
    return_type: str | None
    line: int
    column: int
    parameter_symbol_id: int | None = None


@dataclass(frozen=True)
class ArrayDimension:
    """A declared array dimension with bound expression AST identities."""

    lower_node_id: int
    upper_node_id: int


@dataclass(frozen=True)
class ArrayDescriptor:
    """Semantic facts needed to allocate and index a direct ALGOL array."""

    array_id: int
    name: str
    element_type: str
    storage_class: str
    declaring_block_id: int
    dimensions: tuple[ArrayDimension, ...]
    symbol_id: int
    slot_offset: int
    line: int
    column: int


@dataclass(frozen=True)
class ResolvedArrayAccess:
    """A subscripted array occurrence after lexical lookup."""

    token_id: int
    name: str
    role: str
    array_id: int
    use_block_id: int
    declaration_block_id: int
    lexical_depth_delta: int
    slot_offset: int
    subscript_count: int
    line: int
    column: int


@dataclass(frozen=True)
class LabelDescriptor:
    """A statement label visible within one ALGOL frame."""

    label_id: int
    name: str
    ir_label: str
    declaring_block_id: int
    symbol_id: int
    statement_node_id: int
    line: int
    column: int


@dataclass(frozen=True)
class ResolvedGoto:
    """A local goto after designational-expression lookup."""

    token_id: int
    label_id: int
    target_name: str
    ir_label: str
    source_block_id: int
    target_block_id: int
    lexical_depth_delta: int
    line: int
    column: int
    designational_node_id: int | None = None


@dataclass(frozen=True)
class SwitchDescriptor:
    """A local ALGOL switch table with designational-expression entries."""

    switch_id: int
    name: str
    declaring_block_id: int
    symbol_id: int
    entry_node_ids: tuple[int, ...]
    line: int
    column: int


@dataclass(frozen=True)
class ResolvedSwitchSelection:
    """A switch subscript occurrence after semantic lookup."""

    node_id: int
    token_id: int
    name: str
    switch_id: int
    use_block_id: int
    declaration_block_id: int
    lexical_depth_delta: int
    index_node_id: int
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
    procedures: list[ProcedureDescriptor] = field(default_factory=list)
    procedure_calls: list[ResolvedProcedureCall] = field(default_factory=list)
    arrays: list[ArrayDescriptor] = field(default_factory=list)
    array_accesses: list[ResolvedArrayAccess] = field(default_factory=list)
    labels: list[LabelDescriptor] = field(default_factory=list)
    gotos: list[ResolvedGoto] = field(default_factory=list)
    switches: list[SwitchDescriptor] = field(default_factory=list)
    switch_selections: list[ResolvedSwitchSelection] = field(default_factory=list)
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
        self.semantic_procedures: list[ProcedureDescriptor] = []
        self.procedure_parameter_call_shapes: dict[
            int,
            list[ProcedureFormalCallShape],
        ] = {}
        self.resolved_procedure_calls: list[ResolvedProcedureCall] = []
        self.semantic_arrays: list[ArrayDescriptor] = []
        self.resolved_array_accesses: list[ResolvedArrayAccess] = []
        self.semantic_labels: list[LabelDescriptor] = []
        self.resolved_gotos: list[ResolvedGoto] = []
        self.semantic_switches: list[SwitchDescriptor] = []
        self.resolved_switch_selections: list[ResolvedSwitchSelection] = []
        self._next_block_id = 0
        self._next_symbol_id = 0
        self._next_procedure_id = 0
        self._next_array_id = 0
        self._next_label_id = 0
        self._next_switch_id = 0
        self._next_static_offset = 0

    def check(self, ast: ASTNode) -> TypeCheckResult:
        self.diagnostics = []
        self.expression_types = {}
        self.semantic_blocks = []
        self.semantic_symbols = []
        self.resolved_references = []
        self.semantic_procedures = []
        self.procedure_parameter_call_shapes = {}
        self.resolved_procedure_calls = []
        self.semantic_arrays = []
        self.resolved_array_accesses = []
        self.semantic_labels = []
        self.resolved_gotos = []
        self.semantic_switches = []
        self.resolved_switch_selections = []
        self._next_block_id = 0
        self._next_symbol_id = 0
        self._next_procedure_id = 0
        self._next_array_id = 0
        self._next_label_id = 0
        self._next_switch_id = 0
        self._next_static_offset = 0
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
            procedures=list(self.semantic_procedures),
            procedure_calls=list(self.resolved_procedure_calls),
            arrays=list(self.semantic_arrays),
            array_accesses=list(self.resolved_array_accesses),
            labels=list(self.semantic_labels),
            gotos=list(self.resolved_gotos),
            switches=list(self.semantic_switches),
            switch_selections=list(self.resolved_switch_selections),
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
        scope = self._new_block_scope(parent, ast_node=block)
        self._check_block_contents(block, scope)
        return scope

    def _check_block_contents(self, block: ASTNode, scope: Scope) -> None:
        for child in _node_children(block):
            if child.rule_name == "statement":
                self._collect_statement_labels(child, scope)

        for child in _node_children(block):
            if child.rule_name == "declaration":
                self._check_declaration(child, scope)

        for child in _node_children(block):
            if child.rule_name == "statement":
                self._check_statement(child, scope)

    def _new_block_scope(
        self,
        parent: Scope,
        *,
        ast_node: ASTNode | None = None,
        owner_procedure_id: int | None = None,
    ) -> Scope:
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
            owner_procedure_id=(
                owner_procedure_id
                if owner_procedure_id is not None
                else parent.owner_procedure_id
            ),
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
                ast_node_id=id(ast_node) if ast_node is not None else None,
                owner_procedure_id=scope.owner_procedure_id,
            )
        )
        return scope

    def _check_declaration(self, declaration: ASTNode, scope: Scope) -> None:
        inner = _first_ast_child(declaration)
        if inner is None:
            return
        if inner.rule_name == "procedure_decl":
            self._check_procedure_declaration(inner, scope)
            return
        if inner.rule_name == "own_decl":
            self._check_scalar_declaration(inner, scope, storage_class=STATIC)
            return
        if inner.rule_name == "own_array_decl":
            self._check_array_declaration(inner, scope, storage_class=STATIC)
            return
        if inner.rule_name == "array_decl":
            self._check_array_declaration(inner, scope, storage_class="frame")
            return
        if inner.rule_name == "switch_decl":
            self._check_switch_declaration(inner, scope)
            return
        if inner.rule_name != "type_decl":
            self._error(inner, f"{inner.rule_name} declarations are not supported yet")
            return

        self._check_scalar_declaration(inner, scope, storage_class="frame")

    def _check_scalar_declaration(
        self,
        node: ASTNode,
        scope: Scope,
        *,
        storage_class: str,
    ) -> None:
        type_node = _first_node(node, "type")
        declared_type = _first_keyword_value(type_node) if type_node is not None else ""
        if declared_type not in {INTEGER, BOOLEAN, REAL, STRING}:
            self._error(
                node, f"{declared_type or 'unknown'} variables are not supported yet"
            )
            return

        ident_list = _first_node(node, "ident_list")
        for name_token in _tokens(ident_list):
            if name_token.type_name != "NAME":
                continue
            symbol = Symbol(
                name=name_token.value,
                type_name=declared_type,
                line=name_token.line,
                column=name_token.column,
                symbol_id=self._next_symbol_id,
                storage_class=storage_class,
                declaring_block_id=scope.block_id,
            )
            if not scope.declare(symbol):
                self._error(
                    name_token,
                    f"{name_token.value!r} is already declared in this scope",
                )
                continue
            self._next_symbol_id += 1
            if storage_class == STATIC:
                self._allocate_static_scalar(symbol)
            elif scope.frame_layout is not None:
                scope.frame_layout.allocate_scalar(symbol)
            self.semantic_symbols.append(symbol)

    def _allocate_static_scalar(self, symbol: Symbol) -> None:
        slot_size = FRAME_REAL_SIZE if symbol.type_name == REAL else FRAME_WORD_SIZE
        symbol.slot_offset = self._next_static_offset
        symbol.slot_size = slot_size
        self._next_static_offset += slot_size

    def _allocate_static_descriptor(self, symbol: Symbol) -> None:
        symbol.slot_offset = self._next_static_offset
        symbol.slot_size = FRAME_WORD_SIZE
        self._next_static_offset += FRAME_WORD_SIZE

    def _check_array_declaration(
        self,
        node: ASTNode,
        scope: Scope,
        *,
        storage_class: str,
    ) -> None:
        type_node = _first_direct_node(node, "type")
        element_type = (
            _first_keyword_value(type_node) if type_node is not None else REAL
        )
        if element_type not in {INTEGER, BOOLEAN, REAL, STRING}:
            self._error(
                node,
                f"{element_type} arrays are not supported yet",
            )
            return

        for segment in _direct_nodes(node, "array_segment"):
            bound_pairs = _direct_nodes(segment, "bound_pair")
            if not bound_pairs:
                self._error(segment, "array declaration is missing bounds")
                continue
            if len(bound_pairs) > MAX_ARRAY_DIMENSIONS:
                self._error(
                    segment,
                    f"arrays support at most {MAX_ARRAY_DIMENSIONS} dimensions "
                    "in this phase",
                )
                continue

            dimensions: list[ArrayDimension] = []
            for bound_pair in bound_pairs:
                bounds = _direct_nodes(bound_pair, "arith_expr")
                if len(bounds) != 2:
                    self._error(bound_pair, "array bounds must be lower:upper pairs")
                    continue
                for bound in bounds:
                    bound_type = self._infer_expr(bound, scope)
                    if bound_type != ERROR and bound_type != INTEGER:
                        self._error(bound, "array bounds must be integer")
                dimensions.append(
                    ArrayDimension(
                        lower_node_id=id(bounds[0]),
                        upper_node_id=id(bounds[1]),
                    )
                )

            ident_list = _first_direct_node(segment, "ident_list")
            for name_token in _tokens(ident_list):
                if name_token.type_name != "NAME":
                    continue
                array_id = self._next_array_id
                self._next_array_id += 1
                symbol = Symbol(
                    name=name_token.value,
                    type_name=element_type,
                    line=name_token.line,
                    column=name_token.column,
                    symbol_id=self._next_symbol_id,
                    kind=ARRAY,
                    storage_class=storage_class,
                    declaring_block_id=scope.block_id,
                    array_id=array_id,
                )
                if not scope.declare(symbol):
                    self._error(
                        name_token,
                        f"{name_token.value!r} is already declared in this scope",
                    )
                    continue
                self._next_symbol_id += 1
                if storage_class == STATIC:
                    self._allocate_static_descriptor(symbol)
                elif scope.frame_layout is not None:
                    scope.frame_layout.allocate_descriptor(symbol)
                self.semantic_symbols.append(symbol)
                if symbol.slot_offset is None:
                    continue
                self.semantic_arrays.append(
                    ArrayDescriptor(
                        array_id=array_id,
                        name=name_token.value,
                        element_type=element_type,
                        storage_class=storage_class,
                        declaring_block_id=scope.block_id,
                        dimensions=tuple(dimensions),
                        symbol_id=symbol.symbol_id,
                        slot_offset=symbol.slot_offset,
                        line=name_token.line,
                        column=name_token.column,
                    )
                )

    def _check_switch_declaration(self, node: ASTNode, scope: Scope) -> None:
        name_token = next(
            (token for token in _direct_tokens(node) if token.type_name == "NAME"),
            None,
        )
        if name_token is None:
            self._error(node, "switch declaration is missing a name")
            return

        switch_id = self._next_switch_id
        self._next_switch_id += 1
        symbol = Symbol(
            name=name_token.value,
            type_name=SWITCH,
            line=name_token.line,
            column=name_token.column,
            symbol_id=self._next_symbol_id,
            kind=SWITCH,
            storage_class="code",
            declaring_block_id=scope.block_id,
            switch_id=switch_id,
        )
        if not scope.declare(symbol):
            self._error(
                name_token,
                f"{name_token.value!r} is already declared in this scope",
            )
            return
        self._next_symbol_id += 1
        self.semantic_symbols.append(symbol)

        switch_list = _first_direct_node(node, "switch_list")
        entries = _direct_nodes(switch_list, "desig_expr")
        if not entries:
            self._error(node, "switch declaration requires at least one entry")
            return
        for entry in entries:
            self._check_designational(
                entry,
                scope,
                allow_nonlocal_label=True,
                allow_switch_selection=True,
                active_switch_id=switch_id,
            )
        self.semantic_switches.append(
            SwitchDescriptor(
                switch_id=switch_id,
                name=name_token.value,
                declaring_block_id=scope.block_id,
                symbol_id=symbol.symbol_id,
                entry_node_ids=tuple(id(entry) for entry in entries),
                line=name_token.line,
                column=name_token.column,
            )
        )

    def _check_procedure_declaration(self, node: ASTNode, scope: Scope) -> None:
        name_token = _procedure_name(node)
        if name_token is None:
            self._error(node, "procedure declaration is missing a name")
            return

        return_type = _procedure_return_type(node)
        if return_type is not None and return_type not in {
            INTEGER,
            BOOLEAN,
            REAL,
            STRING,
        }:
            self._error(
                name_token,
                f"{return_type} procedure results are not supported yet",
            )
            return

        formal_names = _formal_parameter_names(node)
        value_names = _value_parameter_names(node)
        parameter_specs = _parameter_specs(node)
        body = _first_direct_node(node, "proc_body")
        body_inner = _first_ast_child(body) if body is not None else None
        known_procedures = _unique_procedure_names(self.semantic_procedures)
        write_reasons = _by_name_formal_write_reasons(
            body_inner,
            formal_names,
            value_names,
            known_procedures,
            name_token.value,
        )
        for formal in formal_names:
            mode = VALUE if formal.value in value_names else NAME
            parameter_spec = parameter_specs.get(formal.value, _ParameterSpec())
            parameter_kind = parameter_spec.kind
            parameter_type = parameter_spec.type_name
            if parameter_kind == ARRAY:
                if parameter_type is None:
                    parameter_type = REAL
            elif parameter_kind == LABEL:
                parameter_type = LABEL
            elif parameter_kind == SWITCH:
                parameter_type = SWITCH
            elif parameter_kind == "procedure":
                parameter_type = "procedure"
            elif mode == NAME and parameter_type not in {
                INTEGER,
                BOOLEAN,
                REAL,
                STRING,
            }:
                self._error(
                    formal,
                    f"by-name parameter {formal.value!r} must have an integer, "
                    "boolean, real, or string specifier",
                )
            elif mode == VALUE and parameter_type not in {
                INTEGER,
                BOOLEAN,
                REAL,
                STRING,
            }:
                self._error(
                    formal,
                    f"value parameter {formal.value!r} must have an integer, "
                    "boolean, real, or string specifier",
                )

        procedure_id = self._next_procedure_id
        self._next_procedure_id += 1
        label = f"_fn_algol_{procedure_id}_{name_token.value}"
        procedure_symbol = Symbol(
            name=name_token.value,
            type_name=return_type or "procedure",
            line=name_token.line,
            column=name_token.column,
            symbol_id=self._next_symbol_id,
            kind="procedure",
            storage_class="code",
            declaring_block_id=scope.block_id,
            procedure_id=procedure_id,
        )
        if not scope.declare(procedure_symbol):
            self._error(
                name_token,
                f"{name_token.value!r} is already declared in this scope",
            )
            return
        self._next_symbol_id += 1
        self.semantic_symbols.append(procedure_symbol)

        if body_inner is None:
            self._error(node, "procedure declaration is missing a body")
            return

        proc_scope = self._new_block_scope(
            scope,
            ast_node=body_inner,
            owner_procedure_id=procedure_id,
        )
        parameters: list[ProcedureParameter] = []

        if return_type is not None:
            result_symbol = Symbol(
                name=name_token.value,
                type_name=return_type,
                line=name_token.line,
                column=name_token.column,
                symbol_id=self._next_symbol_id,
                kind="procedure_result",
                declaring_block_id=proc_scope.block_id,
                procedure_id=procedure_id,
            )
            proc_scope.declare(result_symbol)
            self._next_symbol_id += 1
            if proc_scope.frame_layout is not None:
                proc_scope.frame_layout.allocate_scalar(result_symbol)
            self.semantic_symbols.append(result_symbol)

        for formal in formal_names:
            mode = VALUE if formal.value in value_names else NAME
            parameter_spec = parameter_specs.get(formal.value, _ParameterSpec())
            parameter_kind = parameter_spec.kind
            parameter_type = parameter_spec.type_name
            if parameter_kind == ARRAY:
                if parameter_type is None:
                    parameter_type = REAL
            elif parameter_kind == LABEL:
                parameter_type = LABEL
            elif parameter_kind == SWITCH:
                parameter_type = SWITCH
            elif (
                parameter_kind == "procedure"
                and parameter_type not in {INTEGER, BOOLEAN, REAL, STRING}
            ):
                parameter_type = "procedure"
            param_symbol = Symbol(
                name=formal.value,
                type_name=(
                    parameter_type
                    if parameter_type
                    in {INTEGER, BOOLEAN, REAL, STRING, LABEL, SWITCH, "procedure"}
                    else INTEGER
                ),
                line=formal.line,
                column=formal.column,
                symbol_id=self._next_symbol_id,
                kind=(
                    parameter_kind
                    if parameter_kind in {ARRAY, LABEL, SWITCH}
                    else "procedure_parameter"
                    if parameter_kind == "procedure"
                    else "parameter"
                ),
                storage_class=(
                    PARAMETER_STORAGE if parameter_kind == ARRAY else "frame"
                ),
                declaring_block_id=proc_scope.block_id,
                procedure_id=procedure_id,
                array_id=(
                    self._next_array_id if parameter_kind == ARRAY else None
                ),
                parameter_mode=mode,
            )
            if not proc_scope.declare(param_symbol):
                self._error(
                    formal,
                    f"{formal.value!r} is already declared in this procedure",
                )
                continue
            self._next_symbol_id += 1
            if parameter_kind == ARRAY:
                self._next_array_id += 1
                if proc_scope.frame_layout is not None:
                    proc_scope.frame_layout.allocate_descriptor(param_symbol)
            elif proc_scope.frame_layout is not None:
                proc_scope.frame_layout.allocate_scalar(param_symbol)
            self.semantic_symbols.append(param_symbol)
            if (
                parameter_kind == ARRAY
                and param_symbol.slot_offset is not None
                and param_symbol.array_id is not None
            ):
                self.semantic_arrays.append(
                    ArrayDescriptor(
                        array_id=param_symbol.array_id,
                        name=formal.value,
                        element_type=param_symbol.type_name,
                        storage_class=PARAMETER_STORAGE,
                        declaring_block_id=proc_scope.block_id,
                        dimensions=tuple(),
                        symbol_id=param_symbol.symbol_id,
                        slot_offset=param_symbol.slot_offset,
                        line=formal.line,
                        column=formal.column,
                    )
                )
            if param_symbol.slot_offset is not None:
                parameters.append(
                    ProcedureParameter(
                        name=formal.value,
                        type_name=param_symbol.type_name,
                        mode=mode,
                        symbol_id=param_symbol.symbol_id,
                        slot_offset=param_symbol.slot_offset,
                        kind=parameter_kind,
                        may_write=(
                            formal.value in write_reasons
                            if parameter_kind == "scalar"
                            else False
                        ),
                        write_reason=(
                            write_reasons.get(formal.value)
                            if parameter_kind == "scalar"
                            else None
                        ),
                    )
                )

        descriptor = ProcedureDescriptor(
            procedure_id=procedure_id,
            name=name_token.value,
            label=label,
            declaring_block_id=scope.block_id,
            body_block_id=proc_scope.block_id,
            body_node_id=id(body_inner),
            return_type=return_type,
            parameters=tuple(parameters),
            line=name_token.line,
            column=name_token.column,
        )
        descriptor_index = len(self.semantic_procedures)
        self.semantic_procedures.append(descriptor)

        if body_inner.rule_name == "block":
            self._check_block_contents(body_inner, proc_scope)
        else:
            self._collect_statement_labels(body_inner, proc_scope)
            self._check_statement(body_inner, proc_scope)
        updated_parameters = tuple(
            self._procedure_parameter_with_call_shapes(parameter)
            for parameter in parameters
        )
        if updated_parameters != descriptor.parameters:
            self.semantic_procedures[descriptor_index] = ProcedureDescriptor(
                procedure_id=descriptor.procedure_id,
                name=descriptor.name,
                label=descriptor.label,
                declaring_block_id=descriptor.declaring_block_id,
                body_block_id=descriptor.body_block_id,
                body_node_id=descriptor.body_node_id,
                return_type=descriptor.return_type,
                parameters=updated_parameters,
                line=descriptor.line,
                column=descriptor.column,
            )

    def _procedure_parameter_with_call_shapes(
        self,
        parameter: ProcedureParameter,
    ) -> ProcedureParameter:
        if parameter.kind != "procedure":
            return parameter
        shapes = tuple(
            self.procedure_parameter_call_shapes.get(parameter.symbol_id, ())
        )
        if shapes == parameter.procedure_call_shapes:
            return parameter
        return ProcedureParameter(
            name=parameter.name,
            type_name=parameter.type_name,
            mode=parameter.mode,
            symbol_id=parameter.symbol_id,
            slot_offset=parameter.slot_offset,
            kind=parameter.kind,
            may_write=parameter.may_write,
            write_reason=parameter.write_reason,
            procedure_call_shapes=shapes,
        )

    def _collect_statement_labels(self, statement: ASTNode, scope: Scope) -> None:
        label_node = _first_direct_node(statement, "label")
        if label_node is not None:
            label_token = _label_token(label_node)
            if label_token is not None:
                self._declare_label(label_token, statement, scope)

        body = _statement_body(statement)
        if body is None or body.rule_name == "block":
            return
        for child in _node_children(body):
            if child.rule_name == "statement":
                self._collect_statement_labels(child, scope)
            elif child.rule_name == "unlabeled_stmt":
                nested = _first_ast_child(child)
                if nested is not None and nested.rule_name != "block":
                    for nested_statement in _direct_nodes(nested, "statement"):
                        self._collect_statement_labels(nested_statement, scope)

    def _declare_label(
        self,
        label_token: Token,
        statement: ASTNode,
        scope: Scope,
    ) -> None:
        label_id = self._next_label_id
        self._next_label_id += 1
        symbol = Symbol(
            name=label_token.value,
            type_name=LABEL,
            line=label_token.line,
            column=label_token.column,
            symbol_id=self._next_symbol_id,
            kind=LABEL,
            storage_class="code",
            declaring_block_id=scope.block_id,
        )
        if not scope.declare(symbol):
            self._error(
                label_token,
                f"label {label_token.value!r} is already declared in this frame",
            )
            return
        self._next_symbol_id += 1
        self.semantic_symbols.append(symbol)
        self.semantic_labels.append(
            LabelDescriptor(
                label_id=label_id,
                name=label_token.value,
                ir_label=_label_ir_name(label_id, label_token.value),
                declaring_block_id=scope.block_id,
                symbol_id=symbol.symbol_id,
                statement_node_id=id(statement),
                line=label_token.line,
                column=label_token.column,
            )
        )

    def _check_statement(self, statement: ASTNode, scope: Scope) -> None:
        inner = _statement_body(statement)
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
        elif inner.rule_name == "proc_stmt":
            self._check_procedure_call(inner, scope, role="statement")
        elif inner.rule_name == "goto_stmt":
            self._check_goto(inner, scope)
        else:
            self._error(inner, f"{inner.rule_name} is not supported yet")

    def _check_goto(self, node: ASTNode, scope: Scope) -> None:
        desig_expr = _first_direct_node(node, "desig_expr")
        if desig_expr is None:
            self._error(node, "goto statement is missing a designational expression")
            return
        goto = self._check_designational(desig_expr, scope)
        if goto is not None:
            self.resolved_gotos.append(goto)

    def _check_designational(
        self,
        node: ASTNode,
        scope: Scope,
        *,
        allow_nonlocal_label: bool = True,
        allow_switch_selection: bool = True,
        active_switch_id: int | None = None,
    ) -> ResolvedGoto | None:
        if any(token.value == "if" for token in _direct_tokens(node)):
            bool_expr = _first_direct_node(node, "bool_expr")
            cond_type = (
                self._infer_expr(bool_expr, scope) if bool_expr is not None else ERROR
            )
            if cond_type != ERROR and cond_type != BOOLEAN:
                self._error(node, "conditional designational condition must be boolean")
            then_desig = _first_direct_node(node, "simple_desig")
            else_desig = _first_direct_node(node, "desig_expr")
            if then_desig is not None:
                self._check_simple_designational(
                    then_desig,
                    scope,
                    allow_nonlocal_label=allow_nonlocal_label,
                    allow_switch_selection=allow_switch_selection,
                    active_switch_id=active_switch_id,
                )
            if else_desig is not None:
                self._check_designational(
                    else_desig,
                    scope,
                    allow_nonlocal_label=allow_nonlocal_label,
                    allow_switch_selection=allow_switch_selection,
                    active_switch_id=active_switch_id,
                )
            return ResolvedGoto(
                token_id=id(node),
                label_id=-1,
                target_name="conditional designational expression",
                ir_label="",
                source_block_id=scope.block_id,
                target_block_id=scope.block_id,
                lexical_depth_delta=0,
                line=node.start_line or 1,
                column=node.start_column or 1,
                designational_node_id=id(node),
            )

        simple = _first_direct_node(node, "simple_desig")
        if simple is None:
            self._error(node, "unsupported designational expression")
            return None
        return self._check_simple_designational(
            simple,
            scope,
            outer_node=node,
            allow_nonlocal_label=allow_nonlocal_label,
            allow_switch_selection=allow_switch_selection,
            active_switch_id=active_switch_id,
        )

    def _check_simple_designational(
        self,
        node: ASTNode,
        scope: Scope,
        *,
        outer_node: ASTNode | None = None,
        allow_nonlocal_label: bool = True,
        allow_switch_selection: bool = True,
        active_switch_id: int | None = None,
    ) -> ResolvedGoto | None:
        target_token = _direct_label_from_simple_designational(node)
        if target_token is not None:
            return self._check_direct_label_designational(
                target_token,
                scope,
                outer_node=outer_node or node,
                allow_nonlocal_label=allow_nonlocal_label,
            )

        if any(token.value == "[" for token in _direct_tokens(node)):
            if not allow_switch_selection:
                self._error(
                    node,
                    "switch entries that select another switch require Phase 7b "
                    "dispatch lowering",
                )
                return None
            self._check_switch_selection(
                node,
                scope,
                active_switch_id=active_switch_id,
            )
            return ResolvedGoto(
                token_id=id(outer_node or node),
                label_id=-1,
                target_name="switch designational expression",
                ir_label="",
                source_block_id=scope.block_id,
                target_block_id=scope.block_id,
                lexical_depth_delta=0,
                line=node.start_line or 1,
                column=node.start_column or 1,
                designational_node_id=id(outer_node or node),
            )

        nested = _first_direct_node(node, "desig_expr")
        if nested is not None:
            return self._check_designational(
                nested,
                scope,
                allow_nonlocal_label=allow_nonlocal_label,
                allow_switch_selection=allow_switch_selection,
                active_switch_id=active_switch_id,
            )

        self._error(node, "unsupported designational expression")
        return None

    def _check_direct_label_designational(
        self,
        target_token: Token,
        scope: Scope,
        *,
        outer_node: ASTNode,
        allow_nonlocal_label: bool,
    ) -> ResolvedGoto | None:
        resolved = scope.resolve_with_scope(target_token.value)
        if resolved is None:
            self._error(target_token, f"label {target_token.value!r} is not declared")
            return

        symbol, declaring_scope, lexical_depth_delta = resolved
        if symbol.kind != LABEL:
            self._error(target_token, f"{target_token.value!r} is not a label")
            return
        if symbol.parameter_mode is not None and symbol.slot_offset is not None:
            return ResolvedGoto(
                token_id=id(target_token),
                label_id=-1,
                target_name=target_token.value,
                ir_label="",
                source_block_id=scope.block_id,
                target_block_id=declaring_scope.block_id,
                lexical_depth_delta=lexical_depth_delta,
                line=target_token.line,
                column=target_token.column,
                designational_node_id=id(outer_node),
            )
        if lexical_depth_delta != 0 and not allow_nonlocal_label:
            self._error(
                target_token,
                f"nonlocal designational label {target_token.value!r} requires "
                "a direct Phase 7b goto",
            )
            return

        label = next(
            (
                descriptor
                for descriptor in self.semantic_labels
                if descriptor.symbol_id == symbol.symbol_id
            ),
            None,
        )
        if label is None:
            self._error(target_token, f"label {target_token.value!r} has no descriptor")
            return
        return ResolvedGoto(
            token_id=id(target_token),
            label_id=label.label_id,
            target_name=target_token.value,
            ir_label=label.ir_label,
            source_block_id=scope.block_id,
            target_block_id=declaring_scope.block_id,
            lexical_depth_delta=lexical_depth_delta,
            line=target_token.line,
            column=target_token.column,
            designational_node_id=id(outer_node),
        )

    def _check_switch_selection(
        self,
        node: ASTNode,
        scope: Scope,
        *,
        active_switch_id: int | None = None,
    ) -> None:
        name_token = next(
            (token for token in _direct_tokens(node) if token.type_name == "NAME"),
            None,
        )
        if name_token is None:
            self._error(node, "switch selection is missing a name")
            return
        resolved = scope.resolve_with_scope(name_token.value)
        if resolved is None:
            self._error(name_token, f"switch {name_token.value!r} is not declared")
            return
        symbol, declaring_scope, lexical_depth_delta = resolved
        if symbol.kind != SWITCH:
            self._error(name_token, f"{name_token.value!r} is not a switch")
            return

        indexes = _direct_nodes(node, "arith_expr")
        if len(indexes) != 1:
            self._error(node, "switch selection requires exactly one index")
            return
        index_type = self._infer_expr(indexes[0], scope)
        if index_type != ERROR and index_type != INTEGER:
            self._error(indexes[0], "switch index must be integer")

        if symbol.parameter_mode is not None and symbol.slot_offset is not None:
            switch_id = -1
        elif symbol.switch_id is None:
            self._error(name_token, f"switch {name_token.value!r} has no descriptor")
            return
        else:
            switch_id = symbol.switch_id

        self.resolved_switch_selections.append(
            ResolvedSwitchSelection(
                node_id=id(node),
                token_id=id(name_token),
                name=name_token.value,
                switch_id=switch_id,
                use_block_id=scope.block_id,
                declaration_block_id=declaring_scope.block_id,
                lexical_depth_delta=lexical_depth_delta,
                index_node_id=id(indexes[0]),
                line=name_token.line,
                column=name_token.column,
            )
        )

    def _check_assignment(self, assign: ASTNode, scope: Scope) -> None:
        left_parts = _direct_nodes(assign, "left_part")
        if not left_parts:
            self._error(assign, "assignment needs at least one target")
            return

        expr = _first_direct_node(assign, "expression")
        value_type = self._infer_expr(expr, scope) if expr is not None else ERROR
        for left_part in left_parts:
            variable = _first_node(left_part, "variable")
            if variable is None:
                self._error(left_part, "assignment target must be a variable")
                continue

            name = _variable_head_name(variable)
            if name is None:
                self._error(left_part, "assignment target is missing a name")
                continue

            if _variable_subscripts(variable):
                access = self._check_array_access(variable, scope, role="write")
                if access is None:
                    target_type = ERROR
                else:
                    descriptor = next(
                        (
                            array
                            for array in self.semantic_arrays
                            if array.array_id == access.array_id
                        ),
                        None,
                    )
                    target_type = (
                        ERROR if descriptor is None else descriptor.element_type
                    )
            else:
                symbol = self._resolve_name(name, scope, role="write")
                if symbol is not None and symbol.kind == ARRAY:
                    self._error(name, f"array {name.value!r} requires subscripts")
                    target_type = ERROR
                elif symbol is not None and symbol.kind == LABEL:
                    self._error(name, f"label {name.value!r} is not assignable")
                    target_type = ERROR
                else:
                    target_type = ERROR if symbol is None else symbol.type_name

            if (
                target_type != ERROR
                and value_type != ERROR
                and not self._can_assign(target_type, value_type)
            ):
                self._error(
                    name,
                    f"cannot assign {value_type} to {target_type} variable "
                    f"{name.value!r}",
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
        loop_type = ERROR if symbol is None else symbol.type_name
        if loop_type != ERROR and loop_type not in {INTEGER, REAL}:
            self._error(loop_name, "for loop control variable must be integer or real")

        for elem in _direct_nodes(_first_direct_node(node, "for_list"), "for_elem"):
            arith_nodes = _direct_nodes(elem, "arith_expr")
            bool_node = _first_direct_node(elem, "bool_expr")
            if bool_node is not None:
                if len(arith_nodes) != 1:
                    self._error(
                        elem,
                        "for while-element must have one value expression",
                    )
                    continue
                start_type = self._infer_expr(arith_nodes[0], scope)
                if (
                    loop_type != ERROR
                    and start_type != ERROR
                    and not self._can_assign(loop_type, start_type)
                ):
                    self._error(
                        arith_nodes[0],
                        f"cannot assign {start_type} to {loop_type} for loop control "
                        f"variable {loop_name.value!r}",
                    )
                condition_type = self._infer_expr(bool_node, scope)
                if condition_type != ERROR and condition_type != BOOLEAN:
                    self._error(
                        bool_node,
                        "for while-element condition must be boolean",
                    )
                continue
            if len(arith_nodes) == 1:
                value_type = self._infer_expr(arith_nodes[0], scope)
                if (
                    loop_type != ERROR
                    and value_type != ERROR
                    and not self._can_assign(loop_type, value_type)
                ):
                    self._error(
                        arith_nodes[0],
                        f"cannot assign {value_type} to {loop_type} for loop control "
                        f"variable {loop_name.value!r}",
                    )
                continue
            if len(arith_nodes) != 3:
                self._error(elem, "unsupported for-element form")
                continue

            start_type = self._infer_expr(arith_nodes[0], scope)
            if (
                loop_type != ERROR
                and start_type != ERROR
                and not self._can_assign(loop_type, start_type)
            ):
                self._error(
                    arith_nodes[0],
                    f"cannot assign {start_type} to {loop_type} for loop control "
                    f"variable {loop_name.value!r}",
                )

            step_type = self._infer_expr(arith_nodes[1], scope)
            if step_type != ERROR and not _is_numeric_type(step_type):
                self._error(arith_nodes[1], "for loop step must be numeric")
            elif (
                loop_type != ERROR
                and step_type != ERROR
                and not self._can_assign(loop_type, step_type)
            ):
                self._error(
                    arith_nodes[1],
                    f"cannot assign {step_type} to {loop_type} for loop control "
                    f"variable {loop_name.value!r}",
                )

            limit_type = self._infer_expr(arith_nodes[2], scope)
            if limit_type != ERROR and not _is_numeric_type(limit_type):
                self._error(arith_nodes[2], "for loop limit must be numeric")

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
            if _variable_subscripts(expr):
                access = self._check_array_access(expr, scope, role="read")
                if access is None:
                    inferred = ERROR
                else:
                    descriptor = next(
                        (
                            array
                            for array in self.semantic_arrays
                            if array.array_id == access.array_id
                        ),
                        None,
                    )
                    inferred = ERROR if descriptor is None else descriptor.element_type
            else:
                name = _variable_head_name(expr)
                if name is None:
                    self._error(expr, "variable is missing a name")
                    inferred = ERROR
                    self.expression_types[id(expr)] = inferred
                    return inferred
                call = self._check_bare_procedure_expression(name, scope)
                if call is not None:
                    inferred = call.return_type or ERROR
                    self.expression_types[id(expr)] = inferred
                    return inferred
                symbol = self._resolve_name(name, scope, role="read")
                if symbol is not None and symbol.kind == ARRAY:
                    self._error(name, f"array {name.value!r} requires subscripts")
                    inferred = ERROR
                else:
                    inferred = ERROR if symbol is None else symbol.type_name
            self.expression_types[id(expr)] = inferred
            return inferred

        if expr.rule_name == "proc_call":
            call = self._check_procedure_call(expr, scope, role="expression")
            inferred = ERROR if call is None else call.return_type or ERROR
            self.expression_types[id(expr)] = inferred
            return inferred

        inferred = self._infer_ast_expr(expr, scope)
        self.expression_types[id(expr)] = inferred
        return inferred

    def _infer_ast_expr(self, expr: ASTNode, scope: Scope) -> str:
        meaningful = _meaningful_children(expr)
        if not meaningful:
            return ERROR

        conditional = _conditional_expression_parts(meaningful)
        if conditional is not None:
            condition, then_expr, else_expr = conditional
            return self._infer_conditional_expression(
                expr,
                condition,
                then_expr,
                else_expr,
                scope,
            )

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
                return self._require_numeric_unary(expr, meaningful[1], scope)

        if expr.rule_name in {
            "expr_eqv",
            "expr_impl",
            "expr_or",
            "expr_and",
            "simple_bool",
            "implication",
            "bool_factor",
            "bool_term",
        }:
            return self._fold_binary(expr, meaningful, scope, BOOLEAN, BOOLEAN)

        if expr.rule_name in {"expr_cmp", "relation"}:
            if any(
                isinstance(child, Token)
                and child.value in {"=", "!=", "<", "<=", ">", ">="}
                for child in meaningful
            ):
                return self._infer_numeric_comparison(expr, meaningful, scope)
            return self._infer_expr(meaningful[0], scope)

        if expr.rule_name in {"expr_add", "simple_arith", "expr_mul", "term"}:
            return self._infer_numeric_chain(expr, meaningful, scope)

        if expr.rule_name in {"expr_pow", "factor"}:
            if any(
                isinstance(child, Token) and child.value in {"**", "^"}
                for child in meaningful
            ):
                return self._infer_power_chain(expr, meaningful, scope)
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

    def _infer_conditional_expression(
        self,
        node: ASTNode,
        condition: ASTNode,
        then_expr: ASTNode,
        else_expr: ASTNode,
        scope: Scope,
    ) -> str:
        condition_type = self._infer_expr(condition, scope)
        then_type = self._infer_expr(then_expr, scope)
        else_type = self._infer_expr(else_expr, scope)
        if condition_type != ERROR and condition_type != BOOLEAN:
            self._error(
                node,
                "conditional expression requires boolean condition, got "
                f"{condition_type}",
            )
            return ERROR
        if then_type == ERROR or else_type == ERROR:
            return ERROR
        result_type = _conditional_branch_type(then_type, else_type)
        if result_type is None:
            self._error(
                node,
                "conditional expression branches must have compatible types",
            )
            return ERROR
        return result_type

    def _infer_token(self, token: Token, scope: Scope) -> str:
        if token.type_name == "INTEGER_LIT":
            return INTEGER
        if token.type_name == "REAL_LIT":
            return REAL
        if token.type_name == "STRING_LIT":
            return STRING
        if token.value in {"true", "false"}:
            return BOOLEAN
        if token.type_name == "NAME":
            symbol = self._resolve_name(token, scope, role="read")
            if symbol is None:
                return ERROR
            if symbol.kind == ARRAY:
                self._error(token, f"array {token.value!r} requires subscripts")
                return ERROR
            return symbol.type_name
        self._error(token, f"unsupported expression token {token.value!r}")
        return ERROR

    def _check_array_access(
        self,
        variable: ASTNode,
        scope: Scope,
        *,
        role: str,
        allow_whole: bool = False,
    ) -> ResolvedArrayAccess | None:
        name_token = _variable_head_name(variable)
        if name_token is None:
            self._error(variable, "array access is missing a name")
            return None

        resolved = scope.resolve_with_scope(name_token.value)
        if resolved is None:
            self._error(
                name_token,
                f"{name_token.value!r} is not declared in block {scope.block_id} "
                "or its lexical parents",
            )
            return None

        symbol, declaring_scope, lexical_depth_delta = resolved
        if symbol.kind != ARRAY:
            self._error(
                name_token,
                f"scalar variable {name_token.value!r} is not an array",
            )
            return None
        if symbol.slot_offset is None or symbol.array_id is None:
            self._error(
                name_token,
                f"array {name_token.value!r} has no descriptor slot",
            )
            return None

        descriptor = next(
            (
                array
                for array in self.semantic_arrays
                if array.array_id == symbol.array_id
            ),
            None,
        )
        if descriptor is None:
            self._error(name_token, f"array {name_token.value!r} has no descriptor")
            return None

        subscripts = _variable_subscripts(variable)
        if (
            descriptor.storage_class != PARAMETER_STORAGE
            and not allow_whole
            and len(subscripts) != len(descriptor.dimensions)
        ):
            self._error(
                name_token,
                f"array {name_token.value!r} expects "
                f"{len(descriptor.dimensions)} subscript(s), got {len(subscripts)}",
            )
        for subscript in subscripts:
            subscript_type = self._infer_expr(subscript, scope)
            if subscript_type != ERROR and subscript_type != INTEGER:
                self._error(subscript, "array subscripts must be integer")

        access = ResolvedArrayAccess(
            token_id=id(name_token),
            name=name_token.value,
            role=role,
            array_id=symbol.array_id,
            use_block_id=scope.block_id,
            declaration_block_id=declaring_scope.block_id,
            lexical_depth_delta=lexical_depth_delta,
            slot_offset=symbol.slot_offset,
            subscript_count=len(subscripts),
            line=name_token.line,
            column=name_token.column,
        )
        self.resolved_array_accesses.append(access)
        return access

    def _check_procedure_call(
        self,
        node: ASTNode,
        scope: Scope,
        *,
        role: str,
    ) -> ResolvedProcedureCall | None:
        name_token = next(
            (token for token in _direct_tokens(node) if token.type_name == "NAME"),
            None,
        )
        if name_token is None:
            self._error(node, "procedure call is missing a name")
            return None

        resolved = self._resolve_procedure(name_token, scope)
        if resolved is None:
            resolved = self._resolve_builtin_procedure(name_token, scope)
        if resolved is None:
            self._error(
                name_token,
                f"{name_token.value!r} is not a procedure visible from block "
                f"{scope.block_id}",
            )
            return None

        descriptor, declaring_scope, lexical_depth_delta = resolved
        arguments = _direct_nodes(
            _first_direct_node(node, "actual_params"),
            "expression",
        )
        return self._check_resolved_procedure_call(
            name_token,
            descriptor,
            declaring_scope,
            lexical_depth_delta,
            arguments,
            scope,
            role=role,
        )

    def _check_bare_procedure_expression(
        self,
        name_token: Token,
        scope: Scope,
    ) -> ResolvedProcedureCall | None:
        nearest = scope.resolve_with_scope(name_token.value)
        if nearest is not None and nearest[0].kind == "procedure_result":
            return None

        resolved = self._resolve_procedure(name_token, scope)
        if resolved is None:
            return None

        descriptor, declaring_scope, lexical_depth_delta = resolved
        return self._check_resolved_procedure_call(
            name_token,
            descriptor,
            declaring_scope,
            lexical_depth_delta,
            [],
            scope,
            role="expression",
        )

    def _check_resolved_procedure_call(
        self,
        name_token: Token,
        descriptor: ProcedureDescriptor,
        declaring_scope: Scope,
        lexical_depth_delta: int,
        arguments: list[ASTNode],
        scope: Scope,
        *,
        role: str,
    ) -> ResolvedProcedureCall | None:
        if descriptor.parameter_symbol_id is not None:
            return self._check_formal_procedure_call(
                name_token,
                descriptor,
                declaring_scope,
                lexical_depth_delta,
                arguments,
                scope,
                role=role,
            )
        if len(arguments) != len(descriptor.parameters):
            self._error(
                name_token,
                f"procedure {name_token.value!r} expects "
                f"{len(descriptor.parameters)} argument(s), got {len(arguments)}",
            )
        if descriptor.procedure_id == -1:
            return_type = self._check_builtin_procedure_call(
                name_token,
                arguments,
                scope,
                role=role,
            )
            if role == "expression" and return_type is None:
                self._error(
                    name_token,
                    f"procedure {name_token.value!r} does not return a value",
                )
            call = ResolvedProcedureCall(
                token_id=id(name_token),
                name=name_token.value,
                role=role,
                procedure_id=descriptor.procedure_id,
                label=descriptor.label,
                use_block_id=scope.block_id,
                declaration_block_id=declaring_scope.block_id,
                lexical_depth_delta=lexical_depth_delta,
                argument_count=len(arguments),
                return_type=return_type,
                line=name_token.line,
                column=name_token.column,
                parameter_symbol_id=descriptor.parameter_symbol_id,
            )
            self.resolved_procedure_calls.append(call)
            return call

        for argument, parameter in zip(arguments, descriptor.parameters, strict=False):
            if parameter.kind == ARRAY:
                self._check_array_parameter_actual(argument, scope, parameter)
                continue
            if parameter.kind == LABEL:
                self._check_label_parameter_actual(argument, scope, parameter)
                continue
            if parameter.kind == SWITCH:
                self._check_switch_parameter_actual(argument, scope, parameter)
                continue
            if parameter.kind == "procedure":
                self._check_procedure_parameter_actual(argument, scope, parameter)
                continue
            actual_type = self._infer_expr(argument, scope)
            if actual_type != ERROR and not self._parameter_accepts_type(
                parameter.mode,
                parameter.type_name,
                actual_type,
            ):
                if parameter.mode == NAME:
                    self._error(
                        argument,
                        f"by-name parameter {parameter.name!r} expects "
                        f"{parameter.type_name}, got {actual_type}",
                    )
                    continue
                self._error(
                    argument,
                    f"parameter {parameter.name!r} expects {parameter.type_name}, "
                    f"got {actual_type}",
                )
                continue
            if (
                parameter.mode == NAME
                and parameter.may_write
                and (
                    not _is_assignable_actual(argument)
                    or self._resolved_bare_procedure_expression(argument) is not None
                )
            ):
                self._error(
                    argument,
                    f"by-name parameter {parameter.name!r} is assigned, but actual "
                    "expression is not assignable",
                )
                continue
            if parameter.mode == NAME and parameter.may_write:
                self._record_assignable_actual_write(argument, scope)

        if role == "expression" and descriptor.return_type is None:
            self._error(
                name_token,
                f"procedure {name_token.value!r} does not return a value",
            )
        call = ResolvedProcedureCall(
            token_id=id(name_token),
            name=name_token.value,
            role=role,
            procedure_id=descriptor.procedure_id,
            label=descriptor.label,
            use_block_id=scope.block_id,
            declaration_block_id=declaring_scope.block_id,
            lexical_depth_delta=lexical_depth_delta,
            argument_count=len(arguments),
            return_type=descriptor.return_type,
            line=name_token.line,
            column=name_token.column,
            parameter_symbol_id=descriptor.parameter_symbol_id,
        )
        self.resolved_procedure_calls.append(call)
        return call

    def _check_builtin_procedure_call(
        self,
        name_token: Token,
        arguments: list[ASTNode],
        scope: Scope,
        *,
        role: str,
    ) -> str | None:
        if not arguments:
            return None
        actual_type = self._infer_expr(arguments[0], scope)
        if name_token.value in _OUTPUT_BUILTINS:
            if actual_type != ERROR and actual_type not in {
                INTEGER,
                BOOLEAN,
                REAL,
                STRING,
            }:
                self._error(
                    arguments[0],
                    f"builtin procedure {name_token.value!r} expects integer, "
                    f"boolean, real, or string, got {actual_type}",
                )
            return None
        if actual_type != ERROR and not _is_numeric_type(actual_type):
            self._error(
                arguments[0],
                f"builtin function {name_token.value!r} expects integer or real, "
                f"got {actual_type}",
            )
            return ERROR
        if name_token.value == "abs":
            return actual_type
        if name_token.value in _FIXED_RETURN_NUMERIC_BUILTINS:
            return _FIXED_RETURN_NUMERIC_BUILTINS[name_token.value]
        return ERROR

    def _resolved_bare_procedure_expression(
        self,
        argument: ASTNode,
    ) -> ResolvedProcedureCall | None:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            return None
        name = _variable_head_name(variable)
        if name is None:
            return None
        return next(
            (
                call
                for call in self.resolved_procedure_calls
                if call.token_id == id(name) and call.role == "expression"
            ),
            None,
        )

    def _check_formal_procedure_call(
        self,
        name_token: Token,
        descriptor: ProcedureDescriptor,
        declaring_scope: Scope,
        lexical_depth_delta: int,
        arguments: list[ASTNode],
        scope: Scope,
        *,
        role: str,
    ) -> ResolvedProcedureCall:
        argument_types: list[str] = []
        argument_kinds: list[str] = []
        argument_assignable: list[bool] = []
        procedure_argument_ids: list[int | None] = []
        for argument in arguments:
            array_type = self._formal_array_argument_type(argument, scope)
            if array_type is not None:
                argument_types.append(array_type)
                argument_kinds.append(ARRAY)
                argument_assignable.append(False)
                procedure_argument_ids.append(None)
                continue
            nonscalar = self._formal_nonscalar_argument_shape(argument, scope)
            if nonscalar is not None:
                argument_kind, argument_type, procedure_id = nonscalar
                argument_types.append(argument_type)
                argument_kinds.append(argument_kind)
                argument_assignable.append(False)
                procedure_argument_ids.append(procedure_id)
                continue
            actual_type = self._infer_expr(argument, scope)
            argument_types.append(actual_type)
            argument_kinds.append("scalar")
            argument_assignable.append(
                _is_assignable_actual(argument)
                and self._resolved_bare_procedure_expression(argument) is None
            )
            procedure_argument_ids.append(None)
            if actual_type != ERROR and actual_type not in {
                INTEGER,
                BOOLEAN,
                REAL,
                STRING,
            }:
                self._error(
                    argument,
                    f"procedure parameter {name_token.value!r} argument must be "
                    f"integer, boolean, real, or string, got {actual_type}",
                )
        return_type = descriptor.return_type
        if role == "expression" and return_type is None:
            self._error(
                name_token,
                f"procedure parameter {name_token.value!r} does not return a value",
            )
        elif ERROR not in argument_types and descriptor.parameter_symbol_id is not None:
            self._record_procedure_parameter_call_shape(
                descriptor.parameter_symbol_id,
                ProcedureFormalCallShape(
                    role=role,
                    argument_types=tuple(argument_types),
                    argument_kinds=tuple(argument_kinds),
                    argument_assignable=tuple(argument_assignable),
                    procedure_argument_ids=tuple(procedure_argument_ids),
                    return_type=return_type if role == "expression" else None,
                ),
            )
        call = ResolvedProcedureCall(
            token_id=id(name_token),
            name=name_token.value,
            role=role,
            procedure_id=descriptor.procedure_id,
            label=descriptor.label,
            use_block_id=scope.block_id,
            declaration_block_id=declaring_scope.block_id,
            lexical_depth_delta=lexical_depth_delta,
            argument_count=len(arguments),
            return_type=return_type,
            line=name_token.line,
            column=name_token.column,
            parameter_symbol_id=descriptor.parameter_symbol_id,
        )
        self.resolved_procedure_calls.append(call)
        return call

    def _formal_array_argument_type(
        self,
        argument: ASTNode,
        scope: Scope,
    ) -> str | None:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            return None
        name = _variable_head_name(variable)
        if name is None:
            return None
        resolved = scope.resolve_with_scope(name.value)
        if resolved is None or resolved[0].kind != ARRAY:
            return None
        access = self._check_array_access(
            variable,
            scope,
            role="actual",
            allow_whole=True,
        )
        if access is None:
            return ERROR
        descriptor = next(
            (
                array
                for array in self.semantic_arrays
                if array.array_id == access.array_id
            ),
            None,
        )
        return ERROR if descriptor is None else descriptor.element_type

    def _formal_nonscalar_argument_shape(
        self,
        argument: ASTNode,
        scope: Scope,
    ) -> tuple[str, str, int | None] | None:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            return None
        name = _variable_head_name(variable)
        if name is None:
            return None
        resolved = scope.resolve_with_scope(name.value)
        if resolved is None:
            return None
        symbol, _, _ = resolved
        if symbol.kind == LABEL:
            return LABEL, LABEL, None
        if symbol.kind == SWITCH:
            return SWITCH, SWITCH, None
        if symbol.kind == "procedure":
            descriptor = next(
                (
                    procedure
                    for procedure in self.semantic_procedures
                    if procedure.procedure_id == symbol.procedure_id
                ),
                None,
            )
            if (
                descriptor is not None
                and descriptor.return_type is not None
                and not descriptor.parameters
            ):
                return None
            return "procedure", symbol.type_name, symbol.procedure_id
        if symbol.kind == "procedure_parameter" and symbol.type_name == "procedure":
            return "procedure", symbol.type_name, None
        return None

    def _record_procedure_parameter_call_shape(
        self,
        symbol_id: int,
        shape: ProcedureFormalCallShape,
    ) -> None:
        shapes = self.procedure_parameter_call_shapes.setdefault(symbol_id, [])
        if shape not in shapes:
            shapes.append(shape)

    def _check_array_parameter_actual(
        self,
        argument: ASTNode,
        scope: Scope,
        parameter: ProcedureParameter,
    ) -> ResolvedArrayAccess | None:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            self._error(
                argument,
                f"array parameter {parameter.name!r} expects a whole-array actual",
            )
            return None
        access = self._check_array_access(
            variable,
            scope,
            role="actual",
            allow_whole=True,
        )
        if access is None:
            return None
        descriptor = next(
            (
                array
                for array in self.semantic_arrays
                if array.array_id == access.array_id
            ),
            None,
        )
        if (
            descriptor is not None
            and descriptor.element_type != ERROR
            and descriptor.element_type != parameter.type_name
        ):
            self._error(
                argument,
                f"array parameter {parameter.name!r} expects "
                f"{parameter.type_name} elements, got {descriptor.element_type}",
            )
        return access

    def _check_label_parameter_actual(
        self,
        argument: ASTNode,
        scope: Scope,
        parameter: ProcedureParameter,
    ) -> Symbol | None:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            self._error(
                argument,
                f"label parameter {parameter.name!r} expects a direct label actual",
            )
            return None
        name = _variable_head_name(variable)
        if name is None:
            self._error(argument, "label actual is missing a name")
            return None
        resolved = scope.resolve_with_scope(name.value)
        if resolved is None:
            self._error(
                name,
                f"{name.value!r} is not declared in block {scope.block_id} "
                "or its lexical parents",
            )
            return None
        symbol, _, _ = resolved
        if symbol.kind != LABEL:
            self._error(
                name,
                f"label parameter {parameter.name!r} expects a label actual",
            )
            return None
        return symbol

    def _check_switch_parameter_actual(
        self,
        argument: ASTNode,
        scope: Scope,
        parameter: ProcedureParameter,
    ) -> Symbol | None:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            self._error(
                argument,
                f"switch parameter {parameter.name!r} expects a direct switch actual",
            )
            return None
        name = _variable_head_name(variable)
        if name is None:
            self._error(argument, "switch actual is missing a name")
            return None
        resolved = scope.resolve_with_scope(name.value)
        if resolved is None:
            self._error(
                name,
                f"{name.value!r} is not declared in block {scope.block_id} "
                "or its lexical parents",
            )
            return None
        symbol, _, _ = resolved
        if symbol.kind != SWITCH:
            self._error(
                name,
                f"switch parameter {parameter.name!r} expects a switch actual",
            )
            return None
        return symbol

    def _check_procedure_parameter_actual(
        self,
        argument: ASTNode,
        scope: Scope,
        parameter: ProcedureParameter,
    ) -> Symbol | None:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            self._error(
                argument,
                f"procedure parameter {parameter.name!r} expects a direct "
                "procedure actual",
            )
            return None
        name = _variable_head_name(variable)
        if name is None:
            self._error(argument, "procedure actual is missing a name")
            return None
        resolved = scope.resolve_with_scope(name.value)
        if resolved is None:
            self._error(
                name,
                f"{name.value!r} is not declared in block {scope.block_id} "
                "or its lexical parents",
            )
            return None
        symbol, _, _ = resolved
        if symbol.kind == "procedure_parameter":
            for shape in parameter.procedure_call_shapes:
                self._record_procedure_parameter_call_shape(symbol.symbol_id, shape)
            return symbol
        if symbol.kind != "procedure" or symbol.procedure_id is None:
            self._error(
                name,
                f"procedure parameter {parameter.name!r} expects a procedure actual",
            )
            return None
        descriptor = next(
            (
                procedure
                for procedure in self.semantic_procedures
                if procedure.procedure_id == symbol.procedure_id
            ),
            None,
        )
        if descriptor is None:
            self._error(name, f"procedure {name.value!r} has no descriptor")
            return None
        if not self._procedure_actual_satisfies_formal_shapes(
            name,
            descriptor,
            parameter,
        ):
            return None
        return symbol

    def _procedure_actual_satisfies_formal_shapes(
        self,
        token: Token,
        descriptor: ProcedureDescriptor,
        parameter: ProcedureParameter,
    ) -> bool:
        for shape in parameter.procedure_call_shapes:
            if shape.role == "expression":
                if shape.return_type is None:
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} has no typed "
                        "expression result",
                    )
                    return False
                if descriptor.return_type is None:
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} expects a "
                        f"{shape.return_type} procedure actual",
                    )
                    return False
                if not self._procedure_return_type_satisfies(
                    expected_type=shape.return_type,
                    actual_type=descriptor.return_type,
                ):
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} expects a "
                        f"{shape.return_type} procedure actual, got "
                        f"{descriptor.return_type}",
                    )
                    return False
            elif shape.role == "statement":
                if descriptor.return_type is not None:
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} expects a "
                        "statement procedure actual",
                    )
                    return False
            else:
                self._error(
                    token,
                    f"procedure parameter {parameter.name!r} has unsupported "
                    f"{shape.role} call shape",
                )
                return False
            if len(descriptor.parameters) != len(shape.argument_types):
                expected = len(shape.argument_types)
                actual = len(descriptor.parameters)
                if expected == 0:
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} expects a "
                        "no-argument statement procedure actual",
                    )
                else:
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} expects a "
                        f"statement procedure actual accepting {expected} "
                        f"argument(s), got {actual}",
                    )
                return False
            for argument_index, (actual_parameter, actual_type) in enumerate(
                zip(
                    descriptor.parameters,
                    shape.argument_types,
                    strict=True,
                )
            ):
                argument_kind = (
                    shape.argument_kinds[argument_index]
                    if argument_index < len(shape.argument_kinds)
                    else "scalar"
                )
                argument_assignable = (
                    shape.argument_assignable[argument_index]
                    if argument_index < len(shape.argument_assignable)
                    else False
                )
                procedure_argument_id = (
                    shape.procedure_argument_ids[argument_index]
                    if argument_index < len(shape.procedure_argument_ids)
                    else None
                )
                if argument_kind == ARRAY:
                    if actual_parameter.kind != ARRAY:
                        self._error(
                            token,
                            f"procedure parameter {parameter.name!r} passes an "
                            "array to a scalar actual parameter "
                            f"{actual_parameter.name!r}",
                        )
                        return False
                    if actual_parameter.type_name != actual_type:
                        self._error(
                            token,
                            f"procedure parameter {parameter.name!r} passes "
                            f"{actual_type} array to actual parameter "
                            f"{actual_parameter.name!r} expecting "
                            f"{actual_parameter.type_name}",
                        )
                        return False
                    continue
                if argument_kind == LABEL:
                    if actual_parameter.kind != LABEL:
                        self._error(
                            token,
                            f"procedure parameter {parameter.name!r} passes a "
                            "label to non-label actual parameter "
                            f"{actual_parameter.name!r}",
                        )
                        return False
                    continue
                if argument_kind == SWITCH:
                    if actual_parameter.kind != SWITCH:
                        self._error(
                            token,
                            f"procedure parameter {parameter.name!r} passes a "
                            "switch to non-switch actual parameter "
                            f"{actual_parameter.name!r}",
                        )
                        return False
                    continue
                if argument_kind == "procedure":
                    if actual_parameter.kind != "procedure":
                        self._error(
                            token,
                            f"procedure parameter {parameter.name!r} passes a "
                            "procedure to non-procedure actual parameter "
                            f"{actual_parameter.name!r}",
                        )
                        return False
                    if procedure_argument_id is not None:
                        descriptor = self._procedure_descriptor_for_id(
                            procedure_argument_id
                        )
                        if descriptor is None:
                            self._error(
                                token,
                                f"procedure parameter {parameter.name!r} passes a "
                                "procedure actual with no descriptor",
                            )
                            return False
                        if not self._procedure_actual_satisfies_formal_shapes(
                            token,
                            descriptor,
                            actual_parameter,
                        ):
                            return False
                    if not self._procedure_parameter_type_satisfies(
                        expected_type=actual_parameter.type_name,
                        actual_type=actual_type,
                    ):
                        self._error(
                            token,
                            f"procedure parameter {parameter.name!r} passes "
                            f"{actual_type} procedure to actual parameter "
                            f"{actual_parameter.name!r} expecting "
                            f"{actual_parameter.type_name}",
                        )
                        return False
                    continue
                if argument_kind != "scalar":
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} has unsupported "
                        f"{argument_kind} argument shape",
                    )
                    return False
                if actual_parameter.kind != "scalar":
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} expects a "
                        "procedure actual with scalar parameters",
                    )
                    return False
                if actual_parameter.mode not in {VALUE, NAME}:
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} expects a "
                        "procedure actual with scalar value or name parameters",
                    )
                    return False
                if not self._parameter_accepts_type(
                    actual_parameter.mode,
                    actual_parameter.type_name,
                    actual_type,
                ):
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} passes "
                        f"{actual_type} to actual parameter "
                        f"{actual_parameter.name!r} expecting "
                        f"{actual_parameter.type_name}",
                    )
                    return False
                if actual_parameter.mode == NAME and (
                    actual_parameter.may_write and not argument_assignable
                ):
                    self._error(
                        token,
                        f"procedure parameter {parameter.name!r} passes a "
                        f"non-assignable actual to written by-name parameter "
                        f"{actual_parameter.name!r}",
                    )
                    return False
                continue
        return True

    def _procedure_descriptor_for_id(
        self,
        procedure_id: int,
    ) -> ProcedureDescriptor | None:
        return next(
            (
                procedure
                for procedure in self.semantic_procedures
                if procedure.procedure_id == procedure_id
            ),
            None,
        )

    def _procedure_parameter_type_satisfies(
        self,
        *,
        expected_type: str,
        actual_type: str,
    ) -> bool:
        if expected_type == "procedure" or actual_type == "procedure":
            return expected_type == actual_type
        return self._procedure_return_type_satisfies(
            expected_type=expected_type,
            actual_type=actual_type,
        )

    def _procedure_return_type_satisfies(
        self,
        *,
        expected_type: str,
        actual_type: str,
    ) -> bool:
        if expected_type == REAL and actual_type == INTEGER:
            return True
        return expected_type == actual_type

    def _resolve_builtin_procedure(
        self,
        token: Token,
        scope: Scope,
    ) -> tuple[ProcedureDescriptor, Scope, int] | None:
        if token.value not in _READ_ONLY_BUILTINS:
            return None
        return_type = None
        if token.value == "abs":
            return_type = INTEGER
        elif token.value in _FIXED_RETURN_NUMERIC_BUILTINS:
            return_type = _FIXED_RETURN_NUMERIC_BUILTINS[token.value]
        descriptor = ProcedureDescriptor(
            procedure_id=-1,
            name=token.value,
            label=f"__algol_builtin_{token.value}",
            declaring_block_id=scope.block_id,
            body_block_id=scope.block_id,
            body_node_id=-1,
            return_type=return_type,
            parameters=(
                ProcedureParameter(
                    name="value",
                    type_name=STRING,
                    mode=VALUE,
                    symbol_id=-1,
                    slot_offset=0,
                ),
            ),
            line=token.line,
            column=token.column,
        )
        return descriptor, scope, 0

    def _record_assignable_actual_write(self, argument: ASTNode, scope: Scope) -> None:
        variable = _single_variable_expr(argument)
        if variable is None:
            return
        if _variable_subscripts(variable):
            self._check_array_access(variable, scope, role="write")
            return
        name = _variable_head_name(variable)
        if name is not None:
            self._resolve_name(name, scope, role="write")

    def _resolve_procedure(
        self, token: Token, scope: Scope
    ) -> tuple[ProcedureDescriptor, Scope, int] | None:
        current: Scope | None = scope
        lexical_depth_delta = 0
        while current is not None:
            symbol = current.symbols.get(token.value)
            if symbol is not None:
                if symbol.kind == "procedure":
                    descriptor = next(
                        (
                            procedure
                            for procedure in self.semantic_procedures
                            if procedure.procedure_id == symbol.procedure_id
                        ),
                        None,
                    )
                    if descriptor is None:
                        return None
                    return descriptor, current, lexical_depth_delta
                if symbol.kind == "procedure_parameter":
                    return_type = (
                        symbol.type_name
                        if symbol.type_name in {INTEGER, BOOLEAN, REAL, STRING}
                        else None
                    )
                    return (
                        ProcedureDescriptor(
                            procedure_id=-2,
                            name=token.value,
                            label="__algol_dynamic_procedure_parameter",
                            declaring_block_id=current.block_id,
                            body_block_id=current.block_id,
                            body_node_id=-1,
                            return_type=return_type,
                            parameters=tuple(),
                            line=token.line,
                            column=token.column,
                            parameter_symbol_id=symbol.symbol_id,
                        ),
                        current,
                        lexical_depth_delta,
                    )
                if symbol.kind == "procedure_result":
                    current = current.parent
                    lexical_depth_delta += 1
                    continue
                return None
            current = current.parent
            lexical_depth_delta += 1
        return None

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
                storage_class=symbol.storage_class,
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

    def _require_numeric_unary(
        self,
        node: ASTNode,
        operand: ASTNode | Token,
        scope: Scope,
    ) -> str:
        actual = self._infer_expr(operand, scope)
        if actual == ERROR:
            return ERROR
        if not _is_numeric_type(actual):
            self._error(node, f"operator requires numeric operand, got {actual}")
            return ERROR
        return actual

    def _infer_numeric_chain(
        self,
        node: ASTNode,
        children: list[ASTNode | Token],
        scope: Scope,
    ) -> str:
        current_type = self._infer_expr(children[0], scope)
        if current_type == ERROR:
            return ERROR
        if not _is_numeric_type(current_type):
            self._error(node, f"operator requires numeric operand, got {current_type}")
            return ERROR

        saw_operator = False
        index = 1
        while index < len(children):
            operator = children[index]
            right = children[index + 1]
            right_type = self._infer_expr(right, scope)
            if right_type == ERROR:
                return ERROR
            if not isinstance(operator, Token):
                self._error(node, "expected numeric operator")
                return ERROR
            if not _is_numeric_type(right_type):
                self._error(
                    node,
                    f"operator requires numeric operand, got {right_type}",
                )
                return ERROR
            saw_operator = True
            if operator.value in {"div", "mod"}:
                if current_type != INTEGER or right_type != INTEGER:
                    self._error(
                        node,
                        f"operator {operator.value!r} requires integer operands",
                    )
                    return ERROR
                current_type = INTEGER
            elif operator.value == "/":
                current_type = REAL
            elif operator.value in {"+", "-", "*"}:
                current_type = (
                    REAL if REAL in {current_type, right_type} else INTEGER
                )
            else:
                self._error(
                    node,
                    f"numeric operator {operator.value!r} is not supported",
                )
                return ERROR
            index += 2
        return current_type if saw_operator else self._infer_expr(children[0], scope)

    def _infer_power_chain(
        self,
        node: ASTNode,
        children: list[ASTNode | Token],
        scope: Scope,
    ) -> str:
        current_type = self._infer_expr(children[0], scope)
        if current_type == ERROR:
            return ERROR
        if not _is_numeric_type(current_type):
            self._error(node, f"operator requires numeric operand, got {current_type}")
            return ERROR

        index = 1
        while index < len(children):
            operator = children[index]
            exponent = children[index + 1]
            exponent_type = self._infer_expr(exponent, scope)
            if exponent_type == ERROR:
                return ERROR
            if not isinstance(operator, Token):
                self._error(node, "expected exponentiation operator")
                return ERROR
            if operator.value not in {"**", "^"}:
                self._error(
                    node,
                    f"numeric operator {operator.value!r} is not supported",
                )
                return ERROR
            if exponent_type != INTEGER:
                self._error(node, "exponentiation exponent must be integer")
                return ERROR
            current_type = REAL if current_type == REAL else INTEGER
            index += 2
        return current_type

    def _infer_numeric_comparison(
        self,
        node: ASTNode,
        children: list[ASTNode | Token],
        scope: Scope,
    ) -> str:
        left = self._infer_expr(children[0], scope)
        right = self._infer_expr(children[2], scope)
        if left == ERROR or right == ERROR:
            return ERROR
        if not _is_numeric_type(left):
            self._error(node, f"operator requires numeric operand, got {left}")
            return ERROR
        if not _is_numeric_type(right):
            self._error(node, f"operator requires numeric operand, got {right}")
            return ERROR
        return BOOLEAN

    def _can_assign(self, target_type: str, value_type: str) -> bool:
        if target_type == value_type:
            return True
        return target_type == REAL and value_type == INTEGER

    def _parameter_accepts_type(
        self,
        mode: str,
        expected_type: str,
        actual_type: str,
    ) -> bool:
        if expected_type == actual_type:
            return True
        return mode == VALUE and expected_type == REAL and actual_type == INTEGER

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


def _is_numeric_type(type_name: str) -> bool:
    return type_name in {INTEGER, REAL}


def _conditional_branch_type(then_type: str, else_type: str) -> str | None:
    if then_type == else_type:
        return then_type
    if _is_numeric_type(then_type) and _is_numeric_type(else_type):
        return REAL if REAL in {then_type, else_type} else INTEGER
    return None


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


def _statement_body(statement: ASTNode) -> ASTNode | None:
    return next(
        (
            child
            for child in statement.children
            if isinstance(child, ASTNode) and child.rule_name != "label"
        ),
        None,
    )


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


def _conditional_expression_parts(
    children: list[ASTNode | Token],
) -> tuple[ASTNode, ASTNode, ASTNode] | None:
    first = children[0] if children else None
    if not isinstance(first, Token) or first.value != "if":
        return None
    then_index = _keyword_child_index(children, "then")
    else_index = _keyword_child_index(children, "else")
    if then_index is None or else_index is None or then_index >= else_index:
        return None
    condition = _first_ast_child_between(children, 1, then_index)
    then_expr = _first_ast_child_between(children, then_index + 1, else_index)
    else_expr = _first_ast_child_between(children, else_index + 1, len(children))
    if condition is None or then_expr is None or else_expr is None:
        return None
    return condition, then_expr, else_expr


def _keyword_child_index(
    children: list[ASTNode | Token],
    keyword: str,
) -> int | None:
    for index, child in enumerate(children):
        if isinstance(child, Token) and child.value == keyword:
            return index
    return None


def _first_ast_child_between(
    children: list[ASTNode | Token],
    start: int,
    end: int,
) -> ASTNode | None:
    return next(
        (child for child in children[start:end] if isinstance(child, ASTNode)),
        None,
    )


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


def _procedure_name(node: ASTNode) -> Token | None:
    return next(
        (token for token in _direct_tokens(node) if token.type_name == "NAME"),
        None,
    )


def _procedure_return_type(node: ASTNode) -> str | None:
    type_node = _first_direct_node(node, "type")
    return _first_keyword_value(type_node) if type_node is not None else None


def _formal_parameter_names(node: ASTNode) -> list[Token]:
    formal_params = _first_direct_node(node, "formal_params")
    ident_list = _first_direct_node(formal_params, "ident_list")
    return [token for token in _tokens(ident_list) if token.type_name == "NAME"]


def _value_parameter_names(node: ASTNode) -> set[str]:
    value_part = _first_direct_node(node, "value_part")
    ident_list = _first_direct_node(value_part, "ident_list")
    return {
        token.value for token in _tokens(ident_list) if token.type_name == "NAME"
    }


def _parameter_specs(node: ASTNode) -> dict[str, _ParameterSpec]:
    specs: dict[str, _ParameterSpec] = {}
    for spec_part in _direct_nodes(node, "spec_part"):
        specifier = _first_direct_node(spec_part, "specifier")
        specifier_name = _first_keyword_value(specifier) or ""
        ident_list = _first_direct_node(spec_part, "ident_list")
        for token in _tokens(ident_list):
            if token.type_name == "NAME":
                current = specs.get(token.value, _ParameterSpec())
                if specifier_name in {INTEGER, BOOLEAN, REAL, STRING}:
                    specs[token.value] = _ParameterSpec(
                        kind=current.kind,
                        type_name=specifier_name,
                    )
                elif specifier_name in {ARRAY, LABEL, SWITCH, "procedure"}:
                    specs[token.value] = _ParameterSpec(
                        kind=specifier_name,
                        type_name=current.type_name,
                    )
    return specs


def _label_token(node: ASTNode) -> Token | None:
    return next(
        (
            token
            for token in _direct_tokens(node)
            if token.type_name in {"NAME", "INTEGER_LIT"}
        ),
        None,
    )


def _direct_label_from_simple_designational(node: ASTNode | None) -> Token | None:
    if node is None:
        return None
    if any(token.value == "[" for token in _direct_tokens(node)):
        return None
    label_node = _first_direct_node(node, "label")
    if label_node is None:
        nested = _first_direct_node(node, "desig_expr")
        if nested is None or any(
            token.value == "if" for token in _direct_tokens(nested)
        ):
            return None
        simple = _first_direct_node(nested, "simple_desig")
        return _direct_label_from_simple_designational(simple)
    return _label_token(label_node)

def _label_ir_name(label_id: int, source_name: str) -> str:
    safe_name = "".join(
        char if char.isalnum() or char == "_" else "_"
        for char in source_name
    )
    return f"algol_label_{label_id}_{safe_name}"


def _by_name_formal_write_reasons(
    body: ASTNode | None,
    formal_names: list[Token],
    value_names: set[str],
    known_procedures: dict[str, ProcedureDescriptor | None],
    current_procedure_name: str,
) -> dict[str, str]:
    ordered_names = [formal.value for formal in formal_names]
    ordered_modes = [
        VALUE if formal.value in value_names else NAME for formal in formal_names
    ]
    names = {
        formal.value for formal in formal_names if formal.value not in value_names
    }
    reasons: dict[str, str] = {}
    call_edges: list[tuple[str, str | None, int]] = []

    def record(name: str | None, reason: str, active_names: set[str]) -> None:
        if name in active_names:
            reasons.setdefault(name, reason)

    def visit(
        node: ASTNode | None,
        active_names: set[str],
        local_procedure_names: set[str],
    ) -> None:
        if node is None or not active_names:
            return
        if node.rule_name == "block":
            active_names = active_names - _direct_block_declared_names(node)
            local_procedure_names = (
                local_procedure_names | _direct_block_procedure_names(node)
            )
        elif node.rule_name == "procedure_decl":
            hidden = {formal.value for formal in _formal_parameter_names(node)}
            procedure_name = _procedure_name(node)
            if procedure_name is not None:
                hidden.add(procedure_name.value)
                local_procedure_names = local_procedure_names | {procedure_name.value}
            active_names = active_names - hidden

        if node.rule_name == "assign_stmt":
            for left_part in _direct_nodes(node, "left_part"):
                variable = _first_node(left_part, "variable")
                record(
                    _variable_head_name_value(variable),
                    "local assignment",
                    active_names,
                )
        elif node.rule_name == "for_stmt":
            loop_token = next(
                (token for token in _direct_tokens(node) if token.type_name == "NAME"),
                None,
            )
            loop_name = loop_token.value if loop_token is not None else None
            record(loop_name, "local assignment", active_names)
        elif node.rule_name in {"proc_call", "proc_stmt"}:
            callee_name = _procedure_call_name(node)
            if callee_name in local_procedure_names:
                callee_name = None
            actual_params = _first_direct_node(node, "actual_params")
            for index, actual in enumerate(_direct_nodes(actual_params, "expression")):
                actual_name = _single_variable_expr_name(actual)
                if actual_name in active_names:
                    call_edges.append((actual_name, callee_name, index))

        for child in _node_children(node):
            visit(child, active_names, local_procedure_names)

    visit(body, names, set())
    self_edges: list[tuple[str, int]] = []
    for actual_name, callee_name, argument_index in call_edges:
        if callee_name == current_procedure_name:
            self_edges.append((actual_name, argument_index))
        elif _callee_may_write_argument(callee_name, argument_index, known_procedures):
            reasons.setdefault(actual_name, "transitive call")

    changed = True
    while changed:
        changed = False
        for actual_name, argument_index in self_edges:
            if argument_index >= len(ordered_names):
                reasons.setdefault(actual_name, "transitive call")
                continue
            if ordered_modes[argument_index] != NAME:
                continue
            target_name = ordered_names[argument_index]
            if target_name in reasons and actual_name not in reasons:
                reasons[actual_name] = "transitive call"
                changed = True
    return reasons


def _unique_procedure_names(
    procedures: list[ProcedureDescriptor],
) -> dict[str, ProcedureDescriptor | None]:
    by_name: dict[str, ProcedureDescriptor | None] = {}
    for procedure in procedures:
        if procedure.name in by_name:
            by_name[procedure.name] = None
        else:
            by_name[procedure.name] = procedure
    return by_name


def _procedure_call_name(node: ASTNode) -> str | None:
    token = next(
        (token for token in _direct_tokens(node) if token.type_name == "NAME"),
        None,
    )
    return token.value if token is not None else None


def _callee_may_write_argument(
    callee_name: str | None,
    argument_index: int,
    known_procedures: dict[str, ProcedureDescriptor | None],
) -> bool:
    if callee_name is None:
        return True
    if callee_name in _READ_ONLY_BUILTINS:
        return False
    procedure = known_procedures.get(callee_name)
    if procedure is None or argument_index >= len(procedure.parameters):
        return True
    parameter = procedure.parameters[argument_index]
    return parameter.mode == NAME and parameter.may_write


def _direct_block_declared_names(node: ASTNode) -> set[str]:
    declared: set[str] = set()
    for declaration in _direct_nodes(node, "declaration"):
        inner = _first_ast_child(declaration)
        if inner is None:
            continue
        if inner.rule_name == "type_decl":
            ident_list = _first_node(inner, "ident_list")
            declared.update(
                token.value
                for token in _tokens(ident_list)
                if token.type_name == "NAME"
            )
        elif inner.rule_name == "array_decl":
            for segment in _direct_nodes(inner, "array_segment"):
                ident_list = _first_direct_node(segment, "ident_list")
                declared.update(
                    token.value
                    for token in _tokens(ident_list)
                    if token.type_name == "NAME"
                )
        elif inner.rule_name == "procedure_decl":
            procedure_name = _procedure_name(inner)
            if procedure_name is not None:
                declared.add(procedure_name.value)
    return declared


def _direct_block_procedure_names(node: ASTNode) -> set[str]:
    names: set[str] = set()
    for declaration in _direct_nodes(node, "declaration"):
        inner = _first_ast_child(declaration)
        if inner is None or inner.rule_name != "procedure_decl":
            continue
        procedure_name = _procedure_name(inner)
        if procedure_name is not None:
            names.add(procedure_name.value)
    return names


def _is_assignable_actual(node: ASTNode) -> bool:
    return _single_variable_expr(node) is not None


def _single_variable_expr_name(node: ASTNode | None) -> str | None:
    variable = _single_variable_expr(node)
    return _variable_head_name_value(variable)


def _single_variable_expr(node: ASTNode | None) -> ASTNode | None:
    if node is None:
        return None
    if node.rule_name == "variable":
        return node
    meaningful = _meaningful_children(node)
    if len(meaningful) != 1 or not isinstance(meaningful[0], ASTNode):
        return None
    return _single_variable_expr(meaningful[0])


def _variable_head_name_value(node: ASTNode | None) -> str | None:
    token = _variable_head_name(node)
    return token.value if token is not None else None


def _variable_name(node: ASTNode) -> Token | None:
    variable = node if node.rule_name == "variable" else _first_node(node, "variable")
    if variable is None:
        return None
    if _variable_subscripts(variable):
        return None
    names = [token for token in _tokens(variable) if token.type_name == "NAME"]
    return names[0] if len(names) == 1 else None


def _variable_head_name(node: ASTNode | None) -> Token | None:
    if node is None:
        variable = None
    elif node.rule_name == "variable":
        variable = node
    else:
        variable = _first_node(node, "variable")
    if variable is None:
        return None
    return next(
        (token for token in _direct_tokens(variable) if token.type_name == "NAME"),
        None,
    )


def _variable_subscripts(node: ASTNode | None) -> list[ASTNode]:
    if node is None:
        variable = None
    elif node.rule_name == "variable":
        variable = node
    else:
        variable = _first_node(node, "variable")
    subscripts = _first_direct_node(variable, "subscripts")
    return _direct_nodes(subscripts, "arith_expr")
