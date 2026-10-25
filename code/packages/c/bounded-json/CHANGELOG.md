# Changelog

## 0.1.0 — 2026-10-25

- Repair a confirmed CI coverage gap: dependency-aware bounded-json selection
  now retains the existing Windows/MSVC job and executes the shared harness and
  package front, with failures owned by the required build roll-up.

- NN47: independent pure C17 owned JSON parser with explicit statuses/const views.
- Enforce 4 MiB/128-depth/65536-node/16 MiB allocation ceilings, tighter options,
  checked sizes, injectable allocators and complete partial-parse cleanup.
- Validate strict number/UTF-8/escape grammar; retain ordered duplicate names,
  embedded-NUL lengths and exact int64 lexical metadata without coercion.
- Convert numbers with the host's rounded strtod after translating JSON's point
  to the current locale spelling, without changing the caller's locale.
- Add strict multi-compiler BUILD fronts, CMake static-library install/export,
  native boundary/fault/fixture probes and sanitizer/coverage evidence.
- Repair confirmed shared Windows ISO-harness scalar-count/UTF-8 diagnostic
  compatibility without replacing its scheduler or weakening strict flags.
- This is parsing only, not C dense training or six-native conformance.
