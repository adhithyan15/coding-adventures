# Changelog

## 0.1.0 — 2026-10-26

- Add the bounded checked ZipCPU revision-3 no-CIS functional profile from the
  public revision-3 specification and immutable open-source evidence commit.
- Model both register banks, both retained PCs, canonical CC fields including
  per-bank software-preserved bits 22:16, sampled interrupt input, 64 KiB
  big-endian memory, installed-image metadata and complete restorable state.
- Implement the admitted integer, shift, multiply/divide, memory, conditional,
  banked MOV, LDI, BREAK and NOOP families with exact transaction traces.
- Add typed transactional load, restore, step, run and execute operations plus
  named deterministic seams at every real retained allocation boundary.
- Check in independently reproducible 1,954-row C++ and 2,016-row Verilated
  RTL corpora, source-lock their hashes, and compare complete projected state
  and ordered instruction-fetch transactions for all 3,970 rows.
- Add 31 completion-contract tests, two corpus-consumer tests and one doctest.
  Strict checks pass with 92.67% release line coverage (1,265/1,365).
