"""Lower the first ALGOL 60 compiler subset into compiler IR."""

from __future__ import annotations

from dataclasses import dataclass, field

from algol_type_checker import (
    ArrayDescriptor,
    ProcedureDescriptor,
    ProcedureParameter,
    ResolvedArrayAccess,
    ResolvedProcedureCall,
    ResolvedReference,
    SemanticBlock,
    Symbol,
    TypeCheckResult,
    check_algol,
)
from compiler_ir import (
    IDGenerator,
    IrDataDecl,
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
_ZERO_REG = 0
_RESULT_REG = 1
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
_RUNTIME_STATE_BYTES = 20
_STATIC_LINK_PARAM_REG = 2
_VALUE_PARAM_BASE_REG = 3
_FIRST_GENERAL_REG = 32
_ARRAY_DESCRIPTOR_SIZE = 24
_ARRAY_ELEMENT_TYPE_INTEGER = 1
_ARRAY_DIMENSION_ENTRY_SIZE = 12
_ARRAY_DIM_LOWER_OFFSET = 0
_ARRAY_DIM_UPPER_OFFSET = 4
_ARRAY_DIM_STRIDE_OFFSET = 8
_ARRAY_TOTAL_COUNT_OFFSET = 8
_ARRAY_ELEMENT_WIDTH_OFFSET = 12
_ARRAY_DATA_POINTER_OFFSET = 16
_ARRAY_BOUNDS_POINTER_OFFSET = 20
_ARRAY_WORD_BYTES = 4
_ARRAY_MAX_ELEMENTS = 4096
_VALUE_MODE = "value"
_NAME_MODE = "name"


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
    procedure_signatures: dict[str, int] = field(default_factory=dict)


class CompileError(Exception):
    """Raised when checked ALGOL cannot be lowered to the current IR subset."""


@dataclass(frozen=True)
class _FrameScope:
    """The active lexical block during frame-backed lowering."""

    semantic_block: SemanticBlock
    frame_base_reg: int
    heap_mark_reg: int | None = None
    parent: _FrameScope | None = None

    @property
    def block_id(self) -> int:
        return self.semantic_block.block_id


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
        self.procedures: dict[int, ProcedureDescriptor] = {}
        self.parameters_by_symbol: dict[int, ProcedureParameter] = {}
        self.arrays: dict[int, ArrayDescriptor] = {}
        self.arrays_by_block: dict[int, list[ArrayDescriptor]] = {}
        self.frame_offsets: dict[int, int] = {}
        self.variable_slots: dict[str, int] = {}
        self.legacy_variable_slots: dict[str, int] = {}
        self.procedure_signatures: dict[str, int] = {}

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
        self.heap_base_reg = -1
        self.heap_pointer_reg = -1
        self.heap_limit_reg = -1
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
        self.procedures = {
            procedure.procedure_id: procedure
            for procedure in type_result.semantic.procedures
        }
        self.parameters_by_symbol = {
            parameter.symbol_id: parameter
            for procedure in type_result.semantic.procedures
            for parameter in procedure.parameters
        }
        self.arrays = {
            array.array_id: array for array in type_result.semantic.arrays
        }
        self.arrays_by_block = {}
        for array in type_result.semantic.arrays:
            self.arrays_by_block.setdefault(array.declaring_block_id, []).append(array)
        self.procedure_signatures = {
            procedure.label: 1 + len(procedure.parameters)
            for procedure in type_result.semantic.procedures
        }
        self.frame_offsets = self._layout_frames(self.semantic_blocks)
        self.variable_slots = self._collect_variable_slots(self.semantic_blocks)
        self.legacy_variable_slots = self._collect_legacy_variable_slots(
            self.semantic_blocks
        )

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

        block = _first_node(type_result.ast, "block")
        if block is None:
            raise CompileError("ALGOL program must contain a block")
        root_scope = self._compile_block(block, parent=None)

        result_symbol = root_scope.semantic_block.scope.symbols.get("result")
        if result_symbol is None:
            raise CompileError(
                "compiled ALGOL programs must declare integer variable 'result'"
            )
        self._emit_load_symbol(result_symbol, root_scope, _RESULT_REG)
        self._emit(IrOp.HALT)
        self._compile_procedures(type_result.semantic.procedures)

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
        frame_base_reg = self._emit_enter_frame(semantic_block, static_parent_reg)
        scope = _FrameScope(
            semantic_block=semantic_block,
            frame_base_reg=frame_base_reg,
            heap_mark_reg=self._snapshot_heap_pointer(),
            parent=parent,
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
        self._store_word_const(frame_base, _RETURN_TOKEN_OFFSET, 0)
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
            self._store_word_const(scope.frame_base_reg, slot.offset, 0)

    def _allocate_arrays(self, scope: _FrameScope) -> None:
        for array in self.arrays_by_block.get(scope.block_id, []):
            self._allocate_array(array, scope)

    def _allocate_array(self, array: ArrayDescriptor, scope: _FrameScope) -> None:
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
        word_bytes = self._const_reg(_ARRAY_WORD_BYTES)
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

        self._store_word_const(heap_pointer, 0, _ARRAY_ELEMENT_TYPE_INTEGER)
        self._store_word_const(heap_pointer, 4, len(array.dimensions))
        self._store_word_reg(
            value_reg=total_reg,
            base_reg=heap_pointer,
            offset=_ARRAY_TOTAL_COUNT_OFFSET,
        )
        self._store_word_const(
            heap_pointer,
            _ARRAY_ELEMENT_WIDTH_OFFSET,
            _ARRAY_WORD_BYTES,
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

        self._store_word_reg(
            value_reg=heap_pointer,
            base_reg=scope.frame_base_reg,
            offset=array.slot_offset,
        )

    def _compile_statement(self, statement: ASTNode, scope: _FrameScope) -> None:
        inner = _first_ast_child(statement)
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
        else:
            raise CompileError(
                f"{inner.rule_name} is not supported by algol-ir-compiler"
            )

    def _compile_assignment(self, assign: ASTNode, scope: _FrameScope) -> None:
        left_part = _first_direct_node(assign, "left_part")
        expr = _first_direct_node(assign, "expression")
        variable = _first_node(left_part, "variable") if left_part is not None else None
        if variable is None or expr is None:
            raise CompileError("assignment needs a variable target and expression")
        value = self._compile_expr(expr, scope)
        if _variable_subscripts(variable):
            self._compile_array_store(variable, scope, value)
            return
        name = _variable_name(variable)
        if name is None:
            raise CompileError("only scalar assignments are supported")
        target = self._require_reference(name, "write")
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

        elem = _first_direct_node(_first_direct_node(node, "for_list"), "for_elem")
        pieces = _direct_nodes(elem, "arith_expr")
        if elem is None or len(pieces) != 3:
            raise CompileError("only step/until for-elements are supported")

        start = self._compile_expr(pieces[0], scope)
        self._emit_store_reference(loop_reference, scope, start)

        index = self.loop_count
        self.loop_count += 1
        start_label = f"loop_{index}_start"
        end_label = f"loop_{index}_end"

        self._label(start_label)
        loop_value = self._emit_load_reference(loop_reference, scope)
        limit = self._compile_expr(pieces[2], scope)
        should_stop = self._fresh_reg()
        self._emit(
            IrOp.CMP_GT,
            IrRegister(should_stop),
            IrRegister(loop_value),
            IrRegister(limit),
        )
        self._emit(IrOp.BRANCH_NZ, IrRegister(should_stop), IrLabel(end_label))

        body = _first_direct_node(node, "statement")
        if body is not None:
            self._compile_statement(body, scope)

        current_value = self._emit_load_reference(loop_reference, scope)
        step = self._compile_expr(pieces[1], scope)
        next_value = self._fresh_reg()
        self._emit(
            IrOp.ADD,
            IrRegister(next_value),
            IrRegister(current_value),
            IrRegister(step),
        )
        self._emit_store_reference(loop_reference, scope, next_value)
        self._emit(IrOp.JUMP, IrLabel(start_label))
        self._label(end_label)

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
        index = 1
        while index < len(children):
            operator = children[index]
            right = self._compile_expr(children[index + 1], scope)
            if not isinstance(operator, Token):
                raise CompileError("expected numeric operator")
            current = self._emit_numeric(operator.value, current, right)
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
            if value == "and" or rule_name in {"expr_and", "bool_term"}:
                dst = self._fresh_reg()
                self._emit(
                    IrOp.AND, IrRegister(dst), IrRegister(current), IrRegister(right)
                )
                current = dst
            elif value == "or" or rule_name in {"expr_or", "simple_bool"}:
                summed = self._fresh_reg()
                dst = self._fresh_reg()
                self._emit(
                    IrOp.ADD, IrRegister(summed), IrRegister(current), IrRegister(right)
                )
                self._emit(
                    IrOp.CMP_NE, IrRegister(dst), IrRegister(summed), IrRegister(0)
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
        if not isinstance(operator, Token):
            raise CompileError("expected comparison operator")
        dst = self._fresh_reg()
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

    def _emit_numeric(self, operator: str, left: int, right: int) -> int:
        dst = self._fresh_reg()
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
        procedure = self.procedures[call.procedure_id]
        static_link = self._emit_static_link_for_call(call, scope)
        actuals = _direct_nodes(_first_direct_node(node, "actual_params"), "expression")
        if len(actuals) != len(procedure.parameters):
            raise CompileError(
                f"procedure {procedure.name!r} expects "
                f"{len(procedure.parameters)} argument(s), got {len(actuals)}"
            )
        arguments = []
        for argument, parameter in zip(actuals, procedure.parameters, strict=True):
            if parameter.mode == _VALUE_MODE:
                arguments.append(self._compile_expr(argument, scope))
            else:
                arguments.append(
                    self._compile_by_name_actual_pointer(argument, parameter, scope)
                )
        self._emit(
            IrOp.CALL,
            IrLabel(call.label),
            IrRegister(static_link),
            *(IrRegister(argument) for argument in arguments),
        )
        result = self._fresh_reg()
        self._copy_reg(dst=result, src=_RESULT_REG)
        return result

    def _compile_by_name_actual_pointer(
        self,
        argument: ASTNode,
        parameter: ProcedureParameter,
        scope: _FrameScope,
    ) -> int:
        variable = _single_variable_expr(argument)
        if variable is None:
            raise CompileError(
                f"by-name parameter {parameter.name!r} received a read-only "
                "expression actual; eval thunk lowering is not implemented yet"
            )
        if _variable_subscripts(variable):
            raise CompileError(
                f"by-name parameter {parameter.name!r} received an array element "
                "actual; re-evaluating element thunk lowering is not implemented yet"
            )
        name = _variable_name(variable)
        if name is None:
            raise CompileError("by-name scalar actual is missing a name")
        reference = self._require_reference(name, "read")
        return self._emit_reference_pointer(reference, scope)

    def _compile_procedures(
        self, procedures: list[ProcedureDescriptor]
    ) -> None:
        for procedure in procedures:
            self._compile_procedure(procedure)

    def _compile_procedure(self, procedure: ProcedureDescriptor) -> None:
        body = self._find_ast_by_id(procedure.body_node_id)
        if body is None:
            raise CompileError(f"missing body AST for procedure {procedure.name!r}")
        semantic_block = self.semantic_blocks_by_id[procedure.body_block_id]
        self._label(procedure.label)
        frame_base_reg = self._emit_enter_frame(
            semantic_block,
            _STATIC_LINK_PARAM_REG,
        )
        scope = _FrameScope(
            semantic_block=semantic_block,
            frame_base_reg=frame_base_reg,
            heap_mark_reg=self._snapshot_heap_pointer(),
            parent=None,
        )
        self._initialize_scalar_slots(scope)
        for index, parameter in enumerate(procedure.parameters):
            self._store_word_reg(
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

    def _compile_array_load(self, variable: ASTNode, scope: _FrameScope) -> int:
        data_pointer, byte_offset = self._compile_array_element_address(
            variable,
            scope,
            role="read",
        )
        dst = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(dst),
            IrRegister(data_pointer),
            IrRegister(byte_offset),
        )
        return dst

    def _compile_array_store(
        self,
        variable: ASTNode,
        scope: _FrameScope,
        value_reg: int,
    ) -> None:
        data_pointer, byte_offset = self._compile_array_element_address(
            variable,
            scope,
            role="write",
        )
        self._emit(
            IrOp.STORE_WORD,
            IrRegister(value_reg),
            IrRegister(data_pointer),
            IrRegister(byte_offset),
        )

    def _compile_array_element_address(
        self,
        variable: ASTNode,
        scope: _FrameScope,
        *,
        role: str,
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

        element_index = self._const_reg(0)
        subscripts = _variable_subscripts(variable)
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
            self._emit_runtime_failure_guard(below, scope)
            self._emit(
                IrOp.CMP_GT,
                IrRegister(above),
                IrRegister(subscript_reg),
                IrRegister(upper),
            )
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

        byte_offset = self._fresh_reg()
        self._emit(
            IrOp.MUL,
            IrRegister(byte_offset),
            IrRegister(element_index),
            IrRegister(self._const_reg(_ARRAY_WORD_BYTES)),
        )
        return data_pointer, byte_offset

    def _emit_load_reference(
        self, reference: ResolvedReference, scope: _FrameScope
    ) -> int:
        pointer = self._emit_reference_pointer(reference, scope)
        dst = self._fresh_reg()
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(dst),
            IrRegister(pointer),
            IrRegister(self._const_reg(0)),
        )
        return dst

    def _emit_reference_pointer(
        self, reference: ResolvedReference, scope: _FrameScope
    ) -> int:
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
        return parameter is not None and parameter.mode == _NAME_MODE

    def _emit_store_reference(
        self,
        reference: ResolvedReference,
        scope: _FrameScope,
        value_reg: int,
    ) -> None:
        pointer = self._emit_reference_pointer(reference, scope)
        self._emit(
            IrOp.STORE_WORD,
            IrRegister(value_reg),
            IrRegister(pointer),
            IrRegister(self._const_reg(0)),
        )

    def _emit_frame_for_reference(
        self, reference: ResolvedReference, scope: _FrameScope
    ) -> int:
        if reference.use_block_id != scope.block_id:
            raise CompileError(
                f"reference {reference.name!r} was resolved for block "
                f"{reference.use_block_id}, but codegen is in block {scope.block_id}"
            )
        frame_reg = scope.frame_base_reg
        for _ in range(reference.lexical_depth_delta):
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

    def _emit_load_array_descriptor(
        self,
        access: ResolvedArrayAccess,
        scope: _FrameScope,
    ) -> int:
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
        if symbol.declaring_block_id != scope.block_id:
            raise CompileError(f"symbol {symbol.name!r} does not belong to root block")
        if symbol.slot_offset is None:
            raise CompileError(f"symbol {symbol.name!r} has no planned frame slot")
        offset_reg = self._const_reg(symbol.slot_offset)
        self._emit(
            IrOp.LOAD_WORD,
            IrRegister(dst_reg),
            IrRegister(scope.frame_base_reg),
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

    def _store_word_const(self, base_reg: int, offset: int, value: int) -> None:
        value_reg = self._const_reg(value)
        self._store_word_reg(value_reg=value_reg, base_reg=base_reg, offset=offset)

    def _copy_reg(self, *, dst: int, src: int) -> None:
        self._emit(IrOp.ADD_IMM, IrRegister(dst), IrRegister(src), IrImmediate(0))

    def _const_reg(self, value: int) -> int:
        reg = self._fresh_reg()
        self._emit(IrOp.LOAD_IMM, IrRegister(reg), IrImmediate(value))
        return reg

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

    def _emit_stack_overflow_guard(self, overflow_reg: int) -> None:
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"
        self._emit(IrOp.BRANCH_Z, IrRegister(overflow_reg), IrLabel(else_label))
        self._emit(IrOp.LOAD_IMM, IrRegister(_RESULT_REG), IrImmediate(0))
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
        self._emit(IrOp.LOAD_IMM, IrRegister(_RESULT_REG), IrImmediate(0))
        self._emit_unwind_for_return(scope)
        self._emit(IrOp.RET)
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
