# Changelog

## Unreleased

- Add `tests/skill_to_surface.rs`, pinning the SKILL.md-to-tool-surface
  contract: a document an author could write, through the signed manifest
  shape, into a registered and invocable tool, plus three negative cases
  (undeclared tool, above-tier tool, partially wired surface).
- Placed here rather than in `chief-of-staff-host-runtime` because this crate
  depends on both `skill-parser` and `host-runtime` in `[dependencies]`. The
  build tool ignores `[dev-dependencies]` and CI is diff-based, so as a
  host-runtime dev-dependency the test would not have run on a PR changing the
  parser -- precisely the PR it exists to catch.

## 0.1.0

- Build signed, SKILL-only Level 1 agent packages without overwriting paths.
- Load parsed skills from the exact authenticated package snapshot and require
  the signed manifest to equal the manifest derived from `SKILL.md`.
