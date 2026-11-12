# Changelog

## Unreleased

- Removed the `gf256_native`, `polynomial_native`, `paint_codec_png_native`, `irc-server-native`
  and `conduit` NIF-link exceptions and the `conduit-hello` NIF-dependency exception: every
  Elixir NIF now builds and tests natively on Windows (OTP passes the NIF a callback table
  instead of an import library, which `erl-nif-bridge` uses). Only the Metal exception
  (`paint_vm_metal_native`, Apple-only) remains.

## 1.0.0 — 2026-08-27

- Established the pinned Windows runner and BEAM installer contract.
- Registered five direct NIF-link exceptions, one transitive NIF-dependency
  exception, and one Metal-only exception.
- Defined the closed `BUILD_TOOL_UNSUPPORTED` status protocol.
