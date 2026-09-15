# Lesson 95 — ZStd FHD Content_Checksum_Flag is bit 2, not bit 4 (repo-wide mistake in spec + Go + Rust)

**Date:** 2026-08-03

**What happened:** Rescuing `java/zstd` (CMP07) from a stale branch and running it against the real `zstd` CLI for TC-9 interop testing surfaced that `code/specs/CMP07-zstd.md`, `code/packages/go/zstd/zstd.go`, and `code/packages/rust/zstd/src/lib.rs` all document/parse the Frame Header Descriptor's `Content_Checksum_Flag` at **bit 4**. That is wrong. Verified empirically: `zstd -c file.txt` (checksum on by default) emits FHD byte `0x64`; `zstd -c --no-check file.txt` emits FHD byte `0x60` — the differing bit is **bit 2**, and the checksummed output is exactly 4 bytes longer (the trailing xxHash64). RFC 8878 §3.1.1.1 agrees: bit 4 is `Unused_bit`, bit 2 is `Content_Checksum_Flag`.

**Why it went unnoticed:** Go and Rust's decoders read the (wrong) bit 4, silently discard the value (`_ = (fhd >> 4) & 1` / `let _checksum_flag = ...`), and never actually skip checksum bytes or reject trailing data after the last block — so a real checksummed `.zst` frame still "worked" by accident (the trailing 4 bytes were just ignored, per the anti-pattern in Lesson 94). The bug only becomes fatal once a decoder correctly implements the Lesson-94 "reject trailing bytes after the last block" check without ALSO fixing the checksum-flag bit position — that combination throws a false "trailing data" error on every real-world checksummed frame, breaking TC-9 CLI interop in the CLI→ours direction.

**Rule:** `Content_Checksum_Flag` is FHD bit 2 (`(fhd >> 2) & 1`), not bit 4. When adding/verifying a Lesson-94-style trailing-bytes check to any ZStd port, the checksum-flag bit MUST be fixed first (or simultaneously), and the check must skip 4 bytes when it's set, before comparing final `pos` to `data.length`. Fixed in `java/zstd` (PR landing this lesson); Go and Rust still have the wrong bit documented/parsed as of this writing — low urgency there only because neither enforces the trailing-bytes check yet, but both should be corrected the next time either package is touched, to avoid the same trap resurfacing if the trailing-bytes check is ever added.

---
