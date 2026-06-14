"""wasm-backend: BackendProtocol implementation for WebAssembly 1.0.

This package wires together LANG21 (cir-to-compiler-ir) and LANG20
(ir-to-wasm-compiler) to deliver the missing link in the Tetrad
compilation pipeline:

  JIT/AOT specialisation → list[CIRInstr]
      ↓  LANG21: lower_cir_to_ir_program()
  IrProgram
      ↓  LANG20: IrToWasmCompiler().compile()
  WasmModule → bytes
      ↓  wasm-runtime: WasmRuntime().load_and_run()
  result

``WASMBackend`` satisfies the ``BackendProtocol`` / ``CIRBackend`` structural
protocol from ``codegen-core``, making it a drop-in backend for
``jit_core.JITCore`` and ``TetradRuntime.run_with_jit()``:

    from wasm_backend import WASMBackend
    from tetrad_runtime import TetradRuntime

    rt = TetradRuntime()
    result = rt.run_with_jit("fn main() -> u8 { return 40 + 2; }",
                             backend=WASMBackend())
    # result == 42

Public API
----------
``WASMBackend``
    BackendProtocol implementation — ``compile(cir)`` → bytes,
    ``run(binary, args)`` → Any.
"""

from wasm_backend.backend import WASMBackend

__all__ = ["WASMBackend"]
