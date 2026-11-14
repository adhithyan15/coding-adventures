# warrant-grammar

Canonical Warrant v0/v1 grammar data for [VER04 P006a](../../../specs/VER04-warrant-language.md#11-grammar-artifacts-and-syntax-corpus-staging-p006).
The existing grammar-tools compiler turns the repository's `.tokens` and
`.grammar` files into native Rust structures committed in `src/`.

```rust
use coding_adventures_warrant_grammar::{parser_grammar, token_grammar};
let tokens = token_grammar();
let rules = parser_grammar();
assert!(!tokens.definitions.is_empty());
assert!(!rules.rules.is_empty());
```

The additive [VER04a source stage](../../../specs/VER04a-warrant-source-admission.md)
also admits bytes, normalizes CRLF and maps normalized ranges to original UTF-8
bytes. LF input borrows its original buffer. Other input must fit the 1-MiB
source, 64-MiB owned-buffer and 3-MiB scan-work ceilings; callers may lower caps.
Malformed UTF-8, leading BOM, NUL and bare CR reject with fixed `W0001` errors.
Document Debug omits source. Range coordinates are one-based line/byte-column,
with zero-based raw byte offsets. Paths/revision binding remain a later layer.

```rust
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
let source = SourceDocument::admit(b"warrant x\r\n", SourceLimits::default()).unwrap();
assert_eq!(source.normalized(), "warrant x\n");
let newline = source.original_span(9, 10).unwrap();
assert_eq!(&source.original()[newline.start.byte_offset..newline.end.byte_offset], "\r\n");
```

The additive [VER04b layout stage](../../../specs/VER04b-warrant-layout-capture.md)
captures significant-layout tokens (`INDENT`, `DEDENT`, `NEWLINE`) over admitted
source documents. Synthetic layout tokens possess exact zero-width origins, mapping
via `doc.original_span(at, at)` to zero-width original source intervals without
fictitious positive-width whitespace spans:

```rust
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
let tokens = capture_layout(&source, LayoutLimits::default()).unwrap();
```

The additive [VER04c parser stage](../../../specs/VER04c-guarded-warrant-parser.md)
constructs a typed, bounded Abstract Syntax Tree (AST) conforming to `warrant.grammar`.
It operates directly on the `&[LayoutToken]` stream and enforces strict quotas:
maximum call-stack recursion depth (`max_depth: 64`, ceiling `128`) preventing CWE-674,
maximum AST node budget (`max_nodes: 65,536`, ceiling `1,048,576`), and work limits.
All diagnostics return closed `W0001` codes and never leak raw token text.

```rust
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits};
let ast = parse_document(&tokens, &source, ParserLimits::default()).unwrap();
```

The additive [VER04d type-checking and binding stage](../../../specs/VER04d-warrant-type-system.md)
performs semantic analysis, scope resolution, cycle detection, static type checking,
and host-language binding schema verification. It enforces strict subtyping (`Nat <: Int`,
`Bytes<N> <: Bytes`), exhaustive pattern matching, operation contracts (`labels`, `bound`,
`input`, `assume`), and stable diagnostics (`W0201`..`W0602`):

```rust
use coding_adventures_warrant_grammar::checker::check_warrant;
use coding_adventures_warrant_grammar::binding::{validate_bindings, BindingManifest};

let checked = check_warrant(&ast).unwrap();
validate_bindings(&checked, &manifest).unwrap();
```

The additive [VER04e code generation stage](../../../specs/VER04e-warrant-codegen.md)
compiles checked Warrant modules into canonical portable test vectors (TFV) and
standalone Rust L0 test harnesses running on `test-bootstrap`. It features a bounded
ground model evaluator (depth <= 64, work <= 1,000,000), real export invocations,
fresh-output non-aliasing validation, panic containment/normalization via `catch_unwind`,
and adapter negative controls:

```rust
use coding_adventures_warrant_grammar::codegen::{compile_warrant_to_tfv, compile_rust_l0_harness, CodegenLimits, RustL0Options};

let tfv = compile_warrant_to_tfv(&ast, CodegenLimits::default()).unwrap();
let harness = compile_rust_l0_harness(&ast, &checked, &manifest, &options).unwrap();
```

Source admission, layout capture, grammar data, parser, type checking and codegen do not
establish solvers, property engine, proof or release authorization (VER-P009+).

## Warrant v1: fixed-width integers and bitwise operations (VER04j WX-B, WX-C)

A document headed `warrant name v1` enables eight fixed-width types (`U8 U16 U32
U64 I8 I16 I32 I64`), suffixed and hex literals (`5u16`, `-128i8`, `0x1du8`) and
a family of total built-ins. Arithmetic wraps; checking is explicit.

```text
warrant demo v1
  model xtime(x: U8) -> U8 = if x >= 128u8 then bit_xor(shl(x, 1), 0x1du8) else shl(x, 1)
  example wraps:
    max_value<U32>() + 1u32 == 0u32
  example clamp_is_explicit:
    unwrap_or(try<U8>(300), 255u8) == 255u8
```

| Group | Built-ins |
|---|---|
| conversion | `widen<T>` (range contained), `wrap<T>` (mod 2^w), `try<T>` (`Option`), `to_int`, `to_nat`, `max_value<T>()`, `min_value<T>()` |
| checked | `checked_add checked_sub checked_mul`, `div rem` (unsigned or `Int`), `div_trunc rem_trunc` (signed), `unwrap_or` |
| bitwise | `bit_and bit_or bit_xor bit_not`, `shl shr sar rotl rotr` (the count is a `Nat`; `n >= w` gives `0`/`-1`, never the host's masked shift), `popcount clz ctz bit` |
| endian | `be_bytes le_bytes` (to `Bytes<w/8>`), `from_be<T> from_le<T>` |

`fixed::FixedTy` and the functions in [`src/fixed.rs`](src/fixed.rs) are the one
definition of what these mean; the checker, the ground evaluator and the tests
share them, and the generated harness carries a separate copy that
`tests/wx_evidence.rs` holds to it. A v0 document is unaffected: the new names
exist only under the `v1` header, and `ct-compare`'s and `gf256`'s generated
evidence is byte-identical (`tests/v0_compat.rs`).

For evidence, a v1 document's binders get the domains of VER04j 2.6 (every value
of `U8 I8 U16 I16`, a biased edge set of 281-623 values for wider types, demoted
leftmost-first under the exhaustive budget) and a shift-count `Nat` gets the
count-aware domain; the scalar adapter shape accepts fixed-width parameters and
the faults `saturate-add`, `trap-as-zero-mul`, `narrow-width`, `sign-confuse`,
`off-by-one-range`, `shift-mask`, `shift-ub`, `rotate-as-shift-left`,
`sar-as-shr`, `popcount-off` and friends in an `evidence.bindings` `faults=` list.
`tests/fixtures/wx_fixed.warrant` is the test fixture; its generated harness is
checked in at `tests/generated/wx_evidence.rs` (`WARRANT_REGEN=1 cargo test -p
coding_adventures_warrant_grammar --test wx_regen` rewrites it).

## Warrant v1: tuples, records and sum types (VER04j WX-A)

The same `v1` header also enables structured values: tuples, records and
non-recursive sum types, with nested patterns and an exhaustiveness check.

```text
warrant leb v1
  error LebError = TRUNCATED | OVERLONG
  record Span(start: U8, len: U8)
  variant Shape = Circle(r: U8) | Rect(w: U8, h: U8) | Empty
  model Pair = (U8, U8)

  model dec2(b0: U8, b1: U8) -> Result<(U64, Nat), LebError> =
    if b0 < 128u8 then Ok((widen<U64>(b0), 1)) else Err(LebError.OVERLONG)
  model area(s: Shape) -> U64 =
    match s with Circle(r) -> widen<U64>(r) * widen<U64>(r)
               | Rect(w, h) -> widen<U64>(w) * widen<U64>(h)
               | Empty -> 0u64
  example gap:
    normalize(9u8, 4u8).len == 5u8
```

| Construct | Meaning |
|---|---|
| `(T1, T2, ...)`, `(e1, e2, ...)` | structural tuple, arity 2 to 8; `t.0`, `f(x).1` project |
| `record R(a: T, ...)` | nominal, one constructor named `R`, 1 to 32 fields; `r.a` projects |
| `variant V = A(x: T) \| B \| ...` | nominal sum, 2 to 256 constructors, positional `A(e)`, bare `B`; no recursion (W0203) |
| `match e with p -> e \| ...` | patterns `_`, binders, `true`/`false`, `(p, q)`, `C(p, ...)`, `Some(p)`, `Ok(p)`, `Err(E.CASE)`; non-exhaustive or redundant is W0301 |

Everything is v1-only: in a v0 document each of these is `W0101`, and a v0
document means exactly what it did (`tests/v0_compat.rs`). Composites are
invariant (`(Nat, Nat)` is not an `(Int, Int)`), unordered (`<` is W0301) and
compared structurally.

- [`src/patterns.rs`](src/patterns.rs) is the pattern-matrix usefulness algorithm
  behind exhaustiveness and redundancy; `tests/wxa_patterns.rs` holds it to
  brute-force enumeration over small types.
- The ground evaluator builds `EvalValue::Tuple` and `EvalValue::Con`, and
  `EvalValue::encode_portable` writes the cross-language corpus form (tuple and
  record `L<n> ...`, variant `L2 S<ctor name> L<k> ...`, `Option` `L0`/`L1 x`,
  `Result` as the VER04i `M1` form); nesting past TFV's 64 containers is `W0709`.
- Evidence: composite binders get the domains of VER04j 4.7 (a product,
  lexicographic; a variant's constructors in order; `Option`, `Result`, `List`),
  the leftmost-first `U16`/`I16` demotion reaches into them, generators draw
  constructors uniformly and relation-bias one leaf, and shrinkers work
  component-wise. Operations whose signature mentions a composite are bound
  with `struct`/`enum` lines and `ret=`/`errors=` in `evidence.bindings`:

```text
struct Span path=subject::Span fields=start:u8,len:u8
enum Shape path=subject::Shape ctors=Circle(r:u8),Rect(w:u8,h:u8),Empty
op decode2 export=decode2 args=u8,u8 ret=(u64,usize) errors=subject::LebErr/Overlong=OVERLONG faults=off-by-one-consumed,drop-consumed
op classify export=classify ret=Shape faults=wrong-ctor
```

  The faults are `drop-consumed`, `swap-components`, `field-zero`, `wrong-ctor`,
  `ok-as-err` and `off-by-one-consumed` (the LEB128 bug class: right value, wrong
  bytes consumed). A value a host integer cannot hold is counted as
  `unreachable_by_type` in the manifest, never narrowed. The checked-in pilot is
  `tests/fixtures/wxa.warrant` with its bindings and generated harness
  (`tests/generated/wxa_evidence.rs`, `WARRANT_REGEN=1 cargo test -p
  coding_adventures_warrant_grammar --test wxa_regen` rewrites it);
  `tests/wxa_evidence.rs` also runs the ground evaluator against the generated
  model code on every composite domain within budget.

## Warrant v1: fixed-length byte arrays (VER04j WX-N)

`Bytes<N>` was always in the language; the binding now maps it to the Rust array
`[u8; N]` with the host type `bytes<N>` (a top-level parameter is `&[u8; N]`),
and `err_any=CASE` binds a host `Result<T, E>` whose error type is opaque:

```text
op x25519 export=x25519 args=bytes<32>,bytes<32> ret=bytes<32> err_any=LOW_ORDER faults=flip-result-bit,missing-error
```

A model value of the wrong length is unreachable by type (never padded or
truncated); an export that returns the wrong length fails the harness. A
`Bytes<N>` binder with N >= 2 enumerates a *sample* of the `256^N` values and the
manifest says `sample(SIZE)`; only N <= 1 is `full`. Fixed arrays of integers
(`[u32; 8]`) are not supported (backlog WD-015). The pilot is
`code/specs/x25519.warrant`; the compiled fixture is `tests/fixtures/wxn.warrant`.

## Warrant v1: output bounds (VER04j WX-H)

A v1 `bound` block gains a fourth metric, `output`, that bounds the *size of the
successful result* (a `Bytes` length, a list's element count, the `Ok` payload of
a `Result`); a bound expression may use parameters, `len`, `to_int`, `unwrap_or`
and the new total helpers `imin`/`imax`.

```text
op decompress(data: Bytes, max_output: U32) -> Result<Bytes, LzError> fresh
bound decompress:
  output <= to_int(max_output)
  heap   <= 2 * to_int(max_output) + 8 * len(data) + 4096
```

For every operation with an `output` or `heap` row the generator registers
`exhaustive`/`property`/`fuzz` suites named `<op>/output-bound` and runs *every*
instance of the operation under an **allocation observer**: a counting global
allocator and a per-call window. The cap (64 MiB) is enforced by containment, not
by failing the allocation (a `GlobalAlloc` may not unwind, and a null return aborts
the process on stable Rust): the over-cap request is never made, its worker thread
is parked and abandoned, and the instance fails with `bound-heap-exceeded` and the
size it asked for. A test that tripped once fails its later observed instances
without running them, and at most 64 workers holding 1 GiB are ever abandoned.
The faults `ignore-limit`, `off-by-one-limit`, `limit-after-alloc` and
`truncate-silently` attack the limit parameter the `output` row mentions.
`tests/fixtures/wxh.warrant` (a toy run-length decompressor) with a vulnerable, a
wasteful and a capped subject is the pilot (`wxh_evidence`, `wxh_vulnerable`,
`wxh_wasteful`, `wxh_caps*`); no `lz*` API is touched.

## Warrant v1: pinned external tables (VER04j WX-E)

```text
vectors synthetic_digests:
  source "warrant-vectors-synthetic-v1/digests.tfv"
  sha256 b"<64 lowercase hex digits: SHA-256 of the file>"
  rows 23
  column msg: Bytes
  column md: Bytes<32>
  holds digest(msg) == md
```

The data lives in a TF01 TFV fixture under `code/specs/fixtures/`; the document
pins its identity. `evidence::load_document_tables` is the tool half
(`src/vectors.rs`): it checks the path rules, refuses symlinks, caps the file at
16 MiB and the table at 100,000 rows (250,000 per document), compares the SHA-256
**before** parsing, refuses maps with more than one entry, parses with the lane's
own TFV parser and decodes every row against its column types (W0708 / W0709 /
W0710); it never touches the network. The generator copies the verified bytes into
the package (`tests/generated/tables/<name>.tfv`, embedded with `include_bytes!`),
the generated test hashes the copy again (`corpus-malformed` on a mismatch), runs
`holds` per row, and records one manifest entry per table (`kind: "table"`,
`cases`, `rows_pinned`, `table_sha256`, `failed_ids`). `gen_evidence` materialises
and drift-checks the copies. A table whose expected column is constant is W0601.
The shipped fixtures are synthetic (`warrant-vectors-synthetic-v1`, README there
records provenance); adding a NIST/RFC set is an owner decision and the loader is
the hook.

## Warrant v1: sequence built-ins, checker and evaluator (VER04k WX-D1)

```text
model last_two(b: Bytes) -> Bytes = drop(b, len(b) - 2)       -- clamps at 0
model zeros(n: Int) -> Bytes = to_bytes(replicate(0u8, n))
claim prefix: forall b: Bytes, n: Nat. to_bytes(take(to_list(b), n)) == take(b, n)
```

Seven sequence forms are v1 built-ins: `take drop slice replicate range to_list
to_bytes` (VER04k section 2.2). Counts are `Int`; a negative count means zero and
a count past the end means "all of it", so `take(xs, -1) == []` and
`range(-3) == []`. `take`/`drop`/`slice` of any `Bytes` give a dynamic `Bytes`;
an empty list literal gets its element type from its position, so `to_bytes([])`
and `replicate([], n)` type-check. The five higher-order names `fold map count
all any` are reserved too, and refused with W0202 "not yet implemented in this
version" until WX-D2. In v0 all twelve stay ordinary identifiers. A v1 list
literal has at most 4,096 elements (W0602), a v1 identifier at most 128 bytes
(W0602), and a v1 parameter or quantifier binder may not spell any reserved
built-in (W0203).

The ground evaluator meters every v1 evaluation with work, depth, a cumulative
64 MiB data counter (each constructing node pays the `dsize` of its value,
including the type, constructor and error names a value stores) and a
65,536-element cap per sequence. A form checks the cap on the exact count, then
reserves its data, then charges its work, and only then allocates, so
`replicate(0u8, 2^70)` fails with W0703 having allocated nothing
(`tests/wxd_alloc_probe.rs`). `and`, `or` and `=>` short-circuit in v1. Generated
evidence still refuses the forms (W0701) until slice S2a.

The data counter bounds each value, not the memory of a whole evaluation: a read
of a binder deep-copies its value and charges nothing, so many reads of one
large value can take far more memory than its charge. VER04k section 9.1 (note
on E4) records this as an open hazard, tracked as backlog WX-RD1.

## Portable corpus (VER04i WD-003)

`examples/gen_corpus.rs` compiles the checked `ct-compare` Warrant v0 source into
`code/specs/fixtures/ct-compare-warrant-v1/`. The generated corpus separates
`subject_op` calls from pure `model_call` reference expressions, enumerates the
closed supported domains in canonical order, and pins independent input and
expected-result digest streams. The first profile includes examples, exhaustive
suites and their required negative-control descriptors; it rejects sharding and
Warrant constructs outside the explicitly supported profile.

The independent stdlib-only validator is `code/scripts/warrant_corpus_oracle.py`.
It derives the accepted declarations and suite shape from the Warrant source,
then independently recomputes domains, refinements, example records and digest
streams. The Python BUILD tests also mutate the artifact to ensure missing or
misbound evidence is rejected. This validates corpus generation; it does not
run a subject lane or establish a proof.

```sh
cargo run -p coding_adventures_warrant_grammar --example gen_corpus -- --check
python3 ../../../scripts/warrant_corpus_oracle.py
python3 -m unittest discover -s ../../../scripts/tests -p test_warrant_corpus_oracle.py
```
## Tests and regeneration

From the Rust workspace:

```sh
cargo test -p coding_adventures_warrant_grammar
cargo clippy -p coding_adventures_warrant_grammar --all-targets -- -D warnings
cargo run -p coding_adventures_warrant_grammar --example regenerate -- --check
```

Omit `-- --check` to regenerate the two embedded modules after editing the
canonical files in `code/grammars/warrant`. Do not format/edit generated files;
their modules are excluded from rustfmt and tests compare exact emitter bytes.

The authored [syntax corpus](../../../specs/fixtures/warrant-syntax-v1/README.md)
is projected from JSON to LF-only TFV by the standard-library Python gate.
Runtime tests embed TFV rather than parse JSON. They compare syntax verdicts,
whole subtree witnesses and runtime-loaded/compiled AST equality on 57 cases.
Three adapter controls reject forced acceptance, forgotten witnesses and a
wrong-precedence grammar. The latter still parses its source but loses the
independently authored `product=2 * 3` subtree. A separate unit test proves this
distinction and a shortened witness must fail exact membership.

```sh
cargo test -p coding_adventures_warrant_grammar --test self_corpus -- --sabotage always-pass
```

This command must exit nonzero with three failing controls. BUILD checks that
from outside the harness. Ordinary runs pass 60 L0 checks plus regeneration and
adapter unit tests. Test adapters suppress raw source/parser exception text;
they use a shared-parser rule cap on bounded embedded fixtures, which is not a
multi-shape stack-safety proof or an implementation of VER04's full budgets.

Eleven source-stage tests separately check authored raw byte/line coordinates,
all scalar-boundary ranges of small mixed sources, invalid UTF-8 and forbidden
bytes, UTF-8 query boundaries, exact lowered limits and a dense 1-MiB newline
input. None exposes an untrusted-source lexer/parser. The shared CV04a range
API still deliberately refuses indentation/layout synthesis.

Some parser positives deliberately have duplicate names, a Bool/Int mismatch
or an unreachable refinement. Those belong to later checkers; their presence
demonstrates why a parsed tree must not be labelled a checked specification.

## Evidence generation (VER04h)

`warrant_grammar::evidence::compile_rust_evidence_harness` turns a checked
document into a Rust test target containing, for every `example` and `claim`,
the L0 case, a bounded-exhaustive suite, a property suite, a fuzz suite and
sabotage controls. The oracle is the *translated claim body*. See
`examples/gen_evidence.rs` and [VER04h](../../../specs/VER04h-warrant-derived-evidence.md).
