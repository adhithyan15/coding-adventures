# LANG81 — Perl source to native InterpreterIR

**Status:** Draft, 2026-10-07. First bounded Perl-language frontend under
LANG78.

## Execution path

Add a Perl-language token definition and parser grammar, then Rust
`perl-lexer`, `perl-parser`, and `perl-iir-compiler` crates. The production path
is Perl source → Rust lexer/parser → grammar tree → `interpreter-ir` → Rust
`vm-core`. Semantic IR and a host Perl interpreter are not execution stages.
A host Perl 5.40 process may only serve as a conformance oracle in tests.

## First accepted subset

The first slice accepts a file of one or more `print(<expression>);` statements.
Expressions contain decimal integer literals, parentheses, unary `-`, and
binary `+`, `-`, and `*`, with Perl's precedence and left associativity.
Each `print` has exactly one argument. It writes the argument's decimal form
without adding a newline. Statement order determines output order.

The expression is in Perl scalar context. To avoid claiming Perl's full
numeric scalar model, accept a program only when each literal and intermediate
result is provably within signed 32-bit range; reject the rest before emitting
IIR. Within that range, integer arithmetic and decimal output agree with the
selected Perl host oracle. Division, strings, variables, interpolation,
coercion, list context, builtins beyond `print`, and every other Perl form are
explicitly unsupported. The parser must reject trailing unconsumed tokens.

Bound source input before lexing, direct AST input before recursive lowering,
VM instructions, and captured output. Error messages must distinguish malformed
source, unsupported syntax, and out-of-range arithmetic.

## Acceptance

- `print(1 + 2);` emits validated IIR and runs on `vm-core`, producing `3`.
- `print(7 - 2 * 3);` produces `1`, preserving precedence.
- `print(-4); print(2);` produces `-42` with no implicit newline.
- Unsupported syntax and out-of-range arithmetic fail explicitly.
- A separate Perl oracle agrees on accepted examples; the native runner never
  launches Perl to execute a compiled program.

This pilot does not imply a complete Perl engine. Scalar values, strings,
variables, arrays, hashes, references, scope, control flow, functions, regular
expressions, modules, exceptions, and JIT tiers remain later work.
