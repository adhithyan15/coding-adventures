# UI66: File Values and Multipart Submission

File inputs cross a host boundary because only the host can display a trusted
picker. Venture keeps that boundary explicit: activation emits a
`ControlFilePickerRequest` containing normalized `accept` filters and the
`multiple` policy. A host returns `HostFileSelection` values with an opaque
identifier, a display name, an optional media type, and bounded bytes. It never
returns a filesystem path.

`BrowserControlModel` filters each result using case-insensitive extensions,
exact media types, and media ranges. It caps item count, per-file bytes,
aggregate retained bytes, and diagnostics; rejected files never enter retained
state. Layout and paint see
only sanitized leaf names. `ControlFileState` exposes names, sizes, media types,
an accessible summary, and diagnostics without copying payload bytes into the
accessibility tree.

Reset clears file selections. Successful-control collection preserves source
order and excludes disabled or unnamed controls. GET and URL-encoded POST use
sanitized filenames. Multipart POST emits text and file parts in source order,
normalizes line endings for text controls, supplies a safe media type fallback,
escapes disposition parameters, derives a repeatable boundary from the data,
and retries if that boundary occurs in a value or payload. Planning fails
before navigation when the complete body exceeds the shared byte limit.

The contract is shared by native and web shells. Platform adapters own picker
presentation and byte acquisition only; filtering, retained state,
accessibility, reset, serialization, diagnostics, and transport stay in the
reusable browser crates.
