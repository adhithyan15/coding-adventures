# Changelog

## Unreleased

- Let the repository gate accept a lane whose package tree moved after its
  verified revision only through dependency lockfiles, so Dependabot and
  repo-wide advisory sweeps no longer fail the pinned `package_tree` check.
  Source, test, manifest, and capability changes still require re-verification.
- Bind Ruby and Swift adoption evidence to PR #16361's durable squash-merge
  revision so fresh clones can verify the recorded package trees.
- Expand the adoption registry to all twelve established implementations with
  exact canonical roots for Elixir, Lua, Ruby, and Swift.
- Preserve Elixir, Lua, Ruby, and Swift as truthful pending adoptions until
  their complete native conformance evidence is published.

## 1.0.0 - 2026-09-30

- Define the bounded `barcode-layout-1d-v1` corpus and closed JSON schemas.
- Add 56 generated vectors spanning expansion, layout, scene projection,
  limits, validation precedence, metadata integrity, and text rejection.
- Register the eight existing implementation lanes as pending adoption without
  claiming conformance.
- Add a strict loader and byte-for-byte generator drift test.
