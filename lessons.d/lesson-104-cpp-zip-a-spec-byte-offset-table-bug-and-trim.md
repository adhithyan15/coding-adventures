# Lesson 104 — `cpp/zip`: a spec byte-offset table bug, and "trim to declared size" silently defeats an aggregate decompression-bomb budget

**Date:** 2026-08-05

**What happened, part 1 (spec bug):** `code/specs/CMP09-zip.md`'s Local File
Header and Central Directory Header wire-format tables mis-sized
`Last_Mod_File_Time`/`Last_Mod_File_Date` as 4 bytes each instead of 2,
cascading a 4-byte offset error through every field after them (the table
claimed a 34-byte fixed Local Header with CRC-32 at offset 18; the correct,
universally-implemented layout — confirmed against `rust/zip` and
`python/zip`'s actual `struct.pack`/byte-offset code, both of which use
2-byte `mod_time`/`mod_date` fields — is a 30-byte fixed Local Header with
CRC-32 at offset 14, and a 46-byte fixed Central Directory Header). Every
implementation in the repo, including the reference Rust port, had always
written and read the CORRECT layout; only this one table's prose was wrong,
undetected because no port's tests round-trip against the written spec text
byte-by-byte — they round-trip against each other's code. Fixed the table
and its two downstream mentions (a corrupted-CRC-byte test-vector comment
still said "bytes 18–21" after the first pass fixed the table but not that
comment — caught in a later security-review round, not the same pass that
fixed the table; check EVERY prose reference to a byte offset, not just the
table itself, when correcting one).

**What happened, part 2 (security-review finding, the more important one):**
`ZipReader::read`'s original design silently TRIMMED an over-large
decompressed buffer down to the Central Directory's declared
`Uncompressed_Size` field before returning — matching the Rust reference's
own `if decompressed.len() > entry.size { decompressed.truncate(entry.size) }`
comment ("guards against a decompressor over-read"). `zip::unzip()`'s
aggregate decompression-bomb budget (a configurable total across every entry
it decompresses, on top of the existing per-entry `ca::deflate::inflate` cap)
then accounted the RETURNED (trimmed) size against that budget. Since
`Uncompressed_Size` is an attacker-controlled Central Directory field, a
crafted entry can declare `Uncompressed_Size = 0` while its real DEFLATE
stream still costs the FULL per-entry cap (256 MB) of genuine CPU/memory work
to decompress — the trim happens AFTER that work is done, so the caller-side
budget sees a 0-byte result and never grows, letting arbitrarily many such
entries each smuggle real decompression work past an "aggregate" cap that
never appeared to be approaching its limit. This is the identical bug class
independently found and fixed three separate times in `haskell/zip`
(Lessons in that package's own CHANGELOG — "the aggregate budget counted the
post-truncation size, not the actual decode work performed") and is a strong
signal that **every** `zip`/decompression-container port in this repo that
enforces an aggregate budget by trimming-then-measuring, rather than
rejecting a declared/actual size mismatch outright, is suspect until
individually audited.

**Rule:**
- A decompression-bomb budget that measures its input AFTER a lossy
  trim/truncate step is measuring the wrong thing. The trim itself must be
  replaced with a hard rejection (throw/error) whenever the actual
  decompressed size disagrees with a declared size sourced from untrusted
  input — for any HONESTLY-produced archive the two are always exactly
  equal by construction (a writer that compressed N bytes always declares
  `Uncompressed_Size = N`, and a correct DEFLATE stream decoding that entry
  always reproduces exactly N bytes), so rejecting a mismatch cannot break
  legitimate files, only crafted ones.
- Bounds-check arithmetic that combines an attacker-controlled field with a
  small constant (`local_offset + 6`, `local_offset + 26`) must be done in a
  width wide enough that the ADDITION ITSELF cannot wrap — not just the
  eventual `offset > buffer.size()` comparison. Doing the comparison safely
  in `uint64_t` while the offset argument was already computed via a
  narrower (`size_t`) addition one call-site up still leaves the wraparound
  where it always was, just one line earlier; the fix has to move with the
  addition, not just the check. In `cpp/zip` this meant changing
  `read_u16`/`read_u32`'s parameter type from `size_t` to `uint64_t`
  specifically so every call site is forced to widen the base value BEFORE
  adding to it, and having `ZipReader::read` widen `entry.local_offset` to a
  `uint64_t` exactly once rather than re-deriving a `size_t` copy at each of
  three call sites.
- The same "silent truncation into a structurally-misleading result" failure
  mode applies symmetrically on the WRITE side of any binary container
  format with fixed-width wire fields: an oversized single value (entry
  name/data past a 16-bit/32-bit field), a cumulative running total (Local
  Header offsets, Central Directory offset/size past 4 GiB), and a count
  field (more entries than a 16-bit count can represent) are three distinct
  places the same bug can hide, found across three separate review rounds
  on this package (`NameTooLong`/`DataTooLarge`, then `ArchiveTooLarge`, then
  `TooManyEntries`) — auditing only the first one found is not sufficient;
  grep every `static_cast<uintN_t>(...)` in a writer for the ones narrowing
  an unbounded (or merely uncapped) `size()`/count, not just the one the
  first pass happened to touch.

Fixed in `code/packages/cpp/zip/include/zip.hpp` (`ZipError::DeclaredSizeMismatch`,
`detail::require_fits_u32`, `read_u16`/`read_u32` widened to `uint64_t`
offsets, `NameTooLong`/`DataTooLarge`/`ArchiveTooLarge`/`TooManyEntries`).
Verified via dedicated regression tests for each finding
(`test_declared_size_mismatch_rejected`, `test_writer_name_too_long_rejected`,
`test_writer_too_many_entries_rejected`, `test_extreme_local_offset_rejected`)
plus the full 12-TC spec suite, real CLI-interop against the system
`zip`/`unzip`, and a real dynamic-Huffman fixture — 90 checks total, all
passing under GCC and Clang with `-pedantic-errors -Wall -Wextra -Werror`.
Every other `zip`/archive-container port in this repo that enforces an
aggregate decompression-bomb budget should be audited for the same
trim-then-measure pattern, not just `cpp/zip` and the three `haskell/zip`
rounds that found it independently first.
