# VER07y — Portable core-report metadata compatibility

VER07b defines paired-report analysis and VER07d defines the reference
collector. A core report is the portable observation and analysis record; it
may be produced without command-line metadata, including by guest collectors
whose boundary details live in a separate sidecar.

## Portable core shape

Every core report contains exactly these required members:

- `config`: the six VER07d configuration fields.
- `batch_size`: the common positive signed-64-bit operation count.
- `pairs`: the configured number of ordered positive-duration observations.
- `analysis`: the recomputed ratio, interval (or `null` when unavailable), and
  verdict.

The four-member shape is valid on its own. It must replay to the analysis
derived from its raw pairs and configuration. Readers continue to reject
unknown members, duplicate members at every object depth, malformed values,
unbalanced pair order, and serialized analysis that differs from recomputation.

## Optional driver metadata

A producer may attach the standard driver metadata group: `workload`,
`invocation`, `revision`, `runtime`, `os`, and `arch`. When any member of this
group is present, all six are required. The five descriptive values are
non-empty UTF-8 strings; `workload` is an identifier selected by the producer,
not a lane-specific constant imposed by the reader. A present workload ID must
not be empty or consist only of ASCII whitespace (U+0020 or U+0009 through
U+000D); other Unicode whitespace alone is a valid producer identifier.
`invocation` is an array
of UTF-8 strings and may be empty when the driver was run with no explicit
flags. `profile` is an optional non-empty UTF-8 string and is valid only
alongside the complete metadata group. `null`, wrong types, empty required
strings, invalid UTF-8, and NUL characters in strings are invalid.

These rules preserve strict schemas while allowing readers to accept valid
core reports from all producers. Guest-envelope reports remain ordinary core
reports: host/runtime boundary probes belong in the sidecar and are not
required in the nested report.

## C reader validation

The C reader must accept both the minimal four-member core shape and complete
standard metadata, including generic workload identifiers. It must reject
partial metadata groups, `null` metadata, unknown or duplicate members,
malformed values, unbalanced pair order, and forged analyses. Parsing and
allocation failures must release partial report state and leave the output
zeroed. Existing native C reports retain their full metadata; the C CLI's
indexed-array workload identity remains unchanged.

Validation uses minimal reports, all four retained guest reports, complete
native reports, a generic workload identifier, metadata mutations, and
allocator accounting. GCC, Clang, and clang-cl suites must pass, with the
native forty-pair producer/replay and installed-consumer checks intact.
