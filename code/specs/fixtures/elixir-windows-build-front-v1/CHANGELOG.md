# Changelog

## Unreleased

- Removed the `code/packages/elixir/gf256_native` exception: its NIF now builds and
  tests natively on Windows (OTP passes the NIF a callback table instead of an import
  library, which `erl-nif-bridge` now uses). Four exceptions remain: two direct NIF-link (`conduit`, `irc-server-native`), one transitive NIF-dependency and the Metal one.
- Removed the `polynomial_native` and `paint_codec_png_native` exceptions: both NIFs now build and test natively on Windows through the same callback table.

## 1.0.0 — 2026-08-27

- Established the pinned Windows runner and BEAM installer contract.
- Registered five direct NIF-link exceptions, one transitive NIF-dependency
  exception, and one Metal-only exception.
- Defined the closed `BUILD_TOOL_UNSUPPORTED` status protocol.
