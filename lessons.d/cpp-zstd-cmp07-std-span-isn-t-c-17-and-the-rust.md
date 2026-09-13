# `cpp/zstd` (CMP07): std::span isn't C++17, and the Rust reference itself skips a spec-required check

**Date:** 2026-08-05

**What happened:** Implementing the first C++ port of `zstd` (CMP07), two
small but worth-recording gotchas surfaced while translating the corrected
`code/packages/rust/zstd` reference into pure ISO C++17:

1. The task brief's suggested public API signature used
   `std::span<const std::uint8_t>` as the parameter type. `std::span` is a
   **C++20** addition — this repo's `iso-harness` compiles everything with
   `-std=c++17 -pedantic-errors`, so `std::span` doesn't exist under that
   standard and would fail to compile (not just warn). Used
   `const std::vector<std::uint8_t>&` instead, matching `cpp/lzss`'s
   existing convention. When a task description's example code and a repo's
   actual pinned language standard disagree, the pinned standard wins —
   check `ISO_CXXSTD`/the harness's `-std=` flag before assuming a
   "reasonable-sounding" modern-C++ type is actually available.
2. `code/specs/CMP07-zstd.md`'s Security Considerations and `lessons.md`
   Lesson 94 both require a ZStd decoder to reject trailing bytes after the
   frame's end (past an optional content checksum) rather than silently
   ignoring them. The `code/packages/rust/zstd` reference this port was
   translated from — despite being the corrected, post-audit reference for
   the FSE codec bugs (Lessons 95-97) — does **not** actually implement this
   check (its `decompress()` just returns after optionally skipping the
   checksum bytes, with no `pos == data.len()` assertion). The C++ port
   implements the check anyway, since both the spec and Lesson 94
   explicitly require it and it doesn't affect interop with the real `zstd`
   CLI's own output (a single CLI invocation always produces exactly one
   frame with nothing trailing). **Rule:** a reference implementation
   flagged as "already corrected" for one specific bug class is not
   automatically correct against every requirement in the spec/lessons.md —
   cross-check the actual requirements text, not just the reference's
   current behavior, especially for checks that are cheap to add and
   explicitly called out as security-relevant.

**Verification that the FSE bug class itself (Lessons 95-97) was avoided:**
implemented directly against the corrected reference rather than
re-deriving the algorithm from the RFC text or memory, then verified via
real `zstd` CLI interop (TC-9, both directions) plus a high-sequence-count
regression test at the `Number_of_Sequences` 1-byte/2-byte wire-encoding
boundary — 115 checks passed under both g++ and clang++ with no `zstd` CLI
skip (the binary was available in the dev environment), confirming actual
RFC 8878 wire-format conformance rather than just internal self-consistency.

---
