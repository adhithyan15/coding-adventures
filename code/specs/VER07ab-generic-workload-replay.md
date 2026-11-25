# VER07ab — Generic workload metadata for report replay

VER07b defines the analysis in a paired report, and VER07d defines the
`indexed-array-sum-4096` example. This profile clarifies how a single report
from another producer workload is replayed without teaching the reader to
execute or interpret that workload.

## Report replay

Replay validates the existing closed report schema, checks pair counts,
durations, order balance, signed-64-bit fields, and all existing input bounds,
then recomputes analysis from the raw pairs. It continues to reject unknown
root and nested fields and forged analysis. Preserve each lane's existing
duplicate-key validation path. The raw duplicate-aware ingress change tracked
as MB12 owns Rust's duplicate-key guarantee; this profile must not replace or
reimplement that parser work.

The optional `workload` metadata value is a producer identifier. When present,
it must be a JSON string that is not empty, is not composed only of ASCII
whitespace (`U+0020`, `U+0009` through `U+000D`), and contains no `U+0000`.
JSON strings must be valid UTF-8. Any other valid identifier is accepted and
preserved verbatim; replay does not execute the producer's workload or infer
its semantics from this field. Byte-oriented input adapters reject invalid
UTF-8 before decoding; text-only APIs reject malformed Unicode escapes while
parsing, including lone UTF-16 surrogate escapes. Valid surrogate pairs,
U+FFFD, and literal text that resembles an escape remain valid. Readers may
project producer metadata out of a core analysis result if that is their
existing API, but must validate a present workload field before projecting it
out. The input report must remain unchanged. Omitting the field remains valid
for reports without producer metadata.

This rule applies to one report's analysis replay. A separate comparison tool
that requires matching workloads must keep and apply its own workload-match
guard; acceptance for replay does not make unlike experiments comparable.

## Producer behavior and tests

The indexed-array executable remains unchanged: it still constructs and runs
`indexed-array-sum-4096` and emits that identifier. The generic rule only
changes what a report reader accepts.

Each implementation tests replay acceptance of both the array workload and a
full metadata report with another nonblank workload identifier, and rejects
blank, NUL-containing, and non-string workload values. Tests also retain
encoding/Unicode (including valid and malformed surrogate boundaries),
malformed-schema, bound, pair-order, and forged-analysis rejection at the
applicable input boundary. The MB12 strict-ingress tests
continue to cover duplicate keys. A held generic report is replayed unchanged
by the native readers; no metadata is removed or relabeled before replay.
