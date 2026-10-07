# LANG78 — Native dynamic-language frontends for InterpreterIR

**Status:** Draft, 2026-10-07. This spec tracks four bounded frontend lines.

## Execution contract

JavaScript, Python, Ruby, and Perl programs should eventually run in the
repository's Rust LANG VM. Each language owns its syntax and semantics:

```text
source → language lexer/parser → language AST/CST lowering → IIRModule
       → vm-core interpreter → later JIT tiers
```

Semantic IR is **not** a stage in this execution path. Its existing
source-to-source frontends remain separate; any future integration must not
change the direct language-to-IIR contract. A host runtime may serve as a
test oracle, but must not execute the compiled program in production.

Use one active implementation PR at a time. Interleave this queue with the
existing LANG VM backlog and PREP01 preprocessor queue, selecting the next
bounded item after each verified merge.

## Current foundations

| Language | Rust lexer/parser | Lowering input | First missing piece |
| --- | --- | --- | --- |
| JavaScript | Yes, including a typed `javascript_ast::Program` bridge | Typed AST | Direct AST-to-IIR compiler |
| Python | Yes, versioned through 3.12 | Generic grammar tree | Direct tree-to-IIR compiler |
| Ruby | Yes | Generic grammar tree | Direct tree-to-IIR compiler |
| Perl | No Perl-language grammar, lexer, or parser in Rust | None | Define a bounded Perl grammar and parser first |

The existing `interpreter-ir` and `vm-core` crates already support `f64`
constants and arithmetic, `call_builtin`, and execution through `VMCore`.
That is sufficient for first numeric programs. It is not a promise that
dynamic objects, coercions, closures, exceptions, and built-in libraries
already work unchanged. Emit a clear unsupported-syntax or unsupported-
semantic diagnostic whenever the selected slice cannot preserve behavior.

## Slice A — JavaScript interpreter pilot

Create `code/packages/rust/javascript-iir-compiler` with `compile_source`
and `compile_ast` APIs. Read the typed AST from `javascript-parser`; do not
round-trip through Semantic IR. Compile JavaScript numeric literals as
`Operand::Float` even when their spelling has no decimal point. The first
accepted source subset is expression statements containing numeric literals,
parentheses, unary minus, and binary `+`, `-`, `*`, `/`, plus
`console.log(<accepted expression>)`. Preserve evaluation order. Compile to
an `IIRModule` with a `main` function. `console.log` uses a narrowly named
VM builtin registered by the JavaScript runner; its output matches JavaScript
number formatting for the accepted values. The pilot accepts ordinary finite
display values with magnitude from `1e-6` (inclusive) to `1e21` (exclusive),
plus zero, NaN, and infinities. It rejects other display magnitudes until the
full ECMAScript number-to-string algorithm is available. Reject all other
syntax.

Acceptance requires actual source → parser → IIR → `VMCore` tests for
`console.log(1 + 2)` and `console.log(1 / 2)`, plus a negative test for an
unsupported construct. A separate Node oracle may cross-check results.
The native runtime test may not invoke Node to execute the compiled module.

## Slice B — Python

Create `python-iir-compiler` against `python-parser`'s grammar tree. Its first
slice may accept only float literals and float arithmetic plus `print`, and
must reject integer programs until arbitrary-precision integer behavior is
provided. The Python `1 / 2`, `//`, truth, and print contracts need their own
lowering and tests; do not reuse JavaScript numeric rules by accident.

## Slice C — Ruby

Create `ruby-iir-compiler` against `ruby-parser`'s grammar tree. Its first
slice may accept only float literals and float arithmetic plus `puts`, and
must reject integer programs until Ruby integer and `/` behavior can be
preserved. Add native VM execution and Ruby-oracle conformance tests.

## Slice D — Perl

First add a bounded Perl-language token and grammar definition plus Rust
lexer/parser. Then create `perl-iir-compiler` against that parser. The first
executable slice may be numeric scalar expressions and `say`, but must define
Perl's scalar context, coercion, and version target before lowering them.

## Shared runtime follow-ups

After the scalar pilots, expand each language separately through bindings,
assignments and scope, strings and coercions, control flow, functions,
closures, collections, objects, exceptions, and library calls. Add the
language binding/runtime hooks where semantics require them. Only then add
JIT execution against the same IIR with guards and deoptimization; retain
interpreter tests as the reference behavior.

Do not declare a language complete from its first scalar pilot.
