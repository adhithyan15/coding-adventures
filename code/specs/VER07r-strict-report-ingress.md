# VER07r — Reject duplicate members at microbenchmark JSON ingress

`VER07b` defines paired-report analysis and `VER07d` defines the raw report
shape. A JSON object can contain the same member name more than once; common
decoders silently keep one value. The Rust, TypeScript, and Ruby raw-report
entry points must reject duplicate object member names before replay so one
text document cannot produce different claims in readers with different
duplicate-key behavior.

## Scope

This contract applies to the text ingress APIs:

- Rust `Report::from_json`.
- TypeScript `replayJson`.
- Ruby `replay_json`.

It applies recursively to every JSON object in the input, including report
metadata, configuration, each pair, and analysis. It does not change the
object-based APIs (`replayReport` and `replay_report`), because their callers
may already have lost duplicate members while parsing. It does not add new
metadata, field, numeric, size, or report-schema restrictions; each reader
keeps its existing validation rules after duplicate detection.

## Member-name equality

Member names are compared after JSON string escape decoding. Thus `"pairs"`
and `"pa\u0069rs"` are duplicate names in the same object. The comparison is
exact on the decoded string; it does not apply Unicode normalization. Reusing
the same name in a different nested object is valid. Repeating a string value
in any number of fields is valid.

## Reader behavior

Raw-report ingress rejects a document when any object contains a duplicate
decoded member name. It continues to reject malformed JSON according to its
existing JSON parser. On accepted input, the current signed-64-bit integer
handling, schema checks, metadata acceptance, and analysis replay stay
unchanged. In particular, integer tokens at both signed-64-bit endpoints
remain exact, and valid reports with 10,000 pairs remain accepted.

The implementation should preserve each language's established parser:

- Rust uses Serde's deserializer with a recursive value visitor that tracks
  decoded keys per object before constructing the existing `serde_json::Value`
  consumed by `Report::from_json`.
- TypeScript performs a linear string-aware preflight over object scopes to
  compare decoded key tokens, then uses its existing Node JSON parser and
  lossless-integer reviver. The preflight is not a replacement JSON parser.
- Ruby supplies a duplicate-aware object class to its existing JSON parser so
  the parser rejects a repeated key as each object is built. It also explicitly
  disables the parser's duplicate-key allowance: some JSON gem versions warn
  and continue by default, and the object-class hook alone does not reliably
  observe duplicate assignments in those versions. The same rejection tests
  must pass with the pinned CI JSON gem and the locally bundled JSON gem.

No reader uses substring search for raw key spelling: escape aliases must be
detected, while ordinary repeated values and keys in separate objects must
remain valid.

## Validation

Each text-ingress suite tests duplicate names in the configuration, a pair,
the analysis object, and top-level metadata; it also tests an escaped alias
and malformed JSON. Valid controls include the shared paired-observation
fixture, repeated string values, a repeated name in distinct object scopes,
signed-64-bit minimum and maximum values, and a valid full-size 10,000-pair
report. The existing native 40-pair report and cross-language replay checks
must remain successful. No speed threshold is part of these checks.

## Implementation provenance

The duplicate-aware ingress design and initial Rust, TypeScript, and Ruby
implementations were established in the #20097 work at commits
`68a951bd31` and `250723acaa`. Ruby parser-version compatibility was clarified
in `d8ea1bd96b` and implemented in `8e908523df`. This contract is being
selectively incorporated into the newer benchmark stack; it does not authorize
replacing newer metadata validation, input bounds, signed-number handling, or
object-based APIs with an older snapshot.
