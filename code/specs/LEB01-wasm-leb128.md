# LEB01 — LEB128 Variable-Length Integers (WebAssembly binary format)

## Overview

LEB128 ("Little Endian Base 128") writes an integer as a sequence of bytes
that each carry **seven data bits** plus one **continuation bit**. Small
numbers take one byte, large ones up to ten (for 64 bits). WebAssembly uses it
for every count, index, size and integer immediate in a `.wasm` file, and DWARF
uses it for debug information.

```text
value 624485 = 0b 0100110 0001110 1100101          (three 7-bit groups)
                  g2      g1      g0
bytes          = 1_1100101  1_0001110  0_0100110   (low group first)
               =   0xE5       0x8E       0x26
                   ^continue  ^continue  ^last (bit 7 clear)
```

The often-quoted `E5 88 26` for 624485 is wrong; it decodes to 623717.

This spec is the contract of the Rust package `code/packages/rust/wasm-leb128`
and the prose companion of its Warrant document
[`wasm-leb128.warrant`](wasm-leb128.warrant), which is the executable
specification: every example and claim there is compiled into a test harness
and run against the real exports (VER04h).

## Layer Position

A leaf: no internal dependencies. Used by the WebAssembly module parser,
encoder, validator and runtime packages, and anything else that reads or
writes the WASM binary format.

## The format

### Unsigned (`uN`)

1. Split the value into 7-bit groups, least significant first.
2. Emit each group as one byte; set bit 7 on every byte except the last.

An `N`-bit unsigned value has at most `ceil(N / 7)` groups (10 for `N = 64`,
5 for `N = 32`).

### Signed (`sN`, two's complement)

The groups are taken from the two's-complement bit pattern with an
**arithmetic** shift, and the encoder stops at the first group after which the
rest of the value is all sign bits *and* the group's own top data bit (bit 6)
already equals the sign. That extra condition is why `64` needs two bytes
(`C0 00`): one byte `0x40` would decode as `-64`.

```text
 -2  -> 0x7E                (0b1111110: bit 6 set, rest is all ones)
 64  -> 0xC0 0x00           (0x40 alone would read back as -64)
 i64::MIN -> 80 80 80 80 80 80 80 80 80 7F
```

The decoder sign-extends from the last group read (or, for a bounded width,
from bit `N - 1`).

### Canonical and non-canonical encodings

The **canonical** encoding is the unique shortest one; both encoders here
always produce it. The WebAssembly grammar also accepts **padded** encodings
inside the width's byte budget: `80 00` is a two-byte zero, `82 80 80 80 00` a
five-byte 2 for `u32`. The decoders accept them (they are not errors) and
report the padded length as the number of bytes consumed.

### What makes an encoding malformed

For a width `N` (64 for the plain decoders, `max_bits` for the bounded ones):

| Rule | Condition | WebAssembly wording |
|---|---|---|
| truncated | the input ends before a byte with bit 7 clear | unexpected end |
| overlong | a byte at index `ceil(N/7) - 1` (the last one the width allows) still has bit 7 set | integer representation too long |
| too large (unsigned) | on the last byte, a data bit at or above value bit `N` is set | integer too large |
| too large (signed) | on the last byte, the data bits at or above value bit `N` do not all equal bit `N - 1` | integer too large |
| offset | `offset >= data.len()` | (API precondition) |

Width 0 is legal and degenerate: only the single byte `00` decodes (to 0).

## Rust contract

```rust
pub struct Leb128Error { pub message: String, pub offset: usize }

pub fn encode_unsigned(value: u64) -> Vec<u8>;
pub fn encode_signed(value: i64) -> Vec<u8>;

pub fn decode_unsigned(data: &[u8], offset: usize) -> Result<(u64, usize), Leb128Error>;
pub fn decode_signed(data: &[u8], offset: usize) -> Result<(i64, usize), Leb128Error>;
pub fn decode_unsigned_bounded(data: &[u8], offset: usize, max_bits: u32)
    -> Result<(u64, usize), Leb128Error>;
pub fn decode_signed_bounded(data: &[u8], offset: usize, max_bits: u32)
    -> Result<(i64, usize), Leb128Error>;
```

* **Encoders** are total: every `u64`/`i64` encodes, canonically, in 1 to 10
  bytes. They allocate one `Vec` and nothing else.
* **Decoders** read from `data[offset..]` and return `(value, consumed)`, where
  `consumed` is the number of bytes of the encoding (padding included). They
  never read before `offset`, never read past the first byte with bit 7 clear,
  and ignore trailing bytes. `decode_unsigned(d, o)` is
  `decode_unsigned_bounded(d, o, 64)`; likewise for signed.
* **Bound on bytes consumed.** On success, `1 <= consumed`,
  `offset + consumed <= data.len()`, and `consumed <= max(1, ceil(N / 7))`,
  i.e. at most 10 at 64 bits. On success the value fits the width: below `2^N`
  (unsigned), in `[-2^(N-1), 2^(N-1))` (signed).
* **Errors.** Every malformed input is `Err(Leb128Error)`. `offset` in the error
  is the **starting** offset passed in (not the position of the offending
  byte); `message` names the rule (it is for humans and is not part of the
  contract). There is no error *kind*: callers cannot distinguish the rules
  programmatically.
* **Hostile input.** For every `max_bits` (a width above 64 is rejected), every `data` and every `offset`
  yields `Ok` or `Err`: no panic, and at most `max(1, ceil(max_bits / 7))`
  bytes are inspected.
* **Width precondition.** `max_bits` must be at most 64; a wider width is
  rejected (`Err`) before any byte is read. See Findings.

## Findings

Recorded while writing the Warrant document.

1. **`max_bits > 64` was not rejected (backlog SEC-006) -- FIXED in this
   branch.** `decode_*_bounded` with `max_bits` in `66..=70` accepted a
   ten-byte encoding whose data bits above bit 64 are set and silently dropped
   them (`FF x9 7F` decoded to `u64::MAX` at width 70); with `max_bits >= 71`
   an eleven-byte input reached `value |= data_bits << 70`, which panics in a
   debug build ("attempt to shift left with overflow") and yielded a wrong
   value in release (`80 x10 01` decoded to `(64, 11)`). `decode_bits_core`
   now returns `Err` for `max_bits > 64` before any other check (so the
   reported fault does not depend on `data` or `offset`); `max_bits = 0` is
   unchanged (it admits only the value 0). The Warrant document has the
   `rejects max_bits > 64u32 with Leb128Error.MALFORMED` row, and claims over
   every width 65..=255, the `2^k` widths up to `2^31`, and `u32::MAX`.
2. **The error carries no kind.** A caller (or a specification) can only say
   "rejected". Adding a kind field is an API change across 17 ports and is out
   of scope.

## Cross-language expectations

All ports must agree on: canonical encoders; the five rejection rules above at
the 64-bit width and, where a port has a bounded decoder, at every width
`0..=64`; acceptance of padded in-budget encodings; `consumed` counting the
padding; trailing bytes ignored. A port reuses the Warrant document by writing a
binding (its exports, how its error is observed) and a generator target for its
language (only the Rust generator exists today).

## Verification status (Warrant-derived)

`code/specs/wasm-leb128.warrant` (Warrant v1: WX-A tuples, WX-B fixed-width
integers, WX-C bit operations) is the specification of the six operations;
`tests/generated/warrant_evidence.rs` is generated from it and
`evidence.bindings`, and `BUILD` fails when it drifts. The document states:

* **encoders:** equal for every input of the domain to independent canonical
  models (`eu0`..`eu9`, `es0`..`es9`, written from the format, not the crate);
  a closed-form length (`max(1, (70 - clz v) / 7)` unsigned, one sign bit more
  for signed); 1 to 10 bytes;
* **decoders at 64 bits:** every input of length at most 3 equals an
  independent model (1 + 256 + 65,536 + 16,777,216 = **16,843,009 inputs** for
  each of `decode_unsigned` and `decode_signed`, enumerated); round trip from
  the encoder, also after any prefix (decoding at `len(prefix)`) and with any
  suffix; offset at or past the end rejected; `consumed` bounded (above); the
  ten-byte boundary (nine continued bytes carrying any 63 bits, then every
  possible tenth byte: only `00`/`01` are accepted unsigned, only `00`/`7F`
  signed, a set bit 7 is overlong);
* **bounded decoders, every width 0..=64 (and widths above 64 rejected):** every one- and two-byte input
  equals a width-parameterised model (65 widths x 65,536 two-byte inputs per
  decoder); at 64 they equal the plain decoders on every input of the `Bytes`
  domain; the encoder's output decodes at width `n` exactly when the value fits
  `n` bits; every accepted value fits the width and consumes at most
  `max(1, ceil(n/7))` bytes;
* **known values:** the 624485 vector, `u32`/`u64`/`i32`/`i64` extremes,
  padded zero, `2^64` and `2^32`/`2^31` too large, an eleven-byte overlong
  input, truncation, empty input, offset out of range, width 0.

What the document cannot say, stated rather than approximated:

1. **One error case.** `Leb128Error` holds a `String`, so the binding maps the
   whole struct to `Leb128Error.MALFORMED`: the claims pin accept/reject, the
   value and `consumed` exactly, but not *which* rule rejected an input, and not
   the error's `offset` field.
2. **No byte indexing in evidence bodies.** `at`/`fixed` are W0701 in
   evidence generation, so "every byte but the last has bit 7 set" is stated by
   the canonical-encoder models, and long inputs are built from typed parts
   (`concat(cont9(v), be_bytes(x))`) rather than enumerated.
3. **Exhaustiveness stops at length 3.** All inputs of length 4..=10 are
   `2^80`; they are covered by the boundary claim above, the `Bytes` domain
   (constant fills to length 8, 347 strings), and property/fuzz suites.
4. **Widths above 64 are sampled, not enumerated.** The `rejects` row and the
   width claims state the rejection for every `u32`, but the exhaustive domain
   of a `U64` binder is `0..=255`, every single-bit value and `MAX ^ (1<<i)`;
   the suites bound it to 65..=2^32-1, and `u32::MAX` is a literal claim.
5. **No proofs.** The steps rows are declarations: the Rust-subset front end
   refuses `src/lib.rs` (the anonymous lifetime in
   `fmt::Formatter<'_>`), so `evidence/proof-manifest.json` records every
   steps-bound obligation `not-live` with the front end's message, and every
   heap/depth row `unchecked`. CWE-400 is therefore an unapproved exemption.
6. **Heap.** The encoders' `heap <= 64` rows are checked live: every encoder
   instance runs under the generated harness's allocation observer. The
   decoders allocate only their error `String` and have no `heap` row, because
   the observer runs each call on a worker thread, which the 2^24-case suites
   cannot afford.
7. **CWE-208** does not apply: every operand is labelled `public` (LEB128
   encodes public structure). The assurance report records it as "not
   applicable: no secret operands", an unapproved exemption backed by the
   derived attestation `warrant-spec::all-operands-public` (VER04h section 9),
   not as a constant-time claim.

### Measured evidence

Read from `code/packages/rust/wasm-leb128/evidence/test-manifest.json` (written
by the generated test target; 245 entries (219 before the SEC-006 fix), all `pass`), debug profile:

* 39 examples;
* 42 claims, each with an exhaustive, a property (4,096 cases) and a fuzz
  (5,000 iterations) suite: **46,565,930 exhaustive cases** in total, 0
  unreachable by type; the only discards are the 540 tuples per plain decoder that the
  refinement `o >= len(d)` of the offset-past-end claims excludes and, per
  bounded decoder, the tuples outside the width refinement (56,214 and 41,472);
* per operation (the report's `exhaustive.cases`): `encode_unsigned` 1,131,
  `encode_signed` 1,869, `decode_unsigned` 17,246,170, `decode_signed`
  17,417,140, `decode_unsigned_bounded` 5,941,815, `decode_signed_bounded`
  5,957,805;
* 80 negative controls (6 operations x their faults x example/exhaustive/
  property/fuzz), each catching its fault by a named claim (`caught_by`), plus
  20 BUILD-level sabotage runs (each fault against each operation it is bound
  to), each printing exactly one `not ok`;
* the encoders' heap rows: 1,714,606 observed instances, peak 26 bytes against
  the bound of 64;
* wall clock of the whole target on the authoring machine: about 140 s, of
  which the two 2^24-input claims take 26 s (`decode_unsigned_three_bytes`) and
  29 s (`decode_signed_three_bytes`), each two-byte-at-every-width claim about
  8 s, the fuzz suites together about 13 s and the property suites about 3 s.
  The `exhaustive_budget` is raised to 2^24 (from the default 400,000) for
  exactly those two three-byte claims, as gf256 does for its `U8^3` claims.

### Runtime capability evidence (VER08b)

`required_capabilities.json` declares `"capabilities": []`; the static analyzer
agrees (`effect_certificate.json`). `tests/runtime_capability.rs` adds the
run-time half: phase A builds the vectors unconstrained, phase B arms a
seccomp-bpf + Landlock jail derived mechanically from the manifest (minimal
baseline: allocator, `write` of TAP lines, `exit_group`) and calls every public
operation -- both encoders (boundary and extreme round trips, known vectors),
both plain decoders (padding, offset, trailing bytes, `2^64`, overlong,
unterminated, empty, offset out of range) and both bounded decoders (u32/i32
extremes, width 0, width 64 equal to the plain decoder, and `max_bits > 64`
rejected including `u32::MAX`, on `FF x9 7F` and `80 x10 01`). 12 named checks
pass. An independent `strace -f` observer sees only `write` and `exit_group`
after arming and agrees, and 18 negative controls (file, socket, process,
time, random, stdin, thread, executable mapping, io_uring, bpf, ...) are each
caught by both rings. The container ring is `NOT-LIVE` where no Docker daemon
is reachable (recorded in the evidence file, not counted as a pass). BUILD runs
the gate with `RCG_STRICT=1`. Known residual: vDSO time reads are invisible to
both rings (backlog CAP-RT-006).

Also checked by hand, not committed: mutating `src/lib.rs` so that the overlong
cap is off by one, the signed encoder ignores the sign bit of its last group, or
the signed padding check accepts anything, makes named claims fail
(`decode_unsigned_eleven_bytes_overlong` and
`decode_unsigned_bounded_two_bytes_every_width`;
`encode_signed_matches_canonical_model` and the signed round trips;
`decode_signed_ten_byte_boundary` and
`decode_signed_bounded_one_byte_every_width`).

The assurance report (`--profile varint-codec`) is `report-only`: rungs E0, E1,
E3 and E4 `pass`, E2 and E5 `not-live`; CWE-208 is the not-applicable exemption
above and CWE-400 the unapproved "no step-bound proof is live" exemption quoting
the front end's message; every other row is excluded as for gf256.
