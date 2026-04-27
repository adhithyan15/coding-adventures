"""oct_ir_compiler — Lowers a typed Oct AST into general-purpose IR.

=============================================================================
WHAT THIS PACKAGE DOES
=============================================================================

``oct-ir-compiler`` is Stage 4 of the Oct compiler pipeline. It accepts the
type-annotated ``ASTNode`` tree produced by ``oct-type-checker`` and emits
an ``IrProgram`` (from ``compiler-ir``) containing target-independent IR
instructions.

Stage in the pipeline::

    Source text
        → oct-lexer         (characters → tokens)
        → oct-parser        (tokens → untyped ASTNode tree)
        → oct-type-checker  (untyped AST → typed AST)
        → oct-ir-compiler   (typed AST → IrProgram)   ← this package
        → intel-8008-ir-validator   (pre-flight IR check)
        → ir-to-intel-8008-compiler (IrProgram → 8008 assembly)
        → intel-8008-assembler      (two-pass assemble to binary)
        → intel-8008-packager       (Intel HEX output)

=============================================================================
VIRTUAL REGISTER LAYOUT
=============================================================================

Oct v1 uses a simple fixed allocation::

    v0  = zero constant (always 0, preloaded at _start)
    v1  = scratch / expression result temporary / return value
    v2+ = named variables (locals, params), allocated in declaration order

The Intel 8008 maps v1→A, v2→B, v3→C, v4→D, v5→E.
The IR validator enforces the 4-local limit (v2–v5).

=============================================================================
CALLING CONVENTION
=============================================================================

    Arguments:    v2, v3, v4, v5 (caller prepares before CALL)
    Return value: v1 (callee stores result here before RET)
    Callee regs:  v2+ (fresh allocation per function)

=============================================================================
USAGE
=============================================================================

::

    from oct_parser import parse_oct
    from oct_type_checker import check_oct
    from oct_ir_compiler import compile_oct

    source = '''
        static THRESHOLD: u8 = 128;

        fn process(val: u8) -> bool {
            return val > THRESHOLD;
        }

        fn main() {
            let data: u8 = in(0);
            let high: bool = process(data);
            if high {
                out(1, data);
            }
        }
    '''

    ast = parse_oct(source)
    tc_result = check_oct(ast)
    assert tc_result.ok

    compiled = compile_oct(tc_result.typed_ast)
    # compiled.program: IrProgram with all IR instructions
    # compiled.program.entry_label == "_start"

To target WASM, JVM, or CLR instead of the Intel 8008::

    from oct_ir_compiler import compile_oct, WASM_IO, JVM_IO, CLR_IO

    wasm_result = compile_oct(tc_result.typed_ast, config=WASM_IO)
    jvm_result  = compile_oct(tc_result.typed_ast, config=JVM_IO)
    clr_result  = compile_oct(tc_result.typed_ast, config=CLR_IO)

=============================================================================
PUBLIC API
=============================================================================

- ``compile_oct(typed_ast, config=INTEL_8008_IO) -> OctCompileResult``
  Main entry point.  Takes a type-annotated AST and an optional
  ``OctCompileConfig`` and returns an ``OctCompileResult`` wrapping the
  compiled ``IrProgram``.

- ``OctCompileConfig`` — frozen dataclass controlling how ``in()`` / ``out()``
  lower to SYSCALL IR.  Fields: ``write_byte_syscall``, ``read_byte_syscall``
  (both ``int | None``; ``None`` = 8008 port-based encoding).

- ``INTEL_8008_IO``, ``WASM_IO``, ``JVM_IO``, ``CLR_IO`` — pre-built
  ``OctCompileConfig`` instances for the four supported targets.

- ``OctCompileResult`` — dataclass with ``program: IrProgram``.
"""

from __future__ import annotations

from oct_ir_compiler.compiler import (
    CLR_IO,
    INTEL_8008_IO,
    JVM_IO,
    WASM_IO,
    OctCompileConfig,
    OctCompileResult,
    compile_oct,
)

__all__ = [
    "CLR_IO",
    "INTEL_8008_IO",
    "JVM_IO",
    "OctCompileConfig",
    "OctCompileResult",
    "WASM_IO",
    "compile_oct",
]
