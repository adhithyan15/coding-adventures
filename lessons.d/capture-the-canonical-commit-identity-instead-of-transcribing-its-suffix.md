---
category: Rust
---

# Capture the canonical commit identity instead of transcribing its suffix

An owner invocation passed a mistyped full commit suffix even though the
preceding `git rev-parse HEAD` had printed the canonical identity. The focused
Venn runner's HEAD assertion refused before its source freeze, receipt creation
or any Cargo command. That refusal is not a compiled regression or a test pass.

Capture the observed canonical identity programmatically and pass it through
the execution/receipt path; do not reconstruct or guess the unshown suffix of a
short commit. Keep the original refusal distinct from the corrected invocation
and continue requiring an exact HEAD match before validation.
