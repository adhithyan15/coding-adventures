# VER09c — Bounded TypeScript source to WIR proof bridge (VER-P044)

**Status:** Specified; implementation is the selected next verification-framework
slice. This is a narrow adapter over the existing Rust TypeScript parser, WIR,
bit-blaster, and SAT solver. It is not a TypeScript compiler or a new solver.

**Layer:** Verification ladder, source-bound Level 3 proofs.
**Builds on:** [VER09](VER09-cdcl-sat.md), [VER09a](VER09a-typescript-sat-solver.md),
[VER25](VER25-wir-core-interpreter.md), [VER25b](VER25b-rust-to-wir-frontend.md),
and the Rust `typescript-parser` package.
**First consumer:** GBL-050a TypeScript `game-input-frame`, after this adapter is
merged and verified.

## 1. Purpose and boundary

The repository has a complete versioned TypeScript grammar parser and a mature
WIR-to-bit-blaster/SAT pipeline, but no checked path from a TypeScript function's
actual parsed body to WIR. A handwritten model does not close that gap: its SAT
result says nothing about whether the source contains the modeled operation.

This package admits a deliberately small, finite TypeScript subset, validates
declarations and lexical bindings, lowers each accepted expression node and its
operator token into WIR, type-checks that WIR, and proves explicit bit-vector
claims. Every refusal is fail-closed. Unsupported syntax is not skipped, erased,
or converted into a convenient expected expression.

The first supported slice is a fixture containing exactly three exported,
typed, single-return scalar helpers over a `bigint` mask and an integer `number`
slot. The fixture is helper-source-bound evidence for the future input-frame
consumer. It is not production reducer evidence, cross-language parity, a proof
of all TypeScript, a general compiler, or an unbounded statement about JavaScript
BigInt semantics.

## 2. Trusted components and source binding

The adapter must call `coding-adventures-typescript-parser::parse_typescript`
with the exact supported grammar version `ts5.8`. The wrapper returns the
generic parser's AST type without re-exporting it, so the adapter also declares
the existing `parser` crate as a direct dependency and imports
`parser::grammar_parser::{GrammarASTNode, ASTNodeOrToken}`. It inspects that
returned tree and its raw `Token` leaves. It must not parse a normalized string,
search source text for the expected expression, or reconstruct the operator
from the expected theorem. The current shared TypeScript grammar does not model
the ECMAScript restricted production after `return`: a probe showed it accepts
`return\n(expression)` as a return-with-expression even though JavaScript
returns `undefined`. This bridge scans the original trivia between the `return`
token and the first expression token, and refuses `LF`, `CR`, `U+2028`, or
`U+2029`, including line terminators inside comments. The broader parser repair
is tracked separately as VER-P045; this PR must not silently claim parser-wide
ECMAScript conformance.

The proof test uses `include_bytes!` on the exact checked-in TypeScript helper
fixture path, recomputes SHA-256 from those bytes, and records the
repository-relative path, parser version, function name, admitted preconditions,
and claim. The adapter rejects inputs above 4,096 bytes before UTF-8 conversion
or parsing, tokenizes with the pinned `ts5.8` lexer and refuses above 128 tokens,
then parses and refuses an AST above 512 nodes before walking it. These caps
admit the initial 317-byte fixture (89 AST tokens, 384 grammar nodes) with
bounded headroom; they are visitor/parser-work limits, not a claim about every
TypeScript grammar shape. The existing parser applies its measured recursion
ceiling of 190 rules through `create_typescript_parser`; the adapter must use
that wrapper and must not claim the ceiling is configurable or measured for all
grammar recursion shapes. WIR lowering separately refuses above 64 expression
nodes. Each cap is named, documented, and tested at its accepted/refused edge.

The bridge consumes only existing WIR types and operations, including fixed
width `U64`, `U8`, `Bool`, zero-extension, bitwise AND/OR, left shift, NOT, and
equality/inequality.
It must invoke the existing WIR type checker and symbolic/bit-blasting/SAT
machinery. It may add only the AST visitor and the explicitly justified
`BigInt(number)` conversion needed for this subset. It must not add a second
solver, use a TypeScript evaluator as the proof engine, or replace source
lowering with a manually assembled candidate WIR expression.

## 3. Admitted source subset

The initial fixture declares exactly the following helper roles:

| Role | Parameters | Return | Intended meaning |
|---|---|---|---|
| `bit_is_set` | `mask: bigint`, `slot: number` | `boolean` | Return whether the selected button bit is set. |
| `set_bit` | `mask: bigint`, `slot: number` | `bigint` | Return the mask with the selected bit set. |
| `clear_bit` | `mask: bigint`, `slot: number` | `bigint` | Return the mask with the selected bit cleared. |

The admitted program has exactly these exported declarations and no executable
top-level statements, imports, re-exports, ambient declarations, extra
functions, or mutable bindings. Each function has the exact name, parameter
order, explicit parameter and return annotations, a braced body, and exactly one
return expression. The lexical environment contains only the declared
parameters and the unshadowed global `BigInt` intrinsic. Duplicate declarations,
shadowing, rebinding, aliases, and unresolved names are refusals.

Allowed expression forms are the forms required by the fixture: parameter and
approved intrinsic references; only the BigInt literals `0n` and `1n`; the exact
`BigInt(slot)` conversion; transparent parentheses; the exact shift form
`1n << BigInt(slot)`; unary BigInt bitwise NOT; bitwise AND and OR; and
equality/inequality against zero. The implementation derives these forms from
AST rule nodes and operator tokens. It accepts syntactically supported operator
mutants such as `===` instead of `!==` and `&` instead of `|`, lowers those
actual operators, and lets the SAT query find a counterexample. It must not
reject a mutant merely because its operator differs from the expected theorem.

All other AST node kinds, operators, calls, literals, type syntax, assertions,
casts, non-null assertions, assignments, branches, loops, property access,
indexing, templates, optional chaining, and nested declarations are rejected.
Any use of JavaScript `Number` bitwise operations is rejected. TypeScript's
`number` is admitted only as the typed slot parameter, under the explicit
precondition below; the bridge does not model arbitrary IEEE-754 arithmetic.

## 4. Types, preconditions, and BigInt semantics

The symbolic inputs are a 64-bit unsigned `mask` and a slot formed from six
unconstrained Boolean inputs, zero-extended to the declared `U8` parameter.
This represents exactly `0 <= mask <= 2^64-1` and `0 <= slot <= 63` without an
unchecked symbolic range assumption. `BigInt(slot)` is accepted only as the
unshadowed realm intrinsic applied directly to that parameter; under this
precondition the conversion is exact and zero-extends to `U64` value `0..63`.
The Node replay runs in a clean child realm where the global `BigInt` intrinsic
is unmodified. No other Number-to-BigInt conversion is admitted.

WIR models the BigInt values as fixed-width unsigned values for the proof.
JavaScript's BigInt bitwise NOT is unbounded two's-complement, so the bridge may
lower `~x` to a 64-bit NOT only when the actual expression places that result
under intersection with the finite `mask: U64` value. This establishes equality
of the low 64 result bits for `mask & ~x`; it does not establish unsigned
semantics for free-standing `~x`, which is refused. For the shift, only literal
`1n` shifted left by the directly converted six-bit slot is accepted; this
prevents unbounded literal truncation, negative or fractional shift counts, and
overshift behavior. Right shift, unsigned right shift (`>>>`), nested shifts,
and arbitrary shift operands are refused. The proof compares complete 64-bit
outputs, not only a congruence class or a truncated low-bit claim.

## 5. Proof obligations and negative controls

For every admitted mask and slot, use a 64-bit symbolic mask and six symbolic
slot bits and ask the existing SAT backend to solve a mismatch between the
lowered source function and an independent per-bit reference:

1. `bit_is_set(mask, slot)` equals the selected bit.
2. `set_bit(mask, slot)` equals `mask | (1 << slot)`.
3. `clear_bit(mask, slot)` equals `mask & ~(1 << slot)` in the low 64 bits.

Each baseline mismatch query must be UNSAT. The reference is built as an
independent equality/mux over all 64 bit positions; it must not call the
candidate helper, reuse its lowered operator tree, or share a candidate-derived
truth function.

The same parser, checker, lowerer, and SAT path must admit at least these
source-level mutants and produce SAT witnesses:

- flip the `bit_is_set` comparison from `!== 0n` to `=== 0n`;
- replace the set helper's `|` operator with `&`.

Decode each model into a complete 64-bit decimal mask and slot, then execute the
healthy and mutated helper source under Node and show that the witness
distinguishes the helper from the independent reference. A SAT model without
source mutation, decoding, and runtime replay is not a falsification control.
Proof status must distinguish parsed/checked, lowered, type-checked, SAT-UNSAT,
SAT-mutant, model-decoded, and runtime-replayed stages; failures cannot be
reported as proved.

## 6. Validation and evidence

The package acceptance suite includes:

- exact fixture-byte digest and source-path binding;
- parser acceptance and AST-shape checks for all three helpers;
- positive lowering and type-checking checks, with the actual WIR operator tree
  asserted against the source nodes;
- refusal tests for every unsupported statement/node family, wrong exports,
  wrong/missing types, top-level execution, duplicate/shadowed/rebound names,
  fake or nested BigInt calls, Number bitwise operations, casts, and budget edges;
- exhaustive slot enumeration `0..63` against an independent Boolean-array
  reference, plus both mask-bit values at each slot;
- three baseline mismatch-UNSAT proof queries and the required source-mutant
  SAT queries with full-width decoded witnesses and Node execution;
- seeded bounded property tests and a separate fuzz/refusal campaign for
  source/AST admission. Seeds, generated counts, strata, shrinking where
  available, budgets, and refusals are reported;
- non-vacuity controls that demonstrate unsupported inputs refuse, supported
  mutants reach SAT, the reference can differ, and broken source binding or
  incorrect type/operator lowering fails a test.

The source-binding harness must be exercised with the healthy fixture before
lowering/proving and with independent wrong-path, wrong-digest, and mismatched
embedded-byte inputs. Each broken binding must refuse at the harness gate before
the parser, WIR checker, or SAT prover runs.

Coverage must exceed 95% for meaningful library lines where the repository's
Rust tooling can measure it. A raw line percentage does not replace explicit
admission/refusal coverage, proof controls, or evidence that mutant operators
reach the solver.

## 7. Package and build integration

The package identity is `typescript-warrant-ir` under
`code/packages/rust/typescript-warrant-ir`. It is a proof/development dependency,
not a dependency of TypeScript runtime packages. It must have `Cargo.toml`,
`src/lib.rs`, tests and the checked-in source fixture, README, changelog,
`BUILD`, `BUILD_windows`, required-capabilities metadata, and documentation of
its narrow subset and claim limits. Dependencies must be existing path crates
and include the Rust `typescript-parser`, its `typescript-lexer` for the pre-parse
token count, the generic `parser` crate for AST types, WIR, the bit-blaster for
an independent reference circuit, and SHA-256. WIR owns the CDCL SAT dependency;
the adapter calls WIR's prover rather than opening a second solver. Add the package to every canonical
package matrix/classification that tracks Rust package identity and testing;
presence does not mean it is a TypeScript runtime parity implementation.

The repository build graph must see changes to the proof input TypeScript
fixture as inputs to this package's source-bound proof consumer. Update the
validated dependency/source-input boundary and its generated or pinned digests
where required. `BUILD` and `BUILD_windows` must run the same proof and refusal evidence. The
Cargo workspace manifest declares the path-dependency graph; Cargo resolves and
compiles that closure in dependency order, so no parallel hand-maintained
leaf-to-root command list is needed. The TypeScript fixture is included in the
Rust test resource source-input suffix registry and its pinned boundary digest
is refreshed with this change. No proof input may live outside the
declared source-input boundary.

## 8. Claim limits and follow-up

This slice proves only the three accepted helper expressions from the exact
fixture bytes, for the declared finite domains and the encoded 64-bit semantics.
It does not prove any production consumer before that consumer exists. Once the
GBL-050a TypeScript reducer is added, a separate proof must bind its actual
`digital.ts` bytes, hash, AST, and helpers, then replay mutant witnesses through
the actual public reducer. A successful helper fixture proof cannot be relabeled
as a production reducer proof.

Whole-package TypeScript semantics, general control flow or memory, arbitrary
BigInt, all dynamic Number behavior, the record-based reducer, fixture decoder,
device polling, cross-language parity, and unbounded program behavior remain
outside this contract unless separately specified and proved.

## 9. Delivery record

VER-P044 is complete on main through PR #19685, protected squash
`9511584649b1c3294d38a9742f97eac7ddd9ae5b`, from reviewed head
`02cbd0e67e79dbe12acc5a4f1cefb86bc34b3080`. Exact-head CI run `44486323864`,
Human Languages Books run `44486323844`, and Swift Apps run `44486323707` all
passed. The CI build plan selected this crate and `go/programs/build-tool` on
Linux, macOS, and Windows; all six package outcomes were `BUILT`. The Go package
rows include the same-relative-path, same-length TypeScript fixture A→B→A cache
invalidation test. On Rust 1.100.0, strict all-target Clippy, debug and release
test suites passed; library coverage was 394/408 lines (96.57%).

The CI build tool also runs strict Clippy before the crate's declared `BUILD`
commands. The first PR's macOS CI run exposed a collapsible-if lint in the
library. A subsequent local Rust 1.100 strict all-target Clippy run exposed a
needless-range-loop lint in the independent test oracle; both were repaired
without changing lowering behavior or the oracle's bit order. This gate must be
included when describing the effective build recipe and local reproduction.
These completion receipts do not extend the claim boundary above: the checked
source remains the helper fixture, and production reducer binding, guarded
mutant replay, and proof-tool transitive cache invalidation remain consumer
acceptance work.
