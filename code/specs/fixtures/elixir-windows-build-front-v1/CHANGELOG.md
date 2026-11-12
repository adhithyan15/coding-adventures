# Changelog

## Unreleased

- Removed the `code/packages/elixir/gf256_native` exception: its NIF now builds and
  tests natively on Windows (OTP passes the NIF a callback table instead of an import
  library, which `erl-nif-bridge` now uses). Six exceptions remain: four direct NIF-link, one transitive NIF-dependency and the Metal one.

## 1.0.0 — 2026-08-27

- Established the pinned Windows runner and BEAM installer contract.
- Registered five direct NIF-link exceptions, one transitive NIF-dependency
  exception, and one Metal-only exception.
- Defined the closed `BUILD_TOOL_UNSUPPORTED` status protocol.
