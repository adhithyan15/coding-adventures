# CV04a — preserve resource controls when integrating edge curves

Status: bounded compatibility repair for the sole Closure/shared PR #17288.
Linked to provenance epic #15830 and lexical foundation #16970. Full local
Miri remains **DEFERRED/PENDING**; this does not establish Closure parity,
transformation lineage or output-byte mappings.

Main #17353 adds `SwimlaneEdgeCurve`, a required optional field in both edge
IR types. An actual isolated merge at `a6aa4ad14bd85823b4fcb4c9b2d3e2c85b19b9bd`
passed all seven indexed-target controls, then the complete fourteen-package
diagram suite failed with E0063 before executing any suite target: the existing
duplicate-gap fixture omits `curve`. Preserve this rejected receipt and its
compiler diagnostic. An independently inspected computed merge found the same
missing initializer. This is a fixture integration defect, not a runtime or UB
finding.

## Required behavior

Keep upstream curve parsing, portable cubic lowering and per-field optional
configuration updates. A curve-only directive preserves earlier animation
settings; explicit animate false remains false until a later animation setting
without an explicit animate flag enables it. Preserve authored order, duplicate
edge IDs, node-ID collisions, unnamed edges, class/style precedence and unknown
edge rejection. Curve configurations must use the existing shared ID index:
one E-edge discovery, D indexed lookups and M actual matching updates, with
those operation counts tested separately. Do not claim the whole pipeline is
linear.

The existing gap fixture must alternate Linear and Basis curves and assert each
curve after layout, while retaining every ID, class, style, animation, label,
kind, marker, min-length, spacing, four-direction operation count and extreme
usize-length assertion. Do not replace these controls with merely successful
parsing or a metadata-presence check.

## Acceptance and attribution

Run exact authored-source curve configuration controls at sizes32/128 and a
duplicate-ID/order/style case with a complete expected diagram. Retain all seven
existing edge-target tests. Run the metadata-derived affected diagram suites,
strict all-target lint with unknown-lint denial, reconstructed documents,
lesson validation and whitespace checks. Retained native/compiler/LANG/Mosaic
results require complete unchanged-source proofs and retain their actual
execution heads; do not label them fresh tests.

Obtain independent exact-final security review before updating the same PR.
Required fresh exact-head terminal checks and successful real Closurec native
execution on Linux/macOS/Windows remain mandatory before auto-merge. Verify
actual fetched-main merge reachability and complete payload identity afterward.
Frozen Miri inputs, receipts, run and target remain untouched; the full goal
and deferred final-source validation remain open.
