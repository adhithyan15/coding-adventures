---
category: Security boundaries
---

# Decompressors must not size allocations from untrusted header fields

**What went wrong.** The LZ77/LZ78/LZSS/LZW family (and Huffman, Brotli) ports read
a big-endian `u32` count or length from the first bytes of the input and then used
it to size a buffer or drive a loop. `lz77::decompress(&[0xFF; 4])` in Rust aborted
the whole process (`Vec::with_capacity(4 billion)` = 17 GB, an *abort*, not a
catchable panic); the Go port died with `fatal error: runtime: out of memory` for
the same four bytes; the F# port looped ~4.3 billion times. Four bytes of input,
process dead: CWE-789 / CWE-400. A sweep of all ~60 ports found the same family of
defects in different clothes:

- preallocating from the header (`with_capacity`, `make(.., count)`, `new List(count)`);
- looping over the header count instead of the tokens present;
- reading the `u32` into a signed `int` (Java/Kotlin/C#/F#): >= 2^31 goes negative,
  which either threw (`new ArrayList<>(-1)`), silently decoded nothing, or meant
  "no limit" (`originalLength < 0`);
- expanding every token *before* truncating to the declared length (LZ78 chains are
  quadratic: a 100 KB stream can ask for ~10 GB of output);
- unchecked `output.len() - offset` on a back-reference (wraps in C/C++, traps in
  Swift, OOB read / NULL deref, confirmed under ASan in C lz77);
- Java/Kotlin decode copying the whole output (`toByteArray()`) per copied byte
  (quadratic time).

**The fix (the same everywhere).**

1. A header count is a *claim*, not a size. Clamp it to what the remaining input can
   physically hold (`(len - header) / token_size`) *before* allocating or looping.
   A larger claim means "truncated": decode the tokens that are present.
2. Capacity hints derived from a declared length must be bounded by an input-derived
   quantity (tokens x 4, payload bits) or dropped; grow while decoding.
3. Stop decoding as soon as the declared length is reached; later tokens could only
   be truncated away, and expanding them first is the amplification.
4. Read length fields as unsigned (or widen to 64-bit) before comparing/clamping.
5. Validate every back-reference (`offset == 0 || offset > output.len()`) and every
   dictionary index before using it as an index, in every language, including the
   ones that "only" trap or throw (a trap is still a remote process kill).
6. Do the hot copy on a plain growable array; never re-materialise the buffer per byte.

**Prove it with a test that fails loudly, not by OOM-killing CI.** Hostile-header
tests ship with each package: header values of `0xFFFFFFFF`, counts larger than the
remaining bytes, lengths inconsistent with the tokens, plus round trips over *every*
byte string of length <= 2 and a seeded random sample (so the clamp can't change
behaviour for valid streams). In Rust the test binary installs a `#[global_allocator]`
that refuses any single allocation over 32 MiB and prints `REGRESSION: ...` before
aborting, so a regression fails immediately and cheaply instead of taking the
machine down; in other languages use a time bound, `tracemalloc`/`TotalAlloc`, or
run under ASan/UBSan.

**Do differently.** When writing a decoder, treat *every* integer read from the
stream as hostile and write the clamp and the "stop at declared length" check
before the loop, not after a fuzzer finds it. When porting a decoder to a new
language, port the hostile-header tests with it; the bug travels with the algorithm,
not with the language. Known follow-up: the `decompress` functions return plain
values with no error channel in several ports, so "truncated header" currently
degrades to "decode what is present"; surfacing it as an error is an API change
tracked separately.
