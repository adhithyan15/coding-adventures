# LANG79 — Python float expressions on the native LANG VM

**Status:** Draft, 2026-10-07. This is the first bounded Python-to-IIR slice
under LANG78's four-language frontend program.

## Pipeline

Python 3.12 source is tokenized and parsed by the existing Rust `python-lexer`
and `python-parser` crates. The new `python-iir-compiler` lowers that parser's
grammar tree directly to `interpreter-ir`; `vm-core` executes the result in
Rust. Semantic IR is not an execution stage. The Python host interpreter may
be used only to confirm expected results.

## Accepted source

The first slice accepts a file of expression statements formed only from
Python float literals, parentheses, unary `+` and `-`, binary `+`, `-`, `*`,
and `/`, and `print(<one accepted expression>)`. Statements execute in source
order. Every literal and intermediate arithmetic result has Python `float`
semantics. A numeric spelling without a decimal point or exponent is an
arbitrary-precision Python `int`, so this slice rejects it. It also rejects
assignments, names other than the exact `print` callee, keyword arguments,
multiple print arguments, semicolons, and every other statement or operator.
Errors must identify unsupported syntax rather than silently omit it.
Python float division by zero raises an error; the VM lowering must check that
case rather than inheriting an IEEE infinity result from a generic `f64` divide.

The pilot rejects source larger than 64 KiB before tokenization; the file
runner enforces that bound while reading. Its AST-input
entry point also rejects trees exceeding 16,384 nodes or tokens, or 64 nested
grammar nodes, before recursive lowering. Supplied ASTs additionally have a
1 MiB aggregate text budget and a 64 KiB limit per rule name or token field;
the IIR module name has the same field limit. These limits bound frontend and
generated-IIR work even when an AST is supplied directly by another caller.

`print` is a Python-specific VM builtin registered by the Rust runner. It
prints one float and a newline. Its accepted display range is zero and finite
values with absolute magnitude from `1e-4` through `1e15`; values outside
that range are rejected until Python's full float formatting is implemented.
Within this range, integral floats include a `.0` suffix and negative zero
prints `-0.0`. NaN and infinities are initially rejected at the print
boundary. This display restriction does not change arithmetic semantics.
If a later statement fails at runtime, an earlier successful `print` remains
visible on stdout; the runner's error value retains both that output and the
error message.

## Acceptance

- Source `print(1.0 + 2.0)` executes on `VMCore` and prints `3.0`.
- Source `print(1.0 / 2.0)` executes on `VMCore` and prints `0.5`.
- Source `print(-0.0)` prints `-0.0`, preserving the float sign bit.
- `print(1 + 2)`, `print(1.0 // 2.0)`, and assignment fail explicitly.
- `print(1.0 / 0.0)` raises a division-by-zero error on the native runner.
- `print(1.0)` followed by a failing division still writes `1.0` before the
  error, matching source-order Python output.
- The emitted `IIRModule` validates before execution; tests confirm that the
  Rust runner does not invoke a host Python process.
- Oversized source and directly supplied deep or wide ASTs fail with explicit
  pilot-limit errors before lowering; oversized AST text and module names do too.

The slice is an interpreter pilot, not a Python engine completion claim.
Bindings, integers, strings, truth, control flow, functions, exceptions,
objects, library modules, and JIT tiers remain separate work.

## Bounded follow-up: zero-argument `print()`

Accept the exact Python 3.12 grammar call shape `print()` as a statement. Lower
it directly from the grammar tree to an IIR call of a zero-argument Rust
`vm-core` builtin that appends one newline to captured stdout. It consumes
the same output budget and preserves the same source-order error behavior as
the existing one-float `print` path. Host Python is a conformance oracle only.

This stage does not accept multiple arguments, keyword arguments, other
callees, or new expression forms. Both source parsing and direct AST lowering
must reject malformed call shapes rather than treating them as an empty call.
The callee must be a name token in an `atom` node and both delimiters must be
actual parenthesis tokens; matching token text alone does not validate a
caller-supplied AST. Apply the same check to the one-argument path.
`print()\nprint(1.0)\n` must produce `\n1.0\n`; a completed empty print before
a later float-division failure must remain in the typed run error's output.
