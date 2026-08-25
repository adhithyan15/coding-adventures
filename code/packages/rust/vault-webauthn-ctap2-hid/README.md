# `coding_adventures_vault_webauthn_ctap2_hid` — VLT-PM51 slice 2

Real CTAP2-over-USB-HID hardware I/O for
[`coding_adventures_vault_auth`](../vault-auth)'s `WebAuthnPrfAuthenticator`.
Implements that crate's `Ctap2Transport` trait — "however we talk to a
physical FIDO2 authenticator" — using
[`ctap-hid-fido2`](https://crates.io/crates/ctap-hid-fido2) (built on
[`hidapi`](https://crates.io/crates/hidapi)) to enumerate USB HID
FIDO2 devices and perform a CTAP2 `GetAssertion` request with the
`hmac-secret` extension.

This is the **first package anywhere in the Vault stack** with a
native, hardware-touching, non-workspace dependency — every other
`vault-*` crate depends only on other workspace crates via `path =
"../…"`. See
[`VLT-PM51-hardware-security-keys.md`](../../../specs/VLT-PM51-hardware-security-keys.md)
for the dependency survey (why `ctap-hid-fido2` over
`webauthn-authenticator-rs` and the `yubikey` crate) and the full
design.

## Why this is a separate crate from `vault-auth`

`vault-auth` is the trust-sensitive KDF/authentication crate every
unlock factor in the Vault stack goes through
(`PasswordAuthenticator`, `TotpAuthenticator`). Giving it a native,
hardware-touching dependency would mean every consumer of those two
factors also inherits `ctap-hid-fido2`'s build and runtime footprint
whether or not it ever plugs in a hardware key. So `vault-auth` only
defines a trait (`Ctap2Transport`) — "however we talk to a physical
authenticator" — and this crate is the one real implementation of it.
This is the same protocol-crate/transport-crate split `VLT-PM48`
already uses for the local agent.

## What is real here, and what still refuses

`HidCtap2Transport::get_hmac_secret_assertion` really does:

- Fast, non-blocking device enumeration — with no device attached,
  this returns `Ctap2TransportError::NoDeviceAvailable` immediately.
  A vault with no hardware key configured never slows down because of
  this transport.
- A real CTAP2 `GetAssertion` request, with the `hmac-secret`
  extension, against a real device when one is present.
- A caller-controlled, bounded wait for the physical touch, via a
  worker thread raced against the request's `touch_timeout`.

What still refuses, one layer up in `vault-auth`:
`WebAuthnPrfAuthenticator::verify()` always returns
`AuthError::Unimplemented` as its *final* answer, because ECDSA P-256
assertion-signature verification doesn't exist anywhere in this
workspace yet. This crate's job is exactly "make the hardware I/O
real"; it does not attempt the missing cryptography.

## A real bug this crate's own tests found

Calling `ctap-hid-fido2`'s device-enumeration APIs from more than one
OS thread in a process is not safe — it reliably crashed this crate's
own test binary with `SIGTRAP` under the default parallel test runner
before a fix landed. `HidCtap2Transport` now serializes every native
HID call behind a process-wide `Mutex` (`HID_ACCESS_LOCK`), which
matters for production too: two concurrent `verify()` calls in one
process (plausible once this transport is wired into
`vault-pm-agent-host`) would hit the identical crash without it. See
the doc comment on `HID_ACCESS_LOCK` in `src/lib.rs` for the full
story, including the one honest trade-off it introduces (a new
attempt can block on the lock behind an earlier, abandoned attempt).

## Testing without physical hardware

- `ctap_hid_fido2::get_fidokey_devices()` and `FidoKeyHidFactory::create`
  are safe to call with no device attached, and this crate's tests
  call the *real* functions (no mocking) to exercise that fast-fail
  path for real.
- The request/response mapping logic (`build_hmac_secret_extension`,
  `map_assertion`, `classify_ctap_error`) is pure and is unit-tested
  directly against real `ctap-hid-fido2` types built by hand.
- What is **not** tested here: an actual physical touch. That gap is
  intentional and documented, not hidden — see
  `VLT-PM51-hardware-security-keys.md` for the full testability
  design (and why `vault-auth`'s own tests, against a fake
  `Ctap2Transport`, are where `WebAuthnPrfAuthenticator::verify()`'s
  logic is actually exercised end-to-end).

## Dependency footprint

`ctap-hid-fido2` v3.5.13 (MIT), ~35 transitive crates (CBOR via
`ciborium`, X.509 parsing for attestation via `x509-parser`, AES for
the PIN protocol, `ring` for crypto, `hidapi` for the native HID
layer). All unsafe code in this dependency chain lives inside
`hidapi`'s FFI bindings — `ctap-hid-fido2`'s own protocol-layer source
has zero `unsafe` blocks (verified by inspecting its published
source). This crate itself carries `#![deny(missing_docs)]` and has no
`unsafe` code of its own.

## Where it fits

```text
┌────────────────────────────────────────────┐
│  vault-pm-application / vault-pm-cli        │
└──────────────────┬───────────────────────────┘
                   │  constructs
                   ▼
┌────────────────────────────────────────────┐
│  vault-auth (VLT05)                         │
│  WebAuthnPrfAuthenticator + Ctap2Transport   │  trait boundary
└──────────────────┬───────────────────────────┘
                   │  implemented by
                   ▼
┌────────────────────────────────────────────┐
│  vault-webauthn-ctap2-hid            ◄      │  THIS CRATE
│  HidCtap2Transport (ctap-hid-fido2/hidapi)   │
└────────────────────────────────────────────┘
```

See [`VLT-PM51-hardware-security-keys.md`](../../../specs/VLT-PM51-hardware-security-keys.md)
and [`VLT05-vault-auth.md`](../../../specs/VLT05-vault-auth.md).
