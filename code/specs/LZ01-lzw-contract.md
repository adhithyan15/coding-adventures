# LZ01 — LZW Compression: the byte-API contract of `rust/lzw`

## Overview

LZW (Lempel-Ziv-Welch, 1984) is [CMP03](CMP03-lzw.md): LZ78 with the dictionary
pre-seeded with the 256 single-byte strings, so every symbol of the output is a
dictionary code, packed at 9 to 16 bits. This document is the **contract** of the
Rust package `code/packages/rust/lzw`: what its two functions promise, what a
hostile caller can and cannot make them do, the findings made while writing it
down, and exactly what is and is not verified. It is the prose companion of the
Warrant document [`lzw.warrant`](lzw.warrant), which is the executable
specification: every example and claim there is compiled into a test harness and
run against the real exports (VER04h).

LZW is the first **decompressor** in the Warrant-derived set. A decompressor takes
input that is hostile by definition, and its output size is chosen by the sender,
so its contract is mostly about bounds.

## Layer position

A leaf: no dependencies. Nothing else in the repository depends on it today
(`grep` of every `Cargo.toml`); the other `lz*` crates and ports implement the
same CMP family independently.

## The format

See CMP03 for the algorithm and test vectors. What the contract needs from it:

```text
bytes 0..3    original_length   big-endian u32
bytes 4..     codes, LSB-first, 9 bits growing to 16
              CLEAR(256)  data codes ...  STOP(257)  zero padding to a byte
```

* Codes 0..=255 are the single bytes; 256 is CLEAR (reset the dictionary and the
  width), 257 is STOP; entries are added from 258. The width grows by one bit each
  time `next_code` passes a power of two, up to 16, and the dictionary resets
  (CLEAR) when it holds 65,536 entries.
* The decoder resolves the **tricky token**: a code equal to the entry about to be
  built stands for "the previous string plus its own first byte".
* A string of length `L` needs an entry chain of `L - 1` earlier codes, so the
  longest entry is 65,279 bytes and no single code stands for more.

## Rust contract

```rust
pub const MAX_OUTPUT_LEN: usize = 16 * 1024 * 1024;
pub fn compress(data: &[u8]) -> Vec<u8>;
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String>;
```

**`compress`** is total: it accepts any byte string and returns the CMP03 stream.
It is linear: one dictionary lookup per input byte, a dictionary of at most
65,536 entries keyed by `(prefix code, byte)`. It writes `original_length` as a
**saturating** `u32`: an input of 4 GiB or more cannot be represented, and the
header is `0xFFFFFFFF`, which `decompress` then rejects (it is far above the cap).
Its output is at most `3 * len + 16` bytes (a code is at most 16 bits per input
byte, plus one CLEAR per 65,278 codes, the header, STOP and padding). An input
longer than `MAX_OUTPUT_LEN` makes a stream `decompress` rejects: `compress` has no
error channel, so that is the fail-closed behaviour, not a feature.

**`decompress`** takes untrusted bytes and returns `Ok(output)` or `Err(message)`.
It never panics, always terminates, and:

1. rejects any input shorter than 4 bytes (no header);
2. rejects a payload whose first code is not CLEAR, including a payload too short
   to hold one 9-bit code;
3. rejects an invalid code: neither a known entry nor the next entry to be built,
   and a tricky token with no previous string;
4. stops as soon as `original_length` bytes have been delivered, **mid-entry if
   need be**: the header is an **upper bound**, never an allocation size and never
   a promise, so a stream that ends earlier (with or without STOP) is accepted
   with what it produced (see Limitations);
5. never delivers more than `MAX_OUTPUT_LEN` (16 MiB) bytes: it returns `Err`
   *before* writing the entry that would pass the limit. Only delivered bytes
   count, so a stream that expands to 31 MB behind a header declaring 16 MiB is
   delivered as the 16 MiB it was allowed;
6. holds a dictionary of at most 65,536 eight-byte links (512 KiB) however the
   stream is built, and writes each delivered byte once.

The error is a `String`. The contract is "an `Err`"; the message text is not part
of it (the Warrant binding maps every `Err` to the single case `CORRUPT`).

`MAX_OUTPUT_LEN` is a public constant so that callers and this document name the
same number. It is a constant and not a parameter: see "Findings" and "Cross-language
expectations".

## Findings

The conversion found two defects and recorded two limitations. Each followed the
finding protocol (VER27 section 7.4, P12): a backlog row, a test that failed before
the fix, the fix, a Warrant claim that pins it, a lesson.

### SEC-002 — decompression bomb (CWE-409), contained without an API change

*What.* Writing the `output` and `heap` rows of `lzw.warrant` had no honest value.
The largest ratio an LZW decoder can reach is 32,768 output bytes per input byte
(a 16-bit code standing for a 65,279-byte string), so a ratio bound allows 2 GB
from 64 KB and says nothing; and the second row, `heap`, exposed that the
dictionary was a `Vec<Vec<u8>>`. Each entry in a chain is one byte longer than the
one before it, so `k` codes held `k^2 / 2` bytes of dictionary beside the output:
a 12 MiB output had a 41 MB peak. A 15 KB stream of tricky tokens expands to about
40 MB and the header cannot stop it, because the attacker writes the header.
SEC-001 had hardened the header handling but not the stream.

*Fix.* Entries are `(prefix code, first byte, last byte, length)` links, 8 bytes each,
and a string is written once, into the output, by walking the chain from its end
(`expand`). Delivered output is capped at `MAX_OUTPUT_LEN`. Peak live heap for the
12 MiB stream went from 40,998,792 to 16,842,752 bytes. No API change.

*Pinned by.* `tests/decompression_cap.rs` and `tests/decoder_memory.rs` (written first,
failing at the previous commit); in `lzw.warrant` the `output` and `heap` rows
(every instance observed), and a **real bomb built from a 279-byte unit** doubled
ten times (285 KB, 31 MB of output): 512 units (15,681,536 bytes, under the cap)
decode, 1,024 units are rejected whatever length they declare (`0xFFFFFFFF`, one
byte over the cap, 1 GiB), declaring exactly the cap delivers the cap, and declaring
1,024 bytes delivers them. Before the fix the bomb examples failed in the generated
harness (see "Measured evidence").

*Still open.* A `max_output` parameter is the better API (the caller states what it
can afford) and is a breaking change across every port and the `lz77`/`lz78`/`lzss`
conversions (backlog LP-003, owner question 6). This fix is the absolute backstop
and does not replace it. The cap is Rust-only until then: the other lanes of
SEC-001 still deliver what the header declares.

### SEC-002a — quadratic encoder and a header that wraps

*What.* `compress` kept its dictionary as `HashMap<Vec<u8>, u16>` and asked whether
`w + [b]` is known for every input byte, which hashes the whole current prefix. On
redundant input the prefix grows by a byte per emitted code, so the cost is the
sum of the prefix lengths: 8 MiB of zeros took about 35 s in a debug build and
16 MiB minutes, with no error channel to bound it. The header was written
`data.len() as u32`, so an input of 4 GiB + 10 bytes got the header 10 and
`decompress` returned ten bytes as a success.

*Fix.* The dictionary is keyed by `(prefix code, byte)`; the header saturates.
The output is byte for byte what the old algorithm produced.

*Pinned by.* `tests/encoder_cost.rs` (8 MiB of zeros under 10 s; the old encoder is
kept as a reference and compared byte for byte on structured and random inputs,
including one that fills the dictionary and resets); `header_length` unit tests (an
input over 4 GiB cannot be built in a test, so the rule is a function); the Warrant
claims `compress_one_byte_matches_the_model` and `compress_two_bytes_match_the_model`
and the known-answer examples pin the wire image.

### Recorded, not fixed: a truncated stream is accepted

`decompress` returns `Ok` with what it produced when the stream ends before
`original_length` bytes (a truncated download) or without a STOP code. SEC-001 made
the header an upper bound on purpose (`max_declared_length_on_small_stream`), so
this is documented behaviour, but it means a truncated file is not detected. The
Warrant claim `decompress_two_byte_payload_model` pins it for the shortest case (a
CLEAR code and nothing else is `Ok(b"")` behind any header) so that it changes only
on purpose. Changing it is an API decision (an `Err` on a short result breaks the
SEC-001 tests and every caller that relies on the upper-bound reading) and belongs
with LP-003.

### Recorded: the proof front end refuses the source (P11)

The Rust-subset front end refuses `src/lib.rs` at `struct BitReader<'a>` (a
lifetime parameter), and each export alone at the next construct. Per the P11
principle this is a design question before it is a tool gap: a bit reader that owns
its data (an index into a slice passed to `read`) would lower. It was not reshaped
here, because the goal of this conversion is the contract and the bounds, and
nothing would be proved by the reshaping alone (the decoder loop has data-dependent
indexing and a growing `Vec`). Every steps-bound obligation is recorded `not-live`
with the front end's message.

## Cross-language expectations

The other ports of CMP03 implement the same wire format. `compress` output is
byte-identical across ports (the old encoder, kept as a test reference, is the
spec's transcription). The 16 MiB cap, the linked dictionary and the saturating
header are **Rust-only** at this commit: the ports keep the SEC-001 behaviour (no
allocation from the header, stop at the declared length) and remain bombs until
LP-003 decides the API (`max_output`, an error channel). A port that adopts the
cap should adopt the same number, and the bomb of `lzw.warrant` (hex literal in the
document) is a ready conformance vector: it must be rejected at every declared
length above 16 MiB and delivered in full for 512 units.

## Limitations (what the document does not say)

1. **One error case.** `Err(String)` carries only a message, so the binding maps it
   to `LzwError.CORRUPT`: claims separate accept from reject and the delivered
   bytes exactly, but not which rule rejected an input.
2. **Truncation is accepted** (above): a claim pins it for one shape, nothing says it
   is desirable.
3. **No byte indexing in evidence bodies**, and the Rust code generator emits
   neither `take`, `drop`, `slice` nor `replicate` yet. Streams are therefore built
   from hex literals and `concat`; "the first four bytes are the length" is stated
   by modelling the whole stream of one- and two-byte inputs.
4. **Exhaustiveness stops at length 3 for `decompress` and length 2 for the wire image
   of `compress`**, and the round trip is enumerated over the generator's `Bytes`
   domain (the strings of length 0..=3 and the constant fills of length 4..=8),
   256 byte runs of length 16 and of length 1,024 (`x` repeated), and every byte
   pair repeated eight times. Longer inputs are property and fuzz sampled
   (4,096 cases and 5,000 iterations per claim), plus the hand-written seeded
   tests (`decoder_equivalence`, `encoder_cost`).
5. **The bomb in the document is 285 KB**, the largest the oracle budget builds. The
   128 KB / 2 GB chain of the finding lives in `tests/decompression_cap.rs`.
6. **Bounds are observed, not proved.** The `output` and `heap` rows are checked on
   every instance the suites and examples run, under an allocation observer that
   charges only the calling thread's allocations through the global allocator
   (VER04j section 14.2, item 50): it is a guard against the common bomb, not a
   sandbox. The `steps` and `depth` rows are declarations.
7. **CWE-208 does not apply**: every operand is labelled `public`.

## Verification status (Warrant-derived)

`code/specs/lzw.warrant` (Warrant v1) is the specification of the two operations;
`tests/generated/warrant_evidence.rs` is generated from it and `evidence.bindings`, and
`BUILD` fails when it drifts. The document states:

* **round trip:** `decompress(compress(d)) == Ok(d)` for every string of the generator's
  `Bytes` domain (the strings of length 0..=3 and the constant fills of length 4..=8,
  347 of them), after any prefix, for every byte repeated 16 and 1,024 times (the
  tricky token), and for every byte pair repeated eight times (65,536);
* **`compress` wire image:** every one-byte input (256) and every two-byte input
  (65,536) equals an independent model of the stream (header, CLEAR, STOP, 9-bit LSB-first
  packing, padding); at least 7 bytes for every input; `output <= 3 * len + 16`; seven
  known vectors from an independent implementation of CMP03 (empty, one byte, two
  bytes, ABABAB, AAAAAAA, TOBEORNOTTOBEORTOBEORNOT, and 512 bytes that grow the width
  to 10 bits);
* **hostile `decompress`:** every input of length 0..=3 is rejected (1 + 256 + 65,536 +
  16,777,216 = **16,843,009 inputs**, enumerated); a header alone, a header and one
  payload byte, a payload that does not start with CLEAR, and a first data code of
  258 or above (a tricky token with no previous string, or an unknown code) are
  rejected; every two-byte payload behind a zero header equals a model (65,536); a
  CLEAR code and nothing else is `Ok(b"")` behind any header (the recorded leniency);
* **the declared length is an upper bound:** behind every `u32` header the stream of
  `AAAAAAA` delivers the first `min(header, 7)` bytes, and any header from `0xFF000000` up
  delivers the whole stream;
* **bounds:** `output <= imin(16777216, 32768 * len)` and a `heap` row for `decompress`,
  `output <= 3 * len + 16` and a `heap` row for `compress`, each checked on every
  instance by the allocation observer, and each `output` row as a generated
  exhaustive, property and fuzz suite (`<op>/output-bound`);
* **the bomb:** 512 units (142,848 bytes, 15,681,536 bytes of output) decode; 1,024 units
  (285,696 bytes, 31,363,072 bytes of output) are rejected declaring `0xFFFFFFFF`, 16 MiB + 1
  or 1 GiB, deliver exactly 16 MiB declaring 16 MiB, and deliver 1,024 bytes declaring
  1,024.

What the document cannot say is the list under "Limitations" above.

### Measured evidence

Read from `code/packages/rust/lzw/evidence/test-manifest.json` (written by the
generated test target; 116 entries, all `pass`), debug profile:

* 27 examples;
* 23 claims, each with an exhaustive, a property (4,096 cases) and a fuzz (5,000
  iterations) suite: **17,106,176 exhaustive + 336,475 sampled + 121,862 bounded-set cases (17,564,513 total)**, 0 unreachable by type;
* per operation (the report's `exhaustive.cases`): `compress` 66,486 exhaustive,
  16,384 property and 20,000 fuzz cases; `decompress` 17,498,027, 77,824 and 95,000;
* 20 negative controls (2 operations x their faults x example/exhaustive/property/
  fuzz), each catching its fault by a named claim (`caught_by`), plus 22 BUILD-level
  sabotage runs (a fault against the claim that pins it), each printing `not ok`;
  the first run of the controls found that `flip-result-bit` on `compress` was caught
  by nothing (the decoder reads the header as an upper bound, so a corrupted header
  survives a round trip): that is why the two wire-image claims exist;
* the observer: 18,006,235 observed instances; peak 58,327 bytes for `compress` (bound
  4,200,448 at that instance) and 33,554,496 bytes for `decompress` (the example that
  delivers exactly 16 MiB; bound 70,348,824), against the observer's 64 MiB cap;
* **what SEC-002 looks like in the evidence.** The same generated harness run against
  the previous `src/lib.rs` (kept out of the repository; run once to measure): 113 of 116
  entries pass and exactly three fail, `bomb_1024_units_is_rejected` and the two
  declaring one byte over the cap and 1 GiB, because `decompress` returned `Ok` with
  31,363,072 bytes; the observed peak of those instances was 64,917,568 bytes for a
  31,363,072-byte output (2.1x, within 4 percent of the observer's 67,108,864-byte cap),
  against 16,810,080 after the fix (rejected before the cap is passed). The `<op>/output-bound` suites pass in both
  runs: they sample small inputs and cannot reach a 15 KB bomb, which is why the bomb is
  an *example* in the document and why the discharge of CWE-409 says "checked on the
  enumerated and sampled inputs and the document's bomb examples, not proved";
* wall clock of the whole target on the authoring machine (debug profile, 4 shared
  cores): about 5 minutes. The 2^24-input claim `decompress_three_bytes_rejected` is the
  large one; every instance runs under the observer (a worker thread per test
  thread, one channel round trip per call). The `exhaustive_budget` is 2^24 (the default is
  400,000) for exactly that claim, as wasm-leb128 does.

### Runtime capability evidence (VER08b)

`required_capabilities.json` declares `"capabilities": []`; the static analyzer agrees
(`effect_certificate.json`). `tests/runtime_capability.rs` runs both operations in the
armed phase of a seccomp-bpf + Landlock jail derived from the manifest (minimal baseline:
allocator, `write` of TAP lines, `exit_group`): round trips of vectors, boundaries, runs,
width growth and a 300 KB input that fills the dictionary and resets it; the known wire
bytes; every length-0..3 input, a header alone, a missing CLEAR, an invalid code and a
tricky token with no previous string; a 4 GiB declared length on a small stream; a 40 MB
chain bomb rejected whatever it declares; a bomb stopped by a short declared length and
by exactly the cap; and the largest chain under the cap decoded in full. 10 named
checks pass. Enforcement: landlock ABI 7 and seccomp both enforced, child exit 0.
Observation: `strace -f -v` sees `write` x12, `brk` x8 and `exit_group` x1 after arming,
0 violations; 18 negative controls (file, socket, process, time, random, stdin, thread,
executable mapping, io_uring, bpf, ...) are each caught by both rings. The container ring
is `NOT-LIVE` where no Docker daemon is reachable. BUILD_linux runs the gate with
`RCG_STRICT=1`.

One finding about the gate itself: the first run failed with `RCG001 mremap`. glibc grows
a block above its mmap threshold with `mremap`, the baseline allows `mmap` and `munmap`
but not `mremap` ("nothing is here just in case"), and a decompressor's output `Vec`
grows to 16 MiB by design. The test warms the allocator in the unconstrained phase (frees
one 31 MiB block, which makes glibc raise its mmap threshold to that size) so the armed
phase is served from `brk`, which the baseline allows; the baseline is untouched. Whether
`mremap` belongs in the baseline (it can move or resize the process's own anonymous
mappings and cannot make one executable) is a VER08b decision, recorded here for the
owner. The workaround depends on glibc; on another libc the gate fails loudly.

### The assurance report

`assurance_report.json` (`--profile lossless-compression`) is `report-only`: rungs E0,
E1, E3 and E4 `pass`, E2 and E5 `not-live`; CWE-208 is the not-applicable exemption
(every operand public); CWE-400 the unapproved "no step-bound proof is live" exemption
quoting the front end's message; **CWE-409 is `discharged`** (bounded), citing the six
`output-bound` suites, with the observed peak and its bound in the reason, and never the
exclusion every other package prints (VER04h section 9 item 21). `--check` re-derives it
from the document and the manifest and refuses a report that excludes the class, cites a
different suite set, or discharges on evidence the manifest does not hold. The remaining
rows are excluded as for the other adopters; eight rows rest on `unverified::` static
attestations (nine for wasm-leb128: the CWE-409 one is gone).
