"""engine.py --- WasmExecutionEngine: the core WASM interpreter.

Takes a validated WASM module's runtime state and executes function calls
using the GenericVM infrastructure.
"""

from __future__ import annotations

from typing import Any

from virtual_machine.generic_vm import GenericVM
from virtual_machine.vm import CodeObject
from wasm_types import FuncType, FunctionBody, GlobalType

from wasm_execution.decoder import build_control_flow_map, decode_function_body, to_vm_instructions
from wasm_execution.host_interface import TrapError
from wasm_execution.instructions.control import register_control
from wasm_execution.instructions.dispatch import register_all_instructions
from wasm_execution.linear_memory import LinearMemory
from wasm_execution.table import Table
from wasm_execution.types import WasmExecutionContext
from wasm_execution.values import WasmValue, default_value

MAX_CALL_DEPTH = 1024


class WasmExecutionEngine:
    """The WASM execution engine --- interprets validated WASM modules.

    Usage::

        engine = WasmExecutionEngine(
            memory=linear_memory,
            tables=[func_table],
            globals=[i32(0)],
            global_types=[GlobalType(value_type=ValueType.I32, mutable=True)],
            func_types=[FuncType(params=(ValueType.I32,), results=(ValueType.I32,))],
            func_bodies=[function_body],
            host_functions=[None],
        )
        result = engine.call_function(0, [i32(5)])
    """

    def __init__(
        self,
        memory: LinearMemory | None,
        tables: list[Table],
        globals: list[WasmValue],
        global_types: list[GlobalType],
        func_types: list[FuncType],
        func_bodies: list[FunctionBody | None],
        host_functions: list[Any | None],
    ) -> None:
        self._memory = memory
        self._tables = tables
        self._globals = globals
        self._global_types = global_types
        self._func_types = func_types
        self._func_bodies = func_bodies
        self._host_functions = host_functions

        # Decoded function body cache (decoded once, reused)
        self._decoded_cache: dict[int, Any] = {}

        # Create and configure the GenericVM
        self._vm = GenericVM()
        self._vm.set_max_recursion_depth(MAX_CALL_DEPTH)

        # Register all WASM instruction handlers
        register_all_instructions(self._vm)
        register_control(self._vm)

    def call_function(self, func_index: int, args: list[WasmValue]) -> list[WasmValue]:
        """Call a WASM function by index.

        Args:
            func_index: The function index (imports + module functions).
            args: Function arguments as WasmValues.

        Returns:
            The function's return values as WasmValues.
        """
        if func_index < 0 or func_index >= len(self._func_types):
            msg = f"undefined function index {func_index}"
            raise TrapError(msg)

        func_type = self._func_types[func_index]
        if len(args) != len(func_type.params):
            msg = f"function {func_index} expects {len(func_type.params)} arguments, got {len(args)}"
            raise TrapError(msg)

        # Check if this is a host (imported) function
        host_func = self._host_functions[func_index]
        if host_func is not None:
            return host_func.call(args)

        # Module-defined function
        body = self._func_bodies[func_index]
        if body is None:
            msg = f"no body for function {func_index}"
            raise TrapError(msg)

        # Decode the function body (cached)
        decoded = self._decoded_cache.get(func_index)
        if decoded is None:
            decoded = decode_function_body(body.code)
            self._decoded_cache[func_index] = decoded

        # Build control flow map
        control_flow_map = build_control_flow_map(decoded)

        # Convert to GenericVM instruction format
        vm_instructions = to_vm_instructions(decoded)

        # Initialize locals: arguments + zero-initialized declared locals
        typed_locals: list[WasmValue] = [
            *args,
            *[default_value(t) for t in body.locals],
        ]

        # Build execution context
        ctx = WasmExecutionContext(
            memory=self._memory,
            tables=self._tables,
            globals=self._globals,
            global_types=self._global_types,
            func_types=self._func_types,
            func_bodies=self._func_bodies,
            host_functions=self._host_functions,
            typed_locals=typed_locals,
            label_stack=[],
            control_flow_map=control_flow_map,
            saved_frames=[],
            returned=False,
            return_values=[],
        )

        # Build the CodeObject
        code = CodeObject(
            instructions=vm_instructions,
            constants=[],
            names=[],
        )

        # Reset the VM and execute
        self._vm.reset()
        # Re-register handlers after reset
        register_all_instructions(self._vm)
        register_control(self._vm)

        current_code = code
        while True:
            self._vm.execute_with_context(current_code, ctx)

            pending_code = getattr(ctx, "_pending_code", None)
            if pending_code is not None:
                delattr(ctx, "_pending_code")
                current_code = pending_code
                self._vm.halted = False
                continue

            if ctx.returned and ctx.saved_frames:
                current_code = self._resume_saved_frame(ctx)
                self._vm.halted = False
                ctx.returned = False
                continue

            break

        # Collect return values from the typed stack
        result_count = len(func_type.results)
        results: list[WasmValue] = []
        for _ in range(result_count):
            if len(self._vm.typed_stack) > 0:
                results.insert(0, self._vm.pop_typed())

        return results

    def _resume_saved_frame(self, ctx: WasmExecutionContext) -> CodeObject:
        frame = ctx.saved_frames.pop()
        if len(self._vm.typed_stack) < frame.return_arity:
            raise TrapError("callee returned fewer values than expected")

        results: list[WasmValue] = []
        for _ in range(frame.return_arity):
            results.insert(0, self._vm.pop_typed())

        while len(self._vm.typed_stack) > frame.stack_height:
            self._vm.pop_typed()

        for result in results:
            self._vm.push_typed(result)

        ctx.typed_locals = list(frame.locals)
        ctx.label_stack = list(frame.label_stack)
        ctx.control_flow_map = frame.control_flow_map
        self._vm.jump_to(frame.return_pc)
        return frame.code
