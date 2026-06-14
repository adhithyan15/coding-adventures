"""WASMCodeGenerator — CodeGenerator[IrProgram, WasmModule] adapter (LANG20).

This module adapts the existing ``IrToWasmCompiler`` / ``validate_for_wasm``
to the ``CodeGenerator[IR, Assembly]`` protocol defined in ``codegen-core``.

Pipeline context
----------------

The ``WASMCodeGenerator`` sits at the *codegen boundary*::

    IrProgram
        ↓
    [WASMCodeGenerator.validate()]   — check opcode support, WASM constraints
    [WASMCodeGenerator.generate()]   — emit structured WasmModule
        ↓ WasmModule  (structured WebAssembly 1.0 module)
        ├─→ wasm-module-encoder.encode_module(module)  → bytes  (AOT, future)
        ├─→ wasm-simulator.load(module)                          (simulator pipeline)
        └─→ bytes file on disk                                    (package+run, future)

The generator does **not** encode the module to binary bytes.  Call
``wasm_module_encoder.encode_module(module)`` for the binary form.

``name = "wasm"``
    Short identifier used by ``CodeGeneratorRegistry`` lookups.

Why ``WasmModule`` and not plain ``bytes``?
    ``WasmModule`` is a structured, inspectable object that carries type
    definitions, function bodies, memory config, data segments, and exports.
    Downstream consumers — simulator, optimizer, assembler — benefit from
    structured access.  ``encode_module()`` converts it to standard WASM 1.0
    binary when bytes are needed.
"""

from __future__ import annotations

from compiler_ir import IrProgram
from wasm_types import WasmModule

from ir_to_wasm_compiler.compiler import IrToWasmCompiler, validate_for_wasm


class WASMCodeGenerator:
    """Validate-and-generate adapter for the WebAssembly backend.

    Satisfies ``CodeGenerator[IrProgram, WasmModule]`` structurally.

    The WASM backend emits standard WebAssembly 1.0 module objects.  The
    module can be encoded to binary bytes via ``wasm-module-encoder``'s
    ``encode_module()`` function, or passed directly to the WASM simulator.

    Attributes
    ----------
    name:
        ``"wasm"`` — used by ``CodeGeneratorRegistry`` for lookup.

    Examples
    --------
    >>> from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister
    >>> prog = IrProgram(entry_label="_start")
    >>> load = IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0)
    >>> prog.add_instruction(load)
    >>> prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    >>> gen = WASMCodeGenerator()
    >>> gen.validate(prog)
    []
    >>> module = gen.generate(prog)
    >>> isinstance(module, WasmModule)
    True
    """

    name = "wasm"

    def __init__(self) -> None:
        self._compiler = IrToWasmCompiler()

    def validate(self, ir: IrProgram) -> list[str]:
        """Validate ``ir`` for WebAssembly compatibility.

        Checks performed (see ``validate_for_wasm`` for full details):

        - Every opcode is in the supported WASM backend set.
        - Value ranges and type constraints are satisfied.

        Parameters
        ----------
        ir:
            The ``IrProgram`` to inspect.

        Returns
        -------
        list[str]
            Human-readable error messages.  Empty list = compatible.
        """
        return validate_for_wasm(ir)

    def generate(self, ir: IrProgram) -> WasmModule:
        """Compile ``ir`` to a structured WebAssembly 1.0 module.

        Runs ``validate()`` internally.  Raises ``WasmLoweringError`` if
        the program fails validation.

        Parameters
        ----------
        ir:
            A validated (or to-be-validated) ``IrProgram``.

        Returns
        -------
        WasmModule
            Structured WASM 1.0 module.  Call
            ``wasm_module_encoder.encode_module(module)`` to get binary bytes.

        Raises
        ------
        WasmLoweringError
            If the IR contains unsupported instructions or invalid operands.
        """
        return self._compiler.compile(ir)
