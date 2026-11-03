# VER04a — bounded Warrant source admission (P006b1)

Parent: [VER04 §2](VER04-warrant-language.md#2-source-and-identity).
This slice supplies source admission and normalized/original byte mapping in
`rust/warrant-grammar`. It does not tokenize, parse, validate layout/types,
resolve paths, bind a source revision, produce evidence or authorize publication.
The significant-layout range prerequisite and guarded parser remain P006b2/b3.

## Admission contract

`source::SourceDocument::admit(bytes: &[u8], limits: SourceLimits)` returns an
immutable document borrowing the exact original UTF-8 buffer. Configuration
validation and source/scan-work admission precede UTF-8 validation and scans.
Reject invalid UTF-8, a leading UTF-8 BOM, any NUL and any CR not followed by LF.
The latter makes the parent rule's LF/CRLF line endings explicit; bare CR does
not silently disappear. Interior U+FEFF and other valid Unicode scalars retain
their bytes for later lexical checking. Empty input is admissible source data,
without implying that it is a syntactically valid Warrant document.

Normalize each CRLF to one LF and preserve every other byte. `original()`
returns the original UTF-8 string and `normalized()` the distinct lexing buffer.
With no CRLF, the normalized view borrows the original buffer. Otherwise use a
fallibly reserved String. Neither accessor reads a path or performs I/O.
`is_normalized_borrowed()` exposes this ownership distinction for callers/tests.
Do not expose mutable buffer/index access or derive byte ranges from token values.

## Coordinates and range relation

`original_span(start, end)` accepts half-open byte boundaries in the normalized
buffer and returns `Option<OriginalSpan>`. Out-of-bounds, reversed and positions
inside a UTF-8 scalar return None. Empty ranges, including normalized EOF, are
valid. An OriginalSpan has start/end OriginalPosition records: zero-based raw
`byte_offset`, one-based `line` and one-based `byte_column`. Coordinates count
UTF-8 bytes, not scalars or UTF-16 code units. A newline is LF or CRLF; Unicode
line/paragraph separators are preserved ordinary scalars in this source domain.

The normalized LF replacing CRLF maps to the original two-byte interval from
CR through LF. Its start is on the preceding original line; its end is column1
of the next line. There is no normalized point inside that collapsed pair.
A final newline leaves a trailing empty line, and EOF maps to its column1.

Keep one private record `(original_start, normalized_start)` for the initial
line and each LF's following boundary. For a normalized boundary b, binary
search the last record with normalized_start<=b, then compute:

```
byte_offset = original_start + (b - normalized_start)
line = record_index + 1
byte_column = b - normalized_start + 1
```

Thus each non-newline scalar maps to its exact raw spelling, while CRLF maps
to two original bytes. Query cost is O(log line_count), with no source prefix
rescans or additional allocation. This source-bound construction is distinct
from authentication of detached metadata. Public coordinate records alone do
not prove source identity; later frontend results must retain the actual buffer
and pair spans with their repository-relative path/revision identity.

## Budgets and failure policy

SourceLimits exposes `max_source_bytes`, `max_owned_bytes` and `max_scan_work`.
Defaults and hard ceilings are 1MiB original bytes, 64MiB owned buffer capacity
and 3MiB scan work. Callers may lower any cap, including to zero; a higher
configuration rejects even for empty input. This is the source-stage profile;
VER04's later evaluation/checking/generation work limits remain separate.

Before scanning, reserve `3 * bytes.len()` work units with checked arithmetic:
one byte unit for UTF-8 validation, one for structural preflight and one for
index construction/normalization. Reserving this conservative upper bound is
deliberate, including LF input's cheaper borrowed path. The preflight counts
line starts and CRLF removals, rejects forbidden bytes and computes required
index/string buffer capacity. Account one two-usize line record per initial
line and LF, plus normalized String capacity only when CRLF exists.

Check requested count, byte-size multiplication and combined capacity before
fallible exact reservations. Check actual exposed Vec/String capacities after
reservation and reject if allocator slack crosses the configured owned-byte
cap. `owned_buffer_bytes()` reports the successful document's charged capacity;
allocator bookkeeping and the caller's borrowed buffer are not charged again.
Never return a successful partial document after a scan/reservation failure.
No recursion, callbacks, new external dependencies or unsafe code are needed.

Every admission error has stable code `W0001`, a closed SourceErrorKind and an
optional half-open interval of original raw byte offsets. Encoding/BOM/NUL/CR
errors identify the offending raw interval (UTF-8 error length, or the remaining
truncated suffix); configuration/work/memory/allocation errors have no source
interval. Error Display/Debug must contain no source bytes. Document Debug must
also omit both buffers; callers explicitly request source through accessors.
Failure is source-admission rejection, never a successful parse/checked status.

## Acceptance and boundaries

Authored cases must cover empty/LF/CRLF/mixed endings, trailing and blank lines,
escaped literal spelling, astral/non-ASCII bytes, original raw slices, normalized
EOF and invalid query boundaries. Assert LF input actually borrows the original
buffer. For all scalar boundaries in mixed sources, verify monotonic mapping
and each mapped raw slice against independently normalized source segments.
Test invalid UTF-8, BOM, NUL, bare CR, each lowered/excess/zero cap, exact budget
boundaries and fixed nonreproducing diagnostics/Debug.

Deliberately removing CRLF mapping corrections and bypassing forbidden-byte
admission must cause assertion failures after actual recompilation. Restore and
rebuild the final source before final tests/coverage. Run package tests, strict
Clippy/fmt and unchanged exact grammar regeneration. Measure the new source
module's executable-line coverage and identify unexercised allocation refusals.
README/changelog/capability declarations describe the additive source API.

No tests here establish indentation-token origin capture, complete hostile
parser safety, semantic depth/type checking or VER04 conformance. The existing
shared CV04a range API deliberately refuses indentation/layout; do not relabel
that refusal as successful source capture. Shared lexer changes need a separate
specification, consumer audit and regeneration of affected committed artifacts.
