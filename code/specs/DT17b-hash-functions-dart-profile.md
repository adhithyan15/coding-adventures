# DT17b: Dart hash-functions portability profile

**Status: profile for implementation review.** This document narrows the
existing DT17 hash-functions contract to the Dart implementation, including
Dart compiled to JavaScript. It does not change DT17 or claim Warrant proof.
The child is tracked under the existing
`hash-functions-dart-java-kotlin-lane-parity` owner. Java and Kotlin remain
pending under that owner.

## Scope and public API

The Dart package implements the DT17 byte-input functions and their specified
UTF-8 conveniences: FNV-1a 32-bit and 64-bit, DJB2 64-bit, Murmur3 32-bit with
an optional seed defaulting to zero, SipHash-2-4 with a caller-provided key,
and polynomial rolling hash with optional base and modulus. It also exposes
the DT17 hash-function wrappers and deterministic avalanche and distribution
analysis helpers. The package remains pure and deterministic. It does not
obtain keys, entropy, clocks, files, or other host capabilities.

Public function names use Dart lower-camel case: `fnv1a32`, `fnv1a64`,
`djb2`, `murmur3_32`, `siphash2_4`, `polynomialRolling`,
`polynomialRollingWithParams`, `hashStringFnv1a32`, and
`hashStringSiphash`. Wrappers expose `hash(List<int> data)` and `outputBits`.
The wrapper types are `Fnv1a32`, `Fnv1a64`, `Djb2`, `PolynomialRolling`,
`Murmur3_32`, and `SipHash24`. The SipHash wrapper requires its key at
construction. A wrapper's `hash` returns the same type as its corresponding
function: Dart `int` for 32-bit algorithms and `BigInt` for 64-bit algorithms.
Avalanche and distribution callbacks have the uniform type
`BigInt Function(List<int>)`. Callers adapt 32-bit functions with
`BigInt.from`; callbacks returning `int` are not accepted implicitly. No
helper silently generates samples.

## Numeric and byte domains

Byte inputs are sequences of integers from 0 through 255. Invalid elements
raise `ArgumentError`; values are never truncated or reinterpreted as signed
bytes. Empty byte sequences are valid for every hash. Byte APIs accept
arbitrary byte contents, including
invalid UTF-8. String conveniences encode valid Unicode strings as UTF-8 and
reject unpaired UTF-16 surrogates rather than silently replacing them.

Seeds are Dart `int` values in `0..0xffffffff`. Polynomial base and modulus
are `BigInt` values in `0..0xffffffffffffffff`, with modulus zero rejected.
The parameterized function and wrapper require both parameters as `BigInt`,
including small values. Compatibility defaults are `BigInt.from(31)` and
`BigInt.parse('2305843009213693951')` (`2^61 - 1`). Polynomial steps compute
the exact mathematical intermediate `(hash * base + byte)` before reduction
modulo the modulus. They do not wrap that intermediate at 64 bits.

All arithmetic uses exact `BigInt` intermediates where a result could exceed
JavaScript's exact integer range. FNV-1a and Murmur3 apply their specified
32-bit masks at each algorithm step. DJB2, FNV-1a 64-bit, and SipHash apply
their specified 64-bit modulo arithmetic. Rotations use logical right shifts
of width-masked values. Constants wider than 53 bits are represented directly
as `BigInt`, never through `double` or an imprecise `int` conversion.

Public 32-bit results are nonnegative Dart `int` values in `0..0xffffffff`.
Public 64-bit results are unsigned `BigInt` values in
`0..0xffffffffffffffff`. This includes DJB2, FNV-1a 64-bit, SipHash, and
polynomial rolling results. SipHash keys contain exactly 16 bytes and are
caller-supplied. The wrapper snapshots the key so later caller mutation does
not change its behavior.

## Existing evidence boundary

| Surface | Existing Rust evidence | Evidence required for Dart |
| --- | --- | --- |
| Eight byte functions | `code/packages/rust/hash-functions/src/lib.rs` exports the eight DT17 byte operations. `code/specs/hash-functions.warrant` covers only those operations with 277 Rust tests, including 73 literal examples. | Execute the same neutral byte vectors through Dart public functions. Compare exhaustive expectations against the independent exact model, then cross-check selected known answers against Rust public functions. Rust results are a second implementation comparison, not the expected-answer oracle. |
| UTF-8 conveniences and six strategy wrappers | The Warrant excludes both string conveniences and the trait/strategy types. The string functions are simple Rust forwards, but that does not establish Dart string or wrapper behavior. | Exercise Dart UTF-8 encoding and malformed-string rejection, wrapper output widths, SipHash key ownership, and forwarding behavior through Dart public APIs. |
| Avalanche and distribution analysis | Rust analysis lives in the separate `hash-analysis` crate described by DT17a. Its avalanche helper draws OS randomness and its analysis API is not covered by the eight-operation Warrant. | Use explicit samples only. Hand-computable synthetic callbacks pin flip counts, denominators, bucket counts, and rational chi-squared results. Real hash callbacks use outputs from the independent exact model. Do not use Rust random avalanche values as expected answers. |

The 277 Warrant test count and its claims belong to the existing Rust crate.
They do not establish Dart behavior or a Dart Warrant proof.

## Analysis helpers and failures

Avalanche analysis accepts `outputBits` from 1 through 64, at least one sample,
and no empty samples. For each sample it flips each input bit independently,
compares outputs over the declared output width, and divides by the total
number of compared output bits. It rejects callback results that are negative
or do not fit the declared width. It uses exact counters before converting the
final score to `double`.

Distribution analysis requires at least one input and a Dart `int` bucket
count in `1..9007199254740991` (`2^53 - 1`). Empty byte input remains valid.
Callback values must be `BigInt` values in `0..0xffffffffffffffff`. The helper
maps each result with exact `BigInt` modulo and stores only occupied buckets,
so memory use does not grow with the bucket count. For `N` inputs, `k` buckets,
and occupied counts `c`, it computes the exact rational chi-squared value
`(k * sum(c * c) - N * N) / N`, including implicit empty buckets, and converts
to `double` only for the returned statistic. Counts, bucket indices, and the
rational numerator remain exact on Dart VM and JavaScript targets.

Invalid byte values, seeds, polynomial parameters, SipHash key lengths,
analysis widths, empty analysis corpora, bucket counts, and callback outputs
raise `ArgumentError`. No invalid parameter is silently normalized. Valid
hashing calls remain total for every finite admitted byte sequence. Invalid
Unicode scalar sequences passed to a string convenience also raise
`ArgumentError`.

## Conformance evidence

The implementation milestone first creates a versioned neutral corpus and
independent exact-arithmetic reference model, then cross-checks admitted
published known answers against the existing public Rust package. These are
separate evidence sources: model-generated expectations do not become literal
known-answer vectors. Fixture u64 values use decimal or hexadecimal string
encoding to preserve exactness through JSON parsing. The Dart consumer parses
them directly to `BigInt`.

The corpus includes exhaustive inputs consisting of every byte string of
length zero, one, and two (65,793 inputs per applicable operation) under
Murmur3 seed `0x9747b28c`, SipHash key `000102030405060708090a0b0c0d0e0f`,
and both polynomial entry points: the default operation uses base 31, while
the parameterized operation uses base 257 with modulus
`2305843009213693951`. Each operation's output is stored as a
fixed-width lowercase hexadecimal stream with exactly one output per ordered
input. Dart validates the full stream and consumes every output.
It also includes directed four- and eight-byte block and tail boundaries,
wrap carries, maximum seed and key bytes, and polynomial cases whose exact
intermediate exceeds 64 bits. Longer deterministic property histories and
byte-mutation fuzzing must record seeds, iteration counts, replayable failing
inputs, and shrinking only where the selected engine actually supports it.
Executed source-fault controls must reach and fail the intended assertions;
syntax or setup failures are not controls.

VM and Node runs must execute the same fixture cases through public Dart APIs.
Node evidence must execute those public calls under Node. Compilation alone
does not establish runtime conformance. A test-only generator binds the raw
neutral JSON bytes as chunked base64 constants in ignored `.dart_tool` output.
The VM integrity test compares decoded snapshot bytes exactly with the neutral
JSON file. VM and Node decode the same generated snapshot through
`dart:convert`; production remains free of I/O dependencies. Report the actual
coverage engine and its line/branch capabilities, with at least 95% measured
coverage under the configured native Dart test workflow. No SAT or native
Warrant proof is claimed without executing the actual supporting engine.

Tests cover empty and high-bit bytes, embedded zeroes, UTF-8 including
supplementary scalars, 32-bit and 64-bit wrap boundaries, Murmur tail lengths
and nonzero seeds, SipHash key and output boundaries, polynomial parameters
and exact intermediates, wrapper output widths, input ownership, and analysis
parameter failures. Literal known answers and independent model-generated
exhaustive expectations remain distinguishable in the corpus. Avalanche and
distribution are deterministic checks over caller-supplied samples. They do
not prove cryptographic security or statistical quality for all inputs.

This profile does not establish native Warrant proof, timing behavior,
collision resistance, password-hashing suitability, security certification,
or parity in Java and Kotlin. Those claims and lanes require separate evidence.
