# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of the VLT08 trait crate
  (`code/specs/VLT08-vault-dynamic-secrets.md`).
- `SecretEngine` trait — `mount_path` / `mint` / `revoke` /
  `rotate_root`. `Send + Sync`, object-safe, `&self` everywhere.
- `Role` — name + optional `default_ttl_ms` / `max_ttl_ms`,
  with a `Role::new(name)` constructor for the common case.
- `MintContext` — `principal` (audit breadcrumb) +
  caller-supplied `now_ms` + `requested_ttl_ms` plus
  *engine-specific* per-call input: `path: Option<String>`,
  `input: Option<Zeroizing<Vec<u8>>>`, `cas_token: Option<u64>`.
  Engines read whatever they need; callers omit the rest.
  `Clone` and `Debug` are intentionally *not* derived because
  `input` carries plaintext under `Zeroizing` — cloning would
  duplicate plaintext into a non-zeroizing intermediate, and
  `Debug` would let `dbg!` leak the bytes. A
  `MintContext::simple(principal, now_ms, ttl)` constructor
  covers the common "no per-engine input" case.
- `MintedSecret` — body under `Zeroizing<Vec<u8>>`,
  `secret_ref` revocation handle, `granted_ttl_ms`.
  Hand-rolled `Debug` redacts the body.
- `MintedSecret::into_lease_payload` — the canonical bridge
  between VLT08 (engines) and VLT07 (leases). Every engine mints
  bytes, every caller wraps them in a lease.
- `SecretRef` — `#[non_exhaustive]` enum: `KvV2 { path, version }`,
  `DbUsername`, `PkiSerial`, `AwsSession`, `Other`. New variants
  can land non-breakingly as engines arrive.
- `EngineError` — `UnknownRole` / `InvalidParameter` /
  `Backend` / `Crypto` / `PrincipalDenied` / `UnknownSecret` /
  `Conflict`. Implements `Display` + `std::error::Error`.
- 6 unit tests: object-safety smoke, `Role::new` defaults,
  `EngineError` `Display` output, `MintedSecret` redacted Debug,
  `into_lease_payload` round-trip, `SecretRef` variant
  distinctness.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.
