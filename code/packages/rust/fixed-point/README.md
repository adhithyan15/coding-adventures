# coding-adventures-fixed-point

A dependency-free Rust library for deterministic signed Q16.16 arithmetic. A raw `i32` value
`r` represents exactly `r / 65_536`; production operations use integer intermediates only.

## API

`Q16 { pub raw: i32 }` stores every representable bit pattern. The ten public operations are
`from_integer`, `to_integer`, checked and saturating add/subtract, and checked and saturating
multiply/divide. Multiplication and division take one of four explicit modes: `TowardZero`,
`Floor`, `Ceil`, or `NearestEven`. Checked operations return `Q16Error::Overflow` for an
out-of-range rounded result. Division returns `DivisionByZero` before any rounding or range
check. Saturating operations clamp only after rounding and report whether the exact rounded raw
result was outside `i32` in `SaturatingResult::overflowed`.

`to_integer` rounds the raw value divided by 65,536. Its result always fits `i32` because every
Q16.16 raw value is between -32,768 and 32,768 in integer units. `from_integer` is exact and
returns overflow when the scaled raw value is outside `i32`.

## Validation

The package shares named vectors in `code/specs/fixtures/fixed-point-q16-v1/cases.json` and the
full arithmetic model in `code/specs/fixed-point.warrant`. Tests compare the real Rust exports,
the generated Warrant evaluator, and an independent arbitrary-precision `BigInt` oracle on shared
vectors and deterministic generated inputs. The finite edge campaign enumerates the canonical
559-value operand set and its specified combinations; it does not cover every pair in the full
`i32 × i32` domain. It records 559 raw edge values for integer construction, 2,236 operation/mode
combinations for integer conversion, 312,481 raw operand pairs for each add/sub claim, and 1,249,924
pair/mode combinations for each multiply/divide claim. The seeded PBT and
fuzz manifests record 4,096 cases/iterations per operation claim across all four rounding modes;
malformed fixture transport is tested in a separate campaign. The bounded SAT target proves full-width checked and saturating add/subtract miters for
independent 32-bit operands with DRAT-checked circuits and replayed mutation witnesses. This is
not a source-equivalence proof for multiplication/division or a proof of the whole package.

Scoped line coverage was measured from `code/packages/rust` with:

```sh
cargo tarpaulin --locked -p coding-adventures-fixed-point \
  --include-files 'fixed-point/src/*.rs' --out Json \
  --output-dir /tmp/gbl020a1-tarpaulin-capability
```

The report covers the package library source: `src/lib.rs` is 64/64 executable lines; generated
`src/capabilities.rs` has no coverable executable lines. Dependencies and test harness code are
outside the coverage denominator. The run's JSON and log are retained under `/tmp` for review.

The source frontend currently refuses this Rust module and its record/`Result` signatures, so
canonical source-IR proof dispositions remain `not-live` under VER-P047. The build recipe runs the
real WIR probe, records those refusals in `evidence/proof-manifest.json`, and regenerates and checks
`assurance_report.json`. The report remains `report-only`: E2/E5 are not live, and the missing
Warrant step bounds produce unapproved CWE-400 and CWE-407 dispositions. Finite native tests,
generated-model comparisons, and bounded add/sub SAT results do not change that status.

The crate declares no ambient capabilities and has no runtime dependencies.
The checked-in `effect_certificate.json` is emitted and independently verified by the
capability-analyzer CLI in both platform build recipes.
