# brainfuck-iir-compiler

Brainfuck → InterpreterIR (IIR) compiler, plus a `BrainfuckVM` wrapper around
`vm-core` that runs Brainfuck programs through the generic LANG pipeline.

This package is the Brainfuck mirror of `tetrad-compiler`'s migration path
(LANG01 §"Migration path for Tetrad"): once the source is in IIR form, every
piece of LANG infrastructure — `vm-core`, `jit-core`, `aot-core`,
`debug-integration`, `lsp-integration`, `repl-integration`,
`notebook-kernel` — is available without writing any Brainfuck-specific
runtime code.

See [BF04 spec](../../../specs/BF04-brainfuck-iir-compiler.md) for the full
design and the command → IIR mapping.

## How it fits in the stack

```
Brainfuck source
     │
     ▼  brainfuck.parse_brainfuck   (BF01)
ASTNode tree
     │
     ▼  brainfuck_iir_compiler.compile_to_iir   (THIS PACKAGE)
IIRModule (LANG01)
     │
     ▼  vm_core.VMCore.execute   (LANG02)
program output
```

The `BrainfuckVM` class wraps the bottom three boxes into a single
`run(source) -> bytes` call.

## Quick start

```python
from brainfuck_iir_compiler import BrainfuckVM

vm = BrainfuckVM()
out = vm.run("++++++++[>++++++++<-]>+.")  # outputs '\x41' = 'A'
print(out)   # b'A'
```

Echo program — feed `input_bytes` to satisfy `,` reads:

```python
vm = BrainfuckVM()
out = vm.run(",.,.,.", input_bytes=b"abc")
print(out)   # b'abc'
```

Inspecting the compiled IR:

```python
from brainfuck_iir_compiler import compile_source
module = compile_source("++[-]")
for instr in module.functions[0].instructions:
    print(instr)
```

## Why this exists alongside `brainfuck-ir-compiler`

`brainfuck-ir-compiler` targets the **static AOT** path (CompilerIR →
ISA backend → native binary).  `brainfuck-iir-compiler` targets the
**interpreted + JIT** path (InterpreterIR → vm-core → jit-core).  They are
two different IRs serving two different consumers; see BF04 §"Why a
separate package" for the rationale.

## Status

- ✅ Compiler: AST → IIRModule, FULLY_TYPED, full test coverage
- ✅ VM wrapper: interpreted execution, u8 wrap, builtin-wired stdio
- ⏭️ JIT: `BrainfuckVM(jit=True)` deferred to BF05 (raises
  `NotImplementedError` for now with a pointer to the spec)
