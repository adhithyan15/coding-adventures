# Changelog

## 0.6.0 — WX-N fixed-length byte arrays across the host boundary (VER04j 14.4)

- The language already had `Bytes<N>`; the gap was the binding and generator.
  New host type `bytes<N>` (the Rust `[u8; N]`, `&[u8; N]` as a top-level
  parameter), accepted only against a Warrant `Bytes<N>` of the same literal
  width (W0701 otherwise), width 0 to 4096.
- The composite adapter accepts `Bytes<N>` parameters (it refused them) and
  narrows with `try_from`; a wrong length is the harness's `Unreachable` signal,
  never a truncation.  A `Bytes<N>` result is length-checked whatever the host
  type, outside the panic containment, so a wrong-length export fails the harness.
- New op option `err_any=CASE` for a host `Result<T, E>` with an opaque `E`
  (`&'static str`): every `Err(_)` is the one Warrant case.  Sound only for an
  error type with exactly one case, so it is refused (W0704) otherwise: with
  two or more cases a wrong error would still map to `CASE` and `f(..) ==
  Err(CASE)` would pass vacuously.  Use `errors=` for a multi-case type.
- Known gap (LP-012): suite-level "exhaustive" totals include sampled
  `Bytes<N>` cases; the per-claim `domain` field is the authoritative label.
- Honest domain labels: a v1 `Bytes<N>` binder is `full` for N <= 1 and `sample`
  from N = 2 (it was `set`), through a `bytesn` prelude region that is emitted
  only for a document with a `Bytes<N>` binder, so no existing harness changes
  by a byte (`gen_evidence --check` on all adopters is the proof).
- WD-013 re-checked: closed for v1, pinned by a new test; the v0 evaluator is
  left frozen (backlog row).  Fixed arrays of integers are backlog row WD-015.
- Tests: U8 multiply and `shl` wrapping pinned alongside addition.  `wxn_generator` (host type, refusals, emitted conversions),
  `wxn_evidence` + `wxn_regen` (a compiled, running fixture: suite and negative
  controls, `sample`/`full` labels, both length checks, wrapping `U8`).

## Unreleased — VER04i WD-003 portable corpus

- Add a closed ct-compare Warrant v0 corpus profile generated from the checked
  Warrant source, with distinct subject-operation and pure-model IR, canonical
  admitted-case enumeration, independent input/result SHA-256 streams, generated
  examples and sabotage-control descriptors, and byte-stable `--check` support.
- Add a stdlib-only Python oracle that independently validates source-bound
  declarations, suite semantics, corpus digests and TFV examples, with mutation
  tests for incomplete or misbound evidence. The first profile rejects sharding
  and unsupported Warrant constructs; it does not claim to execute a lane.
- Route the corpus generator, oracle and their tests through package BUILD and
  repository-input selection.

- **`gen_evidence` passes the bindings' host lines through (fix).** The example
  that every package `BUILD` runs (`--example gen_evidence -- [--check] <pkg>`)
  built its `EvidenceOptions` with `hosts: Default::default()`, so the WX-A host
  lines of `evidence.bindings` (`struct`, `enum`, and `args=`/`ret=`/`errors=` on
  an `op` line) were parsed and then dropped. Only the in-test drivers passed
  them, so no real package could bind a host composite: an operation returning
  `Result<(u64, usize), Leb128Error>` failed with W0701 ("an args= entry is not a
  host type") or W0704 ("neither error normalization nor errors="). The driver
  now passes `b.hosts.clone()`. New `tests/gen_evidence_driver.rs` compiles the
  example into the test crate (`#[path]`) and drives `generate_one` on a scratch
  repository built from the WX-A fixture (`struct`/`enum` lines, composite
  `args=`, tuple and record `ret=`, `errors=`): generation succeeds, equals the
  in-test driver's output byte for byte, `--check` passes, and a hand edit is
  reported as drift. Before the fix the first two tests failed with the W0701
  above. A bindings file without host lines parses to the default, so the
  checked-in ct-compare and gf256 harnesses regenerate byte-identically
  (`gen_evidence --check gf256 ct-compare`, `v0_compat`, and the wave-3 regen
  tests). `generate_one` became `pub(crate)` for the test; nothing else changed.
- **`tests/v0_compat.rs` admits the first v1 adopter without relaxing v0.** It
  asserted that every `code/specs/*.warrant` and every bound package's document
  is v0, which `wasm-leb128.warrant` (header `warrant wasm_leb128 v1`) broke.
  The pre-WX-B documents (`ct-compare`, `gf256`) are now pinned by name and must
  still be v0 and present; every document must type-check; a v1 adopter's
  harness must regenerate byte for byte with its bindings' host lines, exactly as
  `gen_evidence --check` does, and record `warrant_version`; and for each v0
  package the harness generated with the host lines passed (the driver's path
  since the fix above) must equal the one generated without them.

## 0.5.0 - Warrant v1: WX-D1 slice S1 (checker and ground evaluator)

Implements the checker and ground-evaluator half of VER04k WX-D1 (slice S1 of
section 8.1), following the S0 errata of section 9.1. All of it is under the `v1`
header: a v0 document means exactly what it did, the checker's `Debug` output for
`ct-compare`/`gf256` is unchanged (`v0_compat`), and every generated harness
(`wx_regen`, `wxa_regen`, `wxh_regen`, `wxv_regen`) is byte-identical. Generation
still refuses the new forms and list literals with W0701 (slice S2a translates
them). `CheckedWarrantModule`, `CodegenLimits` and `EvalValue` keep their shape.

- **Checker.** `V1_FUNCTIONS` grows from 33 to 45 names: the seven D1 forms
  `take drop slice replicate range to_list to_bytes` and the five D2 names
  `fold map count all any`, all reserved in v1 (a v1 declaration so named is
  W0201) and still free in v0 (a call is W0202, a v0 model of that name keeps its
  meaning). The D2 names are refused in v1 with W0202 "reserved built-in `<name>`
  is not yet implemented in this version" (`CheckerErrorKind::NotYetImplemented`,
  S0 E7). The section 2.4 typing of the seven forms, with the expected type
  threaded into `typecheck_v1_builtin`, so `to_bytes([])` and `replicate([], n)`
  type-check; counts are `Int` (`Nat` widens). A v1 list literal of more than
  4,096 elements is W0602 at the literal's span, before its elements are typed
  (`MAX_LIST_LITERAL`, `CheckerErrorKind::ListLiteralTooLong`, S0 E8).
- **Ground evaluator.** The seven forms, with negative counts clamped to zero on
  the exact `i128` count and the check-before-allocate order of section 2.7:
  element cap (`MAX_SEQ_ELEMENTS` = 65,536), then data reservation, then work,
  then allocation. A cumulative per-instance data counter (`DATA_BUDGET_BYTES` =
  64 MiB, reset by `reset_work`) charges, in every v1 evaluation, the `dsize` of
  each node's value except the pass-through nodes (S0 E4), with deep-copy charges
  for `take`/`drop`/`slice`/`replicate` (S0 E5). New `CodegenErrorKind::
  {SequenceTooLong, DataBudgetExceeded}` (both W0703) and the accessors
  `Evaluator::{work_used, data_used}`. `and`, `or` and `=>` short-circuit in v1;
  v0 stays eager (S0 E2).
- **Operation discovery.** New `codegen::find_operation_for_version`: in a v1
  document the argument search also enters list-literal elements (S0 E13); claim
  extraction and the table vacuity check use it. `find_operation_in_expr` is
  unchanged (the evidence generator keeps it until S2a).
- **Tests.** `wxd_syntax` (no grammar change: forms parse as calls; token and
  grammar files pinned by SHA-256), `wxd_checker` (typing, W0301/W0201/W0202/
  W0602 negatives, discovery, generation still W0701, a mechanical scan of the
  in-repository v1 documents), `wxd_semantics` (every form over every list of
  length 0..=4 on a 3-symbol alphabet and as `Bytes` against native iterators;
  counts -2..2, len +/- 1 and 2^70; 1,000/65,536/65,537 elements; pinned work and
  data; both caps; depth 64/65; short-circuit; two mutant oracles killed) and
  `wxd_alloc_probe` (a counting global allocator: refused requests allocate
  nothing before W0703). `wxh_front` now expects 45 v1 functions.
- **Security review fixes** (VER04k section 9.1, E20 and E21).
  - `dsize` counts the bytes of the names a value stores: `len(T) + len(C)` for
    a constructor (nullary ones included) and `len(E) + len(CASE)` for an error
    value, which also charges its names when it is built. Before,
    `replicate(<10,000-byte constructor>, 65536)` was admitted with 2 MiB
    charged and requested 661,727,185 bytes; it is now refused with W0703
    after requesting 10,193. `wxd_alloc_probe` holds real allocation within 4x
    the charge for replicate, list literals, `concat`, `Err(E.C)` and long type
    names (it was 210x to 316x), and pins the budget boundary.
  - A v1 identifier longer than 128 bytes (`MAX_IDENTIFIER_BYTES`) is W0602,
    `CheckerErrorKind::IdentifierTooLong { len, limit }`, checked in a pass
    over every identifier before any other check. v0 is unchanged.
  - A v1 parameter or quantifier binder named like any reserved built-in is
    W0203 (`ShadowedBinding`), as pattern binders already were; before,
    `model exp(take: List<U8>) -> List<U8> = take(take, 0)` type-checked as
    the built-in and evaluated to the parameter. The evaluator now reads a
    name from the environment only when it is not applied, as the checker
    does, in v0 too (a v0 `len(len)` with a `len: Bytes` parameter is now the
    `Nat` the checker typed instead of the parameter).
  - A v0 evaluation no longer computes data charges it discards.
  - A dotted name is an error case or a projection, nothing else (VER04k
    section 9.1, E21). A reference of more than one segment that is neither
    `E.CASE` (a declared error and case) nor a v1 projection from a binder is
    W0202 `UnboundName` of the whole dotted name, in v0 and v1. Before, the
    checker typed it by its first segment (v1 `g.x(a)`, `g` a model, as `U8`;
    `Some.x(a)`, `None.x`, `len.x(a)`; v0 `a.foo` as the binder `a`) while the
    evaluator made it the error value `Err(("g", "x"))`, so an example
    `select_bytes(..) == m.FORGED(..)` checked and `compile_warrant_to_tfv`
    emitted an error id `FORGED` no declaration has. This is a v0 deviation,
    but only for documents that never meant what they checked as: no
    in-repository document uses such a name, and `v0_compat` and the four
    regeneration tests are byte-identical. The evidence generator already
    refused these names; the lanes now agree.
  - Known open hazard: reads deep-copy and charge no data, so the 64 MiB
    counter bounds each value, not an evaluation's memory (VER04k note on E4,
    backlog WX-RD1).

## 0.4.0 - Warrant v1: WX-H (output bounds) and WX-E (pinned external tables)

Implements VER04j sections 5 and 6 (sections 14.2 and 14.3 list every deviation).
All of it is under the `v1` header; a v0 document means exactly what it did
(`output` in a v0 `bound` and `vectors` in a v0 document are `W0101`), and every
generated harness of `ct-compare`, `gf256` and the earlier v1 fixtures is
byte-identical (the new prelude parts are gated by features a document must use).

- **Parser.** `output` as a bound metric; `vectors NAME:` with the contextual row
  words `source`, `sha256`, `rows`, `column`, `holds` (path alphabet and shape,
  64-hex pin, plain decimal row count, at most 32 unique columns); new
  `Declaration::Vectors`, `VectorsDecl`, `ColumnDecl`, `valid_table_path`,
  `Expression::mentioned_names`, `ParserErrorKind::TableIntegrity` (W0708).
- **Checker.** `output` needs a `Bytes`/`Bytes<N>`/`List<T>` result or a `Result`
  of one (W0301); `imin`/`imax` built-ins (`V1_FUNCTIONS` is now 33 names); tables
  share the test-name namespace, are capped at 100,000 rows each and 250,000 per
  document with saturating arithmetic (W0602), empty or vacuous tables are W0601,
  unused or undecodable columns W0710, shadowing W0203. New `DiagnosticCode::{W0601,
  W0710}`, `CheckerErrorKind::{TableRows, VacuousTable, UnusedColumn,
  UndecodableColumn}`, `MAX_TABLE_ROWS`, `MAX_DOCUMENT_ROWS`.
- **Evaluator / model code.** `imin`, `imax`, and byte-string `concat` in the
  generated model translation.
- **Evidence generator (WX-H).** The allocation observer (counting global
  allocator, per-call window, containment by a parked worker, per-test latch, 64
  workers / 1 GiB process cap), bounds as functions of the call arguments (a
  negative or unevaluable bound fails the instance), the synthesized
  `<op>/output-bound` suites, manifest `instances_observed` / `max_observed_heap` /
  `bound_at_max`, relation bias for a limit parameter, the faults `ignore-limit`,
  `off-by-one-limit`, `limit-after-alloc`, `truncate-silently`, and the composite
  adapter as the fallback shape for other v1 signatures (`Bytes` results,
  `Bytes<N>` results, `fresh`).
- **Evidence generator (WX-E).** `vectors::{load_table, read_fixture, Shape,
  check_value, expected_column, is_constant_output}`, `evidence::{TableSet,
  load_document_tables, compile_rust_evidence_harness_with_tables,
  generate_evidence_harness_with_tables}`, the generated table runner and manifest
  entry, the `table` negative-control kind, the discrimination check, the faults
  `flip-result-bit` and `drop-last-result-byte`, and `gen_evidence` materialising
  and drift-checking `tests/generated/tables/<name>.tfv`.
- **Dependencies.** `coding_adventures_test_bootstrap` and `coding_adventures_sha256`
  are now regular dependencies (the loader hashes and parses fixtures).
  `required_capabilities.json` now declares `fs:read` (it was already read by
  `codegen::verify_regeneration`).
- **Fixtures.** `code/specs/fixtures/warrant-vectors-synthetic-v1/` (synthetic
  messages, hashlib digests, stdlib-only generator with a drift check). The build
  tool maps that directory to this package.
- **Tests.** `wxh_front`, `wxh_generator`, `wxh_evidence` (generated pilot harness,
  observer, every fault, ground-evaluator versus generated model differential),
  `wxh_vulnerable`, `wxh_wasteful`, `wxh_caps`, `wxh_caps_count`, `wxh_regen`,
  `wxe_front`, `wxe_loader`, `wxe_evidence` (including a differential run against
  Python `hashlib`), `wxe_edges`, `wxv_regen`.

## 0.3.0 - Warrant v1: WX-A (tuples, records and non-recursive sum types)

Implements VER04j section 4 (its section 14.1 lists every deviation). All of it
is under the `v1` header; a v0 document means exactly what it did (WX-A syntax in
a v0 document is `W0101`), the checker output and evidence of `ct-compare` and
`gf256` are byte-identical, and a v1 document that uses no composite generates
the WX-B/WX-C harness byte for byte.

- **Parser.** Tuple types and literals (arity 2 to 8), `record` and `variant`
  declarations (32 fields, 256 constructors, duplicates are `W0201`), postfix
  projection `t.0` / `f(x).field` (a `.` with no whitespace around it), recursive
  patterns (`_`, `true`/`false`, tuples, constructors, `E.CASE`, `Some(p)`,
  `Ok(p)`, `Err(p)`, depth 8). `Pattern` is now recursive; new AST nodes
  `Declaration::{Record, Variant}`, `TypeExpr::Tuple`,
  `Expression::{Tuple, Project}`, `ProjKey`.
- **Checker.** `Type::{Tuple, Adt}`, a constructor namespace shared with every
  other module symbol (`W0201`), an acyclic type graph through tuples, records,
  variants, `List`, `Option`, `Result` and aliases (`W0203`), projection (including
  `x.f` with a binder head), constructor typing with the v0 per-argument
  subtyping, invariant composites, pattern typing, and a pattern-matrix
  exhaustiveness and redundancy check (`patterns.rs`, `W0301`, `W0602` on 256
  arms, depth or work exhaustion). `labels op: r.field public` accepts a record
  field path. The v0 `match` check is kept for v0 documents.
- **Evaluator.** `EvalValue::{Tuple, Con}`, construction, projection, general
  matching, structural equality, a composite size bound (`W0703`),
  `encode_portable` (VER04j 4.9, `Result` in the VER04i `M1` form), `encode_checked`
  and `W0709` past 64 containers.
- **Evidence generator.** Composite domains (product, constructors in order,
  `Option`, `Result`, `List`), the byte/case budgets and leftmost-first demotion
  inside composites with component paths, `match`/tuple/constructor/projection
  translation, composite-aware generators, relation bias and shrinkers, composite
  adapters bound through `struct`/`enum` lines and `ret=`/`errors=`, the six
  faults, the unreachable-by-type count, manifest `domain` text. New
  `binding::{HostTy, HostSpec, ...}`, `evidence::EvidenceOptions::hosts`.
- **Tests.** `wxa_syntax`, `wxa_checker`, `wxa_patterns` (matrix versus brute
  force), `wxa_semantics`, `wxa_generator`, `wxa_regen` and `wxa_evidence`
  (generated pilot harness, domains, generators, shrinkers, every fault, and the
  ground-evaluator versus generated-model differential on every composite domain
  within budget). `wx_generator` no longer expects a `match` body to be refused.
- Version 0.3.0.

## 0.2.0 — Warrant v1: WX-B (fixed-width integers) and WX-C (bitwise operations)

Implements VER04j sections 1, 2 and 3 (see its section 14 for every deviation).
A document whose header is `warrant name v1` enables the extensions; a v0
document means exactly what it did before. The layout tokens, parsed
declarations and checker output of `ct-compare.warrant` and `gf256.warrant`
hash to the values the previous version produced (pinned in `tests/v0_compat.rs`),
and `ct-compare`'s and `gf256`'s generated evidence regenerates byte-identically.

- **Grammar, lexer, parser.** `warrant.tokens` gains `U16_LITERAL U32_LITERAL
  I8_LITERAL I16_LITERAL I32_LITERAL I64_LITERAL HEX_LITERAL` and
  `warrant.grammar` the optional second header word and the new literal
  alternatives (additive; `warrant-syntax-v1` untouched and passing). The layout
  lexer reads every suffix and `0x...` as one `Number` token; the parser gates
  them on the header: `W0101` for a v1 construct in a v0 document
  (`ParserErrorKind::V1ConstructInV0`), for a header word other than `v1`
  (`InvalidVersionHeader`) and for an out-of-range suffixed literal
  (`LiteralOutOfRange`). `-128i8` is folded into one literal. New AST pieces:
  `Version`, `WarrantFile::version`, `Literal::Fixed`.
- **`fixed` module (new).** `FixedTy` and the single host-independent definition
  of wrapping arithmetic, checked arithmetic, truncating and floor division,
  `wrap`/`try`/`widen` ranges, every bitwise function, endian conversion and the
  evidence edge domains. Tested against native Rust on every pair of `U8`/`I8`
  and large seeded sweeps of the other six types.
- **Checker.** `Type::{U16,U32,I8,I16,I32,I64}`, version-scoped type and
  built-in names (`widen wrap try to_int to_nat max_value min_value checked_*
  rem div_trunc rem_trunc unwrap_or bit_* shl shr sar rotl rotr popcount clz ctz
  bit be_bytes le_bytes from_be from_le`), no implicit conversion in either
  direction, signed-only unary minus, `div`/`rem` unsigned-or-Int only.
- **Evaluator (`codegen::Evaluator`).** `EvalValue::Fixed`, wrapping `+ - *`,
  every built-in with the specification's total semantics, numeric equality by
  value, bare `None`, `W0703` on `Int` overflow, shifts/popcount charge
  `ceil(w/8)` work. `find_operation_in_expr` now also searches call arguments when
  the shallow search finds nothing.
- **Evidence generator.** v1 documents get a typed `V::Fixed` model in which `U8`
  and `U64` wrap (WD-013 for v1; the v0 harness is unchanged), the domain
  descriptors of VER04j 2.6 (`full` for `U8 I8 U16 I16`, `edge` sets of 281, 313,
  377, 527, 559 and 623 values), the leftmost-first `U16`/`I16` demotion rule with
  `W0706`, the count-aware `Nat` domain, a v1 scalar adapter shape (fixed-width
  parameters; fixed-width, `Bool`, `Nat`, `Option` and `Result` results), fifteen
  new faults (`saturate-*`, `trap-as-zero-*`, `narrow-width`, `sign-confuse`,
  `off-by-one-range`, `shift-mask`, `shift-ub`, `rotate-as-shift-*`, `sar-as-shr`,
  `popcount-off`) and manifest fields `warrant_version` and `demoted`. The
  prelude's `//@if` regions now nest and select `v0` or `v1`.
- **Tests.** `v0_compat`, `wx_syntax`, `wx_checker`, `wx_semantics`, `wx_generator`,
  `wx_evidence` (compiles and runs a checked-in generated v1 harness, with every
  negative control) and `wx_regen` (drift check for it).
- Not delivered: the `endian-swap` fault and `Bytes` adapters, a
  `warrant-syntax-v2` fixture, the independent Python reference evaluator.

## Unreleased

- Security fixes (review of the gf256 / package-agnostic branch):
  - L3: `exhaustive_budget` is capped at 2^26 (`MAX_EXHAUSTIVE_BUDGET`) by the `evidence.bindings` parser, with a clear error above it; gf256's 2^24 is unchanged.
  - L4: `package` and every `export` must be Rust identifiers (also re-checked by the generator, together with error cases and the embedded spec path); `spec` / `output` must be plain relative paths (no absolute path, `..`, backslash, drive letter) and `EvidenceBindingFile::validate_paths` pins them to `code/specs/<pkg>.warrant` and a `.rs` file directly under `code/packages/rust/<pkg>/tests/generated/`; `gen_evidence` validates its `<package>` argument (`[A-Za-z0-9][A-Za-z0-9_-]*`) and calls `validate_paths`.
  - L1 (support): a new per-op option `proofs_not_live=<reason>` (`EvidenceBindingFile::proofs_not_live`) declares that an operation's proof obligations cannot run; `ca-assurance-report` accepts a `not-live` proof entry only for such an operation.
  - Generated harnesses (ct-compare, gf256) are byte-identical.
- Round-2: `resolve_sabotage` rejects a filter unless the fault is a real (non-`none`) one; `run_main` takes the crate's `src/` directory and records the framed `src/**/*.rs` digest as `source_sha256`. Generated harness regenerated.
- Security review fixes (VER04h section 3 and 6):
  - A1: exhaustive-domain size uses checked arithmetic in the generator and in
    the generated prelude; an overflowing product (eight `U8` binders is
    `2^64`, which wrapped to `0`) is over budget.
  - A2: the budget also bounds bytes (`DOMAIN_BYTES_BUDGET`), and fixed widths
    above `MAX_FIXED_WIDTH` (4096), as binders or call widths, fail with the new
    diagnostic `W0707`. The prelude checks the budget before allocating domains.
  - A3: the generated target exits `2` when `WARRANT_EVIDENCE_FILTER` is set
    without `WARRANT_EVIDENCE_FAULT`, or when the filter matches no test
    (`resolve_sabotage`, `require_tests`).

## 0.1.0 — Unreleased

- Evidence generator, second package (gf256, VER04h): the per-package binding
  table is now data. `examples/gen_evidence [--check] [<package> ...]` reads
  `code/packages/rust/<package>/evidence.bindings` (parsed by the new
  `binding::parse_evidence_bindings`, which rejects unknown keys and options so a
  typo cannot weaken evidence) instead of a hardcoded op-to-export table.
  ct-compare's generated harness is byte-identical through its new bindings file.
  - `EvidenceOptions` gains `exhaustive_budget` (default 400,000; gf256 uses 2^24
    for the `U8^3` field-law claims, measured at about 9 s per claim in a debug
    build) and `arg_types` (Rust parameter types of an export, e.g. `power`'s
    `u32` exponent against the Warrant `U64`).
  - New scalar adapter shape for operations over `U8`/`U64`/`Nat` returning `U8` or
    `Result<U8, E>` (a panic is normalised to `E` by the binding, as for
    `ct_select_bytes`), with a closed registry of field-arithmetic faults
    (`or-for-xor`, `wrapping-sub`, `mul-as-add`, `alt-poly`, `generator-shift`,
    `exp-mod-256`, `missing-error`). The generated `Fault` enum lists only the
    faults a package uses.
  - A negative control on a scalar operation attacks that operation alone
    (`attacking`/`fault_for`) and stops at the first claim that catches the fault;
    a fault applied to every operation of a field was invisible to every
    relational claim (lesson recorded).
  - Binder refinements may use models only; an operation inside one is refused
    (W0701). `U8` binders stay inside 0..=255 under relation-biased generation,
    `U64` binders draw small values often, and the fuzz coverage map sees `U8`
    values; these prelude regions are feature-gated so other packages' output
    does not change.
  - Tests: `tests/evidence_bindings.rs` (bindings reader, budget, fault and
    refinement rules, byte-for-byte regeneration of both checked-in harnesses).

- Add `evidence` (VER04h / VER-P030): compile a checked Warrant document into a
  Rust evidence harness (examples, bounded-exhaustive, property, fuzz and
  sabotage suites) whose oracle is the *translated claim body*, not a hand-copied
  `a == b`; supports any number of binders and `Bytes<N>` widths. New diagnostic
  `W0706` (exhaustive domain over budget). `examples/gen_evidence` regenerates or
  `--check`s `ct-compare/tests/generated/warrant_evidence.rs`.
- Stop emitting claim helpers from `compile_rust_l0_harness`: the old helper
  hard-coded `a == b`, accepted at most two binders and was never compiled.
  Removed the tests that string-matched it and the `a == b` vs `a == b`
  "sabotage" emulation; `tests/evidence.rs` compiles and runs the generated
  harness against the real `ct-compare` crate instead.
- Add `ct-compare/warrant` claims for `eq_fixed` (widths 0/1/2/4/16),
  `select_bytes` (pick + length mismatch) and `eq_u64`.

- Add VER04f/P011 Warrant Universal Claims to Property Tests Compiler:
  - Extract and validate universally quantified `claim` declarations (`ClaimCase`) calling bound host operations.
  - Compile claims into Level 2 property-based tests executed on `coding_adventures_test_bootstrap::pbt`.
  - Type-to-generator mappings for `Bytes`, `U64`, `U8`, `Bool` with deterministic PRNG replay and isolated sub-seeds.
  - Integrated shrinking minimizing counterexamples across component values.
  - Public host API call binding dispatch with panic isolation and fault injection.
  - Harness emission of claim sabotage negative controls (`claim_sabotage/{id}_broken_impl`) to detect broken implementations.
  - Privacy-preserving failure output in CI mode redacting raw inputs with FNV-1a reproducer digest.
  - Exceed strict ≥90% line and branch/region test coverage (93.19% lines, 94.51% regions) with zero third-party dependencies.
- Add VER04e/P008 Warrant code generation and L0 test vector compiler:
  - Bounded pure compile-time ground model evaluator with strict recursion depth (<= 64)
    and work quotas (<= 1,000,000 steps).
  - Canonical Portable Test Vector (TFV) compiler transforming Warrant `example` and `claim`
    declarations into structured portable records with logical work accounting.
  - Standalone, deterministic Rust L0 test harness generator executing on `test-bootstrap`,
    calling real public exports, verifying fresh-output non-aliasing, containing and
    normalizing declared panics via `catch_unwind`, and incorporating adapter negative controls.
  - Closed diagnostic error codes (`W0701`..`W0705`) and checked regeneration drift detection.
  - Pilot `code/specs/ct-compare.warrant` canonical specification.
  - Exceed strict ≥90% line and branch/region test coverage with zero third-party dependencies.
- Add VER04d/P007 Warrant type checker and Rust binding schema: static type checking,
  scope/symbol resolution, cycle detection on type aliases and pure model functions,
  minimal subtyping (`Nat <: Int`, `Bytes<N> <: Bytes`), exhaustive match verification,
  and contract validation (`labels`, `bound`, `input`, `assume`).
- Enforce closed diagnostic codes (`W0201`, `W0202`, `W0203`, `W0301`, `W0401`, `W0501`, `W0502`, `W0602`)
  and opaque error messages with zero input secret leakage.
- Add closed host binding schema (`BindingManifest`, `OpBinding`) validating complete operation
  coverage, fresh requirements, negative controls, and error normalizations.
- Achieve ≥90% line and branch/region test coverage with zero third-party dependencies.
- Add VER04c/P006b3 guarded Warrant source parser: typed AST construction conforming
  to `warrant.grammar`, strict recursion depth guard (`max_depth: 64`) preventing
  call-stack exhaustion (CWE-674), AST node quota (`max_nodes: 65,536`), and work limits.
- Enforce closed `W0001` diagnostics and verify zero source leakage in error Display/Debug.
- Validate all 57 canonical syntax cases from `cases.tfv` through admitted document,
  layout capture, and guarded AST parser.
- Add VER04b/P006b2 bounded significant-layout capture: space-only indentation,
  indentation stack tracking with configurable limits, tabs-in-indentation rejection,
  and exact zero-width synthetic token origins (INDENT/DEDENT/NEWLINE) without
  fictitious positive-width byte spans.
- Add VER04a/P006b1 source admission: bounded UTF-8/BOM/NUL/CR validation,
  borrowed LF views, fallible CRLF normalization and original-byte/one-based
  line/byte-column mapping through a source-bound line-start index.
- Enforce lowerable byte/capacity/work caps before scans/reservations; use
  closed W0001 errors and suppress source in error/document Debug output.
- Test authored Unicode/CRLF ranges, exhaustive small scalar ranges, malformed
  encodings, source/buffer/work boundaries and opaque diagnostics. Route the
  source contract to the Warrant consumer in the build tool.
- Add canonical significant-indentation Warrant token/EBNF grammars and
  deterministically generated Rust TokenGrammar/ParserGrammar constructors.
- Exercise 57 authored syntax cases through TFV/L0, exact subtree witnesses and
  dynamic/embedded grammar agreement. Add three adapter controls and a
  separately falsified precedence witness, source-shape guards and opaque errors.
- Check exact generated bytes and wire grammar/corpus/reference changes into
  repository input selection plus platform BUILD commands.
- Keep significant-layout origin capture, the guarded parser and its remaining
  quotas, typing/binding validation, proof and publishing work deferred.
