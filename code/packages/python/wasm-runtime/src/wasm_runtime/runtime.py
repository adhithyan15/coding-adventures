"""runtime.py --- WasmRuntime: the complete WebAssembly runtime.

Composes the parser, validator, and execution engine into a single API.
"""

from __future__ import annotations

from typing import Any

from wasm_module_parser import WasmModuleParser
from wasm_types import ExternalKind, FuncType, FunctionBody, GlobalType, ValueType, WasmModule
from wasm_validator import ValidatedModule, validate

from wasm_execution import (
    LinearMemory,
    Table,
    TrapError,
    WasmExecutionEngine,
    WasmValue,
    evaluate_const_expr,
    f32,
    f64,
    i32,
    i64,
)

from wasm_runtime.instance import WasmInstance


class WasmRuntime:
    """Complete WebAssembly 1.0 runtime.

    Usage::

        runtime = WasmRuntime()
        result = runtime.load_and_run(square_bytes, "square", [5])
        # result == [25]
    """

    def __init__(self, host: Any | None = None) -> None:
        self._parser = WasmModuleParser()
        self._host = host

    def load(self, wasm_bytes: bytes | bytearray) -> WasmModule:
        """Parse a .wasm binary into a WasmModule."""
        return self._parser.parse(wasm_bytes)

    def validate(self, module: WasmModule) -> ValidatedModule:
        """Validate a parsed module."""
        return validate(module)

    def instantiate(self, module: WasmModule) -> WasmInstance:
        """Create a live instance from a parsed module."""
        func_types: list[FuncType] = []
        func_bodies: list[FunctionBody | None] = []
        host_functions: list[Any | None] = []
        global_types: list[GlobalType] = []
        globals_list: list[WasmValue] = []

        memory: LinearMemory | None = None
        tables: list[Table] = []

        # Resolve imports
        for imp in module.imports:
            if imp.kind == ExternalKind.FUNCTION:
                type_idx = imp.type_info
                func_type = module.types[type_idx]
                func_types.append(func_type)
                func_bodies.append(None)
                host_func = self._host.resolve_function(imp.module_name, imp.name) if self._host else None
                host_functions.append(host_func)
            elif imp.kind == ExternalKind.MEMORY:
                imported_mem = self._host.resolve_memory(imp.module_name, imp.name) if self._host else None
                if imported_mem:
                    memory = imported_mem
            elif imp.kind == ExternalKind.TABLE:
                imported_table = self._host.resolve_table(imp.module_name, imp.name) if self._host else None
                if imported_table:
                    tables.append(imported_table)
            elif imp.kind == ExternalKind.GLOBAL:
                imported_global = self._host.resolve_global(imp.module_name, imp.name) if self._host else None
                if imported_global:
                    global_types.append(imported_global["type"])
                    globals_list.append(imported_global["value"])

        # Add module-defined functions
        for i, type_idx in enumerate(module.functions):
            func_types.append(module.types[type_idx])
            func_bodies.append(module.code[i] if i < len(module.code) else None)
            host_functions.append(None)

        # Allocate memory
        if memory is None and len(module.memories) > 0:
            mem_type = module.memories[0]
            memory = LinearMemory(
                mem_type.limits.min,
                mem_type.limits.max,
            )

        # Allocate tables
        for table_type in module.tables:
            tables.append(Table(
                table_type.limits.min,
                table_type.limits.max,
            ))

        # Initialize globals
        for global_def in module.globals:
            global_types.append(global_def.global_type)
            value = evaluate_const_expr(global_def.init_expr, globals_list)
            globals_list.append(value)

        # Apply data segments
        if memory is not None:
            for seg in module.data:
                offset = evaluate_const_expr(seg.offset_expr, globals_list)
                memory.write_bytes(offset.value, seg.data)

        # Apply element segments
        for elem in module.elements:
            if elem.table_index < len(tables):
                table = tables[elem.table_index]
                offset = evaluate_const_expr(elem.offset_expr, globals_list)
                offset_num = offset.value
                for j, func_idx in enumerate(elem.function_indices):
                    table.set(offset_num + j, func_idx)

        # Build export map
        exports: dict[str, dict[str, Any]] = {}
        for exp in module.exports:
            exports[exp.name] = {"kind": exp.kind, "index": exp.index}

        instance = WasmInstance(
            module=module,
            memory=memory,
            tables=tables,
            globals=globals_list,
            global_types=global_types,
            func_types=func_types,
            func_bodies=func_bodies,
            host_functions=host_functions,
            exports=exports,
            host=self._host,
        )

        # Bind linear memory on the WASI host if it exposes set_memory().
        if self._host is not None and hasattr(self._host, "set_memory") and memory is not None:
            self._host.set_memory(memory)

        # Call start function
        if module.start is not None:
            engine = WasmExecutionEngine(
                memory=instance.memory,
                tables=instance.tables,
                globals=instance.globals,
                global_types=instance.global_types,
                func_types=instance.func_types,
                func_bodies=instance.func_bodies,
                host_functions=instance.host_functions,
            )
            engine.call_function(module.start, [])

        return instance

    def call(self, instance: WasmInstance, name: str, args: list[int | float]) -> list[int | float]:
        """Call an exported function by name."""
        exp = instance.exports.get(name)
        if exp is None:
            msg = f'export "{name}" not found'
            raise TrapError(msg)
        if exp["kind"] != ExternalKind.FUNCTION:
            msg = f'export "{name}" is not a function'
            raise TrapError(msg)

        func_type = instance.func_types[exp["index"]]

        # Convert plain numbers to WasmValues
        wasm_args: list[WasmValue] = []
        for idx, arg in enumerate(args):
            param_type = func_type.params[idx] if idx < len(func_type.params) else ValueType.I32
            if param_type == ValueType.I32:
                wasm_args.append(i32(int(arg)))
            elif param_type == ValueType.I64:
                wasm_args.append(i64(int(arg)))
            elif param_type == ValueType.F32:
                wasm_args.append(f32(float(arg)))
            elif param_type == ValueType.F64:
                wasm_args.append(f64(float(arg)))
            else:
                wasm_args.append(i32(int(arg)))

        engine = WasmExecutionEngine(
            memory=instance.memory,
            tables=instance.tables,
            globals=instance.globals,
            global_types=instance.global_types,
            func_types=instance.func_types,
            func_bodies=instance.func_bodies,
            host_functions=instance.host_functions,
        )

        results = engine.call_function(exp["index"], wasm_args)

        # Convert WasmValues back to plain numbers
        return [r.value for r in results]

    def load_and_run(
        self,
        wasm_bytes: bytes | bytearray,
        entry: str = "_start",
        args: list[int | float] | None = None,
    ) -> list[int | float]:
        """Parse, validate, instantiate, and call in one step."""
        if args is None:
            args = []
        module = self.load(wasm_bytes)
        self.validate(module)
        instance = self.instantiate(module)
        return self.call(instance, entry, args)
