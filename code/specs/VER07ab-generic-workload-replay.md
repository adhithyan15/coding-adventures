# VER07ab — Generic workload metadata for report replay

VER07b defines the analysis in a paired report, and VER07d defines the
`indexed-array-sum-4096` example. This profile clarifies how a single report
from another producer workload is replayed without teaching the reader to
execute or interpret that workload.

## Report replay

Replay validates the existing closed report schema, checks pair counts,
durations, order balance, signed-64-bit fields, duplicate-key rejection, and
all existing input bounds, then recomputes analysis from the raw pairs. It
continues to reject unknown root and nested fields and forged analysis.

The optional `workload` metadata value is a producer identifier. When present,
it must be a JSON string that is not empty, is not composed only of ASCII
whitespace (`U+0020`, `U+0009` through `U+000D`), and contains no `U+0000`.
JSON strings must be valid UTF-8. Any other valid identifier is accepted and
preserved verbatim; replay does not execute the producer's workload or infer
its semantics from this field. Omitting the field remains valid for reports
without producer metadata.

This rule applies to one report's analysis replay. A separate comparison tool
that requires matching workloads must keep and apply its own workload-match
guard; acceptance for replay does not make unlike experiments comparable.

## Producer behavior and tests

The indexed-array executable remains unchanged: it still constructs and runs
`indexed-array-sum-4096` and emits that identifier. The generic rule only
changes what a report reader accepts.

Each implementation tests round-trip replay of both the array workload and a
full metadata report with another nonblank workload identifier, and rejects
blank, NUL-containing, and non-string workload values. Tests also retain
duplicate-key, invalid UTF-8, malformed-schema, bound, pair-order, and forged
analysis rejection. A held generic report is replayed unchanged by the native
readers; no metadata is removed or relabeled for this check.
