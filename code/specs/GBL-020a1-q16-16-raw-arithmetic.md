# GBL-020a1: raw signed Q16.16 arithmetic

**Status:** selected specification; implementation has not started. This is the first
slice of GBL-020a, not completion of that parent.

## Purpose and boundary

Define a reusable deterministic fixed-point arithmetic package for simulation code.
The initial public profile is signed Q16.16 backed by the complete `i32` raw domain.
It is suitable for callers that already own a simulation step and state. It does not
provide a game loop, clock, physics model, floating-point compatibility layer, or
language-port parity claim. The existing Lunar Lander and audio Q16.16 helpers remain
domain-specific until separately migrated against this contract.

Raw integer `r` denotes the exact rational value `r / 65536`. The representable raw
range is `i32::MIN..=i32::MAX`, corresponding to `-32768` through
`32767 + 65535/65536`. No operation may use host floating point or depend on machine
overflow behavior.

## Public API and errors

The package identity is path `code/packages/rust/fixed-point`, crate
`coding-adventures-fixed-point`. It exports `Q16 { pub raw: i32 }` (`Copy + Eq`),
`RoundingMode`, `Q16Error`, and
`SaturatingResult { pub value: Q16, pub overflowed: bool }`. `RoundingMode` has
exactly four unit variants: `TowardZero`, `Floor`, `Ceil`, and `NearestEven`.
`Q16Error` has exactly the payload-free variants `Overflow` and `DivisionByZero`.

The `Q16::from_raw(raw: i32) -> Q16` constructor and `Q16::raw(self) -> i32`
accessor are separate convenience APIs and must be tested for losslessness over all
`i32` patterns. The following ten crate-root free functions are the canonical
generator targets:

```text
from_integer(value: i32) -> Result<Q16, Q16Error>
checked_add(a: Q16, b: Q16) -> Result<Q16, Q16Error>
checked_sub(a: Q16, b: Q16) -> Result<Q16, Q16Error>
saturating_add(a: Q16, b: Q16) -> SaturatingResult
saturating_sub(a: Q16, b: Q16) -> SaturatingResult
checked_mul(a: Q16, b: Q16, mode: RoundingMode) -> Result<Q16, Q16Error>
saturating_mul(a: Q16, b: Q16, mode: RoundingMode) -> SaturatingResult
checked_div(a: Q16, b: Q16, mode: RoundingMode) -> Result<Q16, Q16Error>
saturating_div(a: Q16, b: Q16, mode: RoundingMode)
    -> Result<SaturatingResult, Q16Error>
to_integer(value: Q16, mode: RoundingMode) -> i32
```

No implicit numeric casts or arithmetic operator traits are provided. Record
arguments/results must be modeled explicitly in native evidence; this contract makes
no portable record-binding claim. The rounding mode is an explicit argument to
multiplication, division, and `to_integer`; `from_integer` is exact and takes no
mode. It is never global or hidden.

| Operation | Checked result | Saturating result |
|---|---|---|
| `from_integer(i32)` | `Result<Q16, Q16Error>` | not applicable |
| add/sub | `Result<Q16, Q16Error>` | `SaturatingResult` |
| multiply | `Result<Q16, Q16Error>` | `SaturatingResult` |
| divide | `Result<Q16, Q16Error>` | `Result<SaturatingResult, Q16Error>` |
| `to_integer(Q16, RoundingMode)` | `i32` (infallible) | not applicable |

After committing this specification, the first implementation step is a minimal real
package API sufficient for a canonical generator admission probe. The probe must
compile and call these actual exports, preserve record/result/enum types and exact
errors, and demonstrate a successful value plus each typed error. If the generator
cannot represent the actual API without panic normalization, missing error variants,
or a wrapper treated as the subject, stop implementation and record the smallest
required generator prerequisite. Do not claim evidence for a surrogate.

## Arithmetic semantics

Addition and subtraction operate on exact signed raw integers. Their exact result is
checked against the `i32` range. Checked forms return `Overflow`; saturating forms
clamp to `i32::MIN` or `i32::MAX` and set `overflowed=true`. In-range results have
`overflowed=false`.

For multiplication, let `n = a.raw * b.raw` and `d = 65536`. For division, zero
denominator returns `DivisionByZero`; otherwise let `n = a.raw * 65536` and
`d = abs(i64::from(b.raw))`, with the sign of `b.raw` applied to the rational
result. Widen before absolute value so `i32::MIN` is valid; normalize the denominator
to positive before applying signed floor/ceiling rules. Apply the selected rounding mode to the exact rational result first, then test the rounded
integer against the raw `i32` range. Only a post-round out-of-range integer is an
overflow. Checked forms return `Overflow`; saturating forms clamp that rounded
integer and report `overflowed=true`. Saturation must not alter division-by-zero:
it remains `DivisionByZero` for every division variant. Intermediate products must
be exact (the full `i32` product fits in `i64`; division scaling must also be
computed without overflow).

Rounding is defined on signed exact rationals:

* `TowardZero`: discard the fractional magnitude.
* `Floor`: greatest integer less than or equal to the rational.
* `Ceil`: least integer greater than or equal to the rational.
* `NearestEven`: nearest integer; an exact half chooses the even integer, including
  negative ties.

`from_integer(x)` computes the exact raw integer `x * 65536` and returns `Overflow`
when it is outside `i32`; it never saturates. `to_integer` rounds the exact rational
`raw / 65536` using the requested mode. The result is always within `-32768..=32768`,
so this conversion is infallible. The conversions are deterministic and
do not pass through `f32` or `f64`.

## Canonical inputs and independent oracle

Add a versioned shared fixture for Q16.16 containing named vectors for every
operation, mode, checked/saturating result, exact error, and overflow flag. The
canonical binary edge domain is the output of the existing
`code/packages/rust/warrant-grammar/src/fixed.rs::edge_domain(FixedTy::I32)`
algorithm: interpret as signed two's-complement `i32` each `u32` pattern in
`0..=255`, each `1 << i` for `i=8..=31`, `u32::MAX`, and each
`u32::MAX ^ (1 << i)` for `i=0..=31`; add `-255..=-1`; deduplicate and sort by
signed value. Pin the algorithm source hash/version and verify the count is exactly
559. This produces 312,481 ordered operand pairs per binary operation; each
four-mode multiplication and division campaign therefore has 1,249,924
operation/mode/pair cases. Do not replace this domain with a smaller hand-picked
list or imply it is the full `2^64` operand-pair domain. Add separate named
Q16-specific vectors for raw endpoints and adjacent values, zero and signed-unit/
scale boundaries, sign combinations, exact and inexact quotients, half ties on both
signs, and post-round boundaries; supplemental vectors do not change the 559-value
edge-set definition.

The test oracle uses arbitrary-precision signed integer numerator/denominator
arithmetic independent of the production implementation. It independently applies
the four rounding rules, then checked/clamped range semantics, and compares complete
public results: raw value, exact error discriminant, saturation flag, and conversion
result. At minimum include independent controls for exact halves on each side of
zero, division by zero under every mode/overflow policy, minimum signed operands,
and these boundary discriminators:

* `65537 * 2147450880 / 65536 = i32::MAX + 1/2`: toward-zero/floor return MAX;
  ceil/nearest-even overflow to MAX+1.
* `82048 * -1715306752 / 65536 = i32::MIN - 1/2`: toward-zero/ceil/nearest-even
  return MIN; floor overflows to MIN-1.
* `1073353 * 131119481 / 65536 = i32::MAX + 1/65536`: toward-zero/floor/nearest-even
  return MAX, while ceil overflows to MAX+1; this detects pre-round checking.

Property tests use a named, fixed seed set committed with fixture metadata and
sample uniformly over the full raw `i32` domain. Execute at least 4,096 valid
generated cases each for `from_integer`, add, subtract, and conversion, and at least
4,096 cases for each `(multiply|divide, rounding mode)` pair. Every case compares
all result fields against the independent oracle; each binary campaign exercises
checked and saturating variants. A shrink-negative-control must reduce an injected
oracle mismatch while preserving the same public outcome class. Keep valid typed-value
generation separate from malformed serialization or adapter-boundary campaigns.
Report seed, generated case count, actual oracle/public call count, and
per-operation/per-mode outcome strata; do not count parser refusals or construction
panics as arithmetic results.

Run the repository-supported Rust fuzz engine as two campaigns: typed arithmetic
values drawn from the complete raw domain, and malformed serialized/API inputs. The
typed campaign must reach success, checked overflow, saturation with both overflow
flag values, and division by zero where applicable for each relevant operation/mode;
record actual reducer/oracle calls, not only iterations. The malformed campaign
checks construction/adapter refusals and never counts them as arithmetic outcomes.
Run at least 4,096 actual typed cases per operation/mode fuzz stratum and 4,096
malformed-boundary cases per campaign. Preserve a fixed seed and minimized replay
for each discovered failure.

## Warrant and bounded proof evidence

Run the canonical evidence generator against the actual checked-in exports and exact
source-input hashes. Require native Warrant evaluation and the public Rust functions
to agree with the independent oracle on all shared vectors and seeded generated
inputs, comparing complete typed results. Include a negative control for each
operation's mutation-sensitive behavior, rounding tie direction, overflow boundary,
checked versus saturating policy, and error mapping. Every required error must be
observed as its actual typed result; a panic or refusal is not a successful kill.

A mandatory bounded SAT baseline proves full-width 32-bit add and subtract results,
including signed overflow detection, checked refusal, and saturation value plus
flag. Construct arithmetic and range-comparison circuits independently; verify
gate truth tables, arithmetic cells, and endpoint controls, then DRAT-check each
theorem. Replay decoded witnesses unchanged through public exports and the exact
oracle. Include admitted add/sub and overflow/clamp mutants with SAT witnesses. This
is a full bit-vector circuit result, not generated-source equivalence.
Multiplication/division SAT is required only if the current encoder faithfully models
signed quotient rounding and typed results; otherwise report that operation's
unsupported disposition exactly and retain native Warrant/vector/property evidence.
Name every proof domain and circuit construction. Never call the 559-value edge
campaign exhaustive over all `i32` pairs, substitute unsigned division or wraparound,
describe a hand-coded circuit as source-derived proof, or reduce exact-result
equivalence to `is_ok` only.

## Package delivery and cache identity

The implementation is a zero-runtime-dependency Rust package with normal package
README/changelog, `BUILD` and `BUILD_windows`, toolchain/capability metadata, and
strict Rust formatting, all-target Clippy, debug and release tests. Meaningful
coverage must exceed 95% of the package library source; report the exact instrumented
file scope and exclusions. The standard evidence command must run the actual generator checks, native Warrant
controls, vectors, property/fuzz campaigns, and bounded add/sub proof controls. Wire
the repository's existing exhaustive/property/fuzz and proof manifests; derive
counts from actual executions and run their canonical `--check` paths. If these
schemas cannot represent a required result or unsupported proof without fabricated
counters, record the exact generator/report prerequisite before claiming conformance.
Refresh the canonical package-presence report; presence is not cross-language
behavioral parity.

Every external fixture, generator, Warrant source, and transitive evidence input
actually consumed by the package must be selected and included in effective cache
identity. Demonstrate same-length valid semantic input mutation causing source
selection and a persisted-cache miss, unchanged input yielding a hit, and restored
input returning to the original identity. Exercise the repository's actual filtered
plan emit/replay and fallback-cache paths; metadata annotations alone are not proof
of selection or hashing. Keep the input list exact and source-origin scoped in the
canonical boundary registry.

## Explicitly deferred

Q32.32, square root, CORDIC, arbitrary precisions, mixed precision, floating-point
conversion, trigonometric functions, language ports, Lunar Lander migration, and
whole-game simulation remain in parent GBL-020a or later items. This slice makes no
whole-domain solver, portable-source-equivalence, or cross-language parity claim.
