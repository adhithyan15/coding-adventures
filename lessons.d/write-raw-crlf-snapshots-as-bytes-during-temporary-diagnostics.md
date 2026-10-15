---
category: Rust
---

# Write raw CRLF snapshots as bytes during temporary diagnostics

A temporary trace decoded raw CRLF bytes and passed them to write_text on
Windows, which translated each LF again and produced CRCRLF. Rust rejected the
diagnostic before tests. The finally block restored the exact original bytes.
Use write_bytes for raw snapshots, or normalize before text translation. Keep
the failed diagnostic distinct from the actual trace and accepted source runs.
