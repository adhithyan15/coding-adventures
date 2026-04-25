"""Lower the first ALGOL 60 compiler subset into compiler IR."""

from __future__ import annotations

from dataclasses import dataclass, field

from algol_type_checker import (
    ArrayDescriptor,
    LabelDescriptor,
    ProcedureDescriptor,
    ProcedureParameter,
    ResolvedArrayAccess,
    ResolvedGoto,
    ResolvedProcedureCall,
    ResolvedReference,
    ResolvedSwitchSelection,
    SemanticBlock,
    SwitchDescriptor,
    Symbol,
    TypeCheckResult,
    check_algol,
)
from compiler_ir import (
    IDGenerator,
    IrDataDecl,
    IrFloatImmediate,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from lang_parser import ASTNode
from lexer import Token

_FRAME_MEMORY_LABEL = "__algol_frames"
_HEAP_MEMORY_LABEL = "__algol_heap"
_STATIC_MEMORY_LABEL = "__algol_static"
_ZERO_REG = 0
_RESULT_REG = 1
_REAL_RESULT_REG = 31
_DYNAMIC_LINK_OFFSET = 0
_STATIC_LINK_OFFSET = 4
_RETURN_TOKEN_OFFSET = 8
_FRAME_SIZE_OFFSET = 12
_BLOCK_ID_OFFSET = 16
_FRAME_MEMORY_BYTES = 64 * 1024
_HEAP_MEMORY_BYTES = 64 * 1024
_RUNTIME_CURRENT_FRAME_OFFSET = 0
_RUNTIME_STACK_POINTER_OFFSET = 4
_RUNTIME_STACK_LIMIT_OFFSET = 8
_RUNTIME_HEAP_POINTER_OFFSET = 12
_RUNTIME_HEAP_LIMIT_OFFSET = 16
_RUNTIME_THUNK_HEAP_MARK_OFFSET = 20
_RUNTIME_THUNK_FAILURE_OFFSET = 24
_RUNTIME_THUNK_HELPER_DEPTH_OFFSET = 28
_RUNTIME_PENDING_GOTO_LABEL_OFFSET = 32
_RUNTIME_OUTPUT_BYTES_OFFSET = 36
_RUNTIME_STATE_BYTES = 40
_STATIC_LINK_PARAM_REG = 2
_THUNK_HEAP_MARK_PARAM_REG = 3
_VALUE_PARAM_BASE_REG = 4
_FIRST_GENERAL_REG = 32
_ARRAY_DESCRIPTOR_SIZE = 24
_ARRAY_ELEMENT_TYPE_INTEGER = 1
_ARRAY_ELEMENT_TYPE_REAL = 2
_ARRAY_DIMENSION_COUNT_OFFSET = 4
_ARRAY_DIMENSION_ENTRY_SIZE = 12
_ARRAY_DIM_LOWER_OFFSET = 0
_ARRAY_DIM_UPPER_OFFSET = 4
_ARRAY_DIM_STRIDE_OFFSET = 8
_ARRAY_TOTAL_COUNT_OFFSET = 8
_ARRAY_ELEMENT_WIDTH_OFFSET = 12
_ARRAY_DATA_POINTER_OFFSET = 16
_ARRAY_BOUNDS_POINTER_OFFSET = 20
_ARRAY_WORD_BYTES = 4
_ARRAY_REAL_BYTES = 8
_ARRAY_MAX_ELEMENTS = 4096
_VALUE_MODE = "value"
_NAME_MODE = "name"
_THUNK_EVAL_LABEL = "_fn_algol_eval_thunk"
_THUNK_STORE_LABEL = "_fn_algol_store_thunk"
_THUNK_EVAL_REAL_LABEL = "_fn_algol_eval_real_thunk"
_THUNK_STORE_REAL_LABEL = "_fn_algol_store_real_thunk"
_SWITCH_EVAL_LABEL = "_fn_algol_eval_switch"
_THUNK_DESCRIPTOR_SIZE = 12
_THUNK_CODE_ID_OFFSET = 0
_THUNK_CALLER_FRAME_OFFSET = 4
_THUNK_FLAGS_OFFSET = 8
_THUNK_DESCRIPTOR_TAG = 1
_THUNK_FLAG_STORE = 1
_SWITCH_DESCRIPTOR_SIZE = 8
_SWITCH_DESCRIPTOR_ID_OFFSET = 0
_SWITCH_DESCRIPTOR_CALLER_FRAME_OFFSET = 4
_MAX_EVAL_THUNKS = 256
_INTEGER_TYPE = "integer"
_BOOLEAN_TYPE = "boolean"
_REAL_TYPE = "real"
_STRING_TYPE = "string"
_STRING_DESCRIPTOR_LENGTH_OFFSET = 0
_STRING_DESCRIPTOR_DATA_POINTER_OFFSET = 4
_STRING_DESCRIPTOR_SIZE = 8
_MAX_STRING_OUTPUT_BYTES = 4096
_MAX_TOTAL_OUTPUT_BYTES = 8192
_BUILTIN_PRINT_LABEL = "__algol_builtin_print"
_BUILTIN_OUTPUT_LABEL = "__algol_builtin_output"
_WRITE_SYSCALL = 1


@dataclass(frozen=True)
class ProcedureSignaturePlan:
    """WASM-relevant procedure signature facts produced by ALGOL lowering."""

    param_types: tuple[str, ...]
    return_type: str | None

    @property
    def param_count(self) -> int:
        return len(self.param_types)


@dataclass(frozen=True)
class CompileResult:
    """The IR compiler output and useful frame metadata.

    ``variable_registers`` is kept as a backward-compatible metadata field for
    callers from the register-only prototype. It now records the first planned
    frame slot offset seen for each source name.
    """

    program: IrProgram
    variable_registers: dict[str, int]
    max_register: int
    variable_slots: dict[str, int] = field(default_factory=dict)
    frame_offsets: dict[int, int] = field(default_factory=dict)
    frame_memory_label: str = _FRAME_MEMORY_LABEL
    heap_memory_label: str = _HEAP_MEMORY_LABEL
    procedure_signatures: dict[str, ProcedureSignaturePlan] = field(
        default_factory=dict
    )


class CompileError(Exception):
    """Raised when checked ALGOL cannot be lowered to the current IR subset."""


@dataclass(frozen=True)
class _FrameScope:
    """The active lexical block during frame-backed lowering."""

    semantic_block: SemanticBlock
    frame_base_reg: int
    heap_mark_reg: int | None = None
    parent: _FrameScope | None = None
    goto_parent: _FrameScope | None = None
    function_owner_procedure_id: int | None = None
    active_thunk_heap_mark_reg: int | None = None
    helper_failure: bool = False

    @property
    def block_id(self) -> int:
        return self.semantic_block.block_id


@dataclass(frozen=True)
class _EvalThunk:
    """A by-name expression captured for helper dispatch."""

    thunk_id: int
    expression: ASTNode
    block_id: int
    type_name: str
    is_array_element: bool = False
    store_capable: bool = False


@dataclass(frozen=True)
class _ForElementPlan:
    """A lowered for-list element and the labels needed to execute it."""

    kind: str
    node: ASTNode
    entry_label: str
    advance_label: str
    dispatch_value: int
    check_label: str | None = None


class AlgolIrCompiler:
    """Compile a typed ALGOL AST into the repository's register IR.

    Scalar variables are backed by explicit ALGOL activation frames. Registers
    remain useful as temporary expression values, frame pointers, and address
    operands for the generic memory instructions consumed by the WASM backend.
    """

    def __init__(self) -> None:
        self.ids = IDGenerator()
        self.program = IrProgram(entry_label="_start")
        self.source_ast: ASTNode | None = None
        self.next_reg = _FIRST_GENERAL_REG
        self.if_count = 0
        self.loop_count = 0
        self.switch_count = 0
        self.output_count = 0
        self.current_frame_reg = -1
        self.stack_base_reg = -1
        self.stack_pointer_reg = -1
        self.stack_limit_reg = -1
        self.heap_base_reg = -1
        self.heap_pointer_reg = -1
        self.heap_limit_reg = -1
        self.semantic_blocks: list[SemanticBlock] = []
        self.semantic_blocks_by_ast: dict[int, SemanticBlock] = {}
        self.semantic_blocks_by_id: dict[int, SemanticBlock] = {}
        self.references: dict[tuple[int, str], ResolvedReference] = {}
        self.procedure_calls: dict[tuple[int, str], ResolvedProcedureCall] = {}
        self.array_accesses: dict[tuple[int, str], ResolvedArrayAccess] = {}
        self.labels: dict[int, LabelDescriptor] = {}
        self.labels_by_symbol: dict[int, LabelDescriptor] = {}
        self.labels_by_block_name: dict[tuple[int, str], LabelDescriptor] = {}
        self.gotos: dict[int, ResolvedGoto] = {}
        self.gotos_by_designational: dict[int, ResolvedGoto] = {}
        self.switches: dict[int, SwitchDescriptor] = {}
        self.switch_selections: dict[int, ResolvedSwitchSelection] = {}
        self.procedures: dict[int, ProcedureDescriptor] = {}
        self.parameters_by_symbol: dict[int, ProcedureParameter] = {}
        self.arrays: dict[int, ArrayDescriptor] = {}
        self.arrays_by_block: dict[int, list[ArrayDescriptor]] = {}
        self.frame_offsets: dict[int, int] = {}
        self.variable_slots: dict[str, int] = {}
        self.legacy_variable_slots: dict[str, int] = {}
        self.procedure_signatures: dict[str, ProcedureSignaturePlan] = {}
        self.expression_types: dict[int, str] = {}
        self.current_function_return_type: str | None = _INTEGER_TYPE
        self.eval_thunks: list[_EvalThunk] = []
        self.has_by_name_parameters = False
        self.has_switch_parameters = False
        self.string_literal_offsets: dict[str, int] = {}

    def compile(self, typed: TypeCheckResult | ASTNode) -> CompileResult:
        type_result = (
            typed if isinstance(typed, TypeCheckResult) else check_algol(typed)
        )
        if not type_result.ok:
            details = "\n".join(
                f"Line {diag.line}, Col {diag.column}: {diag.message}"
                for diag in type_result.diagnostics
            )
            raise CompileError(details)
        if type_result.semantic is None or type_result.semantic.root_block is None:
            raise CompileError("ALGOL type checking did not produce semantic frames")

        self.ids = IDGenerator()
        self.program = IrProgram(entry_label="_start")
        self.source_ast = type_result.ast
        self.next_reg = _FIRST_GENERAL_REG
        self.if_count = 0
        self.loop_count = 0
        self.switch_count = 0
        self.output_count = 0
        self.heap_base_reg = -1
        self.heap_pointer_reg = -1
        self.heap_limit_reg = -1
        self.eval_thunks = []
        self.has_by_name_parameters = False
        self.has_switch_parameters = False
        self.string_literal_offsets = {}
        self.semantic_blocks = list(type_result.semantic.blocks)
        self.semantic_blocks_by_ast = {
            block.ast_node_id: block
            for block in self.semantic_blocks
            if block.ast_node_id is not None
        }
        self.semantic_blocks_by_id = {
            block.block_id: block for block in self.semantic_blocks
        }
        self.references = {
            (reference.token_id, reference.role): reference
            for reference in type_result.semantic.references
        }
        self.procedure_calls = {
            (call.token_id, call.role): call
            for call in type_result.semantic.procedure_calls
        }
        self.array_accesses = {
            (access.token_id, access.role): access
            for access in type_result.semantic.array_accesses
        }
        self.labels = {
            label.statement_node_id: label
            for label in type_result.semantic.labels
        }
        self.labels_by_id = {
            label.label_id: label for label in type_result.semantic.labels
        }
        self.labels_by_symbol = {
            label.symbol_id: label for label in type_result.semantic.labels
        }
        self.labels_by_block_name = {
            (label.declaring_block_id, label.name): label
            for label in type_result.semantic.labels
        }
        self.gotos = {
            goto.token_id: goto for goto in type_result.semantic.gotos
        }
        self.gotos_by_designational = {
            goto.designational_node_id: goto
            for goto in type_result.semantic.gotos
            if goto.designational_node_id is not None
        }
        self.switches = {
            switch.switch_id: switch for switch in type_result.semantic.switches
        }
        self.switch_selections = {
            selection.node_id: selection
            for selection in type_result.semantic.switch_selections
        }
        self.procedures = {
            procedure.procedure_id: procedure
            for procedure in type_result.semantic.procedures
        }
        self.parameters_by_symbol = {
            parameter.symbol_id: parameter
            for procedure in type_result.semantic.procedures
            for parameter in procedure.parameters
        }
        self.has_by_name_parameters = any(
            parameter.mode == _NAME_MODE
            for procedure in type_result.semantic.procedures
            for parameter in procedure.parameters
        )
        self.has_switch_parameters = any(
            parameter.kind == "switch"
            for procedure in type_result.semantic.procedures
            for parameter in procedure.parameters
        )
        self.arrays = {
            array.array_id: array for array in type_result.semantic.arrays
        }
        self.arrays_by_block = {}
        for array in type_result.semantic.arrays:
            if array.storage_class in {"frame", "static"}:
                self.arrays_by_block.setdefault(
                    array.declaring_block_id,
                    [],
                ).append(array)
        self.procedure_signatures = {
            procedure.label: ProcedureSignaturePlan(
                param_types=(
                    _INTEGER_TYPE,
                    _INTEGER_TYPE,
                    *(
                        _INTEGER_TYPE
                        if parameter.mode == _NAME_MODE
                        or parameter.kind in {"array", "label", "switch"}
                        else parameter.type_name
                        for parameter in procedure.parameters
                    ),
                ),
                return_type=procedure.return_type,
            )
            for procedure in type_result.semantic.procedures
        }
        if self.has_by_name_parameters:
            self.procedure_signatures[_THUNK_EVAL_LABEL] = ProcedureSignaturePlan(
                param_types=(_INTEGER_TYPE, _INTEGER_TYPE),
                return_type=_INTEGER_TYPE,
            )
            self.procedure_signatures[_THUNK_STORE_LABEL] = ProcedureSignaturePlan(
                param_types=(_INTEGER_TYPE, _INTEGER_TYPE, _INTEGER_TYPE),
                return_type=_INTEGER_TYPE,
            )
            self.procedure_signatures[_THUNK_EVAL_REAL_LABEL] = (
                ProcedureSignaturePlan(
                    param_types=(_INTEGER_TYPE, _INTEGER_TYPE),
                    return_type=_REAL_TYPE,
                )
            )
            self.procedure_signatures[_THUNK_STORE_REAL_LABEL] = (
                ProcedureSignaturePlan(
                    param_types=(_INTEGER_TYPE, _INTEGER_TYPE, _REAL_TYPE),
                    return_type=_INTEGER_TYPE,
                )
            )
        if self.has_switch_parameters:
            self.procedure_signatures[_SWITCH_EVAL_LABEL] = ProcedureSignaturePlan(
                param_types=(_INTEGER_TYPE, _INTEGER_TYPE, _INTEGER_TYPE),
                return_type=_INTEGER_TYPE,
            )
        self.frame_offsets = self._layout_frames(self.semantic_blocks)
        self.variable_slots = self._collect_variable_slots(self.semantic_blocks)
        self.legacy_variable_slots = self._collect_legacy_variable_slots(
            self.semantic_blocks
        )
        self.expression_types = dict(type_result.expression_types)

        total_frame_bytes = sum(
            block.frame_layout.frame_size for block in self.semantic_blocks
        )
        total_memory_bytes = _RUNTIME_STATE_BYTES + total_frame_bytes
        if total_memory_bytes > _FRAME_MEMORY_BYTES:
            raise CompileError(
                "ALGOL frame memory requires "
                f"{total_frame_bytes} frame bytes plus "
                f"{_RUNTIME_STATE_BYTES} runtime bytes, exceeding the "
                f"{_FRAME_MEMORY_BYTES} byte phase-3 limit"
            )
        self.program.add_data(IrDataDecl(_FRAME_MEMORY_LABEL, _FRAME_MEMORY_BYTES, 0))
        self.program.add_data(IrDataDecl(_HEAP_MEMORY_LABEL, _HEAP_MEMORY_BYTES, 0))
        static_bytes = self._static_storage_bytes(type_result.semantic.symbols)
        static_bytes = self._plan_string_literals(type_result.ast, static_bytes)
        if static_bytes > 0:
            self.program.add_data(IrDataDecl(_STATIC_MEMORY_LABEL, static_bytes, 0))

        self._label("_start")
        self._emit(IrOp.LOAD_IMM, IrRegister(_ZERO_REG), IrImmediate(0))
        self.stack_base_reg = self._fresh_reg()
        self.stack_pointer_reg = self._fresh_reg()
        self.stack_limit_reg = self._fresh_reg()
        self.current_frame_reg = self._fresh_reg()
        self.heap_base_reg = self._fresh_reg()
        self.heap_pointer_reg = self._fresh_reg()
        self.heap_limit_reg = self._fresh_reg()
        self._emit(
            IrOp.LOAD_ADDR,
            IrRegister(self.stack_base_reg),
            IrLabel(_FRAME_MEMORY_LABEL),
        )
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(self.stack_pointer_reg),
            IrRegister(self.stack_base_reg),
            IrImmediate(_RUNTIME_STATE_BYTES),
        )
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(self.stack_limit_reg),
            IrRegister(self.stack_base_reg),
            IrImmediate(_FRAME_MEMORY_BYTES),
        )
        self._emit(
            IrOp.LOAD_ADDR,
            IrRegister(self.heap_base_reg),
            IrLabel(_HEAP_MEMORY_LABEL),
        )
        self._initialize_string_literals()
        self._copy_reg(dst=self.heap_pointer_reg, src=self.heap_base_reg)
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(self.heap_limit_reg),
            IrRegister(self.heap_base_reg),
            IrImmediate(_HEAP_MEMORY_BYTES),
        )
        self._emit(IrOp.LOAD_IMM, IrRegister(self.current_frame_reg), IrImmediate(0))
        self._store_runtime_state(_RUNTIME_CURRENT_FRAME_OFFSET, self.current_frame_reg)
        self._store_runtime_state(_RUNTIME_STACK_POINTER_OFFSET, self.stack_pointer_reg)
        self._store_runtime_state(_RUNTIME_STACK_LIMIT_OFFSET, self.stack_limit_reg)
        self._store_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, self.heap_pointer_reg)
        self._store_runtime_state(_RUNTIME_HEAP_LIMIT_OFFSET, self.heap_limit_reg)
        self._store_runtime_state(_RUNTIME_THUNK_HEAP_MARK_OFFSET, _ZERO_REG)
        self._store_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, _ZERO_REG)
        self._store_runtime_state(_RUNTIME_THUNK_HELPER_DEPTH_OFFSET, _ZERO_REG)
        self._store_runtime_state(_RUNTIME_PENDING_GOTO_LABEL_OFFSET, _ZERO_REG)
        self._store_runtime_state(_RUNTIME_OUTPUT_BYTES_OFFSET, _ZERO_REG)

        block = _first_node(type_result.ast, "block")
        if block is None:
            raise CompileError("ALGOL program must contain a block")
        self.current_function_return_type = _INTEGER_TYPE
        root_scope = self._compile_block(block, parent=None)

        result_symbol = root_scope.semantic_block.scope.symbols.get("result")
        if result_symbol is None:
            raise CompileError(
                "compiled ALGOL programs must declare integer variable 'result'"
            )
        self._emit_load_symbol(result_symbol, root_scope, _RESULT_REG)
        self._emit(IrOp.HALT)
        self._compile_procedures(type_result.semantic.procedures)
        if self.has_by_name_parameters:
            self._compile_eval_thunk_dispatcher(
                label=_THUNK_EVAL_LABEL,
                thunk_kind="word",
            )
            self._compile_eval_thunk_dispatcher(
                label=_THUNK_EVAL_REAL_LABEL,
                thunk_kind=_REAL_TYPE,
            )
            self._compile_store_thunk_dispatcher(
                label=_THUNK_STORE_LABEL,
                thunk_kind="word",
            )
            self._compile_store_thunk_dispatcher(
                label=_THUNK_STORE_REAL_LABEL,
                thunk_kind=_REAL_TYPE,
            )
        if self.has_switch_parameters:
            self._compile_switch_eval_dispatcher()

        return CompileResult(
            program=self.program,
            variable_registers=dict(self.legacy_variable_slots),
            max_register=max(1, self.next_reg - 1),
            variable_slots=dict(self.variable_slots),
            frame_offsets=dict(self.frame_offsets),
            frame_memory_label=_FRAME_MEMORY_LABEL,
            heap_memory_label=_HEAP_MEMORY_LABEL,
            procedure_signatures=dict(self.procedure_signatures),
        )

    def _compile_block(self, block: ASTNode, parent: _FrameScope | None) -> _FrameScope:
        semantic_block = self._semantic_block_for_ast(block)
        static_parent_reg = parent.frame_base_reg if parent is not None else _ZERO_REG
        heap_mark_reg = self._snapshot_heap_pointer()
        frame_base_reg = self._emit_enter_frame(
            semantic_block,
            static_parent_reg,
            heap_mark_reg,
        )
        scope = _FrameScope(
            semantic_block=semantic_block,
            frame_base_reg=frame_base_reg,
            heap_mark_reg=heap_mark_reg,
            parent=parent,
            goto_parent=parent,
            function_owner_procedure_id=(
                parent.function_owner_procedure_id if parent is not None else None
            ),
            active_thunk_heap_mark_reg=(
                parent.active_thunk_heap_mark_reg if parent is not None else None
            ),
        )

        self._initialize_scalar_slots(scope)
        self._allocate_arrays(scope)
        for statement in _direct_nodes(block, "statement"):
            self._compile_statement(statement, scope)

        self._emit_leave_frame(scope)
        return scope

    def _emit_enter_frame(
        self,
        semantic_block: SemanticBlock,
        static_parent_reg: int,
        heap_mark_reg: int,
    ) -> int:
        frame_base = self._fresh_reg()
        new_stack_pointer = self._fresh_reg()
        overflow = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_STACK_POINTER_OFFSET, frame_base)
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(new_stack_pointer),
            IrRegister(frame_base),
            IrImmediate(semantic_block.frame_layout.frame_size),
        )
        self._load_runtime_state(_RUNTIME_STACK_LIMIT_OFFSET, self.stack_limit_reg)
        self._emit(
            IrOp.CMP_GT,
            IrRegister(overflow),
            IrRegister(new_stack_pointer),
            IrRegister(self.stack_limit_reg),
        )
        self._emit_stack_overflow_guard(overflow)
        self._load_runtime_state(_RUNTIME_CURRENT_FRAME_OFFSET, self.current_frame_reg)
        self._store_runtime_state(_RUNTIME_STACK_POINTER_OFFSET, new_stack_pointer)

        self._store_word_reg(
            value_reg=self.current_frame_reg,
            base_reg=frame_base,
            offset=_DYNAMIC_LINK_OFFSET,
        )
        self._store_word_reg(
            value_reg=static_parent_reg,
            base_reg=frame_base,
            offset=_STATIC_LINK_OFFSET,
        )
        self._store_word_reg(
            value_reg=heap_mark_reg,
            base_reg=frame_base,
            offset=_RETURN_TOKEN_OFFSET,
        )
        self._store_word_const(
            frame_base,
            _FRAME_SIZE_OFFSET,
            semantic_block.frame_layout.frame_size,
        )
        self._store_word_const(frame_base, _BLOCK_ID_OFFSET, semantic_block.block_id)
        self._copy_reg(dst=self.current_frame_reg, src=frame_base)
        self._store_runtime_state(_RUNTIME_CURRENT_FRAME_OFFSET, self.current_frame_reg)
        return frame_base

    def _emit_leave_frame(self, scope: _FrameScope) -> None:
        self._restore_heap_pointer(scope)
        offset_reg = self._const_reg(_DYNAMIC_LINK_OFFSET)
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(self.current_frame_reg),
            IrRegister(scope.frame_base_reg),
            IrRegister(offset_reg),
        )
        self._store_runtime_state(_RUNTIME_CURRENT_FRAME_OFFSET, self.current_frame_reg)
        self._store_runtime_state(_RUNTIME_STACK_POINTER_OFFSET, scope.frame_base_reg)

    def _initialize_scalar_slots(self, scope: _FrameScope) -> None:
        for slot in scope.semantic_block.frame_layout.slots:
            if slot.type_name == _REAL_TYPE:
                self._store_f64_const(scope.frame_base_reg, slot.offset, 0.0)
            else:
                self._store_word_const(scope.frame_base_reg, slot.offset, 0)

    def _allocate_arrays(self, scope: _FrameScope) -> None:
        for array in self.arrays_by_block.get(scope.block_id, []):
            self._allocate_array(array, scope)

    def _allocate_array(self, array: ArrayDescriptor, scope: _FrameScope) -> None:
        static_base_reg: int | None = None
        if array.storage_class == "static":
            static_base_reg = self._fresh_reg()
            existing_descriptor = self._fresh_reg()
            initialized = self._fresh_reg()
            end_label = f"algol_label_static_array_{array.array_id}_ready"
            self._emit(
                IrOp.LOAD_ADDR,
                IrRegister(static_base_reg),
                IrLabel(_STATIC_MEMORY_LABEL),
            )
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(existing_descriptor),
                IrRegister(static_base_reg),
                IrRegister(self._const_reg(array.slot_offset)),
            )
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(initialized),
                IrRegister(existing_descriptor),
                IrRegister(_ZERO_REG),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(initialized), IrLabel(end_label))

        lower_regs: list[int] = []
        upper_regs: list[int] = []
        length_regs: list[int] = []
        total_reg = self._const_reg(1)
        max_elements_reg = self._const_reg(_ARRAY_MAX_ELEMENTS)

        for dimension in array.dimensions:
            lower_node = self._find_ast_by_id(dimension.lower_node_id)
            upper_node = self._find_ast_by_id(dimension.upper_node_id)
            if lower_node is None or upper_node is None:
                raise CompileError(f"missing bounds for array {array.name!r}")
            lower = self._compile_expr(lower_node, scope)
            upper = self._compile_expr(upper_node, scope)

            invalid_order = self._fresh_reg()
            self._emit(
                IrOp.CMP_GT,
                IrRegister(invalid_order),
                IrRegister(lower),
                IrRegister(upper),
            )
            self._emit_runtime_failure_guard(invalid_order, scope)

            raw_length = self._fresh_reg()
            self._emit(
                IrOp.SUB,
                IrRegister(raw_length),
                IrRegister(upper),
                IrRegister(lower),
            )
            length = self._fresh_reg()
            self._emit(
                IrOp.ADD_IMM,
                IrRegister(length),
                IrRegister(raw_length),
                IrImmediate(1),
            )
            too_large = self._fresh_reg()
            self._emit(
                IrOp.CMP_GT,
                IrRegister(too_large),
                IrRegister(length),
                IrRegister(max_elements_reg),
            )
            self._emit_runtime_failure_guard(too_large, scope)

            allowed_total = self._fresh_reg()
            self._emit(
                IrOp.DIV,
                IrRegister(allowed_total),
                IrRegister(max_elements_reg),
                IrRegister(length),
            )
            product_too_large = self._fresh_reg()
            self._emit(
                IrOp.CMP_GT,
                IrRegister(product_too_large),
                IrRegister(total_reg),
                IrRegister(allowed_total),
            )
            self._emit_runtime_failure_guard(product_too_large, scope)

            next_total = self._fresh_reg()
            self._emit(
                IrOp.MUL,
                IrRegister(next_total),
                IrRegister(total_reg),
                IrRegister(length),
            )
            total_reg = next_total
            lower_regs.append(lower)
            upper_regs.append(upper)
            length_regs.append(length)

        stride_regs = [0] * len(length_regs)
        stride_reg = self._const_reg(1)
        for index in range(len(length_regs) - 1, -1, -1):
            stride_regs[index] = stride_reg
            next_stride = self._fresh_reg()
            self._emit(
                IrOp.MUL,
                IrRegister(next_stride),
                IrRegister(stride_reg),
                IrRegister(length_regs[index]),
            )
            stride_reg = next_stride

        heap_pointer = self._fresh_reg()
        heap_limit = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, heap_pointer)
        self._load_runtime_state(_RUNTIME_HEAP_LIMIT_OFFSET, heap_limit)

        data_bytes = self._fresh_reg()
        element_width = self._array_element_width(array.element_type)
        word_bytes = self._const_reg(element_width)
        self._emit(
            IrOp.MUL,
            IrRegister(data_bytes),
            IrRegister(total_reg),
            IrRegister(word_bytes),
        )
        fixed_bytes = _ARRAY_DESCRIPTOR_SIZE + (
            len(array.dimensions) * _ARRAY_DIMENSION_ENTRY_SIZE
        )
        allocation_size = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(allocation_size),
            IrRegister(data_bytes),
            IrImmediate(fixed_bytes),
        )
        new_heap_pointer = self._fresh_reg()
        self._emit(
            IrOp.ADD,
            IrRegister(new_heap_pointer),
            IrRegister(heap_pointer),
            IrRegister(allocation_size),
        )
        heap_exhausted = self._fresh_reg()
        self._emit(
            IrOp.CMP_GT,
            IrRegister(heap_exhausted),
            IrRegister(new_heap_pointer),
            IrRegister(heap_limit),
        )
        self._emit_runtime_failure_guard(heap_exhausted, scope)
        self._store_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, new_heap_pointer)
        self._emit_zero_memory(heap_pointer, new_heap_pointer)

        bounds_pointer = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(bounds_pointer),
            IrRegister(heap_pointer),
            IrImmediate(_ARRAY_DESCRIPTOR_SIZE),
        )
        data_pointer = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(data_pointer),
            IrRegister(bounds_pointer),
            IrImmediate(len(array.dimensions) * _ARRAY_DIMENSION_ENTRY_SIZE),
        )

        self._store_word_const(
            heap_pointer,
            0,
            (
                _ARRAY_ELEMENT_TYPE_REAL
                if array.element_type == _REAL_TYPE
                else _ARRAY_ELEMENT_TYPE_INTEGER
            ),
        )
        self._store_word_const(
            heap_pointer,
            _ARRAY_DIMENSION_COUNT_OFFSET,
            len(array.dimensions),
        )
        self._store_word_reg(
            value_reg=total_reg,
            base_reg=heap_pointer,
            offset=_ARRAY_TOTAL_COUNT_OFFSET,
        )
        self._store_word_const(
            heap_pointer,
            _ARRAY_ELEMENT_WIDTH_OFFSET,
            element_width,
        )
        self._store_word_reg(
            value_reg=data_pointer,
            base_reg=heap_pointer,
            offset=_ARRAY_DATA_POINTER_OFFSET,
        )
        self._store_word_reg(
            value_reg=bounds_pointer,
            base_reg=heap_pointer,
            offset=_ARRAY_BOUNDS_POINTER_OFFSET,
        )

        for index, lower in enumerate(lower_regs):
            offset = index * _ARRAY_DIMENSION_ENTRY_SIZE
            self._store_word_reg(
                value_reg=lower,
                base_reg=bounds_pointer,
                offset=offset + _ARRAY_DIM_LOWER_OFFSET,
            )
            self._store_word_reg(
                value_reg=upper_regs[index],
                base_reg=bounds_pointer,
                offset=offset + _ARRAY_DIM_UPPER_OFFSET,
            )
            self._store_word_reg(
                value_reg=stride_regs[index],
                base_reg=bounds_pointer,
                offset=offset + _ARRAY_DIM_STRIDE_OFFSET,
            )

        if array.storage_class == "static":
            if static_base_reg is None:
                raise CompileError(
                    f"static array {array.name!r} is missing a static base register"
                )
            self._store_word_reg(
                value_reg=heap_pointer,
                base_reg=static_base_reg,
                offset=array.slot_offset,
            )
            self._label(end_label)
            return

        self._store_word_reg(
            value_reg=heap_pointer,
            base_reg=scope.frame_base_reg,
            offset=array.slot_offset,
        )

    def _emit_zero_memory(self, start_pointer: int, end_pointer: int) -> None:
        index = self.loop_count
        self.loop_count += 1
        loop_label = f"loop_{index}_start"
        end_label = f"loop_{index}_end"
        cursor = self._fresh_reg()
        done = self._fresh_reg()
        zero = self._const_reg(0)

        self._copy_reg(dst=cursor, src=start_pointer)
        self._label(loop_label)
        self._emit(
            IrOp.CMP_EQ,
            IrRegister(done),
            IrRegister(cursor),
            IrRegister(end_pointer),
        )
        self._emit(IrOp.BRANCH_NZ, IrRegister(done), IrLabel(end_label))
        self._emit(
            IrOp.STORE_BYTE,
            IrRegister(zero),
            IrRegister(cursor),
            IrRegister(_ZERO_REG),
        )
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(cursor),
            IrRegister(cursor),
            IrImmediate(1),
        )
        self._emit(IrOp.JUMP, IrLabel(loop_label))
        self._label(end_label)

    def _compile_statement(self, statement: ASTNode, scope: _FrameScope) -> None:
        label = self.labels.get(id(statement))
        if label is not None:
            self._label(label.ir_label)

        inner = _statement_body(statement)
        if inner is None:
            return
        if inner.rule_name == "unlabeled_stmt":
            self._compile_unlabeled(inner, scope)
        elif inner.rule_name == "cond_stmt":
            self._compile_if(inner, scope)
        else:
            raise CompileError(
                f"{inner.rule_name} is not supported by algol-ir-compiler"
            )

    def _compile_unlabeled(self, node: ASTNode, scope: _FrameScope) -> None:
        inner = _first_ast_child(node)
        if inner is None:
            return
        if inner.rule_name == "assign_stmt":
            self._compile_assignment(inner, scope)
        elif inner.rule_name == "for_stmt":
            self._compile_for(inner, scope)
        elif inner.rule_name == "compound_stmt":
            for statement in _direct_nodes(inner, "statement"):
                self._compile_statement(statement, scope)
        elif inner.rule_name == "block":
            self._compile_block(inner, scope)
        elif inner.rule_name == "proc_stmt":
            self._compile_procedure_call(inner, scope)
        elif inner.rule_name == "goto_stmt":
            self._compile_goto(inner, scope)
        else:
            raise CompileError(
                f"{inner.rule_name} is not supported by algol-ir-compiler"
            )

    def _compile_goto(self, node: ASTNode, scope: _FrameScope) -> None:
        desig_expr = _first_direct_node(node, "desig_expr")
        if desig_expr is None:
            raise CompileError("goto statement is missing a designational expression")
        self._compile_designational(desig_expr, scope)

    def _compile_designational(
        self,
        node: ASTNode,
        scope: _FrameScope,
        *,
        execution_scope: _FrameScope | None = None,
    ) -> None:
        if execution_scope is None:
            execution_scope = scope
        if any(token.value == "if" for token in _direct_tokens(node)):
            index = self.if_count
            self.if_count += 1
            else_label = f"if_{index}_else"
            bool_expr = _first_direct_node(node, "bool_expr")
            if bool_expr is None:
                raise CompileError("conditional designational is missing a condition")
            condition = self._compile_expr(bool_expr, scope)
            self._emit(IrOp.BRANCH_Z, IrRegister(condition), IrLabel(else_label))
            then_desig = _first_direct_node(node, "simple_desig")
            if then_desig is None:
                raise CompileError("conditional designational is missing then target")
            self._compile_simple_designational(
                then_desig,
                scope,
                execution_scope=execution_scope,
            )
            self._label(else_label)
            else_desig = _first_direct_node(node, "desig_expr")
            if else_desig is None:
                raise CompileError("conditional designational is missing else target")
            self._compile_designational(
                else_desig,
                scope,
                execution_scope=execution_scope,
            )
            return

        simple = _first_direct_node(node, "simple_desig")
        if simple is None:
            raise CompileError("unsupported designational expression")
        self._compile_simple_designational(simple, scope, execution_scope=execution_scope)

    def _compile_simple_designational(
        self,
        node: ASTNode,
        scope: _FrameScope,
        *,
        execution_scope: _FrameScope | None = None,
    ) -> None:
        if execution_scope is None:
            execution_scope = scope
        direct = _direct_label_from_simple_designational(node)
        if direct is not None:
            label = self._resolve_label_in_scope_chain(direct.value, scope)
            if label is not None:
                self._emit_goto_label(label, execution_scope)
                return
            label_value = self._compile_label_parameter_target(direct, scope)
            if label_value is not None:
                self._emit_pending_goto_return_reg(execution_scope, label_value)
                return
            raise CompileError(f"goto target {direct.value!r} was not resolved")
            return

        if any(token.value == "[" for token in _direct_tokens(node)):
            self._compile_switch_selection(
                node,
                scope,
                execution_scope=execution_scope,
            )
            return

        nested = _first_direct_node(node, "desig_expr")
        if nested is not None:
            self._compile_designational(
                nested,
                scope,
                execution_scope=execution_scope,
            )
            return
        raise CompileError("unsupported designational expression")

    def _emit_resolved_goto(self, resolved: ResolvedGoto, scope: _FrameScope) -> None:
        label = self.labels_by_id.get(resolved.label_id)
        if label is None:
            raise CompileError(
                f"goto target {resolved.target_name!r} has no label descriptor"
            )
        self._emit_goto_label(label, scope)

    def _resolve_label_in_scope_chain(
        self,
        name: str,
        scope: _FrameScope,
    ) -> LabelDescriptor | None:
        current: _FrameScope | None = scope
        while current is not None:
            label = self.labels_by_block_name.get((current.block_id, name))
            if label is not None:
                return label
            current = current.goto_parent
        return None

    def _compile_label_parameter_target(
        self,
        token: Token,
        scope: _FrameScope,
    ) -> int | None:
        resolved = self._resolve_symbol_in_scope_chain(token.value, scope)
        if resolved is None:
            return None
        symbol, lexical_depth_delta = resolved
        if symbol.kind != "label" or symbol.parameter_mode is None:
            return None
        if symbol.slot_offset is None:
            raise CompileError(
                f"label parameter {token.value!r} has no planned frame slot"
            )
        frame_reg = self._emit_frame_for_lexical_depth(scope, lexical_depth_delta)
        value_reg = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(value_reg),
            IrRegister(frame_reg),
            IrRegister(self._const_reg(symbol.slot_offset)),
        )
        return value_reg

    def _compile_switch_parameter_pointer(
        self,
        name: str,
        scope: _FrameScope,
    ) -> int | None:
        resolved = self._resolve_symbol_in_scope_chain(name, scope)
        if resolved is None:
            return None
        symbol, lexical_depth_delta = resolved
        if symbol.kind != "switch" or symbol.parameter_mode is None:
            return None
        if symbol.slot_offset is None:
            raise CompileError(
                f"switch parameter {name!r} has no planned frame slot"
            )
        frame_reg = self._emit_frame_for_lexical_depth(scope, lexical_depth_delta)
        descriptor_pointer = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(descriptor_pointer),
            IrRegister(frame_reg),
            IrRegister(self._const_reg(symbol.slot_offset)),
        )
        return descriptor_pointer

    def _emit_goto_label(
        self,
        label: LabelDescriptor,
        scope: _FrameScope,
    ) -> None:
        target_block = self.semantic_blocks_by_id[label.declaring_block_id]
        if (
            not scope.helper_failure
            and target_block.owner_procedure_id == scope.function_owner_procedure_id
        ):
            self._emit_unwind_to_block(scope, label.declaring_block_id)
            self._emit(IrOp.JUMP, IrLabel(label.ir_label))
            return
        self._emit_pending_goto_return(scope, label.label_id)

    def _emit_unwind_to_block(self, scope: _FrameScope, target_block_id: int) -> None:
        current: _FrameScope | None = scope
        while current is not None and current.block_id != target_block_id:
            self._emit_leave_frame(current)
            current = current.goto_parent
        if current is None:
            raise CompileError(
                f"goto target block {target_block_id} is not active in this function"
            )

    def _compile_switch_selection(
        self,
        node: ASTNode,
        scope: _FrameScope,
        *,
        execution_scope: _FrameScope | None = None,
    ) -> None:
        selection = self.switch_selections.get(id(node))
        if selection is None:
            raise CompileError("switch selection was not resolved")
        if execution_scope is None:
            execution_scope = scope
        indexes = _direct_nodes(node, "arith_expr")
        if len(indexes) != 1:
            raise CompileError("switch selection requires exactly one index")
        index_value = self._compile_expr(indexes[0], scope)
        if selection.switch_id < 0:
            descriptor_pointer = self._compile_switch_parameter_pointer(
                selection.name,
                scope,
            )
            if descriptor_pointer is None:
                raise CompileError(
                    f"switch parameter {selection.name!r} was not resolved"
                )
            label_value = self._emit_call_switch_eval(
                descriptor_pointer,
                index_value,
                scope,
            )
            self._emit_pending_goto_return_reg(execution_scope, label_value)
            return
        descriptor = self.switches.get(selection.switch_id)
        if descriptor is None:
            raise CompileError(f"switch {selection.name!r} has no descriptor")
        entry_scope = self._active_scope_for_block(selection.declaration_block_id, scope)
        dispatch_index = self.switch_count
        self.switch_count += 1
        for entry_index, entry_node_id in enumerate(descriptor.entry_node_ids, start=1):
            next_label = f"switch_{dispatch_index}_{entry_index}_next"
            expected = self._const_reg(entry_index)
            matches = self._fresh_reg()
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(matches),
                IrRegister(index_value),
                IrRegister(expected),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(matches), IrLabel(next_label))
            entry = self._find_ast_by_id(entry_node_id)
            if entry is None:
                raise CompileError(
                    f"switch {selection.name!r} entry {entry_index} is missing"
                )
            self._compile_designational(
                entry,
                entry_scope,
                execution_scope=execution_scope,
            )
            self._label(next_label)
        failed = self._const_reg(1)
        self._emit_runtime_failure_guard(failed, execution_scope)

    def _compile_designational_value(
        self,
        node: ASTNode,
        scope: _FrameScope,
    ) -> int:
        if any(token.value == "if" for token in _direct_tokens(node)):
            index = self.if_count
            self.if_count += 1
            else_label = f"if_{index}_else"
            end_label = f"if_{index}_end"
            result = self._fresh_reg()
            bool_expr = _first_direct_node(node, "bool_expr")
            if bool_expr is None:
                raise CompileError("conditional designational is missing a condition")
            condition = self._compile_expr(bool_expr, scope)
            self._emit(IrOp.BRANCH_Z, IrRegister(condition), IrLabel(else_label))
            then_desig = _first_direct_node(node, "simple_desig")
            if then_desig is None:
                raise CompileError("conditional designational is missing then target")
            then_value = self._compile_simple_designational_value(then_desig, scope)
            self._copy_reg(dst=result, src=then_value)
            self._emit(IrOp.JUMP, IrLabel(end_label))
            self._label(else_label)
            else_desig = _first_direct_node(node, "desig_expr")
            if else_desig is None:
                raise CompileError("conditional designational is missing else target")
            else_value = self._compile_designational_value(else_desig, scope)
            self._copy_reg(dst=result, src=else_value)
            self._label(end_label)
            return result

        simple = _first_direct_node(node, "simple_desig")
        if simple is None:
            raise CompileError("unsupported designational expression")
        return self._compile_simple_designational_value(simple, scope)

    def _compile_simple_designational_value(
        self,
        node: ASTNode,
        scope: _FrameScope,
    ) -> int:
        direct = _direct_label_from_simple_designational(node)
        if direct is not None:
            label = self._resolve_label_in_scope_chain(direct.value, scope)
            if label is not None:
                return self._const_reg(label.label_id)
            label_value = self._compile_label_parameter_target(direct, scope)
            if label_value is not None:
                return label_value
            raise CompileError(f"goto target {direct.value!r} was not resolved")

        if any(token.value == "[" for token in _direct_tokens(node)):
            return self._compile_switch_selection_value(node, scope)

        nested = _first_direct_node(node, "desig_expr")
        if nested is not None:
            return self._compile_designational_value(nested, scope)
        raise CompileError("unsupported designational expression")

    def _compile_switch_selection_value(
        self,
        node: ASTNode,
        scope: _FrameScope,
    ) -> int:
        selection = self.switch_selections.get(id(node))
        if selection is None:
            raise CompileError("switch selection was not resolved")
        indexes = _direct_nodes(node, "arith_expr")
        if len(indexes) != 1:
            raise CompileError("switch selection requires exactly one index")
        index_value = self._compile_expr(indexes[0], scope)
        if selection.switch_id < 0:
            descriptor_pointer = self._compile_switch_parameter_pointer(
                selection.name,
                scope,
            )
            if descriptor_pointer is None:
                raise CompileError(
                    f"switch parameter {selection.name!r} was not resolved"
                )
            return self._emit_call_switch_eval(descriptor_pointer, index_value, scope)

        descriptor = self.switches.get(selection.switch_id)
        if descriptor is None:
            raise CompileError(f"switch {selection.name!r} has no descriptor")
        entry_scope = self._rebase_scope_for_selection(scope, selection)
        return self._compile_switch_entries_value(
            descriptor,
            selection,
            index_value,
            entry_scope,
        )

    def _compile_switch_entries_value(
        self,
        descriptor: SwitchDescriptor,
        selection: ResolvedSwitchSelection,
        index_value: int,
        entry_scope: _FrameScope,
    ) -> int:
        dispatch_index = self.switch_count
        self.switch_count += 1
        result = self._const_reg(0)
        end_label = f"switch_value_{dispatch_index}_end"
        for entry_index, entry_node_id in enumerate(descriptor.entry_node_ids, start=1):
            next_label = f"switch_value_{dispatch_index}_{entry_index}_next"
            expected = self._const_reg(entry_index)
            matches = self._fresh_reg()
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(matches),
                IrRegister(index_value),
                IrRegister(expected),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(matches), IrLabel(next_label))
            entry = self._find_ast_by_id(entry_node_id)
            if entry is None:
                raise CompileError(
                    f"switch {selection.name!r} entry {entry_index} is missing"
                )
            result = self._compile_designational_value(entry, entry_scope)
            self._emit(IrOp.JUMP, IrLabel(end_label))
            self._label(next_label)
        failed = self._const_reg(1)
        self._emit_runtime_failure_guard(failed, entry_scope)
        self._label(end_label)
        return result

    def _rebase_scope_for_selection(
        self,
        scope: _FrameScope,
        selection: ResolvedSwitchSelection,
    ) -> _FrameScope:
        frame_base_reg = self._emit_frame_for_lexical_depth(
            scope,
            selection.lexical_depth_delta,
        )
        return _FrameScope(
            semantic_block=self.semantic_blocks_by_id[selection.declaration_block_id],
            frame_base_reg=frame_base_reg,
            heap_mark_reg=scope.heap_mark_reg,
            parent=None,
            goto_parent=None,
            function_owner_procedure_id=scope.function_owner_procedure_id,
            active_thunk_heap_mark_reg=scope.active_thunk_heap_mark_reg,
            helper_failure=scope.helper_failure,
        )

    def _compile_assignment(self, assign: ASTNode, scope: _FrameScope) -> None:
        left_part = _first_direct_node(assign, "left_part")
        expr = _first_direct_node(assign, "expression")
        variable = _first_node(left_part, "variable") if left_part is not None else None
        if variable is None or expr is None:
            raise CompileError("assignment needs a variable target and expression")
        value = self._compile_expr(expr, scope)
        if _variable_subscripts(variable):
            name = _variable_head_name(variable)
            if name is None:
                raise CompileError("array assignment target is missing a name")
            access = self._require_array_access(name, "write")
            value = self._coerce_reg_to_type(
                value,
                self._expr_type(expr),
                expected_type=self.arrays[access.array_id].element_type,
            )
            self._compile_array_store(variable, scope, value)
            return
        name = _variable_name(variable)
        if name is None:
            raise CompileError("only scalar assignments are supported")
        target = self._require_reference(name, "write")
        value = self._coerce_reg_to_type(
            value,
            self._expr_type(expr),
            expected_type=target.type_name,
        )
        self._emit_store_reference(target, scope, value)

    def _compile_if(self, cond: ASTNode, scope: _FrameScope) -> None:
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"

        bool_expr = _first_direct_node(cond, "bool_expr")
        if bool_expr is None:
            raise CompileError("if statement is missing a condition")
        condition = self._compile_expr(bool_expr, scope)
        self._emit(IrOp.BRANCH_Z, IrRegister(condition), IrLabel(else_label))

        seen_then = False
        for child in cond.children:
            if isinstance(child, Token) and child.value == "then":
                seen_then = True
            elif (
                isinstance(child, ASTNode)
                and child.rule_name == "unlabeled_stmt"
                and seen_then
            ):
                self._compile_unlabeled(child, scope)

        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        for child in cond.children:
            if isinstance(child, ASTNode) and child.rule_name == "statement":
                self._compile_statement(child, scope)
        self._label(end_label)

    def _compile_for(self, node: ASTNode, scope: _FrameScope) -> None:
        loop_token = next(
            (tok for tok in _direct_tokens(node) if tok.type_name == "NAME"), None
        )
        if loop_token is None:
            raise CompileError("for loop is missing its control variable")
        loop_reference = self._require_reference(loop_token, "control")
        body = _first_direct_node(node, "statement")
        elements = _direct_nodes(_first_direct_node(node, "for_list"), "for_elem")
        if not elements:
            return

        index = self.loop_count
        self.loop_count += 1
        if len(elements) == 1:
            self._compile_single_for_element(
                elements[0],
                loop_reference=loop_reference,
                body=body,
                scope=scope,
                index=index,
            )
            return

        dispatch_label = f"loop_{index}_dispatch"
        body_label = f"loop_{index}_body"
        after_body_label = f"loop_{index}_after_body"
        end_label = f"loop_{index}_end"
        active_element_reg = self._fresh_reg()
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(active_element_reg),
            IrImmediate(0),
        )
        self._emit(IrOp.JUMP, IrLabel(dispatch_label))

        plans: list[_ForElementPlan] = []
        for dispatch_value, elem in enumerate(elements):
            kind = _for_element_kind(elem)
            entry_label = f"loop_{index}_elem_{dispatch_value}_entry"
            advance_label = f"loop_{index}_elem_{dispatch_value}_advance"
            check_label = (
                f"loop_{index}_elem_{dispatch_value}_check"
                if kind == "step_until"
                else None
            )
            plans.append(
                _ForElementPlan(
                    kind=kind,
                    node=elem,
                    entry_label=entry_label,
                    advance_label=advance_label,
                    dispatch_value=dispatch_value,
                    check_label=check_label,
                )
            )

        self._label(dispatch_label)
        self._emit_for_dispatch(
            active_element_reg,
            dispatch_label,
            tuple((plan.dispatch_value, plan.entry_label) for plan in plans),
            default_label=end_label,
        )

        for plan_index, plan in enumerate(plans):
            next_entry_label = (
                plans[plan_index + 1].entry_label
                if plan_index + 1 < len(plans)
                else end_label
            )
            arith_nodes = _direct_nodes(plan.node, "arith_expr")
            bool_node = _first_direct_node(plan.node, "bool_expr")

            if plan.kind == "simple":
                self._label(plan.entry_label)
                value = self._compile_expr(arith_nodes[0], scope)
                value = self._coerce_reg_to_type(
                    value,
                    self._expr_type(arith_nodes[0]),
                    expected_type=loop_reference.type_name,
                )
                self._emit_store_reference(loop_reference, scope, value)
                self._emit(
                    IrOp.LOAD_IMM,
                    IrRegister(active_element_reg),
                    IrImmediate(plan.dispatch_value),
                )
                self._emit(IrOp.JUMP, IrLabel(body_label))
                self._label(plan.advance_label)
                self._emit(IrOp.JUMP, IrLabel(next_entry_label))
                continue

            if plan.kind == "while":
                if bool_node is None:
                    raise CompileError("for while-element is missing a condition")
                self._label(plan.entry_label)
                value = self._compile_expr(arith_nodes[0], scope)
                value = self._coerce_reg_to_type(
                    value,
                    self._expr_type(arith_nodes[0]),
                    expected_type=loop_reference.type_name,
                )
                self._emit_store_reference(loop_reference, scope, value)
                condition = self._compile_expr(bool_node, scope)
                self._emit(
                    IrOp.BRANCH_Z,
                    IrRegister(condition),
                    IrLabel(next_entry_label),
                )
                self._emit(
                    IrOp.LOAD_IMM,
                    IrRegister(active_element_reg),
                    IrImmediate(plan.dispatch_value),
                )
                self._emit(IrOp.JUMP, IrLabel(body_label))
                self._label(plan.advance_label)
                self._emit(IrOp.JUMP, IrLabel(plan.entry_label))
                continue

            if plan.kind != "step_until" or plan.check_label is None:
                raise CompileError("unsupported for-element form")
            self._label(plan.entry_label)
            start = self._compile_expr(arith_nodes[0], scope)
            start = self._coerce_reg_to_type(
                start,
                self._expr_type(arith_nodes[0]),
                expected_type=loop_reference.type_name,
            )
            self._emit_store_reference(loop_reference, scope, start)
            self._emit(IrOp.JUMP, IrLabel(plan.check_label))

            self._label(plan.check_label)
            step_value = self._compile_expr(arith_nodes[1], scope)
            step_type = self._expr_type(arith_nodes[1])
            limit_value = self._compile_expr(arith_nodes[2], scope)
            limit_type = self._expr_type(arith_nodes[2])
            self._emit_step_until_dispatch(
                active_element_reg=active_element_reg,
                dispatch_value=plan.dispatch_value,
                loop_reference=loop_reference,
                loop_scope=scope,
                step_value=step_value,
                step_type=step_type,
                limit_value=limit_value,
                limit_type=limit_type,
                continue_label=body_label,
                stop_label=next_entry_label,
            )

            self._label(plan.advance_label)
            current_value = self._emit_load_reference(loop_reference, scope)
            next_value = self._emit_numeric(
                "+",
                current_value,
                loop_reference.type_name,
                step_value,
                step_type,
            )
            next_type = self._result_type_for_numeric_operator(
                "+",
                loop_reference.type_name,
                step_type,
            )
            next_value = self._coerce_reg_to_type(
                next_value,
                next_type,
                expected_type=loop_reference.type_name,
            )
            self._emit_store_reference(loop_reference, scope, next_value)
            self._emit(IrOp.JUMP, IrLabel(plan.check_label))

        self._label(body_label)
        if body is not None:
            self._compile_statement(body, scope)
        self._label(after_body_label)
        self._emit_for_dispatch(
            active_element_reg,
            after_body_label,
            tuple((plan.dispatch_value, plan.advance_label) for plan in plans),
            default_label=end_label,
        )
        self._label(end_label)

    def _compile_single_for_element(
        self,
        elem: ASTNode,
        *,
        loop_reference: ResolvedReference,
        body: ASTNode | None,
        scope: _FrameScope,
        index: int,
    ) -> None:
        kind = _for_element_kind(elem)
        if kind == "simple":
            value_node = _direct_nodes(elem, "arith_expr")[0]
            value = self._compile_expr(value_node, scope)
            value = self._coerce_reg_to_type(
                value,
                self._expr_type(value_node),
                expected_type=loop_reference.type_name,
            )
            self._emit_store_reference(loop_reference, scope, value)
            if body is not None:
                self._compile_statement(body, scope)
            return

        if kind == "while":
            value_node = _direct_nodes(elem, "arith_expr")[0]
            condition_node = _first_direct_node(elem, "bool_expr")
            if condition_node is None:
                raise CompileError("for while-element is missing a condition")
            start_label = f"loop_{index}_start"
            end_label = f"loop_{index}_end"
            self._label(start_label)
            value = self._compile_expr(value_node, scope)
            value = self._coerce_reg_to_type(
                value,
                self._expr_type(value_node),
                expected_type=loop_reference.type_name,
            )
            self._emit_store_reference(loop_reference, scope, value)
            condition = self._compile_expr(condition_node, scope)
            self._emit(IrOp.BRANCH_Z, IrRegister(condition), IrLabel(end_label))
            if body is not None:
                self._compile_statement(body, scope)
            self._emit(IrOp.JUMP, IrLabel(start_label))
            self._label(end_label)
            return

        if kind != "step_until":
            raise CompileError("unsupported for-element form")

        nodes = _direct_nodes(elem, "arith_expr")
        start_node = nodes[0]
        step_node = nodes[1]
        limit_node = nodes[2]
        start = self._compile_expr(start_node, scope)
        start = self._coerce_reg_to_type(
            start,
            self._expr_type(start_node),
            expected_type=loop_reference.type_name,
        )
        self._emit_store_reference(loop_reference, scope, start)

        start_label = f"loop_{index}_start"
        body_label = f"loop_{index}_body"
        end_label = f"loop_{index}_end"
        self._label(start_label)
        step_value = self._compile_expr(step_node, scope)
        step_type = self._expr_type(step_node)
        limit_value = self._compile_expr(limit_node, scope)
        limit_type = self._expr_type(limit_node)
        self._emit_step_until_branch(
            loop_reference=loop_reference,
            loop_scope=scope,
            step_value=step_value,
            step_type=step_type,
            limit_value=limit_value,
            limit_type=limit_type,
            label_prefix=f"loop_{index}",
            continue_label=body_label,
            stop_label=end_label,
        )
        self._label(body_label)
        if body is not None:
            self._compile_statement(body, scope)
        current_value = self._emit_load_reference(loop_reference, scope)
        next_value = self._emit_numeric(
            "+",
            current_value,
            loop_reference.type_name,
            step_value,
            step_type,
        )
        next_type = self._result_type_for_numeric_operator(
            "+",
            loop_reference.type_name,
            step_type,
        )
        next_value = self._coerce_reg_to_type(
            next_value,
            next_type,
            expected_type=loop_reference.type_name,
        )
        self._emit_store_reference(loop_reference, scope, next_value)
        self._emit(IrOp.JUMP, IrLabel(start_label))
        self._label(end_label)

    def _emit_step_until_branch(
        self,
        *,
        loop_reference: ResolvedReference,
        loop_scope: _FrameScope,
        step_value: int,
        step_type: str,
        limit_value: int,
        limit_type: str,
        label_prefix: str,
        continue_label: str,
        stop_label: str,
    ) -> None:
        positive_label = f"{label_prefix}_positive"
        negative_or_zero_label = f"{label_prefix}_negative_or_zero"
        negative_label = f"{label_prefix}_negative"
        compare_type = (
            _REAL_TYPE
            if _REAL_TYPE in {loop_reference.type_name, limit_type}
            else _INTEGER_TYPE
        )
        step_type_for_compare = self._result_type_for_numeric_operator(
            "+",
            step_type,
            _INTEGER_TYPE,
        )
        zero = (
            self._const_f64_reg(0.0)
            if step_type_for_compare == _REAL_TYPE
            else self._const_reg(0)
        )
        step_for_compare = self._coerce_reg_to_type(
            step_value,
            step_type,
            expected_type=step_type_for_compare,
        )
        is_positive = self._fresh_reg()
        if step_type_for_compare == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_GT,
                IrRegister(is_positive),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        else:
            self._emit(
                IrOp.CMP_GT,
                IrRegister(is_positive),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(is_positive),
            IrLabel(positive_label),
        )

        self._label(negative_or_zero_label)
        is_negative = self._fresh_reg()
        if step_type_for_compare == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_LT,
                IrRegister(is_negative),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        else:
            self._emit(
                IrOp.CMP_LT,
                IrRegister(is_negative),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(is_negative),
            IrLabel(negative_label),
        )
        self._emit(IrOp.JUMP, IrLabel(continue_label))

        self._label(positive_label)
        loop_value = self._emit_load_reference(loop_reference, loop_scope)
        positive_loop = self._coerce_reg_to_type(
            loop_value,
            loop_reference.type_name,
            expected_type=compare_type,
        )
        positive_limit = self._coerce_reg_to_type(
            limit_value,
            limit_type,
            expected_type=compare_type,
        )
        should_stop_positive = self._fresh_reg()
        if compare_type == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_GT,
                IrRegister(should_stop_positive),
                IrRegister(positive_loop),
                IrRegister(positive_limit),
            )
        else:
            self._emit(
                IrOp.CMP_GT,
                IrRegister(should_stop_positive),
                IrRegister(positive_loop),
                IrRegister(positive_limit),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(should_stop_positive),
            IrLabel(stop_label),
        )
        self._emit(IrOp.JUMP, IrLabel(continue_label))

        self._label(negative_label)
        loop_value = self._emit_load_reference(loop_reference, loop_scope)
        negative_loop = self._coerce_reg_to_type(
            loop_value,
            loop_reference.type_name,
            expected_type=compare_type,
        )
        negative_limit = self._coerce_reg_to_type(
            limit_value,
            limit_type,
            expected_type=compare_type,
        )
        should_stop_negative = self._fresh_reg()
        if compare_type == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_LT,
                IrRegister(should_stop_negative),
                IrRegister(negative_loop),
                IrRegister(negative_limit),
            )
        else:
            self._emit(
                IrOp.CMP_LT,
                IrRegister(should_stop_negative),
                IrRegister(negative_loop),
                IrRegister(negative_limit),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(should_stop_negative),
            IrLabel(stop_label),
        )
        self._emit(IrOp.JUMP, IrLabel(continue_label))

    def _emit_for_dispatch(
        self,
        active_element_reg: int,
        label_prefix: str,
        branches: tuple[tuple[int, str], ...],
        *,
        default_label: str,
    ) -> None:
        for dispatch_value, target_label in branches:
            next_label = f"{label_prefix}_{dispatch_value}_next"
            expected = self._const_reg(dispatch_value)
            matches = self._fresh_reg()
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(matches),
                IrRegister(active_element_reg),
                IrRegister(expected),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(matches), IrLabel(next_label))
            self._emit(IrOp.JUMP, IrLabel(target_label))
            self._label(next_label)
        self._emit(IrOp.JUMP, IrLabel(default_label))

    def _emit_step_until_dispatch(
        self,
        *,
        active_element_reg: int,
        dispatch_value: int,
        loop_reference: ResolvedReference,
        loop_scope: _FrameScope,
        step_value: int,
        step_type: str,
        limit_value: int,
        limit_type: str,
        continue_label: str,
        stop_label: str,
    ) -> None:
        positive_label = f"{continue_label}_{dispatch_value}_positive"
        negative_or_zero_label = f"{continue_label}_{dispatch_value}_negative_or_zero"
        negative_label = f"{continue_label}_{dispatch_value}_negative"
        compare_type = (
            _REAL_TYPE
            if _REAL_TYPE in {loop_reference.type_name, limit_type}
            else _INTEGER_TYPE
        )
        step_type_for_compare = self._result_type_for_numeric_operator(
            "+",
            step_type,
            _INTEGER_TYPE,
        )
        zero = (
            self._const_f64_reg(0.0)
            if step_type_for_compare == _REAL_TYPE
            else self._const_reg(0)
        )
        step_for_compare = self._coerce_reg_to_type(
            step_value,
            step_type,
            expected_type=step_type_for_compare,
        )
        is_positive = self._fresh_reg()
        if step_type_for_compare == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_GT,
                IrRegister(is_positive),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        else:
            self._emit(
                IrOp.CMP_GT,
                IrRegister(is_positive),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(is_positive),
            IrLabel(positive_label),
        )

        self._label(negative_or_zero_label)
        is_negative = self._fresh_reg()
        if step_type_for_compare == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_LT,
                IrRegister(is_negative),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        else:
            self._emit(
                IrOp.CMP_LT,
                IrRegister(is_negative),
                IrRegister(step_for_compare),
                IrRegister(zero),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(is_negative),
            IrLabel(negative_label),
        )
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(active_element_reg),
            IrImmediate(dispatch_value),
        )
        self._emit(IrOp.JUMP, IrLabel(continue_label))

        self._label(positive_label)
        loop_value = self._emit_load_reference(loop_reference, loop_scope)
        positive_loop = self._coerce_reg_to_type(
            loop_value,
            loop_reference.type_name,
            expected_type=compare_type,
        )
        positive_limit = self._coerce_reg_to_type(
            limit_value,
            limit_type,
            expected_type=compare_type,
        )
        should_stop_positive = self._fresh_reg()
        if compare_type == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_GT,
                IrRegister(should_stop_positive),
                IrRegister(positive_loop),
                IrRegister(positive_limit),
            )
        else:
            self._emit(
                IrOp.CMP_GT,
                IrRegister(should_stop_positive),
                IrRegister(positive_loop),
                IrRegister(positive_limit),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(should_stop_positive),
            IrLabel(stop_label),
        )
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(active_element_reg),
            IrImmediate(dispatch_value),
        )
        self._emit(IrOp.JUMP, IrLabel(continue_label))

        self._label(negative_label)
        loop_value = self._emit_load_reference(loop_reference, loop_scope)
        negative_loop = self._coerce_reg_to_type(
            loop_value,
            loop_reference.type_name,
            expected_type=compare_type,
        )
        negative_limit = self._coerce_reg_to_type(
            limit_value,
            limit_type,
            expected_type=compare_type,
        )
        should_stop_negative = self._fresh_reg()
        if compare_type == _REAL_TYPE:
            self._emit(
                IrOp.F64_CMP_LT,
                IrRegister(should_stop_negative),
                IrRegister(negative_loop),
                IrRegister(negative_limit),
            )
        else:
            self._emit(
                IrOp.CMP_LT,
                IrRegister(should_stop_negative),
                IrRegister(negative_loop),
                IrRegister(negative_limit),
            )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(should_stop_negative),
            IrLabel(stop_label),
        )
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(active_element_reg),
            IrImmediate(dispatch_value),
        )
        self._emit(IrOp.JUMP, IrLabel(continue_label))

    def _compile_expr(self, expr: ASTNode | Token, scope: _FrameScope) -> int:
        if isinstance(expr, Token):
            return self._compile_token(expr, scope)
        if expr.rule_name == "proc_call":
            return self._compile_procedure_call(expr, scope)
        if expr.rule_name == "variable":
            if _variable_subscripts(expr):
                return self._compile_array_load(expr, scope)
            name = _variable_name(expr)
            if name is None:
                raise CompileError("variable is missing a name")
            reference = self._require_reference(name, "read")
            return self._emit_load_reference(reference, scope)

        meaningful = _meaningful_children(expr)
        if not meaningful:
            raise CompileError(f"empty expression node {expr.rule_name}")
        if len(meaningful) == 1:
            return self._compile_expr(meaningful[0], scope)

        if expr.rule_name in {"expr_not", "bool_factor", "bool_secondary"}:
            first = meaningful[0]
            if isinstance(first, Token) and first.value == "not":
                return self._invert_bool(self._compile_expr(meaningful[1], scope))

        if expr.rule_name in {"expr_add", "simple_arith"} and isinstance(
            meaningful[0], Token
        ):
            operator = meaningful[0].value
            value = self._compile_expr(meaningful[1], scope)
            if operator == "+":
                return value
            if operator == "-":
                if self._expr_type(expr) == _REAL_TYPE:
                    zero = self._const_f64_reg(0.0)
                    operand = self._coerce_reg_to_type(
                        value,
                        self._expr_type(meaningful[1]),
                    )
                    dst = self._fresh_reg()
                    self._emit(
                        IrOp.F64_SUB,
                        IrRegister(dst),
                        IrRegister(zero),
                        IrRegister(operand),
                    )
                    return dst
                dst = self._fresh_reg()
                self._emit(IrOp.SUB, IrRegister(dst), IrRegister(0), IrRegister(value))
                return dst

        if expr.rule_name in {"expr_cmp", "relation"} and _has_comparison(meaningful):
            return self._compile_comparison(meaningful, scope)

        if expr.rule_name in {"expr_add", "simple_arith", "expr_mul", "term"}:
            return self._compile_numeric_chain(meaningful, scope)

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
            return self._compile_bool_chain(expr.rule_name, meaningful, scope)

        if expr.rule_name in {"expr_atom", "primary", "bool_primary"}:
            token = next(
                (child for child in meaningful if isinstance(child, Token)), None
            )
            if token is not None and token.value not in {"(", ")"}:
                return self._compile_token(token, scope)
            nested = next(
                (child for child in meaningful if isinstance(child, ASTNode)), None
            )
            if nested is not None:
                return self._compile_expr(nested, scope)

        if expr.rule_name in {
            "expression",
            "arith_expr",
            "bool_expr",
            "expr_pow",
            "factor",
        }:
            if any(
                isinstance(child, Token) and child.value in {"**", "^"}
                for child in meaningful
            ):
                raise CompileError("exponentiation is not supported yet")
            return self._compile_expr(meaningful[0], scope)

        return self._compile_expr(meaningful[0], scope)

    def _compile_token(self, token: Token, scope: _FrameScope) -> int:
        if token.type_name == "INTEGER_LIT":
            return self._const_reg(int(token.value))
        if token.type_name == "REAL_LIT":
            return self._const_f64_reg(float(token.value))
        if token.type_name == "STRING_LIT":
            return self._emit_string_literal_pointer(token.value[1:-1])
        if token.value in {"true", "false"}:
            return self._const_reg(1 if token.value == "true" else 0)
        if token.type_name == "NAME":
            reference = self._require_reference(token, "read")
            return self._emit_load_reference(reference, scope)
        raise CompileError(f"unsupported expression token {token.value!r}")

    def _compile_numeric_chain(
        self, children: list[ASTNode | Token], scope: _FrameScope
    ) -> int:
        current = self._compile_expr(children[0], scope)
        current_type = self._expr_type(children[0])
        index = 1
        while index < len(children):
            operator = children[index]
            right = self._compile_expr(children[index + 1], scope)
            if not isinstance(operator, Token):
                raise CompileError("expected numeric operator")
            right_type = self._expr_type(children[index + 1])
            current = self._emit_numeric(
                operator.value,
                current,
                current_type,
                right,
                right_type,
            )
            current_type = self._result_type_for_numeric_operator(
                operator.value,
                current_type,
                right_type,
            )
            index += 2
        return current

    def _compile_bool_chain(
        self,
        rule_name: str,
        children: list[ASTNode | Token],
        scope: _FrameScope,
    ) -> int:
        current = self._compile_expr(children[0], scope)
        index = 1
        while index < len(children):
            operator = children[index]
            right = self._compile_expr(children[index + 1], scope)
            if not isinstance(operator, Token):
                raise CompileError("expected boolean operator")
            value = operator.value
            if value == "and":
                dst = self._fresh_reg()
                self._emit(
                    IrOp.AND, IrRegister(dst), IrRegister(current), IrRegister(right)
                )
                current = dst
            elif value == "or":
                summed = self._fresh_reg()
                dst = self._fresh_reg()
                self._emit(
                    IrOp.ADD, IrRegister(summed), IrRegister(current), IrRegister(right)
                )
                self._emit(
                    IrOp.CMP_NE, IrRegister(dst), IrRegister(summed), IrRegister(0)
                )
                current = dst
            elif value == "impl":
                inverted = self._invert_bool(current)
                summed = self._fresh_reg()
                dst = self._fresh_reg()
                self._emit(
                    IrOp.ADD,
                    IrRegister(summed),
                    IrRegister(inverted),
                    IrRegister(right),
                )
                self._emit(
                    IrOp.CMP_NE, IrRegister(dst), IrRegister(summed), IrRegister(0)
                )
                current = dst
            elif value == "eqv":
                dst = self._fresh_reg()
                self._emit(
                    IrOp.CMP_EQ, IrRegister(dst), IrRegister(current), IrRegister(right)
                )
                current = dst
            else:
                raise CompileError(f"boolean operator {value!r} is not supported yet")
            index += 2
        return current

    def _compile_comparison(
        self, children: list[ASTNode | Token], scope: _FrameScope
    ) -> int:
        left = self._compile_expr(children[0], scope)
        operator = children[1]
        right = self._compile_expr(children[2], scope)
        left_type = self._expr_type(children[0])
        right_type = self._expr_type(children[2])
        if not isinstance(operator, Token):
            raise CompileError("expected comparison operator")
        dst = self._fresh_reg()
        if _REAL_TYPE in {left_type, right_type}:
            left = self._coerce_reg_to_type(
                left,
                left_type,
                expected_type=_REAL_TYPE,
            )
            right = self._coerce_reg_to_type(
                right,
                right_type,
                expected_type=_REAL_TYPE,
            )
            if operator.value == "=":
                self._emit(
                    IrOp.F64_CMP_EQ,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator.value == "!=":
                self._emit(
                    IrOp.F64_CMP_NE,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator.value == "<":
                self._emit(
                    IrOp.F64_CMP_LT,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator.value == ">":
                self._emit(
                    IrOp.F64_CMP_GT,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator.value == "<=":
                self._emit(
                    IrOp.F64_CMP_LE,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator.value == ">=":
                self._emit(
                    IrOp.F64_CMP_GE,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            else:
                raise CompileError(
                    f"comparison operator {operator.value!r} is not supported"
                )
        else:
            if operator.value == "=":
                self._emit(
                    IrOp.CMP_EQ, IrRegister(dst), IrRegister(left), IrRegister(right)
                )
            elif operator.value == "!=":
                self._emit(
                    IrOp.CMP_NE, IrRegister(dst), IrRegister(left), IrRegister(right)
                )
            elif operator.value == "<":
                self._emit(
                    IrOp.CMP_LT, IrRegister(dst), IrRegister(left), IrRegister(right)
                )
            elif operator.value == ">":
                self._emit(
                    IrOp.CMP_GT, IrRegister(dst), IrRegister(left), IrRegister(right)
                )
            elif operator.value == "<=":
                self._emit(
                    IrOp.CMP_GT, IrRegister(dst), IrRegister(left), IrRegister(right)
                )
                dst = self._invert_bool(dst)
            elif operator.value == ">=":
                self._emit(
                    IrOp.CMP_LT, IrRegister(dst), IrRegister(left), IrRegister(right)
                )
                dst = self._invert_bool(dst)
            else:
                raise CompileError(
                    f"comparison operator {operator.value!r} is not supported"
                )
        return dst

    def _emit_numeric(
        self,
        operator: str,
        left: int,
        left_type: str,
        right: int,
        right_type: str,
    ) -> int:
        dst = self._fresh_reg()
        result_type = self._result_type_for_numeric_operator(
            operator,
            left_type,
            right_type,
        )
        if result_type == _REAL_TYPE:
            left = self._coerce_reg_to_type(
                left,
                left_type,
                expected_type=_REAL_TYPE,
            )
            right = self._coerce_reg_to_type(
                right,
                right_type,
                expected_type=_REAL_TYPE,
            )
            if operator == "+":
                self._emit(
                    IrOp.F64_ADD,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator == "-":
                self._emit(
                    IrOp.F64_SUB,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator == "*":
                self._emit(
                    IrOp.F64_MUL,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            elif operator == "/":
                self._emit(
                    IrOp.F64_DIV,
                    IrRegister(dst),
                    IrRegister(left),
                    IrRegister(right),
                )
            else:
                raise CompileError(f"numeric operator {operator!r} is not supported")
            return dst
        if operator == "+":
            self._emit(IrOp.ADD, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "-":
            self._emit(IrOp.SUB, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "*":
            self._emit(IrOp.MUL, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "div":
            self._emit(IrOp.DIV, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "mod":
            quotient = self._fresh_reg()
            product = self._fresh_reg()
            self._emit(
                IrOp.DIV, IrRegister(quotient), IrRegister(left), IrRegister(right)
            )
            self._emit(
                IrOp.MUL, IrRegister(product), IrRegister(quotient), IrRegister(right)
            )
            self._emit(IrOp.SUB, IrRegister(dst), IrRegister(left), IrRegister(product))
        else:
            raise CompileError(f"numeric operator {operator!r} is not supported")
        return dst

    def _invert_bool(self, source: int) -> int:
        dst = self._fresh_reg()
        self._emit(IrOp.ADD_IMM, IrRegister(dst), IrRegister(source), IrImmediate(-1))
        self._emit(IrOp.AND_IMM, IrRegister(dst), IrRegister(dst), IrImmediate(1))
        return dst

    def _compile_procedure_call(self, node: ASTNode, scope: _FrameScope) -> int:
        name = next(
            (token for token in _direct_tokens(node) if token.type_name == "NAME"),
            None,
        )
        if name is None:
            raise CompileError("procedure call is missing a name")
        role = "statement" if node.rule_name == "proc_stmt" else "expression"
        call = self._require_procedure_call(name, role)
        if call.label in {_BUILTIN_PRINT_LABEL, _BUILTIN_OUTPUT_LABEL}:
            return self._compile_builtin_output(node, scope)
        procedure = self.procedures[call.procedure_id]
        static_link = self._emit_static_link_for_call(call, scope)
        actuals = _direct_nodes(_first_direct_node(node, "actual_params"), "expression")
        if len(actuals) != len(procedure.parameters):
            raise CompileError(
                f"procedure {procedure.name!r} expects "
                f"{len(procedure.parameters)} argument(s), got {len(actuals)}"
            )
        thunk_actuals = [
            (index, argument, parameter)
            for index, (argument, parameter) in enumerate(
                zip(actuals, procedure.parameters, strict=True)
            )
            if parameter.kind not in {"array", "label", "switch"}
            and parameter.mode == _NAME_MODE
            and self._requires_by_name_thunk_descriptor(argument)
        ]
        switch_actuals = [
            (index, argument, parameter)
            for index, (argument, parameter) in enumerate(
                zip(actuals, procedure.parameters, strict=True)
            )
            if parameter.kind == "switch"
        ]
        for _, argument, parameter in thunk_actuals:
            variable = _single_variable_expr(argument)
            if variable is None and parameter.may_write:
                raise CompileError(
                    f"by-name parameter {parameter.name!r} is assigned, but "
                    "expression actual has no store thunk lowering yet"
                )
            if variable is None:
                self._require_eval_thunk_expression(argument, parameter)

        arguments: list[int | None] = [None] * len(actuals)
        for index, (argument, parameter) in enumerate(
            zip(actuals, procedure.parameters, strict=True)
        ):
            if parameter.kind == "array":
                if parameter.mode == _VALUE_MODE:
                    raise CompileError(
                        f"value array parameter {parameter.name!r} is not supported yet"
                    )
                continue
            if parameter.kind == "label":
                arguments[index] = self._compile_label_actual_value(argument, scope)
                continue
            if parameter.kind == "switch":
                continue
            if parameter.mode == _VALUE_MODE:
                value = self._compile_expr(argument, scope)
                arguments[index] = self._coerce_reg_to_type(
                    value,
                    self._expr_type(argument),
                    expected_type=parameter.type_name,
                )

        temp_descriptor_bytes = (
            len(thunk_actuals) * _THUNK_DESCRIPTOR_SIZE
            + len(switch_actuals) * _SWITCH_DESCRIPTOR_SIZE
        )
        descriptor_heap_mark = (
            self._emit_reserve_temp_descriptor_space(temp_descriptor_bytes, scope)
            if temp_descriptor_bytes
            else None
        )
        active_thunk_heap_mark = (
            descriptor_heap_mark
            if descriptor_heap_mark is not None
            else (
                scope.active_thunk_heap_mark_reg
                if scope.active_thunk_heap_mark_reg is not None
                else self._const_reg(0)
            )
        )
        descriptor_offsets = {
            argument_index: descriptor_index * _THUNK_DESCRIPTOR_SIZE
            for descriptor_index, (argument_index, _, _) in enumerate(
                thunk_actuals
            )
        }
        switch_descriptor_offsets = {
            argument_index: (
                len(thunk_actuals) * _THUNK_DESCRIPTOR_SIZE
                + descriptor_index * _SWITCH_DESCRIPTOR_SIZE
            )
            for descriptor_index, (argument_index, _, _) in enumerate(
                switch_actuals
            )
        }
        for index, (argument, parameter) in enumerate(
            zip(actuals, procedure.parameters, strict=True)
        ):
            if parameter.kind == "array":
                arguments[index] = self._compile_array_actual_pointer(argument, scope)
                continue
            if parameter.kind == "label":
                arguments[index] = self._compile_label_actual_value(argument, scope)
                continue
            if parameter.kind == "switch":
                descriptor = None
                if descriptor_heap_mark is not None and index in switch_descriptor_offsets:
                    descriptor = self._emit_descriptor_at(
                        descriptor_heap_mark,
                        switch_descriptor_offsets[index],
                    )
                arguments[index] = self._compile_switch_actual_pointer(
                    argument,
                    scope,
                    descriptor,
                )
                continue
            if parameter.mode != _NAME_MODE:
                continue
            descriptor = None
            if descriptor_heap_mark is not None and index in descriptor_offsets:
                descriptor = self._emit_descriptor_at(
                    descriptor_heap_mark,
                    descriptor_offsets[index],
                )
            arguments[index] = self._compile_by_name_actual_pointer(
                argument,
                parameter,
                scope,
                descriptor,
            )
        call_arguments = [
            argument
            for argument in arguments
            if argument is not None
        ]
        self._emit(
            IrOp.CALL,
            IrLabel(call.label),
            IrRegister(static_link),
            IrRegister(active_thunk_heap_mark),
            *(IrRegister(argument) for argument in call_arguments),
        )
        if descriptor_heap_mark is not None:
            self._store_runtime_state(
                _RUNTIME_HEAP_POINTER_OFFSET,
                descriptor_heap_mark,
            )
        if scope.helper_failure:
            self._emit_helper_return_on_thunk_failure(scope)
        self._emit_handle_pending_goto_after_call(scope)
        result = self._fresh_reg()
        self._copy_scalar_reg(
            type_name=call.return_type or _INTEGER_TYPE,
            dst=result,
            src=_REAL_RESULT_REG if call.return_type == _REAL_TYPE else _RESULT_REG,
        )
        return result

    def _compile_builtin_output(self, node: ASTNode, scope: _FrameScope) -> int:
        actuals = _direct_nodes(_first_direct_node(node, "actual_params"), "expression")
        if len(actuals) != 1:
            raise CompileError("builtin output expects exactly 1 argument")
        actual = actuals[0]
        actual_type = self._expr_type(actual)
        actual_reg = self._compile_expr(actual, scope)
        saved_arg_reg: int | None = None
        if self.next_reg > _VALUE_PARAM_BASE_REG:
            saved_arg_reg = self._fresh_reg()
            self._copy_reg(dst=saved_arg_reg, src=_VALUE_PARAM_BASE_REG)
        if actual_type == _STRING_TYPE:
            self._emit_output_string(actual_reg, scope)
        elif actual_type == _BOOLEAN_TYPE:
            self._emit_output_boolean(actual_reg, scope)
        elif actual_type == _INTEGER_TYPE:
            self._emit_output_integer(actual_reg, scope)
        elif actual_type == _REAL_TYPE:
            self._emit_output_real(actual_reg, scope)
        else:
            raise CompileError(
                "builtin output currently supports integer, boolean, real, "
                "and string"
            )

        if saved_arg_reg is not None:
            self._copy_reg(dst=_VALUE_PARAM_BASE_REG, src=saved_arg_reg)
        return _ZERO_REG

    def _emit_output_chars(self, text: str, scope: _FrameScope) -> None:
        for char in text:
            self._emit_output_reg(self._const_reg(ord(char)), scope)

    def _emit_output_reg(self, value_reg: int, scope: _FrameScope) -> None:
        total_output_reg = self._fresh_reg()
        next_total_reg = self._fresh_reg()
        limit_exceeded_reg = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_OUTPUT_BYTES_OFFSET, total_output_reg)
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(next_total_reg),
            IrRegister(total_output_reg),
            IrImmediate(1),
        )
        self._emit(
            IrOp.CMP_GT,
            IrRegister(limit_exceeded_reg),
            IrRegister(next_total_reg),
            IrRegister(self._const_reg(_MAX_TOTAL_OUTPUT_BYTES)),
        )
        self._emit_runtime_failure_guard(limit_exceeded_reg, scope)
        self._store_runtime_state(_RUNTIME_OUTPUT_BYTES_OFFSET, next_total_reg)
        self._copy_reg(dst=_VALUE_PARAM_BASE_REG, src=value_reg)
        self._emit(
            IrOp.SYSCALL,
            IrImmediate(_WRITE_SYSCALL),
            IrRegister(_VALUE_PARAM_BASE_REG),
        )

    def _emit_output_string(self, pointer_reg: int, scope: _FrameScope) -> None:
        index = self.output_count
        self.output_count += 1
        loop_label = f"algol_label_output_string_{index}_loop"
        end_label = f"algol_label_output_string_{index}_end"
        data_guard_done_label = f"algol_label_output_string_{index}_data_guard_done"
        current_reg = self._fresh_reg()
        remaining_reg = self._fresh_reg()
        char_reg = self._fresh_reg()
        next_remaining_reg = self._fresh_reg()
        negative_length_reg = self._fresh_reg()
        too_large_reg = self._fresh_reg()
        missing_data_reg = self._fresh_reg()

        self._emit(IrOp.BRANCH_Z, IrRegister(pointer_reg), IrLabel(end_label))
        self._emit_load_scalar(
            _INTEGER_TYPE,
            remaining_reg,
            pointer_reg,
            _STRING_DESCRIPTOR_LENGTH_OFFSET,
        )
        self._emit_load_scalar(
            _INTEGER_TYPE,
            current_reg,
            pointer_reg,
            _STRING_DESCRIPTOR_DATA_POINTER_OFFSET,
        )
        self._emit(
            IrOp.CMP_LT,
            IrRegister(negative_length_reg),
            IrRegister(remaining_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit_runtime_failure_guard(negative_length_reg, scope)
        self._emit(
            IrOp.CMP_GT,
            IrRegister(too_large_reg),
            IrRegister(remaining_reg),
            IrRegister(self._const_reg(_MAX_STRING_OUTPUT_BYTES)),
        )
        self._emit_runtime_failure_guard(too_large_reg, scope)
        self._emit(
            IrOp.BRANCH_Z,
            IrRegister(remaining_reg),
            IrLabel(data_guard_done_label),
        )
        self._emit(
            IrOp.CMP_EQ,
            IrRegister(missing_data_reg),
            IrRegister(current_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit_runtime_failure_guard(missing_data_reg, scope)
        self._label(data_guard_done_label)
        self._label(loop_label)
        self._emit(IrOp.BRANCH_Z, IrRegister(remaining_reg), IrLabel(end_label))
        self._emit(
            IrOp.LOAD_BYTE,
            IrRegister(char_reg),
            IrRegister(current_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit_output_reg(char_reg, scope)
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(current_reg),
            IrRegister(current_reg),
            IrImmediate(1),
        )
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(next_remaining_reg),
            IrRegister(remaining_reg),
            IrImmediate(-1),
        )
        self._copy_reg(dst=remaining_reg, src=next_remaining_reg)
        self._emit(IrOp.JUMP, IrLabel(loop_label))
        self._label(end_label)

    def _emit_output_boolean(self, value_reg: int, scope: _FrameScope) -> None:
        index = self.output_count
        self.output_count += 1
        false_label = f"algol_label_output_bool_{index}_false"
        end_label = f"algol_label_output_bool_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(value_reg), IrLabel(false_label))
        self._emit_output_chars("true", scope)
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(false_label)
        self._emit_output_chars("false", scope)
        self._label(end_label)

    def _emit_output_integer(self, value_reg: int, scope: _FrameScope) -> None:
        index = self.output_count
        self.output_count += 1
        extract_loop_label = f"algol_label_output_int_{index}_extract_loop"
        emit_label = f"algol_label_output_int_{index}_emit"
        emit_loop_label = f"algol_label_output_int_{index}_emit_loop"
        end_label = f"algol_label_output_int_{index}_end"
        negative_label = f"algol_label_output_int_{index}_negative"
        zero_label = f"algol_label_output_int_{index}_zero"

        work_reg = self._fresh_reg()
        digit_count_reg = self._fresh_reg()
        buffer_base_reg = self._snapshot_heap_pointer()
        ten_reg = self._const_reg(10)
        ascii_zero_reg = self._const_reg(ord("0"))
        self._copy_reg(dst=work_reg, src=value_reg)
        self._emit(IrOp.LOAD_IMM, IrRegister(digit_count_reg), IrImmediate(0))

        is_negative_reg = self._fresh_reg()
        self._emit(
            IrOp.CMP_LT,
            IrRegister(is_negative_reg),
            IrRegister(work_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit(IrOp.BRANCH_Z, IrRegister(is_negative_reg), IrLabel(zero_label))
        self._label(negative_label)
        self._emit_output_chars("-", scope)
        self._label(zero_label)
        is_zero_reg = self._fresh_reg()
        self._emit(
            IrOp.CMP_EQ,
            IrRegister(is_zero_reg),
            IrRegister(work_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit(IrOp.BRANCH_Z, IrRegister(is_zero_reg), IrLabel(extract_loop_label))
        self._emit_output_chars("0", scope)
        self._emit(IrOp.JUMP, IrLabel(end_label))

        self._label(extract_loop_label)
        self._emit_extract_integer_digit(
            work_reg,
            digit_count_reg,
            buffer_base_reg,
            ten_reg,
            ascii_zero_reg,
            negative_reg=is_negative_reg,
        )
        continue_reg = self._fresh_reg()
        self._emit(
            IrOp.CMP_NE,
            IrRegister(continue_reg),
            IrRegister(work_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit(
            IrOp.BRANCH_NZ,
            IrRegister(continue_reg),
            IrLabel(extract_loop_label),
        )

        self._label(emit_label)
        emit_index_reg = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(emit_index_reg),
            IrRegister(digit_count_reg),
            IrImmediate(-1),
        )
        self._label(emit_loop_label)
        done_reg = self._fresh_reg()
        self._emit(
            IrOp.CMP_LT,
            IrRegister(done_reg),
            IrRegister(emit_index_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit(IrOp.BRANCH_NZ, IrRegister(done_reg), IrLabel(end_label))
        char_reg = self._fresh_reg()
        self._emit(
            IrOp.LOAD_BYTE,
            IrRegister(char_reg),
            IrRegister(buffer_base_reg),
            IrRegister(emit_index_reg),
        )
        self._emit_output_reg(char_reg, scope)
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(emit_index_reg),
            IrRegister(emit_index_reg),
            IrImmediate(-1),
        )
        self._emit(IrOp.JUMP, IrLabel(emit_loop_label))
        self._label(end_label)

    def _emit_output_real(self, value_reg: int, scope: _FrameScope) -> None:
        index = self.output_count
        self.output_count += 1
        positive_label = f"algol_label_output_real_{index}_positive"
        normalized_label = f"algol_label_output_real_{index}_normalized"
        no_carry_label = f"algol_label_output_real_{index}_no_carry"

        zero_f64 = self._const_f64_reg(0.0)
        abs_reg = self._fresh_reg()
        negative_reg = self._fresh_reg()
        self._emit(
            IrOp.F64_CMP_LT,
            IrRegister(negative_reg),
            IrRegister(value_reg),
            IrRegister(zero_f64),
        )
        self._emit(
            IrOp.BRANCH_Z,
            IrRegister(negative_reg),
            IrLabel(positive_label),
        )
        self._emit_output_chars("-", scope)
        self._emit(
            IrOp.F64_SUB,
            IrRegister(abs_reg),
            IrRegister(zero_f64),
            IrRegister(value_reg),
        )
        self._emit(IrOp.JUMP, IrLabel(normalized_label))
        self._label(positive_label)
        self._copy_scalar_reg(type_name=_REAL_TYPE, dst=abs_reg, src=value_reg)
        self._label(normalized_label)

        integer_part_reg = self._fresh_reg()
        self._emit(
            IrOp.I32_TRUNC_FROM_F64,
            IrRegister(integer_part_reg),
            IrRegister(abs_reg),
        )
        integer_part_f64 = self._coerce_reg_to_type(
            integer_part_reg,
            _INTEGER_TYPE,
            expected_type=_REAL_TYPE,
        )
        fractional_reg = self._fresh_reg()
        self._emit(
            IrOp.F64_SUB,
            IrRegister(fractional_reg),
            IrRegister(abs_reg),
            IrRegister(integer_part_f64),
        )
        thousand_f64 = self._const_f64_reg(1000.0)
        half_f64 = self._const_f64_reg(0.5)
        scaled_fraction_reg = self._fresh_reg()
        self._emit(
            IrOp.F64_MUL,
            IrRegister(scaled_fraction_reg),
            IrRegister(fractional_reg),
            IrRegister(thousand_f64),
        )
        rounded_fraction_reg = self._fresh_reg()
        self._emit(
            IrOp.F64_ADD,
            IrRegister(rounded_fraction_reg),
            IrRegister(scaled_fraction_reg),
            IrRegister(half_f64),
        )
        fraction_digits_reg = self._fresh_reg()
        self._emit(
            IrOp.I32_TRUNC_FROM_F64,
            IrRegister(fraction_digits_reg),
            IrRegister(rounded_fraction_reg),
        )
        carry_check_reg = self._fresh_reg()
        thousand_i32 = self._const_reg(1000)
        self._emit(
            IrOp.CMP_EQ,
            IrRegister(carry_check_reg),
            IrRegister(fraction_digits_reg),
            IrRegister(thousand_i32),
        )
        self._emit(
            IrOp.BRANCH_Z,
            IrRegister(carry_check_reg),
            IrLabel(no_carry_label),
        )
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(integer_part_reg),
            IrRegister(integer_part_reg),
            IrImmediate(1),
        )
        self._emit(
            IrOp.LOAD_IMM,
            IrRegister(fraction_digits_reg),
            IrImmediate(0),
        )
        self._label(no_carry_label)

        self._emit_output_integer(integer_part_reg, scope)
        self._emit_output_chars(".", scope)
        self._emit_output_three_digits(fraction_digits_reg, scope)

    def _emit_output_three_digits(self, value_reg: int, scope: _FrameScope) -> None:
        hundred_reg = self._const_reg(100)
        ten_reg = self._const_reg(10)
        ascii_zero_reg = self._const_reg(ord("0"))

        hundreds_reg = self._fresh_reg()
        self._emit(
            IrOp.DIV,
            IrRegister(hundreds_reg),
            IrRegister(value_reg),
            IrRegister(hundred_reg),
        )
        hundreds_product_reg = self._fresh_reg()
        self._emit(
            IrOp.MUL,
            IrRegister(hundreds_product_reg),
            IrRegister(hundreds_reg),
            IrRegister(hundred_reg),
        )
        remainder_reg = self._fresh_reg()
        self._emit(
            IrOp.SUB,
            IrRegister(remainder_reg),
            IrRegister(value_reg),
            IrRegister(hundreds_product_reg),
        )
        tens_reg = self._fresh_reg()
        self._emit(
            IrOp.DIV,
            IrRegister(tens_reg),
            IrRegister(remainder_reg),
            IrRegister(ten_reg),
        )
        tens_product_reg = self._fresh_reg()
        self._emit(
            IrOp.MUL,
            IrRegister(tens_product_reg),
            IrRegister(tens_reg),
            IrRegister(ten_reg),
        )
        ones_reg = self._fresh_reg()
        self._emit(
            IrOp.SUB,
            IrRegister(ones_reg),
            IrRegister(remainder_reg),
            IrRegister(tens_product_reg),
        )
        for digit_reg in (hundreds_reg, tens_reg, ones_reg):
            ascii_reg = self._fresh_reg()
            self._emit(
                IrOp.ADD,
                IrRegister(ascii_reg),
                IrRegister(digit_reg),
                IrRegister(ascii_zero_reg),
            )
            self._emit_output_reg(ascii_reg, scope)

    def _emit_extract_integer_digit(
        self,
        work_reg: int,
        digit_count_reg: int,
        buffer_base_reg: int,
        ten_reg: int,
        ascii_zero_reg: int,
        *,
        negative_reg: int,
    ) -> None:
        quotient_reg = self._fresh_reg()
        product_reg = self._fresh_reg()
        digit_reg = self._fresh_reg()
        ascii_reg = self._fresh_reg()
        self._emit(
            IrOp.DIV,
            IrRegister(quotient_reg),
            IrRegister(work_reg),
            IrRegister(ten_reg),
        )
        self._emit(
            IrOp.MUL,
            IrRegister(product_reg),
            IrRegister(quotient_reg),
            IrRegister(ten_reg),
        )
        self._emit(
            IrOp.SUB,
            IrRegister(digit_reg),
            IrRegister(work_reg),
            IrRegister(product_reg),
        )
        invert_label = f"algol_label_output_digit_{self.output_count}_invert"
        continue_label = f"algol_label_output_digit_{self.output_count}_continue"
        self.output_count += 1
        self._emit(
            IrOp.BRANCH_Z,
            IrRegister(negative_reg),
            IrLabel(continue_label),
        )
        self._label(invert_label)
        self._emit(
            IrOp.SUB,
            IrRegister(digit_reg),
            IrRegister(_ZERO_REG),
            IrRegister(digit_reg),
        )
        self._label(continue_label)
        self._emit(
            IrOp.ADD,
            IrRegister(ascii_reg),
            IrRegister(digit_reg),
            IrRegister(ascii_zero_reg),
        )
        self._emit(
            IrOp.STORE_BYTE,
            IrRegister(ascii_reg),
            IrRegister(buffer_base_reg),
            IrRegister(digit_count_reg),
        )
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(digit_count_reg),
            IrRegister(digit_count_reg),
            IrImmediate(1),
        )
        self._copy_reg(dst=work_reg, src=quotient_reg)

    def _requires_by_name_thunk_descriptor(self, argument: ASTNode) -> bool:
        variable = _single_variable_expr(argument)
        return variable is None or bool(_variable_subscripts(variable))

    def _compile_by_name_actual_pointer(
        self,
        argument: ASTNode,
        parameter: ProcedureParameter,
        scope: _FrameScope,
        descriptor: int | None,
    ) -> int:
        variable = _single_variable_expr(argument)
        if variable is None:
            if descriptor is None:
                raise CompileError(
                    "missing reserved eval thunk descriptor for by-name expression"
                )
            return self._compile_eval_thunk_actual(
                argument,
                parameter,
                scope,
                descriptor,
            )
        if _variable_subscripts(variable):
            if descriptor is None:
                raise CompileError(
                    "missing reserved eval thunk descriptor for by-name array element"
                )
            return self._compile_array_element_thunk_actual(
                variable,
                parameter,
                scope,
                descriptor,
            )
        name = _variable_name(variable)
        if name is None:
            raise CompileError("by-name scalar actual is missing a name")
        reference = self._require_reference(name, "read")
        return self._emit_reference_pointer(reference, scope)

    def _compile_array_actual_pointer(
        self,
        argument: ASTNode,
        scope: _FrameScope,
    ) -> int:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            raise CompileError("array parameter actual must be a whole-array variable")
        name = _variable_name(variable)
        if name is None:
            raise CompileError("array parameter actual is missing a name")
        access = self._require_array_access(name, "actual")
        return self._emit_load_array_descriptor(access, scope)

    def _compile_switch_actual_pointer(
        self,
        argument: ASTNode,
        scope: _FrameScope,
        descriptor: int | None,
    ) -> int:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            raise CompileError("switch parameter actual must be a direct switch")
        name = _variable_name(variable)
        if name is None:
            raise CompileError("switch parameter actual is missing a name")
        resolved = self._resolve_symbol_in_scope_chain(name.value, scope)
        if resolved is None:
            raise CompileError(f"switch actual {name.value!r} was not resolved")
        symbol, lexical_depth_delta = resolved
        if symbol.kind != "switch":
            raise CompileError(
                f"switch parameter actual {name.value!r} is not a switch"
            )
        if symbol.parameter_mode is not None:
            if symbol.slot_offset is None:
                raise CompileError(
                    f"switch parameter actual {name.value!r} has no planned frame slot"
                )
            frame_reg = self._emit_frame_for_lexical_depth(scope, lexical_depth_delta)
            pointer = self._fresh_reg()
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(pointer),
                IrRegister(frame_reg),
                IrRegister(self._const_reg(symbol.slot_offset)),
            )
            return pointer
        if symbol.switch_id is None:
            raise CompileError(f"switch actual {name.value!r} has no descriptor")
        if descriptor is None:
            raise CompileError(
                f"missing reserved descriptor for switch actual {name.value!r}"
            )
        caller_frame = self._emit_frame_for_lexical_depth(scope, lexical_depth_delta)
        self._emit_write_switch_descriptor(
            descriptor,
            symbol.switch_id,
            caller_frame,
        )
        return descriptor

    def _compile_label_actual_value(
        self,
        argument: ASTNode,
        scope: _FrameScope,
    ) -> int:
        variable = _single_variable_expr(argument)
        if variable is None or _variable_subscripts(variable):
            raise CompileError("label parameter actual must be a direct label")
        name = _variable_name(variable)
        if name is None:
            raise CompileError("label parameter actual is missing a name")
        resolved = self._resolve_symbol_in_scope_chain(name.value, scope)
        if resolved is None:
            raise CompileError(f"label actual {name.value!r} was not resolved")
        symbol, lexical_depth_delta = resolved
        if symbol.kind != "label":
            raise CompileError(
                f"label parameter actual {name.value!r} is not a label"
            )
        if symbol.parameter_mode is not None:
            if symbol.slot_offset is None:
                raise CompileError(
                    f"label parameter actual {name.value!r} has no planned frame slot"
                )
            frame_reg = self._emit_frame_for_lexical_depth(scope, lexical_depth_delta)
            value_reg = self._fresh_reg()
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(value_reg),
                IrRegister(frame_reg),
                IrRegister(self._const_reg(symbol.slot_offset)),
            )
            return value_reg
        label = self.labels_by_symbol.get(symbol.symbol_id)
        if label is None:
            raise CompileError(f"label actual {name.value!r} has no descriptor")
        return self._const_reg(label.label_id)

    def _emit_call_switch_eval(
        self,
        descriptor_pointer: int,
        index_value: int,
        scope: _FrameScope,
    ) -> int:
        active_thunk_heap_mark = (
            scope.active_thunk_heap_mark_reg
            if scope.active_thunk_heap_mark_reg is not None
            else self._const_reg(0)
        )
        self._emit(
            IrOp.CALL,
            IrLabel(_SWITCH_EVAL_LABEL),
            IrRegister(descriptor_pointer),
            IrRegister(active_thunk_heap_mark),
            IrRegister(index_value),
        )
        self._emit_propagate_thunk_failure(scope)
        result = self._fresh_reg()
        self._copy_reg(dst=result, src=_RESULT_REG)
        return result

    def _compile_eval_thunk_actual(
        self,
        argument: ASTNode,
        parameter: ProcedureParameter,
        scope: _FrameScope,
        descriptor: int,
    ) -> int:
        thunk = self._register_eval_thunk(
            argument,
            scope.block_id,
            type_name=parameter.type_name,
        )
        self._emit_write_eval_thunk_descriptor(thunk, scope, descriptor)
        tagged_descriptor = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(tagged_descriptor),
            IrRegister(descriptor),
            IrImmediate(_THUNK_DESCRIPTOR_TAG),
        )
        return tagged_descriptor

    def _compile_array_element_thunk_actual(
        self,
        variable: ASTNode,
        parameter: ProcedureParameter,
        scope: _FrameScope,
        descriptor: int,
    ) -> int:
        thunk = self._register_eval_thunk(
            variable,
            scope.block_id,
            type_name=parameter.type_name,
            is_array_element=True,
            store_capable=parameter.may_write,
        )
        self._emit_write_eval_thunk_descriptor(thunk, scope, descriptor)
        tagged_descriptor = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(tagged_descriptor),
            IrRegister(descriptor),
            IrImmediate(_THUNK_DESCRIPTOR_TAG),
        )
        return tagged_descriptor

    def _require_eval_thunk_expression(
        self,
        argument: ASTNode,
        parameter: ProcedureParameter,
    ) -> None:
        # Read-only integer expression thunks now share the normal expression path.
        del argument, parameter

    def _register_eval_thunk(
        self,
        argument: ASTNode,
        block_id: int,
        *,
        type_name: str,
        is_array_element: bool = False,
        store_capable: bool = False,
    ) -> _EvalThunk:
        if len(self.eval_thunks) >= _MAX_EVAL_THUNKS:
            raise CompileError(
                "ALGOL program requires more than "
                f"{_MAX_EVAL_THUNKS} by-name eval thunks"
            )
        thunk = _EvalThunk(
            thunk_id=len(self.eval_thunks) + 1,
            expression=argument,
            block_id=block_id,
            type_name=type_name,
            is_array_element=is_array_element,
            store_capable=store_capable,
        )
        self.eval_thunks.append(thunk)
        return thunk

    def _emit_reserve_eval_thunk_descriptors(
        self,
        count: int,
        scope: _FrameScope,
    ) -> int:
        return self._emit_reserve_temp_descriptor_space(
            count * _THUNK_DESCRIPTOR_SIZE,
            scope,
        )

    def _emit_reserve_temp_descriptor_space(
        self,
        byte_count: int,
        scope: _FrameScope,
    ) -> int:
        descriptor_base = self._fresh_reg()
        heap_limit = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, descriptor_base)
        self._load_runtime_state(_RUNTIME_HEAP_LIMIT_OFFSET, heap_limit)
        next_heap_pointer = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(next_heap_pointer),
            IrRegister(descriptor_base),
            IrImmediate(byte_count),
        )
        exhausted = self._fresh_reg()
        self._emit(
            IrOp.CMP_GT,
            IrRegister(exhausted),
            IrRegister(next_heap_pointer),
            IrRegister(heap_limit),
        )
        self._emit_runtime_failure_guard(exhausted, scope)
        self._store_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, next_heap_pointer)
        return descriptor_base

    def _emit_descriptor_at(self, descriptor_base: int, offset: int) -> int:
        if offset == 0:
            return descriptor_base
        descriptor = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(descriptor),
            IrRegister(descriptor_base),
            IrImmediate(offset),
        )
        return descriptor

    def _emit_write_eval_thunk_descriptor(
        self,
        thunk: _EvalThunk,
        scope: _FrameScope,
        descriptor: int,
    ) -> None:
        self._store_word_const(
            descriptor,
            _THUNK_CODE_ID_OFFSET,
            thunk.thunk_id,
        )
        self._store_word_reg(
            value_reg=scope.frame_base_reg,
            base_reg=descriptor,
            offset=_THUNK_CALLER_FRAME_OFFSET,
        )
        self._store_word_const(
            descriptor,
            _THUNK_FLAGS_OFFSET,
            _THUNK_FLAG_STORE if thunk.store_capable else 0,
        )

    def _emit_write_switch_descriptor(
        self,
        descriptor: int,
        switch_id: int,
        caller_frame_reg: int,
    ) -> None:
        self._store_word_const(
            descriptor,
            _SWITCH_DESCRIPTOR_ID_OFFSET,
            switch_id,
        )
        self._store_word_reg(
            value_reg=caller_frame_reg,
            base_reg=descriptor,
            offset=_SWITCH_DESCRIPTOR_CALLER_FRAME_OFFSET,
        )

    def _compile_procedures(
        self, procedures: list[ProcedureDescriptor]
    ) -> None:
        for procedure in procedures:
            self._compile_procedure(procedure)

    def _compile_eval_thunk_dispatcher(self, *, label: str, thunk_kind: str) -> None:
        previous_return_type = self.current_function_return_type
        self.current_function_return_type = (
            _REAL_TYPE if thunk_kind == _REAL_TYPE else _INTEGER_TYPE
        )
        self._label(label)
        descriptor = _STATIC_LINK_PARAM_REG
        active_thunk_heap_mark = _THUNK_HEAP_MARK_PARAM_REG
        code_id = self._fresh_reg()
        caller_frame = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(code_id),
            IrRegister(descriptor),
            IrRegister(self._const_reg(_THUNK_CODE_ID_OFFSET)),
        )
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(caller_frame),
            IrRegister(descriptor),
            IrRegister(self._const_reg(_THUNK_CALLER_FRAME_OFFSET)),
        )
        for thunk in self.eval_thunks:
            if (thunk_kind == _REAL_TYPE) != (thunk.type_name == _REAL_TYPE):
                continue
            index = self.if_count
            self.if_count += 1
            else_label = f"if_{index}_else"
            end_label = f"if_{index}_end"
            matches = self._fresh_reg()
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(matches),
                IrRegister(code_id),
                IrRegister(self._const_reg(thunk.thunk_id)),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(matches), IrLabel(else_label))
            self._emit_enter_thunk_helper()
            thunk_scope = _FrameScope(
                semantic_block=self.semantic_blocks_by_id[thunk.block_id],
                frame_base_reg=caller_frame,
                heap_mark_reg=None,
                parent=None,
                goto_parent=None,
                function_owner_procedure_id=None,
                active_thunk_heap_mark_reg=active_thunk_heap_mark,
                helper_failure=True,
            )
            if thunk.is_array_element:
                data_pointer, byte_offset = self._compile_array_element_address(
                    thunk.expression,
                    thunk_scope,
                    role="read",
                    helper_failure=True,
                )
                value = self._fresh_reg()
                self._emit(
                    IrOp.LOAD_F64 if thunk.type_name == _REAL_TYPE else IrOp.LOAD_WORD,
                    IrRegister(value),
                    IrRegister(data_pointer),
                    IrRegister(byte_offset),
                )
            else:
                value = self._compile_expr(thunk.expression, thunk_scope)
            self._copy_scalar_reg(type_name=thunk.type_name, dst=_RESULT_REG, src=value)
            self._emit_leave_thunk_helper()
            self._emit(IrOp.RET)
            self._emit(IrOp.JUMP, IrLabel(end_label))
            self._label(else_label)
            self._label(end_label)
        self._emit_zero_result_reg()
        self._emit(IrOp.RET)
        self.current_function_return_type = previous_return_type

    def _compile_store_thunk_dispatcher(self, *, label: str, thunk_kind: str) -> None:
        previous_return_type = self.current_function_return_type
        self.current_function_return_type = _INTEGER_TYPE
        self._label(label)
        descriptor = _STATIC_LINK_PARAM_REG
        active_thunk_heap_mark = _THUNK_HEAP_MARK_PARAM_REG
        value_reg = _VALUE_PARAM_BASE_REG
        code_id = self._fresh_reg()
        caller_frame = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(code_id),
            IrRegister(descriptor),
            IrRegister(self._const_reg(_THUNK_CODE_ID_OFFSET)),
        )
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(caller_frame),
            IrRegister(descriptor),
            IrRegister(self._const_reg(_THUNK_CALLER_FRAME_OFFSET)),
        )
        for thunk in self.eval_thunks:
            if not thunk.store_capable:
                continue
            if (thunk_kind == _REAL_TYPE) != (thunk.type_name == _REAL_TYPE):
                continue
            index = self.if_count
            self.if_count += 1
            else_label = f"if_{index}_else"
            end_label = f"if_{index}_end"
            matches = self._fresh_reg()
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(matches),
                IrRegister(code_id),
                IrRegister(self._const_reg(thunk.thunk_id)),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(matches), IrLabel(else_label))
            self._emit_enter_thunk_helper()
            thunk_scope = _FrameScope(
                semantic_block=self.semantic_blocks_by_id[thunk.block_id],
                frame_base_reg=caller_frame,
                heap_mark_reg=None,
                parent=None,
                goto_parent=None,
                function_owner_procedure_id=None,
                active_thunk_heap_mark_reg=active_thunk_heap_mark,
                helper_failure=True,
            )
            data_pointer, byte_offset = self._compile_array_element_address(
                thunk.expression,
                thunk_scope,
                role="write",
                helper_failure=True,
            )
            self._emit(
                IrOp.STORE_F64 if thunk.type_name == _REAL_TYPE else IrOp.STORE_WORD,
                IrRegister(value_reg),
                IrRegister(data_pointer),
                IrRegister(byte_offset),
            )
            self._emit(IrOp.LOAD_IMM, IrRegister(_RESULT_REG), IrImmediate(0))
            self._emit_leave_thunk_helper()
            self._emit(IrOp.RET)
            self._emit(IrOp.JUMP, IrLabel(end_label))
            self._label(else_label)
            self._label(end_label)
        self._store_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, self._const_reg(1))
        self._emit(IrOp.LOAD_IMM, IrRegister(_RESULT_REG), IrImmediate(0))
        self._emit(IrOp.RET)
        self.current_function_return_type = previous_return_type

    def _compile_switch_eval_dispatcher(self) -> None:
        previous_return_type = self.current_function_return_type
        self.current_function_return_type = _INTEGER_TYPE
        self._label(_SWITCH_EVAL_LABEL)
        descriptor = _STATIC_LINK_PARAM_REG
        active_thunk_heap_mark = _THUNK_HEAP_MARK_PARAM_REG
        index_value = _VALUE_PARAM_BASE_REG
        switch_id = self._fresh_reg()
        caller_frame = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(switch_id),
            IrRegister(descriptor),
            IrRegister(self._const_reg(_SWITCH_DESCRIPTOR_ID_OFFSET)),
        )
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(caller_frame),
            IrRegister(descriptor),
            IrRegister(self._const_reg(_SWITCH_DESCRIPTOR_CALLER_FRAME_OFFSET)),
        )
        for switch in self.switches.values():
            index = self.if_count
            self.if_count += 1
            else_label = f"if_{index}_else"
            end_label = f"if_{index}_end"
            matches = self._fresh_reg()
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(matches),
                IrRegister(switch_id),
                IrRegister(self._const_reg(switch.switch_id)),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(matches), IrLabel(else_label))
            self._emit_enter_thunk_helper()
            switch_scope = _FrameScope(
                semantic_block=self.semantic_blocks_by_id[switch.declaring_block_id],
                frame_base_reg=caller_frame,
                heap_mark_reg=None,
                parent=None,
                goto_parent=None,
                function_owner_procedure_id=None,
                active_thunk_heap_mark_reg=active_thunk_heap_mark,
                helper_failure=True,
            )
            selection = ResolvedSwitchSelection(
                node_id=-1,
                token_id=-1,
                name=switch.name,
                switch_id=switch.switch_id,
                use_block_id=switch.declaring_block_id,
                declaration_block_id=switch.declaring_block_id,
                lexical_depth_delta=0,
                index_node_id=-1,
                line=switch.line,
                column=switch.column,
            )
            label_value = self._compile_switch_entries_value(
                switch,
                selection,
                index_value,
                switch_scope,
            )
            self._copy_reg(dst=_RESULT_REG, src=label_value)
            self._emit_leave_thunk_helper()
            self._emit(IrOp.RET)
            self._emit(IrOp.JUMP, IrLabel(end_label))
            self._label(else_label)
            self._label(end_label)
        self._store_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, self._const_reg(1))
        self._emit_zero_result_reg()
        self._emit(IrOp.RET)
        self.current_function_return_type = previous_return_type

    def _compile_procedure(self, procedure: ProcedureDescriptor) -> None:
        body = self._find_ast_by_id(procedure.body_node_id)
        if body is None:
            raise CompileError(f"missing body AST for procedure {procedure.name!r}")
        semantic_block = self.semantic_blocks_by_id[procedure.body_block_id]
        previous_return_type = self.current_function_return_type
        self.current_function_return_type = procedure.return_type
        self._label(procedure.label)
        heap_mark_reg = self._snapshot_heap_pointer()
        frame_base_reg = self._emit_enter_frame(
            semantic_block,
            _STATIC_LINK_PARAM_REG,
            heap_mark_reg,
        )
        parent: _FrameScope | None = None
        parent_block_id = semantic_block.frame_layout.static_parent_id
        next_parent_reg = _STATIC_LINK_PARAM_REG
        while parent_block_id is not None:
            parent_block = self.semantic_blocks_by_id.get(parent_block_id)
            if parent_block is None:
                raise CompileError(
                    f"missing semantic block {parent_block_id} for procedure "
                    f"{procedure.name!r} parent chain"
                )
            parent = _FrameScope(
                semantic_block=parent_block,
                frame_base_reg=next_parent_reg,
                heap_mark_reg=None,
                parent=parent,
                goto_parent=parent,
                function_owner_procedure_id=procedure.procedure_id,
                active_thunk_heap_mark_reg=None,
            )
            parent_block_id = parent_block.frame_layout.static_parent_id
            if parent_block_id is not None:
                next_parent_reg_next = self._fresh_reg()
                self._emit(
                    IrOp.LOAD_WORD,
                    IrRegister(next_parent_reg_next),
                    IrRegister(next_parent_reg),
                    IrRegister(self._const_reg(_STATIC_LINK_OFFSET)),
                )
                next_parent_reg = next_parent_reg_next
        scope = _FrameScope(
            semantic_block=semantic_block,
            frame_base_reg=frame_base_reg,
            heap_mark_reg=heap_mark_reg,
            parent=None,
            goto_parent=parent,
            function_owner_procedure_id=procedure.procedure_id,
            active_thunk_heap_mark_reg=_THUNK_HEAP_MARK_PARAM_REG,
        )
        self._initialize_scalar_slots(scope)
        for index, parameter in enumerate(procedure.parameters):
            if parameter.kind == "array" or parameter.mode == _NAME_MODE:
                self._store_word_reg(
                    value_reg=_VALUE_PARAM_BASE_REG + index,
                    base_reg=scope.frame_base_reg,
                    offset=parameter.slot_offset,
                )
            else:
                self._store_scalar_reg(
                    type_name=parameter.type_name,
                    value_reg=_VALUE_PARAM_BASE_REG + index,
                    base_reg=scope.frame_base_reg,
                    offset=parameter.slot_offset,
                )
        self._allocate_arrays(scope)

        if body.rule_name == "block":
            for statement in _direct_nodes(body, "statement"):
                self._compile_statement(statement, scope)
        else:
            self._compile_statement(body, scope)

        if procedure.return_type is not None:
            result_symbol = semantic_block.scope.symbols.get(procedure.name)
            if result_symbol is None:
                raise CompileError(
                    f"procedure {procedure.name!r} has no result slot"
                )
            self._emit_load_symbol(result_symbol, scope, _RESULT_REG)
        else:
            self._emit(IrOp.LOAD_IMM, IrRegister(_RESULT_REG), IrImmediate(0))
        self._emit_leave_frame(scope)
        self._emit(IrOp.RET)
        self.current_function_return_type = previous_return_type

    def _compile_array_load(self, variable: ASTNode, scope: _FrameScope) -> int:
        name = _variable_head_name(variable)
        if name is None:
            raise CompileError("array access is missing a name")
        access = self._require_array_access(name, "read")
        data_pointer, byte_offset = self._compile_array_element_address(
            variable,
            scope,
            role="read",
            helper_failure=scope.helper_failure,
        )
        dst = self._fresh_reg()
        self._emit_load_scalar_at_reg(
            self.arrays[access.array_id].element_type,
            dst,
            data_pointer,
            byte_offset,
        )
        return dst

    def _compile_array_store(
        self,
        variable: ASTNode,
        scope: _FrameScope,
        value_reg: int,
    ) -> None:
        name = _variable_head_name(variable)
        if name is None:
            raise CompileError("array access is missing a name")
        access = self._require_array_access(name, "write")
        data_pointer, byte_offset = self._compile_array_element_address(
            variable,
            scope,
            role="write",
        )
        self._emit_store_scalar_at_reg(
            self.arrays[access.array_id].element_type,
            value_reg,
            data_pointer,
            byte_offset,
        )

    def _compile_array_element_address(
        self,
        variable: ASTNode,
        scope: _FrameScope,
        *,
        role: str,
        helper_failure: bool = False,
    ) -> tuple[int, int]:
        name = _variable_head_name(variable)
        if name is None:
            raise CompileError("array access is missing a name")
        access = self._require_array_access(name, role)
        if access.use_block_id != scope.block_id:
            raise CompileError(
                f"array access {access.name!r} was resolved for block "
                f"{access.use_block_id}, but codegen is in block {scope.block_id}"
            )
        descriptor_pointer = self._emit_load_array_descriptor(access, scope)
        data_pointer = self._fresh_reg()
        bounds_pointer = self._fresh_reg()
        data_offset = self._const_reg(_ARRAY_DATA_POINTER_OFFSET)
        bounds_offset = self._const_reg(_ARRAY_BOUNDS_POINTER_OFFSET)
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(data_pointer),
            IrRegister(descriptor_pointer),
            IrRegister(data_offset),
        )
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(bounds_pointer),
            IrRegister(descriptor_pointer),
            IrRegister(bounds_offset),
        )

        subscripts = _variable_subscripts(variable)
        dimension_count = self._fresh_reg()
        dimension_mismatch = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(dimension_count),
            IrRegister(descriptor_pointer),
            IrRegister(self._const_reg(_ARRAY_DIMENSION_COUNT_OFFSET)),
        )
        self._emit(
            IrOp.CMP_NE,
            IrRegister(dimension_mismatch),
            IrRegister(dimension_count),
            IrRegister(self._const_reg(len(subscripts))),
        )
        if helper_failure:
            self._emit_helper_runtime_failure_guard(dimension_mismatch, scope)
        else:
            self._emit_runtime_failure_guard(dimension_mismatch, scope)

        element_index = self._const_reg(0)
        for index, subscript in enumerate(subscripts):
            subscript_reg = self._compile_expr(subscript, scope)
            lower = self._fresh_reg()
            upper = self._fresh_reg()
            stride = self._fresh_reg()
            dim_offset = index * _ARRAY_DIMENSION_ENTRY_SIZE
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(lower),
                IrRegister(bounds_pointer),
                IrRegister(self._const_reg(dim_offset + _ARRAY_DIM_LOWER_OFFSET)),
            )
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(upper),
                IrRegister(bounds_pointer),
                IrRegister(self._const_reg(dim_offset + _ARRAY_DIM_UPPER_OFFSET)),
            )
            below = self._fresh_reg()
            above = self._fresh_reg()
            self._emit(
                IrOp.CMP_GT,
                IrRegister(below),
                IrRegister(lower),
                IrRegister(subscript_reg),
            )
            if helper_failure:
                self._emit_helper_runtime_failure_guard(below, scope)
            else:
                self._emit_runtime_failure_guard(below, scope)
            self._emit(
                IrOp.CMP_GT,
                IrRegister(above),
                IrRegister(subscript_reg),
                IrRegister(upper),
            )
            if helper_failure:
                self._emit_helper_runtime_failure_guard(above, scope)
            else:
                self._emit_runtime_failure_guard(above, scope)
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(stride),
                IrRegister(bounds_pointer),
                IrRegister(self._const_reg(dim_offset + _ARRAY_DIM_STRIDE_OFFSET)),
            )
            adjusted = self._fresh_reg()
            self._emit(
                IrOp.SUB,
                IrRegister(adjusted),
                IrRegister(subscript_reg),
                IrRegister(lower),
            )
            scaled = self._fresh_reg()
            self._emit(
                IrOp.MUL,
                IrRegister(scaled),
                IrRegister(adjusted),
                IrRegister(stride),
            )
            next_index = self._fresh_reg()
            self._emit(
                IrOp.ADD,
                IrRegister(next_index),
                IrRegister(element_index),
                IrRegister(scaled),
            )
            element_index = next_index

        element_width = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(element_width),
            IrRegister(descriptor_pointer),
            IrRegister(self._const_reg(_ARRAY_ELEMENT_WIDTH_OFFSET)),
        )
        byte_offset = self._fresh_reg()
        self._emit(
            IrOp.MUL,
            IrRegister(byte_offset),
            IrRegister(element_index),
            IrRegister(element_width),
        )
        return data_pointer, byte_offset

    def _emit_load_reference(
        self, reference: ResolvedReference, scope: _FrameScope
    ) -> int:
        if self._is_by_name_reference(reference):
            return self._emit_load_by_name_reference(reference, scope)
        pointer = self._emit_reference_pointer(reference, scope)
        dst = self._fresh_reg()
        self._emit_load_scalar(reference.type_name, dst, pointer, 0)
        return dst

    def _emit_load_by_name_reference(
        self,
        reference: ResolvedReference,
        scope: _FrameScope,
    ) -> int:
        pointer = self._emit_reference_pointer(reference, scope)
        tag = self._fresh_reg()
        index = self.if_count
        self.if_count += 1
        storage_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        dst = self._fresh_reg()
        self._emit(
            IrOp.AND_IMM,
            IrRegister(tag),
            IrRegister(pointer),
            IrImmediate(_THUNK_DESCRIPTOR_TAG),
        )
        self._emit(IrOp.BRANCH_Z, IrRegister(tag), IrLabel(storage_label))
        descriptor = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(descriptor),
            IrRegister(pointer),
            IrImmediate(-_THUNK_DESCRIPTOR_TAG),
        )
        active_thunk_heap_mark = (
            scope.active_thunk_heap_mark_reg
            if scope.active_thunk_heap_mark_reg is not None
            else self._const_reg(0)
        )
        self._emit(
            IrOp.CALL,
            IrLabel(
                _THUNK_EVAL_REAL_LABEL
                if reference.type_name == _REAL_TYPE
                else _THUNK_EVAL_LABEL
            ),
            IrRegister(descriptor),
            IrRegister(active_thunk_heap_mark),
        )
        self._emit_propagate_thunk_failure(scope)
        self._emit_handle_pending_goto_after_call(scope)
        self._copy_scalar_reg(
            type_name=reference.type_name,
            dst=dst,
            src=_REAL_RESULT_REG if reference.type_name == _REAL_TYPE else _RESULT_REG,
        )
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(storage_label)
        self._emit_load_scalar(reference.type_name, dst, pointer, 0)
        self._label(end_label)
        return dst

    def _emit_reference_pointer(
        self, reference: ResolvedReference, scope: _FrameScope
    ) -> int:
        if reference.storage_class == "static":
            pointer = self._fresh_reg()
            self._emit(
                IrOp.LOAD_ADDR,
                IrRegister(pointer),
                IrLabel(_STATIC_MEMORY_LABEL),
            )
            if reference.slot_offset != 0:
                adjusted = self._fresh_reg()
                self._emit(
                    IrOp.ADD_IMM,
                    IrRegister(adjusted),
                    IrRegister(pointer),
                    IrImmediate(reference.slot_offset),
                )
                return adjusted
            return pointer
        frame_reg = self._emit_frame_for_reference(reference, scope)
        if self._is_by_name_reference(reference):
            pointer = self._fresh_reg()
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(pointer),
                IrRegister(frame_reg),
                IrRegister(self._const_reg(reference.slot_offset)),
            )
            return pointer
        pointer = self._fresh_reg()
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(pointer),
            IrRegister(frame_reg),
            IrImmediate(reference.slot_offset),
        )
        return pointer

    def _is_by_name_reference(self, reference: ResolvedReference) -> bool:
        parameter = self.parameters_by_symbol.get(reference.symbol_id)
        return (
            parameter is not None
            and parameter.mode == _NAME_MODE
            and parameter.kind == "scalar"
        )

    def _emit_store_reference(
        self,
        reference: ResolvedReference,
        scope: _FrameScope,
        value_reg: int,
    ) -> None:
        pointer = self._emit_reference_pointer(reference, scope)
        if self._is_by_name_reference(reference):
            tag = self._fresh_reg()
            index = self.if_count
            self.if_count += 1
            storage_label = f"if_{index}_else"
            end_label = f"if_{index}_end"
            self._emit(
                IrOp.AND_IMM,
                IrRegister(tag),
                IrRegister(pointer),
                IrImmediate(_THUNK_DESCRIPTOR_TAG),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(tag), IrLabel(storage_label))
            descriptor = self._fresh_reg()
            self._emit(
                IrOp.ADD_IMM,
                IrRegister(descriptor),
                IrRegister(pointer),
                IrImmediate(-_THUNK_DESCRIPTOR_TAG),
            )
            flags = self._fresh_reg()
            has_store = self._fresh_reg()
            no_store = self._fresh_reg()
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(flags),
                IrRegister(descriptor),
                IrRegister(self._const_reg(_THUNK_FLAGS_OFFSET)),
            )
            self._emit(
                IrOp.AND_IMM,
                IrRegister(has_store),
                IrRegister(flags),
                IrImmediate(_THUNK_FLAG_STORE),
            )
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(no_store),
                IrRegister(has_store),
                IrRegister(_ZERO_REG),
            )
            self._emit_runtime_failure_guard(no_store, scope)
            active_thunk_heap_mark = (
                scope.active_thunk_heap_mark_reg
                if scope.active_thunk_heap_mark_reg is not None
                else self._const_reg(0)
            )
            self._emit(
                IrOp.CALL,
                IrLabel(
                    _THUNK_STORE_REAL_LABEL
                    if reference.type_name == _REAL_TYPE
                    else _THUNK_STORE_LABEL
                ),
                IrRegister(descriptor),
                IrRegister(active_thunk_heap_mark),
                IrRegister(value_reg),
            )
            self._emit_propagate_thunk_failure(scope)
            self._emit_handle_pending_goto_after_call(scope)
            self._emit(IrOp.JUMP, IrLabel(end_label))
            self._label(storage_label)
            self._emit_store_scalar(
                reference.type_name,
                value_reg,
                pointer,
                0,
            )
            self._label(end_label)
            return
        self._emit_store_scalar(reference.type_name, value_reg, pointer, 0)

    def _emit_frame_for_reference(
        self, reference: ResolvedReference, scope: _FrameScope
    ) -> int:
        if reference.use_block_id != scope.block_id:
            raise CompileError(
                f"reference {reference.name!r} was resolved for block "
                f"{reference.use_block_id}, but codegen is in block {scope.block_id}"
            )
        return self._emit_frame_for_lexical_depth(scope, reference.lexical_depth_delta)

    def _emit_frame_for_lexical_depth(
        self,
        scope: _FrameScope,
        lexical_depth_delta: int,
    ) -> int:
        frame_reg = scope.frame_base_reg
        for _ in range(lexical_depth_delta):
            next_frame = self._fresh_reg()
            offset_reg = self._const_reg(_STATIC_LINK_OFFSET)
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(next_frame),
                IrRegister(frame_reg),
                IrRegister(offset_reg),
            )
            frame_reg = next_frame
        return frame_reg

    def _resolve_symbol_in_scope_chain(
        self,
        name: str,
        scope: _FrameScope,
    ) -> tuple[Symbol, int] | None:
        resolved = scope.semantic_block.scope.resolve_with_scope(name)
        if resolved is None:
            return None
        symbol, _, lexical_depth_delta = resolved
        return symbol, lexical_depth_delta

    def _emit_load_array_descriptor(
        self,
        access: ResolvedArrayAccess,
        scope: _FrameScope,
    ) -> int:
        array = self.arrays[access.array_id]
        if array.storage_class == "static":
            static_base_reg = self._fresh_reg()
            descriptor_pointer = self._fresh_reg()
            self._emit(
                IrOp.LOAD_ADDR,
                IrRegister(static_base_reg),
                IrLabel(_STATIC_MEMORY_LABEL),
            )
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(descriptor_pointer),
                IrRegister(static_base_reg),
                IrRegister(self._const_reg(access.slot_offset)),
            )
            return descriptor_pointer

        frame_reg = self._emit_frame_for_array_access(access, scope)
        descriptor_pointer = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(descriptor_pointer),
            IrRegister(frame_reg),
            IrRegister(self._const_reg(access.slot_offset)),
        )
        return descriptor_pointer

    def _emit_frame_for_array_access(
        self, access: ResolvedArrayAccess, scope: _FrameScope
    ) -> int:
        frame_reg = scope.frame_base_reg
        for _ in range(access.lexical_depth_delta):
            next_frame = self._fresh_reg()
            offset_reg = self._const_reg(_STATIC_LINK_OFFSET)
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(next_frame),
                IrRegister(frame_reg),
                IrRegister(offset_reg),
            )
            frame_reg = next_frame
        return frame_reg

    def _emit_static_link_for_call(
        self, call: ResolvedProcedureCall, scope: _FrameScope
    ) -> int:
        if call.use_block_id != scope.block_id:
            raise CompileError(
                f"procedure call {call.name!r} was resolved for block "
                f"{call.use_block_id}, but codegen is in block {scope.block_id}"
            )
        frame_reg = scope.frame_base_reg
        for _ in range(call.lexical_depth_delta):
            next_frame = self._fresh_reg()
            offset_reg = self._const_reg(_STATIC_LINK_OFFSET)
            self._emit(
                IrOp.LOAD_WORD,
                IrRegister(next_frame),
                IrRegister(frame_reg),
                IrRegister(offset_reg),
            )
            frame_reg = next_frame
        return frame_reg

    def _emit_load_symbol(
        self,
        symbol: Symbol,
        scope: _FrameScope,
        dst_reg: int,
    ) -> None:
        if symbol.storage_class == "static":
            if symbol.slot_offset is None:
                raise CompileError(f"symbol {symbol.name!r} has no planned static slot")
            base_reg = self._fresh_reg()
            self._emit(
                IrOp.LOAD_ADDR,
                IrRegister(base_reg),
                IrLabel(_STATIC_MEMORY_LABEL),
            )
            self._emit_load_scalar(symbol.type_name, dst_reg, base_reg, symbol.slot_offset)
            return
        if symbol.declaring_block_id != scope.block_id:
            raise CompileError(f"symbol {symbol.name!r} does not belong to root block")
        if symbol.slot_offset is None:
            raise CompileError(f"symbol {symbol.name!r} has no planned frame slot")
        offset_reg = self._const_reg(symbol.slot_offset)
        if symbol.type_name == _REAL_TYPE:
            self._emit(
                IrOp.LOAD_F64,
                IrRegister(dst_reg),
                IrRegister(scope.frame_base_reg),
                IrRegister(offset_reg),
            )
            return
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(dst_reg),
            IrRegister(scope.frame_base_reg),
            IrRegister(offset_reg),
        )

    def _emit_load_scalar(
        self,
        type_name: str,
        dst_reg: int,
        base_reg: int,
        offset: int,
    ) -> None:
        offset_reg = self._const_reg(offset)
        opcode = IrOp.LOAD_F64 if type_name == _REAL_TYPE else IrOp.LOAD_WORD
        self._emit(
            opcode,
            IrRegister(dst_reg),
            IrRegister(base_reg),
            IrRegister(offset_reg),
        )

    def _emit_load_scalar_at_reg(
        self,
        type_name: str,
        dst_reg: int,
        base_reg: int,
        offset_reg: int,
    ) -> None:
        opcode = IrOp.LOAD_F64 if type_name == _REAL_TYPE else IrOp.LOAD_WORD
        self._emit(
            opcode,
            IrRegister(dst_reg),
            IrRegister(base_reg),
            IrRegister(offset_reg),
        )

    def _emit_store_scalar(
        self,
        type_name: str,
        value_reg: int,
        base_reg: int,
        offset: int,
    ) -> None:
        offset_reg = self._const_reg(offset)
        opcode = IrOp.STORE_F64 if type_name == _REAL_TYPE else IrOp.STORE_WORD
        self._emit(
            opcode,
            IrRegister(value_reg),
            IrRegister(base_reg),
            IrRegister(offset_reg),
        )

    def _emit_store_scalar_at_reg(
        self,
        type_name: str,
        value_reg: int,
        base_reg: int,
        offset_reg: int,
    ) -> None:
        opcode = IrOp.STORE_F64 if type_name == _REAL_TYPE else IrOp.STORE_WORD
        self._emit(
            opcode,
            IrRegister(value_reg),
            IrRegister(base_reg),
            IrRegister(offset_reg),
        )

    def _store_word_reg(self, *, value_reg: int, base_reg: int, offset: int) -> None:
        offset_reg = self._const_reg(offset)
        self._emit(
            IrOp.STORE_WORD,
            IrRegister(value_reg),
            IrRegister(base_reg),
            IrRegister(offset_reg),
        )

    def _array_element_width(self, type_name: str) -> int:
        return _ARRAY_REAL_BYTES if type_name == _REAL_TYPE else _ARRAY_WORD_BYTES

    def _store_scalar_reg(
        self,
        *,
        type_name: str,
        value_reg: int,
        base_reg: int,
        offset: int,
    ) -> None:
        if type_name == _REAL_TYPE:
            self._store_f64_reg(value_reg=value_reg, base_reg=base_reg, offset=offset)
            return
        self._store_word_reg(value_reg=value_reg, base_reg=base_reg, offset=offset)

    def _store_word_const(self, base_reg: int, offset: int, value: int) -> None:
        value_reg = self._const_reg(value)
        self._store_word_reg(value_reg=value_reg, base_reg=base_reg, offset=offset)

    def _store_byte_const(self, *, base_reg: int, offset: int, value: int) -> None:
        value_reg = self._const_reg(value)
        offset_reg = self._const_reg(offset)
        self._emit(
            IrOp.STORE_BYTE,
            IrRegister(value_reg),
            IrRegister(base_reg),
            IrRegister(offset_reg),
        )

    def _store_f64_reg(self, *, value_reg: int, base_reg: int, offset: int) -> None:
        self._emit_store_scalar(_REAL_TYPE, value_reg, base_reg, offset)

    def _store_f64_const(self, base_reg: int, offset: int, value: float) -> None:
        value_reg = self._const_f64_reg(value)
        self._store_f64_reg(value_reg=value_reg, base_reg=base_reg, offset=offset)

    def _copy_reg(self, *, dst: int, src: int) -> None:
        self._emit(IrOp.ADD_IMM, IrRegister(dst), IrRegister(src), IrImmediate(0))

    def _copy_scalar_reg(self, *, type_name: str, dst: int, src: int) -> None:
        if type_name == _REAL_TYPE:
            zero = self._const_f64_reg(0.0)
            self._emit(
                IrOp.F64_ADD,
                IrRegister(dst),
                IrRegister(src),
                IrRegister(zero),
            )
            return
        self._copy_reg(dst=dst, src=src)

    def _const_reg(self, value: int) -> int:
        reg = self._fresh_reg()
        self._emit(IrOp.LOAD_IMM, IrRegister(reg), IrImmediate(value))
        return reg

    def _const_f64_reg(self, value: float) -> int:
        reg = self._fresh_reg()
        self._emit(IrOp.LOAD_F64_IMM, IrRegister(reg), IrFloatImmediate(value))
        return reg

    def _coerce_reg_to_type(
        self,
        reg: int,
        actual_type: str,
        *,
        expected_type: str | None = None,
    ) -> int:
        target_type = expected_type or actual_type
        if actual_type == target_type:
            return reg
        if actual_type == _INTEGER_TYPE and target_type == _REAL_TYPE:
            coerced = self._fresh_reg()
            self._emit(IrOp.F64_FROM_I32, IrRegister(coerced), IrRegister(reg))
            return coerced
        raise CompileError(
            f"cannot coerce {actual_type} value to {target_type} during lowering"
        )

    def _result_type_for_numeric_operator(
        self,
        operator: str,
        left_type: str,
        right_type: str,
    ) -> str:
        if operator == "/":
            return _REAL_TYPE
        if operator in {"+", "-", "*"}:
            return (
                _REAL_TYPE
                if _REAL_TYPE in {left_type, right_type}
                else _INTEGER_TYPE
            )
        return _INTEGER_TYPE

    def _expr_type(self, expr: ASTNode | Token | None) -> str:
        if expr is None:
            raise CompileError("missing typed expression")
        inferred = self.expression_types.get(id(expr))
        if inferred is None:
            raise CompileError(f"missing inferred type for {type(expr).__name__}")
        return inferred

    def _load_runtime_state(self, offset: int, dst_reg: int) -> None:
        self._emit(
            IrOp.LOAD_ADDR,
            IrRegister(self.stack_base_reg),
            IrLabel(_FRAME_MEMORY_LABEL),
        )
        offset_reg = self._const_reg(offset)
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(dst_reg),
            IrRegister(self.stack_base_reg),
            IrRegister(offset_reg),
        )

    def _store_runtime_state(self, offset: int, value_reg: int) -> None:
        self._emit(
            IrOp.LOAD_ADDR,
            IrRegister(self.stack_base_reg),
            IrLabel(_FRAME_MEMORY_LABEL),
        )
        self._store_word_reg(
            value_reg=value_reg,
            base_reg=self.stack_base_reg,
            offset=offset,
        )

    def _snapshot_heap_pointer(self) -> int:
        heap_mark = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, heap_mark)
        return heap_mark

    def _restore_heap_pointer(self, scope: _FrameScope) -> None:
        if scope.heap_mark_reg is not None:
            self._store_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, scope.heap_mark_reg)
            return

        token_reg = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(token_reg),
            IrRegister(scope.frame_base_reg),
            IrRegister(self._const_reg(_RETURN_TOKEN_OFFSET)),
        )
        self._store_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, token_reg)

    def _emit_zero_result_reg(self) -> None:
        if self.current_function_return_type == _REAL_TYPE:
            self._emit(
                IrOp.LOAD_F64_IMM,
                IrRegister(_RESULT_REG),
                IrFloatImmediate(0.0),
            )
            return
        self._emit(IrOp.LOAD_IMM, IrRegister(_RESULT_REG), IrImmediate(0))

    def _emit_stack_overflow_guard(self, overflow_reg: int) -> None:
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(overflow_reg), IrLabel(else_label))
        self._emit_zero_result_reg()
        self._emit(IrOp.RET)
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        self._label(end_label)

    def _emit_runtime_failure_guard(self, failed_reg: int, scope: _FrameScope) -> None:
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(failed_reg), IrLabel(else_label))
        self._emit_mark_thunk_failure_if_helper_active()
        self._emit_zero_result_reg()
        self._emit_unwind_for_return(scope)
        if scope.helper_failure:
            self._emit_leave_thunk_helper()
        self._emit_restore_active_thunk_heap_mark(scope)
        self._emit(IrOp.RET)
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        self._label(end_label)

    def _emit_pending_goto_return(
        self,
        scope: _FrameScope,
        label_id: int,
    ) -> None:
        self._emit_pending_goto_return_reg(scope, self._const_reg(label_id))

    def _emit_pending_goto_return_reg(
        self,
        scope: _FrameScope,
        label_reg: int,
    ) -> None:
        self._store_runtime_state(
            _RUNTIME_PENDING_GOTO_LABEL_OFFSET,
            label_reg,
        )
        self._emit_zero_result_reg()
        if not scope.helper_failure:
            self._emit_unwind_for_return(scope)
        if scope.helper_failure:
            self._emit_leave_thunk_helper()
        self._emit_restore_active_thunk_heap_mark(scope)
        self._emit(IrOp.RET)

    def _emit_handle_pending_goto_after_call(self, scope: _FrameScope) -> None:
        pending_label = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_PENDING_GOTO_LABEL_OFFSET, pending_label)
        index = self.if_count
        self.if_count += 1
        no_pending_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(pending_label), IrLabel(no_pending_label))
        if scope.helper_failure:
            self._emit_zero_result_reg()
            self._emit_leave_thunk_helper()
            self._emit_restore_active_thunk_heap_mark(scope)
            self._emit(IrOp.RET)
            self._emit(IrOp.JUMP, IrLabel(end_label))
            self._label(no_pending_label)
            self._label(end_label)
            return

        active_scopes = self._active_function_scopes(scope)
        active_block_ids = {active_scope.block_id for active_scope in active_scopes}
        for label in self.labels_by_id.values():
            if label.declaring_block_id not in active_block_ids:
                continue
            next_label = f"if_{self.if_count}_else"
            next_end_label = f"if_{self.if_count}_end"
            self.if_count += 1
            matches = self._fresh_reg()
            self._emit(
                IrOp.CMP_EQ,
                IrRegister(matches),
                IrRegister(pending_label),
                IrRegister(self._const_reg(label.label_id)),
            )
            self._emit(IrOp.BRANCH_Z, IrRegister(matches), IrLabel(next_label))
            self._store_runtime_state(_RUNTIME_PENDING_GOTO_LABEL_OFFSET, _ZERO_REG)
            self._emit_unwind_to_block(scope, label.declaring_block_id)
            self._emit(IrOp.JUMP, IrLabel(label.ir_label))
            self._emit(IrOp.JUMP, IrLabel(next_end_label))
            self._label(next_label)
            self._label(next_end_label)

        if scope.function_owner_procedure_id is not None:
            self._emit_zero_result_reg()
            self._emit_unwind_for_return(scope)
            self._emit_restore_active_thunk_heap_mark(scope)
            self._emit(IrOp.RET)
        else:
            self._store_runtime_state(_RUNTIME_PENDING_GOTO_LABEL_OFFSET, _ZERO_REG)
            self._emit_zero_result_reg()
            self._emit(IrOp.HALT)

        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(no_pending_label)
        self._label(end_label)

    def _active_function_scopes(self, scope: _FrameScope) -> list[_FrameScope]:
        active_scopes: list[_FrameScope] = []
        current: _FrameScope | None = scope
        while current is not None:
            if current.function_owner_procedure_id != scope.function_owner_procedure_id:
                break
            active_scopes.append(current)
            current = current.parent
        return active_scopes

    def _active_scope_for_block(
        self,
        block_id: int,
        scope: _FrameScope,
    ) -> _FrameScope:
        current: _FrameScope | None = scope
        while current is not None:
            if current.block_id == block_id:
                return current
            current = current.goto_parent
        raise CompileError(f"block {block_id} is not active for switch selection")

    def _emit_helper_runtime_failure_guard(
        self,
        failed_reg: int,
        scope: _FrameScope,
    ) -> None:
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(failed_reg), IrLabel(else_label))
        self._store_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, self._const_reg(1))
        self._emit_zero_result_reg()
        self._emit_leave_thunk_helper()
        self._emit_restore_active_thunk_heap_mark(scope)
        self._emit(IrOp.RET)
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        self._label(end_label)

    def _emit_propagate_thunk_failure(self, scope: _FrameScope) -> None:
        failed = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, failed)
        self._store_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, _ZERO_REG)
        self._emit_runtime_failure_guard(failed, scope)

    def _emit_enter_thunk_helper(self) -> None:
        depth = self._fresh_reg()
        next_depth = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_THUNK_HELPER_DEPTH_OFFSET, depth)
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(next_depth),
            IrRegister(depth),
            IrImmediate(1),
        )
        self._store_runtime_state(_RUNTIME_THUNK_HELPER_DEPTH_OFFSET, next_depth)

    def _emit_leave_thunk_helper(self) -> None:
        depth = self._fresh_reg()
        next_depth = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_THUNK_HELPER_DEPTH_OFFSET, depth)
        self._emit(
            IrOp.ADD_IMM,
            IrRegister(next_depth),
            IrRegister(depth),
            IrImmediate(-1),
        )
        self._store_runtime_state(_RUNTIME_THUNK_HELPER_DEPTH_OFFSET, next_depth)

    def _emit_mark_thunk_failure_if_helper_active(self) -> None:
        depth = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_THUNK_HELPER_DEPTH_OFFSET, depth)
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(depth), IrLabel(else_label))
        self._store_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, self._const_reg(1))
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        self._label(end_label)

    def _emit_helper_return_on_thunk_failure(self, scope: _FrameScope) -> None:
        failed = self._fresh_reg()
        self._load_runtime_state(_RUNTIME_THUNK_FAILURE_OFFSET, failed)
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(failed), IrLabel(else_label))
        self._emit_zero_result_reg()
        self._emit_leave_thunk_helper()
        self._emit_restore_active_thunk_heap_mark(scope)
        self._emit(IrOp.RET)
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        self._label(end_label)

    def _emit_restore_active_thunk_heap_mark(self, scope: _FrameScope) -> None:
        mark_reg = scope.active_thunk_heap_mark_reg
        if mark_reg is None:
            return
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        has_mark = self._fresh_reg()
        self._emit(
            IrOp.CMP_NE,
            IrRegister(has_mark),
            IrRegister(mark_reg),
            IrRegister(_ZERO_REG),
        )
        self._emit(IrOp.BRANCH_Z, IrRegister(has_mark), IrLabel(else_label))
        self._store_runtime_state(_RUNTIME_HEAP_POINTER_OFFSET, mark_reg)
        self._store_runtime_state(_RUNTIME_THUNK_HEAP_MARK_OFFSET, _ZERO_REG)
        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        self._label(end_label)

    def _emit_unwind_for_return(self, scope: _FrameScope) -> None:
        current: _FrameScope | None = scope
        while current is not None:
            self._emit_leave_frame(current)
            current = current.parent

    def _require_reference(self, token: Token, role: str) -> ResolvedReference:
        reference = self.references.get((id(token), role))
        if reference is None:
            raise CompileError(
                f"missing resolved {role} reference for {token.value!r} "
                f"at line {token.line}, column {token.column}"
            )
        return reference

    def _require_array_access(
        self, token: Token, role: str
    ) -> ResolvedArrayAccess:
        access = self.array_accesses.get((id(token), role))
        if access is None:
            raise CompileError(
                f"missing resolved {role} array access for {token.value!r} "
                f"at line {token.line}, column {token.column}"
            )
        return access

    def _require_procedure_call(
        self, token: Token, role: str
    ) -> ResolvedProcedureCall:
        call = self.procedure_calls.get((id(token), role))
        if call is None:
            raise CompileError(
                f"missing resolved {role} procedure call for {token.value!r} "
                f"at line {token.line}, column {token.column}"
            )
        return call

    def _semantic_block_for_ast(self, block: ASTNode) -> SemanticBlock:
        semantic_block = self.semantic_blocks_by_ast.get(id(block))
        if semantic_block is None:
            raise CompileError(f"missing semantic block for AST node {block.rule_name}")
        return semantic_block

    def _find_ast_by_id(self, node_id: int) -> ASTNode | None:
        if self.source_ast is None:
            return None
        pending = [self.source_ast]
        while pending:
            node = pending.pop()
            if id(node) == node_id:
                return node
            pending.extend(
                child for child in node.children if isinstance(child, ASTNode)
            )
        return None

    def _layout_frames(self, blocks: list[SemanticBlock]) -> dict[int, int]:
        offsets: dict[int, int] = {}
        cursor = 0
        for block in blocks:
            offsets[block.block_id] = cursor
            cursor += block.frame_layout.frame_size
        return offsets

    def _collect_variable_slots(self, blocks: list[SemanticBlock]) -> dict[str, int]:
        slots: dict[str, int] = {}
        for block in blocks:
            for symbol in block.scope.symbols.values():
                if symbol.slot_offset is not None:
                    slots[f"{symbol.name}@block{block.block_id}"] = symbol.slot_offset
        return slots

    def _collect_legacy_variable_slots(
        self, blocks: list[SemanticBlock]
    ) -> dict[str, int]:
        slots: dict[str, int] = {}
        for block in blocks:
            for symbol in block.scope.symbols.values():
                if symbol.slot_offset is not None:
                    slots.setdefault(symbol.name, symbol.slot_offset)
        return slots

    def _static_storage_bytes(self, symbols: list[Symbol]) -> int:
        total = 0
        for symbol in symbols:
            if symbol.storage_class != "static":
                continue
            if symbol.slot_offset is None or symbol.slot_size is None:
                raise CompileError(
                    f"static symbol {symbol.name!r} is missing storage layout"
                )
            total = max(total, symbol.slot_offset + symbol.slot_size)
        return total

    def _plan_string_literals(self, ast: ASTNode, start_offset: int) -> int:
        offset = start_offset
        for token in _tokens(ast):
            if token.type_name != "STRING_LIT":
                continue
            text = token.value[1:-1]
            if text in self.string_literal_offsets:
                continue
            self.string_literal_offsets[text] = offset
            offset += _STRING_DESCRIPTOR_SIZE + len(text)
        return offset

    def _initialize_string_literals(self) -> None:
        if not self.string_literal_offsets:
            return
        static_base_reg = self._fresh_reg()
        self._emit(
            IrOp.LOAD_ADDR,
            IrRegister(static_base_reg),
            IrLabel(_STATIC_MEMORY_LABEL),
        )
        for text, offset in self.string_literal_offsets.items():
            self._store_word_const(
                static_base_reg,
                offset + _STRING_DESCRIPTOR_LENGTH_OFFSET,
                len(text),
            )
            data_pointer = self._fresh_reg()
            self._emit(
                IrOp.ADD_IMM,
                IrRegister(data_pointer),
                IrRegister(static_base_reg),
                IrImmediate(offset + _STRING_DESCRIPTOR_SIZE),
            )
            self._store_word_reg(
                value_reg=data_pointer,
                base_reg=static_base_reg,
                offset=offset + _STRING_DESCRIPTOR_DATA_POINTER_OFFSET,
            )
            for index, char in enumerate(text):
                self._store_byte_const(
                    base_reg=static_base_reg,
                    offset=offset + _STRING_DESCRIPTOR_SIZE + index,
                    value=ord(char),
                )

    def _emit_string_literal_pointer(self, text: str) -> int:
        offset = self.string_literal_offsets.get(text)
        if offset is None:
            raise CompileError(f"string literal {text!r} was not planned")
        pointer_reg = self._fresh_reg()
        self._emit(
            IrOp.LOAD_ADDR,
            IrRegister(pointer_reg),
            IrLabel(_STATIC_MEMORY_LABEL),
        )
        if offset != 0:
            adjusted_reg = self._fresh_reg()
            self._emit(
                IrOp.ADD_IMM,
                IrRegister(adjusted_reg),
                IrRegister(pointer_reg),
                IrImmediate(offset),
            )
            return adjusted_reg
        return pointer_reg

    def _fresh_reg(self) -> int:
        reg = self.next_reg
        self.next_reg += 1
        return reg

    def _label(self, name: str) -> None:
        self.program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel(name)], id=-1))

    def _emit(
        self, opcode: IrOp, *operands: IrRegister | IrImmediate | IrLabel
    ) -> None:
        self.program.add_instruction(
            IrInstruction(opcode, list(operands), id=self.ids.next())
        )


def compile_algol(typed: TypeCheckResult | ASTNode) -> CompileResult:
    return AlgolIrCompiler().compile(typed)


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


def _for_element_kind(node: ASTNode) -> str:
    arith_count = len(_direct_nodes(node, "arith_expr"))
    has_bool = _first_direct_node(node, "bool_expr") is not None
    if has_bool:
        return "while"
    if arith_count == 3:
        return "step_until"
    if arith_count == 1:
        return "simple"
    return "unsupported"


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


def _nodes(node: ASTNode | None, rule_name: str) -> list[ASTNode]:
    if node is None:
        return []
    found = [node] if node.rule_name == rule_name else []
    for child in _node_children(node):
        found.extend(_nodes(child, rule_name))
    return found


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


def _variable_name(node: ASTNode | None) -> Token | None:
    if node is None:
        return None
    variable = node if node.rule_name == "variable" else _first_node(node, "variable")
    if variable is None:
        return None
    if _variable_subscripts(variable):
        return None
    names = [token for token in _tokens(variable) if token.type_name == "NAME"]
    return names[0] if len(names) == 1 else None


def _single_variable_expr(node: ASTNode | None) -> ASTNode | None:
    if node is None:
        return None
    if node.rule_name == "variable":
        return node
    meaningful = _meaningful_children(node)
    if len(meaningful) != 1 or not isinstance(meaningful[0], ASTNode):
        return None
    return _single_variable_expr(meaningful[0])


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


def _has_comparison(children: list[ASTNode | Token]) -> bool:
    return any(
        isinstance(child, Token) and child.value in {"=", "!=", "<", "<=", ">", ">="}
        for child in children
    )
