# Changelog

All notable changes to the `sir-conformance` crate will be documented in this file.

## [0.1.0] - 2026-07-02

### Added — cross-backend golden conformance harness

The first test that drives real Ruby **source** all the way to Python,
JavaScript, Go, and Rust and proves the four agree — closing the long-standing
gap where no single test exercised Ruby → {Go, Rust} end-to-end, and where a
frontend-emitted construct missing from a backend runtime (like the `case_eq`
builtin) could pass every per-backend unit test yet crash at runtime.

- **`Program` corpus + reference oracle.** Each program is Ruby source paired
  with the exact stdout a Ruby interpreter would produce; every backend is
  measured against that reference, never against another backend.
- **`run(program, target)` / `lower(program)`.** Public plumbing that lowers a
  program through `ruby_to_semantic_ir`, emits it via a backend, and runs it
  through the real toolchain (`python3` with the `sir-runtime-*` packages
  auto-discovered onto `PYTHONPATH`; `node`; `go run`; `rustc` + execute),
  returning normalised stdout. Reusable by future conformance suites (e.g. the
  SIR21 typed-integer matrix).
- **Graceful skip + hollow-pass guard.** A backend whose toolchain is absent is
  skipped (logged), not failed; the matrix test asserts at least one backend
  actually ran so a toolchain-less host cannot report a hollow pass. The Go
  toolchain is probed with `go version` (not `--version`, which errors).
- **Initial corpus (6 programs):** operator precedence, method-with-params
  implicit return, trailing-`if` return, trailing-`case` return (the `case_eq`
  regression oracle), string concatenation, and a `class`/`.new`/method call.
  Verified locally: 6 corpus × 4 backends = 24 runs, 0 skipped, all agree.

This is the first slice of
[`SIR21` §Provability](../../../specs/SIR21-type-system-and-integer-semantics.md)'s
conformance matrix (P1).
