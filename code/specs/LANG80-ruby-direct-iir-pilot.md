# LANG80 — Ruby integer expressions on the native LANG VM

**Status:** Draft, 2026-10-07. First bounded Ruby-to-IIR pilot.

## Execution path

Ruby 3.0 source is tokenized and parsed by the existing Rust `ruby-lexer` and
`ruby-parser`. A new `ruby-iir-compiler` lowers the parser tree directly to
`interpreter-ir`; Rust `vm-core` executes it. Semantic IR and a host Ruby
runtime are not stages in the execution path. A Ruby installation may only
serve as an independent conformance oracle.

## Accepted source subset

The pilot accepts one or more `puts(<expression>)` statements. Each call has
exactly one positional argument. Expressions contain decimal integer literals,
parentheses, unary `-`, and binary `+`, `-`, `*`, `/` with Ruby's precedence and
left associativity. Variables, assignments, strings, interpolation, method
definitions, multiple arguments, and every other construct are rejected with
an unsupported-syntax diagnostic. The parser's original recursion guard
remains in force.

Ruby integers have arbitrary precision. This pilot accepts only programs for
which each literal and every intermediate expression value is provably within
`i64` range. There are no variables in the subset, so the compiler can check
that property with widened integer arithmetic before emitting IIR. Inputs
outside the range are rejected. The VM's `i64` arithmetic then agrees exactly
with Ruby for accepted `+`, `-`, and `*` expressions, with no runtime overflow.

Ruby integer `/` rounds toward negative infinity, while the VM's generic
integer division rounds toward zero. Emit a Ruby-specific VM builtin for `/`
that implements floor division and reports `ZeroDivisionError` for a zero
divisor. Reject any statically known division whose result would exceed `i64`
range. `puts` uses a separate Ruby-specific builtin that writes the integer's
decimal form and one newline. Output is bounded by the runner.
The pilot limits source to 64 KiB before parsing, a directly supplied AST to
16,384 nodes or tokens and depth 256 before recursive lowering, execution to
100,000 instructions, and captured output to one million bytes. Direct AST
input also has a one MiB aggregate text limit and 64 KiB per rule name or token
field, including correlation IDs; the module name has the same field limit.
The item count is enforced before adding children to the traversal queue.

## Acceptance

- `puts(1 + 2)`, `puts(7 / 2)`, and `puts(-7 / 2)` execute through source →
  Ruby parser → IIR → Rust `vm-core`, producing `3`, `3`, and `-4` respectively.
- The emitted IIR contains arithmetic instructions and the Ruby-specific
  builtins, rather than a precomputed output string or a host Ruby call.
- Integer range overflow, division by zero, unsupported syntax, and malformed
  source fail without panics. In particular, the compiler must use a fallible
  Ruby parser entry point so a lexer error is returned rather than unwrapped.
  A separate Ruby oracle cross-checks representative accepted programs.
- Oversized directly supplied AST item counts, nesting, text fields, and module
  names fail before lowering or copying token text.

This is an interpreter pilot. Bindings, objects, blocks, exceptions, the full
integer tower, float literals, and JIT execution remain later work.
