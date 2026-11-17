# DT17a: Warrant contract for `hash-functions` (WX-P1a)

Status: **complete (WX-P1a and WX-P1a2).** The evidence manifest, harness, sabotage
lines, jail, proof manifest, capability analyzer certificate and package
assurance report are all wired. The randomness that blocked the last two was
split out of the crate into `hash-analysis` (section 7).

This is the contract document that VER30 section 4.2 (Track B, step 1) asks for
before any `.warrant` file. It says what `rust/hash-functions` promises, which
functions the Warrant document covers, which it does not and why, how the claims
are shaped, and what stops the adoption today. The algorithms themselves are in
[DT17](DT17-hash-functions.md); the document is
[`hash-functions.warrant`](hash-functions.warrant).

## 1. Why this crate, why now

Owner decision of 2026-11-16 (VER04o OWN-6): `hash-map` is the generics pilot,
and its only path dependency, `hash-functions`, is verified first (backlog
WX-P1a). The 2026-11-16 inventory lists it as T1: primary blocker `Bytes<N>`
(landed, VER04j section 14.4), secondary blockers floats, generics and text.
That means the T0 subset (the byte-string hashes) can be adopted now and the
rest is listed, not faked.

## 2. Covered operations (eight)

Every one takes a `Bytes` (or `Bytes<16>` key) and returns a fixed-width integer.

| Operation | Rust export | Result |
|---|---|---|
| `fnv1a_32` | `fnv1a_32(&[u8]) -> u32` | `U32` |
| `fnv1a_64` | `fnv1a_64(&[u8]) -> u64` | `U64` |
| `djb2` | `djb2(&[u8]) -> u64` | `U64` |
| `polynomial_rolling` | `polynomial_rolling(&[u8]) -> u64` (base 31, modulus 2^61 - 1) | `U64` |
| `polynomial_rolling_with_params` | `(&[u8], u64, u64) -> u64`, panics on modulus 0 | `Result<U64, PolyError>`, the panic normalised to `MODULUS_ZERO` as gf256 does |
| `murmur3_32` | `murmur3_32(&[u8]) -> u32` | `U32` |
| `murmur3_32_with_seed` | `(&[u8], u32) -> u32` | `U32` |
| `siphash_2_4` | `(&[u8], &[u8; 16]) -> u64` | `U64`, the key a `Bytes<16>` (host `bytes<16>`) |

## 3. Not covered, and why

| Item | Reason |
|---|---|
| `hash_str_fnv1a_32`, `hash_str_siphash` | take `&str`; Warrant text is VER04l (WX-T) and is not available to the evidence generator. They are one-line forwards to the covered byte functions. |
| `avalanche_score`, `distribution_test` | no longer in this crate: moved to `hash-analysis` (WX-P1a2, section 7) because `avalanche_score` draws OS randomness. They return `f64` (floats are VER04p, WX-FL) and take a generic closure `F: Fn(&[u8]) -> H`, so they would not be covered there either. |
| trait `HashFunction` and the structs `Fnv1a32`, `Fnv1a64`, `Djb2`, `PolynomialRolling`, `Murmur3_32`, `SipHash24` | trait objects and stateful values are VER04m / VER04o territory (WX-O, WX-P). They only forward to the covered free functions; `output_bits` is the constant 32 or 64. |
| the constants (offset bases, primes, defaults) | not operations. The examples and models pin their values by use. |

## 4. Shape of the claims

**Wrapping.** FNV, DJB2, Murmur3 and SipHash are defined modulo 2^32 or 2^64.
VER04 section 3 gives `U32` and `U64` `+` and `*` wrapping semantics and the
generated model code uses `wrapping_*` (VER04j section 2.5), so a model that
multiplies two `U32` values is the code's own arithmetic. FNV and DJB2 steps are
written a second time with exact `Int` arithmetic and an explicit `rem`
(`fnv32exact`, `fnv64exact`, `djb2exact`) so the code is pinned to the
mathematical definition and not only to another wrapping expression.

**No fold yet.** Warrant has no `fold` (WX-D2). A variable-length hash is
therefore stated two ways that together fix it:

* unrolled models for the lengths that exercise every code path (0 to 3 bytes
  for FNV and DJB2; lengths 0 to 8 and 11 for Murmur3; 0 to 16 for SipHash:
  tail only, block, block plus tail, two blocks);
* an inductive step for the three hashes whose state is their output (FNV-1a,
  DJB2, polynomial rolling): `hash(d ++ [x]) == step(hash(d), x)` for every `d`
  and `x`. With the empty-input example this determines the function on every
  length. The domain of `d` is a sample, so it is evidence for every length and
  not a proof of it.

**Examples.** 73 known answers (the first 67 below were cross-checked against the crate before the document was written; six more were added in review): the published FNV-1a test vectors ("", "a",
"b", "c", "foobar"), Murmur3 "Hello, world!" under seed `0x9747b28c`, SipHash
lengths 0, 1 and 15 under the key `00 01 .. 0f` (and 17, 24, 32, 63 and a five-block Murmur3 input, added in review), and the rest computed by an
independent Python implementation that reproduces every published value. All 67
original answers were cross-checked against the real crate (out of tree, 0 differences) before
this document was written.

**Labels.** `data`, `base`, `modulus` and `seed` are `public`. The SipHash
`key` is `secret`: its secrecy is the whole point of the algorithm, and
labelling it public would hide CWE-208 from the report. The package does not
claim a constant-time proof, so the report records that row as an unapproved
exemption (applicable and unaddressed), as it does for gf256 (section 7). Domain labels (`full`, `bounded-set`, `sample`) and
case counts come from the generated manifest once it exists; this document
states none, and the owner's standard is to say `sample` for a `Bytes` or
`Bytes<N>` binder.

## 5. Blocker (resolved by WX-FAULT1): no negative control existed for a scalar result

`gen_evidence` requires every bound operation to name at least one fault
(`evidence.bindings: op ... needs faults= (at least one)`), and a fault must
apply to the operation's result type. An operation with a `Bytes` parameter
beside anything, or with a `Bytes` result, goes through the *composite* adapter
(`composite_adapter` in `warrant-grammar/src/evidence.rs`). Its fault check
accepts only:

* `flip-result-bit`, `drop-last-result-byte`: a `Bytes` or `Bytes<N>` result;
* `missing-error`, `ok-as-err`: a `Result` result;
* `drop-consumed`, `off-by-one-consumed`, `swap-components`, `field-zero`,
  `wrong-ctor`: a tuple, record or variant result;
* the limit faults: an operation with an `output` bound.

A `Bytes -> U32` or `Bytes -> U64` operation matches none of them. The
arithmetic faults (`narrow-width`, `saturate-*`) belong to the scalar adapter,
which needs scalar *parameters*. Trying `narrow-width` on `fnv1a_32` ends with
`fault narrow-width is not a composite-operation fault`. So seven of the eight
operations (all but `polynomial_rolling_with_params`, which has `missing-error`)
cannot be bound, and a document whose operations are not all bound cannot be
generated.

Smallest fix, in the shared generator and so **not made here**: let
`flip-result-bit` apply to a fixed-width integer result (the fault check in
`composite_adapter`, plus one arm `V::Fixed(f, x) => V::Fixed(f, x ^ 1)` in
`fault_flip_result_bit` of `evidence_prelude.rs.txt`). The same gap should block any
function from bytes to a scalar (a checksum such as a CRC or Adler sum, a
non-cryptographic hash); a function that returns `Bytes` or `Bytes<N>` is
unaffected. The remaining work on this adoption, once it is closed:

1. `evidence.bindings` (one `op` line per operation; `siphash_2_4` needs
   `args=bytes,bytes<16>`; `polynomial_rolling_with_params` needs
   `error=MODULUS_ZERO faults=missing-error`; every other line the new fault);
2. generate, commit and `--check` the harness and its manifest;
3. one sabotage line per operation, in BUILD, BUILD_linux and BUILD_windows;
4. `required_capabilities.json` (already present, no capabilities), the jail
   test `tests/runtime_capability.rs`, and the strict gate in BUILD_linux;
5. `assurance-profile.json`, `assurance_report.json`, and the proof manifest.
   The report requires `evidence/proof-manifest.json`, which every earlier
   adopter produces with a per-package `warrant-ir/tests/<pkg>_proofs.rs`. That
   is a new file in a shared crate; whether an adoption may add one, or the
   package carries its own, is an owner call.

**Resolution.** WX-FAULT1 (`claude/warrant-fault-fixed-int`, PR #19508, merged into this
branch) made `flip-result-bit` apply to a fixed-width integer result. The first
harness to compile its `V::Fixed` arm is this one: it compiled and behaved as
designed without any change to the generator, and every `flip-result-bit` control
is caught (section 6).

## 6. What is wired

* `evidence.bindings`: eight `op` lines. Seven take `faults=flip-result-bit`;
  `polynomial_rolling_with_params` takes `args=bytes,u64,u64`,
  `error=MODULUS_ZERO`, `faults=missing-error,flip-result-bit` (the second flips
  the hash inside `Ok`; `ok-as-err` is refused for it); `murmur3_32_with_seed` takes
  `args=bytes,u32`; `siphash_2_4` takes `args=bytes,bytes<16>`.
* `tests/generated/warrant_evidence.rs` and `evidence/test-manifest.json`: 277
  tests, all passing (73 examples, 56 exhaustive, 56 property, 56 fuzz, 36
  negative controls). Domains the manifest labels: every `Bytes` binder is a
  `set`/`sample`, `U8` is `full`, `U32`/`U64` are `edge`, so none of the
  inductive claims is a proof for all lengths.
* BUILD, BUILD_linux and BUILD_windows: unit and doc tests, the jail,
  `gen_evidence --check`, the evidence run, nine sabotage lines (one per
  operation, two for `polynomial_rolling_with_params`, each aimed at a cheap
  exhaustive claim), the proof manifest, and `git diff --exit-code -- evidence/`.
  Both manifests are deterministic (fixed seed, content hashes, no timestamps or
  toolchain data; the repo forces LF), so the manifests must be committed
  regenerated; the report's `--check` (section 7) cross-checks them as well.
* `tests/runtime_capability.rs` (VER08b): the eight covered hashes on known
  answers inside the no-capability jail, strict on Linux.
* `warrant-ir/tests/hash_functions_proofs.rs`: the proof test (owner call of
  the earlier draft: it follows the existing pattern and lives in `warrant-ir`).
  The `siphash_2_4` constant-time obligation is `not-live`: the front end
  refuses `algorithms.rs` at its first `pub const`. Since WX-P1a2 it also records
  one `steps-bound` obligation per operation `not-live` (same blocker) and the
  `depth-bound` rows `unchecked`.

**Zero modulus.** The real `polynomial_rolling_with_params` panics on
`modulus == 0` (`assert!(modulus > 0)`). The document's `ensures no_panic` on that
operand holds of the harness adapter, which catches any panic and reports it as
`MODULUS_ZERO`, not of the crate; the adapter cannot tell this panic from another.

## 7. Analyzer and report: the randomness split (WX-P1a2, done)

The first adoption (WX-P1a) could not run the capability analyzer or
`ca-assurance-report --check`, because the crate did not fit the lzw /
wasm-leb128 pattern:

1. `src/analysis.rs` called `getrandom` (`avalanche_score`), so the analyzer
   failed (`CAP001 rand:read:*`) against `"capabilities": []`.
2. Declaring `rand:read:*` instead would have let the analyzer pass but allowed
   the `rand` syscall in the VER08b jail (its `rand-syscall` negative control
   undetectable, `RCG009`), and left the report's CWE-330 row open.
3. Independently, the report stopped at CWE-400 because the document declared no
   `bound` rows.

The owner decided on 2026-11-16 to separate the randomness from the pure hashes.
What was done:

* **New crate `rust/hash-analysis`** (`coding_adventures_hash_analysis`) holds
  `avalanche_score` and `distribution_test`, unchanged. Its
  `required_capabilities.json` declares `rand:read:*` with a justification
  (statistical sampling, never key material); its `BUILD` runs the capability
  analyzer against that declaration. It has no Warrant document (floats and a
  generic closure). The hash functions it measures are a dev-dependency.
* **`hash-functions` 0.2.0** dropped the `analysis` module, its two exports and
  the `getrandom` dependency (breaking: minor bump; its CHANGELOG has a
  "Removed" section). No other Rust package used the helpers.
  `"capabilities": []` is now true of the whole source, so the analyzer passes
  and `effect_certificate.json` is committed (label `proved`), and the jail's
  `rand-syscall` control stays detected (strict gate passes).
* **Bound rows.** Every operation declares `steps <= 64 * len(data) + 4096` and
  `depth <= 8`. There is no `output` row: that metric bounds a `Bytes` or `List`
  result, and these return a 4- or 8-byte integer. Five operations
  (`polynomial_rolling`, `polynomial_rolling_with_params`, `murmur3_32`,
  `murmur3_32_with_seed`, `siphash_2_4`) also declare `heap <= 4096`,
  checked live under the allocation observer (the manifest records a peak of 0
  bytes for the four pure hashes and at most 80 for `polynomial_rolling_with_params`,
  the panic payload of its zero-modulus path). `fnv1a_32`, `fnv1a_64` and `djb2`
  have **no `heap` row**: their exhaustive suites have 2^24-case claims, and the
  observer costs about 35 to 50 microseconds a call on a worker thread, so about
  50 million observed calls would add about half an hour (a trial with a heap row
  on all eight operations had not finished in over 30 minutes). For those three,
  "nothing allocates" is read off the source, not evidenced by the harness. The
  steps rows are declarations, which the proof manifest records `not-live` (the
  Rust-subset front end refuses `algorithms.rs`).
* **Profile and report.** `assurance-profile.json` (`non-cryptographic-hash`,
  kind `transform`) words the static rows 326, 327 and 407; `assurance_report.json`
  is generated from the manifests and `BUILD`/`BUILD_linux`/`BUILD_windows` run
  `ca-assurance-report --check ... --profile non-cryptographic-hash
  --profile-kind transform`. Result: CWE-330 **excluded** (no `rand` capability
  and no randomness in the package), CWE-326 and CWE-327 excluded, CWE-400
  recorded as the usual **unapproved exemption** `EXEMPT-CWE-400-NO-STEP-PROOF`
  (the steps bounds are declarations no tool checks, exactly as for lzw and
  wasm-leb128), CWE-208 an unapproved exemption (the SipHash `key` is `secret`
  and no constant-time proof is live). Verdict `report-only`.

What remains open is not about the split: a live step-bound proof (blocked by the
front end), a constant-time proof for the SipHash key, and `hash_str_*` (WX-T).
`effect_certificate.json` is not verified with `--verify-cert`: that needs a
generated `src/capabilities.rs` witness, which no adopter has yet (backlog
LP-001b).
