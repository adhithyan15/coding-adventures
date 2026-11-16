# DT17a: Warrant contract for `hash-functions` (WX-P1a)

Status: **blocked on a generator gap (section 5); the Warrant document is a
draft and nothing is wired into BUILD.**

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
| `avalanche_score`, `distribution_test` | return `f64` (floats are VER04p, WX-FL) and take a generic closure `F: Fn(&[u8]) -> H`; `avalanche_score` also draws OS randomness. |
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

**Examples.** 67 known answers: the published FNV-1a test vectors ("", "a",
"b", "c", "foobar"), Murmur3 "Hello, world!" under seed `0x9747b28c`, SipHash
lengths 0, 1 and 15 under the key `00 01 .. 0f`, and the rest computed by an
independent Python implementation that reproduces every published value. All 67
were cross-checked against the real crate (out of tree, 0 differences) before
this document was written.

**Labels.** `data`, `base`, `modulus` and `seed` are `public`. The SipHash
`key` is `secret`: its secrecy is the whole point of the algorithm, and
labelling it public would hide CWE-208 from the report. The package does not
claim a constant-time proof, so the report records that row as applicable and
unaddressed, as it does for gf256. Domain labels (`full`, `bounded-set`, `sample`) and
case counts come from the generated manifest once it exists; this document
states none, and the owner's standard is to say `sample` for a `Bytes` or
`Bytes<N>` binder.

## 5. Blocker: no negative control exists for a scalar result

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

## 6. Draft status of `hash-functions.warrant`

The document parses and type-checks (`check_warrant`) as a `v1` document. It has
**not** been run through the harness generator, because the generator stops at
the binding step above, so none of its claims has been executed against the
crate; only its 67 examples have (section 4). Nothing else in the repository
refers to it: no `evidence.bindings`, no BUILD line, no report.
