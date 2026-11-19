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
construction. Avalanche analysis
accepts a hash callback, declared output width, and explicit sample byte
sequences. Distribution analysis accepts a callback, explicit input byte
sequences, and bucket count. No helper silently generates samples.

## Numeric and byte domains

Byte inputs are sequences of integers from 0 through 255. Invalid elements are
rejected, not truncated or reinterpreted as signed bytes. Empty byte sequences
are valid for every hash. Byte APIs accept arbitrary byte contents, including
invalid UTF-8. String conveniences encode valid Unicode strings as UTF-8 and
reject unpaired UTF-16 surrogates rather than silently replacing them.

Seeds are integers in `0..0xffffffff`. Polynomial base and modulus are
integers in `0..0xffffffffffffffff`, with modulus zero rejected. Compatibility
defaults are base 31 and modulus `2^61 - 1`. Polynomial steps compute the exact
mathematical intermediate `(hash * base + byte)` before reduction modulo the
modulus. They do not wrap that intermediate at 64 bits.

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

## Analysis helpers and failures

Avalanche analysis accepts `outputBits` from 1 through 64, at least one sample,
and no empty samples. For each sample it flips each input bit independently,
compares outputs over the declared output width, and divides by the total
number of compared output bits. It rejects callback results that are negative
or do not fit the declared width. It uses exact counters before converting the
final score to `double`.

Distribution analysis requires at least one input and a positive bucket count.
Empty byte input remains valid. It maps each validated unsigned callback
result with exact integer modulo before converting the bounded bucket index.
Its chi-squared result is a `double` computed from the complete supplied
corpus.

Invalid byte values, seeds, polynomial parameters, SipHash key lengths,
analysis widths, empty analysis corpora, bucket counts, and callback outputs
raise `ArgumentError`. No invalid parameter is silently normalized. Valid
hashing calls remain total for every finite admitted byte sequence. Invalid
Unicode scalar sequences passed to a string convenience also raise
`ArgumentError`.

## Conformance evidence

The implementation milestone uses the existing shared DT17 literal vectors as
the independent oracle. Fixture u64 values are encoded as decimal or
hexadecimal strings, not JSON numbers. The Dart consumer parses them directly
to `BigInt`. VM and Node runs must execute the same fixture cases through the
public Dart APIs. Node evidence must execute those public calls under Node.
Compilation alone does not establish runtime conformance. Fixture loading may
use a test-only bridge or generated constant snapshot verified against the
neutral JSON corpus, while production remains free of I/O dependencies.

Tests cover empty and high-bit bytes, embedded zeroes, UTF-8 including
supplementary scalars, 32-bit and 64-bit wrap boundaries, Murmur tail lengths
and nonzero seeds, SipHash key and output boundaries, polynomial parameters
and exact intermediates, wrapper output widths, input ownership, and analysis
parameter failures. Expected answers come from neutral vectors or independent
literals, not another production language's output. Avalanche and distribution
are deterministic checks over caller-supplied samples. They do not prove
cryptographic security or statistical quality for all inputs.

This profile does not establish native Warrant proof, timing behavior,
collision resistance, password-hashing suitability, security certification,
or parity in Java and Kotlin. Those claims and lanes require separate evidence.
