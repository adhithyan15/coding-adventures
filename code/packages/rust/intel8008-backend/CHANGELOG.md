# Changelog — intel8008-backend

## v0.1.0 — 2026-06-03 — Phase 6 of historical-arch backend migration

Initial release.  Minimal viable Backend trait impl over CIR.
Covers `const_*` + `ret_*` — enough to keep the existing lang-aot
Intel 8008 e2e smoke test passing byte-for-byte
(`[0x3E, 0x2A, 0x76]` for `const_i64 v=42; ret_i64 v`).

10 unit tests pin every byte sequence.
