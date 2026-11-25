# VER07t — Paired report replay in the benchmark runner

Add `benchmark-tool paired-replay REPORT [--out DIRECTORY]` to consume a
VER07d native or guest core report. This is the exact paired sign-interval
path; the runner's existing independent-result bootstrap comparison remains
a separate operation with its own interpretation.

The command reads at most 8 MiB of UTF-8 JSON, rejects duplicate decoded object
keys, excessive nesting (16 containers), unknown fields, missing required fields,
null required values and integer tokens outside signed 64-bit bounds. It accepts
1 through 10,000 pairs, positive durations, valid config and batch bounds, and
the balanced baseline-first/candidate-first schedule counts. Standard CLI
metadata is optional; each present field has its documented string or string
array type. When `workload` is present, its string must be nonblank and contain
no U+0000, matching the generic single-report replay contract in VER07ab.
Reject escaped lone UTF-16 surrogates in workload strings; preserve valid
surrogate pairs, U+FFFD, and literal backslash-u text.
An invocation array may be empty when a producer records arguments without the
executable name; null and non-string entries remain invalid.
Integer-form finite `minimum_factor` numbers are valid.

Recompute binary64 ratios through the Go microbenchmark analysis API. Stored
ratio and interval endpoints must agree within 1e-12 relative error, with
1e-15 absolute tolerance, and the verdict and interval presence must agree
exactly. Reject any inconsistency before writing successful output.

Successful stdout is a JSON summary identifying the input SHA-256, pair count,
configuration and recomputed analysis. Optional export creates a new directory
containing the byte-identical input as `paired-report.json`, the summary as
`paired-summary.json`, and a readable `paired-report.md`. Refuse to overwrite an
existing destination. Stage the files beside the destination, reserve the final
directory with an exclusive mkdir, and move the summary last as the completion
record. A rename that can replace an existing empty directory is insufficient.
Remove an incomplete newly reserved directory on error; never report it as success.
The raw report's existing metadata is retained rather than invented.

Validation covers signed extrema and above-2^53 integers, duplicate escaped
keys at each report level, malformed/oversized/deep inputs, missing/null/unknown
fields, incorrect schedules and stored analyses, integer factors, output errors,
byte-identical export, all portable fixture cases, and independently produced
native reports. Fuzz the bounded ingress with malformed bytes. Existing runner
commands and their tests continue to pass. Add transitive dependencies to both
BUILD variants and run vet and coverage.

The runner's fixture tests make it an additional exact fixture consumer.
Register its discovered program identity `go/programs/benchmark-tool` in
selection, Kiln export and fallback hashing, alongside the twenty-four package
readers in VER07s. Keep the program's actual `code/programs/go/benchmark-tool`
path in graph-export tests and verify fixture-only Go selection includes it.
