"""nib_ir_compiler — Translates a typed Nib AST into general-purpose IR.

=============================================================================
WHAT THIS PACKAGE DOES
=============================================================================

``nib-ir-compiler`` is Stage 4 of the Nib compiler pipeline. It takes the
typed ``ASTNode`` tree produced by ``nib-type-checker`` and emits an
``IrProgram`` (from ``compiler-ir``) containing target-independent IR
instructions.

The IR uses virtual registers (infinite supply, named v0, v1, v2, ...) and
a fixed addressing scheme that any backend can map to physical registers.

Stage in the pipeline::

    Source text
        → Nib Lexer          (characters → tokens)
        → Nib Parser         (tokens → untyped ASTNode tree)
        → Nib Type Checker   (untyped AST → typed AST)
        → Nib IR Compiler    (typed AST → IrProgram)   ← this package
        → Backend Validator  (IrProgram → validated IR for specific ISA)
        → Code Generator     (validated IR → Intel 4004 machine code)

=============================================================================
VIRTUAL REGISTER LAYOUT
=============================================================================

Nib v1 uses a simple fixed allocation::

    v0  = zero constant (always 0, preloaded at _start)
    v1  = scratch / expression result temporary / return value
    v2+ = named variables (locals, params, statics), allocated in order

The Intel 4004 has 16 4-bit registers (R0–R15) arranged as 8 pairs. Mapping
virtual registers to physical registers is the backend's job.

=============================================================================
CALLING CONVENTION
=============================================================================

    Arguments:    v2, v3, v4, ... (caller prepares these before CALL)
    Return value: v1 (callee stores result here before RET)
    Callee regs:  v2+ (fresh allocation per function)

=============================================================================
USAGE
=============================================================================

::

    from nib_parser import parse_nib
    from nib_type_checker import check
    from nib_ir_compiler import compile_nib

    source = '''
        fn add(a: u4, b: u4) -> u4 {
            return a +% b;
        }
        fn main() {
            let result: u4 = add(3, 4);
        }
    '''

    ast = parse_nib(source)
    result = check(ast)
    assert result.ok

    compiled = compile_nib(result.typed_ast)
    # compiled.program: IrProgram with all IR instructions
    # compiled.source_map: None (v1 — full source map is a future feature)

=============================================================================
PUBLIC API
=============================================================================

- ``compile_nib(typed_ast, config?) -> CompileResult``
  Main entry point. Takes a typed AST and returns an ``IrProgram``.

- ``CompileResult`` — dataclass with ``program: IrProgram`` and
  ``source_map: SourceMapChain | None``.

- ``BuildConfig`` — controls debug/release mode.
  ``debug_config()``   — emit COMMENT instructions with source info.
  ``release_config()`` — no debug comments.
"""

from __future__ import annotations

from nib_ir_compiler.build_config import BuildConfig, debug_config, release_config
from nib_ir_compiler.compiler import CompileResult, compile_nib

__all__ = [
    "BuildConfig",
    "CompileResult",
    "compile_nib",
    "debug_config",
    "release_config",
]
