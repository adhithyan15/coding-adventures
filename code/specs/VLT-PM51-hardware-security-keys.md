# VLT-PM51 — Hardware Security Key Support (YubiKey and Others)

## Status

Design spec for a user-requested feature: unlocking (and, later,
custody of) a `vault-pm` vault with a hardware security key — "I want
to be able to require things like a Yubikey" (`VLT00-vault-roadmap.md`
"Auth flexibility" section, quoting the user directly). `VLT-PM00`
§6's reuse-map table has carried this as an open closure since Phase
1A: `authentication | vault-auth | password and TOTP factors |
WebAuthn/FIDO2-PRF and replay state where enabled` and `custody |
vault-key-custody | trait, passphrase custodian, selection policy |
real OS keychain/TPM/Secure Enclave providers`. This document makes
the two closures concrete, picks a protocol and a dependency, and
ships the first slice: a real, reviewable `WebAuthnPrfAuthenticator`
**scaffold** in `vault-auth`, matching the exact shape
`vault-key-custody::TpmCustodian` already established for "the
capability and the trait plumbing are real; the hardware call fails
closed until a follow-up PR."

Real CTAP2/WebAuthn hardware I/O and ECDSA P-256 assertion-signature
verification are explicitly deferred — §6 and §7 say why — as is a
hardware-backed *custody* provider (`YubikeyPrfCustodian`) — §8 says
why that's a separate, larger PR rather than part of this one.

**Slice 2 (this update, §11–§18) ships the real CTAP2 hardware
transport §6 deferred.** `ctap-hid-fido2` — recommended but
deliberately not added in slice 1 — is re-verified (§11) and added as
this workspace's first native, hardware-touching dependency, isolated
into its own new crate, `vault-webauthn-ctap2-hid` (§12), so
`vault-auth` itself gains only a trait boundary (`Ctap2Transport`),
never the native dependency. `WebAuthnPrfAuthenticator::verify()` now
performs a real CTAP2 `GetAssertion` with the `hmac-secret` extension
and checks everything about the response that doesn't need ECDSA
P-256 (§13). ECDSA P-256 signature verification is still the one
missing primitive (§7 is unchanged on this point), so `verify()`
still always returns `AuthError::Unimplemented` as its final answer —
now reached only *after* a real hardware round trip, not instead of
one. §14 covers the touch-timeout and failure-mode design, §15 the
CI/testability approach (including a real concurrency bug this
slice's own tests found and fixed), §16 what's still deferred, and
§17 slice 2's acceptance gates.

## 1. What "hardware security key support" means here

A FIDO2/CTAP2-compliant hardware authenticator — a YubiKey 5-series,
a SoloKey, a Feitian key, or any other device implementing the CTAP2
`hmac-secret` extension — can participate in a vault's unlock in two
structurally different ways, and this product needs to be precise
about which one it means before writing a line of code:

1. **Unlock factor** (`vault-auth`, VLT05). The key proves possession
   (a signature over a server/app-chosen challenge) and, if it
   implements the `hmac-secret` extension, additionally returns a
   secret derived from a per-credential HMAC key sealed inside the
   authenticator. That secret can be folded into the vault's unlock
   key derivation alongside the passphrase, exactly the way
   `TotpAuthenticator` folds in a gate-mode 2FA check today and the
   way `PasswordAuthenticator`'s Argon2id tag *is* a bind-mode
   contribution today. This is additive: it widens `combine_key_
   contributions`'s input set, it does not replace anything.
2. **Custody provider** (`vault-key-custody`, VLT03). The key itself
   *is* the thing holding the wrapping key for the vault's master
   KEK — the same role `TpmCustodian` and `PassphraseCustodian`
   already play behind the `KeyCustodian` trait. `wrap`/`unwrap`
   cross the hardware boundary on every vault open.

Both closures are named separately in `VLT-PM00` §6 because they are
genuinely separate pieces of work with separate risk profiles (§8
covers why custody is harder and out of scope for this PR). This
document designs both, at different levels of implementation depth:
factor (1) gets a real, tested scaffold in this PR; provider (2) gets
a design and an explicit deferral.

## 2. Additive, not required — confirming the reasoning holds here

Before treating "hardware key as an unlock requirement" as a design
target at all, it's worth checking whether it's the right shape for
*this* product, because the alternative — a vault that refuses to open
without a physical key present — is a real, common design elsewhere
(a YubiKey-only KeePassXC database with no password fallback is a
supported configuration) and this product has consistently chosen the
opposite default everywhere it has faced the same fork:

- `VLT-PM48-local-agent-ipc.md` §1.1: "The agent is optional; one-shot
  operation always remains." A background agent socket makes commands
  *faster*; its absence never makes them impossible.
- `VLT-PM43-cli-passphrase-rotation.md` and every recovery-flow
  document in this campaign treat the passphrase as the one path that
  is always authoritative — rotation, recovery, and verified restore
  all assume it, never a hardware factor, because the passphrase is
  the one credential a user can always reproduce from memory with no
  device dependency.
- `vault-key-custody`'s own TPM-first policy (§4 below) already drew
  this line once, in the *opposite* direction, for a *different*
  reason: hardware-preferred defaults are fine, hardware-*required*
  defaults are not, unless the caller explicitly, affirmatively opts
  out of software (`force_software`). Nothing in that policy makes
  hardware mandatory; it makes hardware preferred when present and
  still requires an explicit ask to bypass it.

A hardware security key that could brick a vault the moment it's
lost, without any documented recovery path, is a materially different
risk than "the agent socket isn't running today" or "TPM detection
returned nothing" — it is the same shape of risk `VLT-PM00` names for
the passphrase itself and treats as unacceptable without a real
recovery story to go with it (recovery keys, VLT04 recipients). This
spec finds no argument in this product's existing design language
for making a hardware key mandatory, and several arguments against
it. **A hardware key is therefore always an additional, optional
bind-mode factor layered on top of the passphrase, composed through
the existing `combine_key_contributions` mechanism — never a
replacement for it, and never something a vault can be configured to
require without an equally real software-only recovery path already
existing first.** VLT06 policy (already named for exactly this
purpose in `VLT00-vault-roadmap.md`'s "Auth flexibility" diagram —
`require all_of { password, any_of { webauthn_prf, sms_otp } }`) is
where a future PR would let an operator layer a hardware-preferred
*policy* on top without touching this authenticator at all.

## 3. Protocol selection: FIDO2/CTAP2 `hmac-secret`, not YubiKey HMAC-SHA1 challenge-response

Two real protocols were compared, because both exist in the wild and
both show up in the user's phrasing ("YubiKey and others"):

| | FIDO2/CTAP2 `hmac-secret` (WebAuthn `prf`) | YubiKey HMAC-SHA1 challenge-response |
|---|---|---|
| Standard vs. vendor | Open standard (FIDO Alliance CTAP2); any compliant authenticator | Yubico-proprietary OTP-application mode |
| Vendor support | YubiKey 5-series, SoloKey, Feitian, Google Titan (most), many others | YubiKey only (and only the 4/5-series OTP slots) |
| What it authenticates | A full WebAuthn/CTAP2 credential — registration binds an rpId, produces a resident or non-resident credential, and every assertion is signed | A raw HMAC-SHA1(secret, challenge) with no signature, no rpId binding, no replay-resistant assertion structure |
| Where it's already used | Bitwarden, 1Password ("Secret Key" flow uses a similar bind-mode shape), browsers' WebAuthn `prf` API | KeePassXC's "YubiKey Challenge-Response" unlock mode |
| Rust ecosystem | `ctap-hid-fido2` (active — see §5), `webauthn-authenticator-rs` (kanidm, broader but heavier) | `yubico`/`yubico-manager` crates exist but are thin and much less actively maintained than the FIDO2 crates surveyed |
| Fits "and others" | Yes — this is the entire point of picking a standard | No — inherently YubiKey-only by construction |

The recommendation is **FIDO2/CTAP2 with the `hmac-secret` extension**
(browser-facing name: WebAuthn `prf`), for reasons that all point the
same direction:

- It is the only one of the two that is a real *standard* rather than
  one vendor's OTP-application quirk, which is what "YubiKey and
  others" in the user's own request calls for directly.
- It is what Bitwarden and 1Password already ship for exactly this
  use case — `VLT00-vault-roadmap.md`'s reuse-map and authenticator
  table already named `WebAuthnPrfAuthenticator` (bind-mode) as the
  target type before this spec existed, which this document treats
  as strong prior evidence the earlier design work already converged
  on the right answer.
- HMAC-SHA1 challenge-response has no signature and no rpId binding —
  it's a bare shared-secret HMAC over a challenge the caller supplies
  in the clear over USB, with no cryptographic binding to "this
  application" versus "any other application asking this same
  YubiKey for a challenge-response." `hmac-secret` closes exactly
  that gap: the secret returned is derived per-credential, and the
  credential itself is bound to an rpId at registration.

YubiKey HMAC-SHA1 challenge-response is **not** part of this design.
It would only ever cover one vendor, provides strictly weaker binding
than the standards-based alternative, and the only concrete case for
it (KDBX-format compatibility) is not a `vault-pm` goal — `VLT-PM49`
§8 already defers KDBX *import* for unrelated reasons, and even were
it shipped, KDBX's challenge-response unlock mode would live inside
that adapter's own decode path, not as a `vault-auth` factor. If a
concrete need for it surfaces later, it is a vendor-specific
`Authenticator` impl exactly as narrow in scope as this one, added
independently.

## 4. Bind mode, not gate mode

`hmac-secret` is the detail that decides which of `vault-auth`'s two
existing modes this factor uses. A CTAP2 assertion without the
extension is gate-mode — it proves "the right physical key is
present," contributes nothing to key derivation, and is
indistinguishable in kind from `TotpAuthenticator`. With the
extension, the authenticator additionally returns
`HMAC-SHA256(per-credential-secret, salt)`, where the per-credential
secret never leaves the authenticator and is different for every
credential (so it isn't a device-wide fingerprint). That output is
exactly the "does this factor widen the KDF input set" property
`vault-auth`'s module doc already uses to define `Mode::Bind`
(`PasswordAuthenticator`'s Argon2id tag is the standing example). A
plain-signature WebAuthn factor with no `hmac-secret` support would be
gate-mode and is out of scope for this PR — `WebAuthnAuthenticator`
(gate) and `WebAuthnPrfAuthenticator` (bind) are listed as two
separate rows in `VLT00-vault-roadmap.md`'s authenticator table for
exactly this reason, and this PR ships only the bind-mode one because
that is the one the user's request ("require a Yubikey" as a
meaningful factor, not just a checkbox) actually calls for.

## 5. Dependency selection

`vault-pm` is a CLI product; talking to a physical FIDO2 authenticator
means real USB HID I/O, which is a materially different dependency
shape than anything this workspace has taken on for the Vault stack
so far — every existing `vault-*` crate depends only on other
workspace crates via `path = "../…"`, plus the occasional tiny
crates.io leaf like `serde_json` used elsewhere in this workspace.
Native, hardware-touching dependencies were compared on maintenance
health and footprint before picking one:

| Crate | Latest / last publish (as surveyed) | Shape | Notes |
|---|---|---|---|
| `ctap-hid-fido2` | 3.5.13 / 2026-08-12 | Direct CTAP2-over-HID: enumerate devices, `make_credential`, `get_assertion`, extensions via a `Gext` enum that includes `hmac-secret` (`create_hmac_secret_from_string`) | Actively maintained (published days before this survey); built on `hidapi` (cross-platform: Windows/macOS/Linux) plus `ciborium` (CBOR), `x509-parser`, `ring`/`aws-lc-rs` (attestation). README documents Linux needs `libusb-1.0-0-dev`/`libudev-dev` at build time; Windows needs admin rights at run time for raw HID access |
| `webauthn-authenticator-rs` (kanidm) | 0.5.5 stable / 0.6.1-dev | Full WebAuthn *client* — multiple transports (USB HID, NFC, BLE), platform authenticator support, used for interop testing of the `webauthn-rs` server stack | From a well-known, credible WebAuthn org (kanidm/`webauthn-rs`), but broader scope than needed here (this product only ever needs "get an assertion from a USB CTAP2 key," not NFC/BLE/platform authenticators or server-side registration ceremonies) and the dev-tagged latest version signals more API churn in flight |
| `yubikey` (iqlusioninc) | 0.9.0-pre.0 | YubiKey **PIV** application (smartcard-style RSA/ECC sign/decrypt) | Wrong protocol entirely — PIV is a certificate/smartcard application, not FIDO2/CTAP2, and is YubiKey-specific by construction; ruled out by §3, not by dependency weight |
| `yubico`/`yubico-manager` | — | YubiKey HMAC-SHA1 challenge-response | Ruled out by §3 (protocol choice), not evaluated further here |

**Recommendation: `ctap-hid-fido2`**, for the follow-up PR that adds
real hardware I/O — narrower scope than `webauthn-authenticator-rs`
(this product needs exactly "enumerate a USB CTAP2 device, request an
assertion with the `hmac-secret` extension," nothing about NFC/BLE/
platform authenticators), confirmed `hmac-secret` support at the API
level, and freshly active maintenance. Its `hidapi` dependency's
Linux build requirement (`libusb-1.0-0-dev`, `libudev-dev`) is a real,
new addition to this repository's CI matrix — `.github/workflows/
ci.yml` already installs per-package system libraries conditionally
(`libcairo2-dev`, `libgtk-3-dev`, etc.), so the pattern for adding one
more exists, but it is new CI wiring across three OS runners
(Linux/macOS/Windows, per the existing `runs-on: ${{ matrix.os }}`
job) that this PR does not attempt — see §6.

**This PR adds no new dependency.** `Cargo.toml` for `vault-auth` is
unchanged. `ctap-hid-fido2` is the recommendation for the PR that
wires real hardware transport, not a dependency of the scaffold
shipped here — see §6 for why keeping the two separate is deliberate,
not merely deferred-by-default.

> **Slice 2 update:** the above describes slice 1's shipped state.
> §11 re-verifies this recommendation before adding it for real, and
> §12 explains why the dependency lands in a *new* crate
> (`vault-webauthn-ctap2-hid`) rather than in `vault-auth` — the
> "keep the two separate" reasoning below still holds, just aimed at
> a different kind of separation (crate boundary, not PR boundary).

## 6. What this PR ships: the `WebAuthnPrfAuthenticator` scaffold

`code/packages/rust/vault-auth/src/lib.rs` gains a new `Authenticator`
implementation:

```rust
pub struct WebAuthnPrfAuthenticator {
    relying_party_id: String,   // e.g. "vault-pm"; bound into authData as SHA-256(rpId)
    credential_id: Vec<u8>,     // opaque id from registration
    public_key_cose: Vec<u8>,   // COSE-encoded public key from registration
}

impl WebAuthnPrfAuthenticator {
    pub fn new(relying_party_id: impl Into<String>,
               credential_id: impl Into<Vec<u8>>,
               public_key_cose: impl Into<Vec<u8>>) -> Result<Self, AuthError>;
    pub fn relying_party_id(&self) -> &str;
    pub fn credential_id(&self) -> &[u8];
    pub fn public_key_cose(&self) -> &[u8];
}

impl Authenticator for WebAuthnPrfAuthenticator {
    fn kind(&self) -> &'static str { "webauthn-prf" }
    fn mode(&self) -> Mode { Mode::Bind }
    fn verify(&self, _credential: &[u8]) -> Result<AuthAssertion, AuthError> {
        Err(AuthError::Unimplemented {
            backend: "FIDO2 CTAP2 hmac-secret (WebAuthn PRF)",
        })
    }
}
```

`AuthError` gains one new variant, `Unimplemented { backend: &'static
str }`, textually and structurally identical to `vault-key-
custody::CustodyError::Unimplemented` (`vault-key-custody`'s own
`TpmCustodian` uses exactly this shape for `wrap`/`unwrap`).

**Why `verify()` always fails, with no partial validation.** A real
WebAuthn/CTAP2 assertion is CBOR-encoded `authenticatorData` (rpId
hash, flags, sign count, optional attested-credential-data, extension
outputs) plus a raw signature over `authenticatorData ||
SHA-256(clientDataJSON)`, verified with ECDSA over the credential's
registered public key. Two pieces this scaffold does not have yet
make correct verification impossible:

1. **A live assertion from real hardware.** This crate has never done
   I/O of any kind — `PasswordAuthenticator` and `TotpAuthenticator`
   both operate on bytes the caller already obtained. Getting those
   bytes from a physical CTAP2 key needs the transport this spec
   defers to a follow-up PR (§5's `ctap-hid-fido2` recommendation).
2. **ECDSA P-256 signature verification.** Searched this workspace
   for an existing primitive before assuming one was needed — there
   is none. `code/packages/rust/` has `sha256`, `hkdf`, `hmac`,
   `argon2id`, `chacha20-poly1305`, `aes`/`aes-modes`, `rsa`, and
   `canonical-cbor` (already written with FIDO2/COSE in mind — its
   own doc comment names "COSE-Key, used by FIDO2 / WebAuthn-PRF" as
   a design driver), but no elliptic-curve signature primitive of any
   kind. A correct, constant-time P-256 ECDSA verifier is itself a
   real cryptographic-primitive PR, comparable in size and review
   weight to this workspace's existing `argon2id`/`chacha20-poly1305`
   crates, not a few lines alongside a hardware transport crate.

Given both are missing, `verify()` could still attempt to validate
the parts that don't need them — decode the CBOR, check the rpId hash,
check the extension is present — and only fail once it reaches the
signature step. This spec deliberately rejects that middle ground.
Partially validating an assertion and reporting success on the parts
checked would answer a different question than the one `verify()` is
asked: the caller wants to know "is this a valid credential," and a
type that can only answer "is this *shaped like* a valid credential"
must say so by refusing outright, not by silently answering a
narrower question under the same name. `vault-key-custody::
TpmCustodian` drew this exact line for the exact same reason (`wrap`/
`unwrap` refuse unconditionally rather than doing the parts of the
TPM protocol that don't need silicon), and this scaffold mirrors it
byte-for-byte in its error type and behavior.

**What is real about this scaffold, and why that's still worth
shipping on its own.** `kind()`, `mode()`, and the three accessors are
not placeholders — a hardware-key registration ceremony's output
really is exactly `(relying_party_id, credential_id, public_key_cose)`,
and code that composes authenticators (a future VLT06 policy: `all_of
{ password, webauthn_prf }`) or that needs to enumerate "what bind
factors exist for this vault" can already be written against this
type today, exactly as `select_custodian` in `vault-key-custody`
already makes real TPM-first decisions against `TpmCustodian` despite
its `wrap`/`unwrap` being unusable. `combine_key_contributions` also
already handles a `webauthn-prf`-kind assertion correctly the moment
one exists, because it dispatches purely on `Mode`, not on `kind` —
proven by this PR's `webauthn_prf_kind_is_counted_as_extension_
factor_in_summaries` test, which constructs a bind-mode `webauthn-prf`
assertion by hand and confirms `summarize_auth_assertions` and
`can_derive_unlock_key()` treat it identically to a real one.

## 7. Testing without real hardware

Before assuming a mock or a software FIDO2 authenticator library was
needed for this PR, it's worth being precise about what actually
needs testing here: `verify()` has exactly one behavior —
unconditional refusal — for every input, by construction, so there is
nothing hardware-shaped left to simulate. The tests added
(`webauthn_prf_reports_bind_mode_and_kind`,
`webauthn_prf_accessors_round_trip_construction_inputs`,
`webauthn_prf_verify_always_returns_unimplemented` — checked against
both an empty credential and a plausible-looking one, to demonstrate
the refusal really is unconditional rather than an artifact of one
particular malformed input — plus three constructor-validation tests
and the cross-module summary test above) cover the entire real
surface of this scaffold the same way `vault-key-custody`'s
`tpm_reports_hardware_caps`/`tpm_wrap_returns_unimplemented` cover
`TpmCustodian`'s entire real surface. `cargo test -p
coding_adventures_vault_auth` passes 33/33 (24 pre-existing + 9 new)
with no hardware, no CI changes, and no new dependency.

The question of *how* to test the eventual real transport without
physical hardware attached to a CI runner is real and is intentionally
left to the follow-up PR that adds it, once there is verification
logic to exercise. The honest answer, noted here so that PR doesn't
have to re-derive it: CTAP2's command layer (CBOR-encoded requests/
responses over a HID or virtual transport) can be tested against a
software authenticator that implements the CTAP2 state machine
in-process — several exist in other ecosystems (e.g. Python's
`python-fido2` ships `SoftCtap2Device`, no real HID needed) — and the
same shape is achievable in Rust without touching a kernel HID driver
by giving the transport trait a software-loopback implementation
alongside the real `hidapi` one, so unit tests exercise the CBOR
protocol and extension handling for free while only manual/hardware-
in-the-loop testing exercises the actual `hidapi` device enumeration.
That split — protocol logic real-and-tested, physical transport
thin-and-manually-verified — is the same shape `vault-pm-agent-
protocol`/`vault-pm-agent-host` already use for the local agent
(`VLT-PM48` §2), so it is not a new testing pattern for this
workspace, just one this PR doesn't need yet because it has no
protocol logic to test until the transport crate lands.

## 8. Explicitly deferred: `YubikeyPrfCustodian` (custody provider)

`VLT00-vault-roadmap.md`'s VLT03 table already names
`YubikeyPrfCustodian` — "FIDO2 PRF / hmac-secret extension" — wrapping
the vault's master KEK directly under a hardware-derived key, the same
role `TpmCustodian` plays for TPM/Secure Enclave. This is real,
in-scope future work and not part of this PR, for reasons distinct
from — and larger than — the authenticator scaffold's own deferral:

- It needs the **same** missing pieces as §6 (real CTAP2 transport,
  and here also the `hmac-secret` *wrap* direction — deriving a
  stable per-credential secret is only half of what a custodian needs;
  it also has to decide how `wrap`/`unwrap` compose that secret with
  an AEAD the way `PassphraseCustodian` already composes Argon2id
  with one), plus everything §6 defers.
- It changes a different, higher-stakes trust boundary. An
  authenticator scaffold that always refuses is inert — it cannot
  make an existing vault less safe no matter what a caller passes it,
  because it never produces an `Ok`. A custodian scaffold sits
  directly on the vault's master-KEK wrap/unwrap path; even a
  correctly-fail-closed stub interacts with `select_custodian`'s
  TPM-first policy (§4 of `VLT03-vault-key-custody.md`) in ways that
  need their own review — e.g. would a detected hardware key change
  which custodian `select_custodian` prefers over an already-present
  TPM, and does "prefer hardware" still make sense when there are two
  *kinds* of hardware to choose between? `VLT-PM00`'s working
  principle of reviewing one coherent, bounded change at a time (the
  same reasoning `VLT-PM48`'s own multi-round security review history
  demonstrates for a comparably-sized IPC surface) argues for a
  separate PR once the authenticator-side transport work has already
  answered the "how do we even talk to one of these" questions once.
- `vault-key-custody` has zero consumers inside `vault-pm-application`
  today (`PassphraseCustodian`/`TpmCustodian` are both library-only;
  `vault-pm`'s own unlock path derives its KEK directly rather than
  through the `KeyCustodian` trait). Wiring the *first* real hardware
  custodian into a product that has never consumed this trait at all
  is its own integration project, independent of which custodian it
  is.

A follow-up PR owns: the CTAP2 hardware transport (shared with the
authenticator's eventual real `verify()`), a `YubikeyPrfCustodian`
implementing `wrap`/`unwrap` by combining the `hmac-secret` output
with an AEAD exactly as `PassphraseCustodian` combines an Argon2id tag
with one today, and the `vault-pm-application` integration point that
makes `vault-key-custody` a real dependency of the product for the
first time.

## 9. Out of scope (slice 1 — see §17 for what slice 2 still defers)

- ~~Real CTAP2/WebAuthn hardware transport~~ — **shipped in slice 2**,
  §12–§16.
- ECDSA P-256 signature verification (§6) — no elliptic-curve
  primitive exists in this workspace yet. Still out of scope after
  slice 2 — see §17.
- `YubikeyPrfCustodian` / any hardware-backed `KeyCustodian` (§8).
  Still out of scope.
- YubiKey HMAC-SHA1 challenge-response (§3) — ruled out as a design
  direction, not merely deferred.
- ~~CI wiring for native hardware-I/O dependencies~~ — **shipped in
  slice 2** for the platform that needed an explicit step (§16); see
  that section for why macOS and Windows needed none.
- VLT06 policy composition rules that would let an operator require
  `webauthn_prf` as part of an unlock policy — the authenticator has
  to exist and actually work before a policy can reference it. Still
  out of scope: `verify()`'s final answer is still always
  `Err`, so there is still nothing for a policy to require yet.
- A registration/enrollment ceremony (`vault-pm hardware-key add` or
  similar) — still ships the verification-side type only.

## 10. Acceptance gates

1. `WebAuthnPrfAuthenticator::new` rejects an empty relying-party id,
   an empty credential id, and an empty public key, each with
   `AuthError::InvalidParameter`.
2. `kind()` returns `"webauthn-prf"` and `mode()` returns `Mode::Bind`.
3. `verify()` returns `AuthError::Unimplemented` for every input tried,
   including an empty credential and a non-empty, plausible-looking
   one — proving the refusal is unconditional, not an artifact of one
   malformed shape.
4. `AuthError::Unimplemented`'s `Display` string is a static literal
   containing the backend name, consistent with every other
   `AuthError`/`CustodyError` variant in this codebase never
   interpolating attacker-controlled bytes.
5. A hand-built bind-mode `AuthAssertion` of kind `"webauthn-prf"`
   round-trips correctly through `summarize_auth_assertions` (counted
   as an extension factor, contributes to `can_derive_unlock_key()`)
   and through `combine_key_contributions` (dispatch is by `Mode`, not
   by `kind`, so a real future assertion needs no changes to either
   function).
6. `cargo build --workspace`, `cargo test`, `cargo fmt --check`, and
   `cargo clippy --all-targets -- -D warnings` are green for
   `coding_adventures_vault_auth` with no new dependency in its
   `Cargo.toml`.
7. `VLT05-vault-auth.md`, `VLT00-vault-roadmap.md`, and this crate's
   `README.md`/`CHANGELOG.md` reflect the scaffold's real scope —
   what ships versus what's deferred — matching this document.

## 11. Slice 2 — re-verifying the dependency choice

Slice 1 recommended `ctap-hid-fido2` without adding it. Before adding
it for real, its state was re-checked rather than trusted from a
five-section-old table:

| Check | Slice 1's claim | Re-verified (slice 2) |
|---|---|---|
| Latest version | 3.5.13 | **Confirmed unchanged**: `crates.io`'s API (`/api/v1/crates/ctap-hid-fido2`) reports `max_stable_version` and `newest_version` both `3.5.13`. |
| Last publish | 2026-08-12 | **Confirmed unchanged**: `updated_at` `2026-08-12T00:00:50Z`. |
| License | (not checked in slice 1) | MIT, confirmed by reading the vendored `LICENSE` file after adding the dependency. |
| `hmac-secret` support | "extensions via a `Gext` enum that includes `hmac-secret`" | **Confirmed by reading the actual source**, not just the claim: `fidokey::get_assertion::get_assertion_params::Extension::HmacSecret(Option<[u8; 32]>)` (single-salt) and `HmacSecret2` (two-salt), plus `Extension::create_hmac_secret_from_string` and a full decrypt path in `get_assertion_response.rs` that turns the authenticator's encrypted extension output into the raw secret bytes this design needs as `hmac_secret_output`. |
| Device enumeration / `GetAssertion` API | "enumerate devices, `make_credential`, `get_assertion`" | **Confirmed and used directly**: `ctap_hid_fido2::get_fidokey_devices()`, `FidoKeyHidFactory::create`, and `FidoKeyHid::get_assertion_with_extensios(rpid, challenge, credential_ids, pin, extensions)` are exactly the functions §12/§13 build on. |
| Physical touch / timeout | "README documents ... Windows needs admin rights at run time" | **A gap slice 1 didn't catch**: the crate's own `GetAssertion` call has **no timeout parameter of any kind** — it blocks on `CTAPHID_KEEPALIVE` frames until the device answers or gives up on its own, with no way to bound that from the public API. §14 covers the wrapper this requires. |
| Windows admin rights | Flagged, not further investigated | Still true and still not something this slice can verify without a Windows machine in the loop; carried forward as a documented gap (§16). |

**One correction to slice 1's own table.** §5's dependency-comparison
table listed `ctap-hid-fido2`'s notes as needing
`libusb-1.0-0-dev`/`libudev-dev` on Linux, generalizing from
`hidapi`'s README. Reading `hidapi`'s actual `build.rs` (source, not
docs) after adding the dependency shows this is more specific:
`ctap-hid-fido2`'s own `Cargo.toml` pins `hidapi`'s
`linux-static-hidraw` feature with `default-features = false` — the
`hidraw`-backed path, which links only against `libudev` via
`pkg_config::probe_library("libudev")`. The `libusb`-backed paths
(which do need `libusb-1.0-0-dev`) aren't compiled in at all. §15's CI
change installs `libudev-dev` only, not both.

**Transitive dependency tree, checked by actually resolving it**
(`cargo tree -p ctap-hid-fido2`), not estimated: ~50 unique crates.
The notable ones and why each is there: `ciborium` (CBOR, matching
`canonical-cbor`'s own COSE/CBOR framing elsewhere in this workspace),
`x509-parser`/`asn1-rs`/`der-parser` (attestation certificate parsing
— unused by this slice, which never calls `make_credential`, but part
of the crate's compiled surface regardless), `aes`/`cbc` (the CTAP2
PIN protocol's AEAD, used by the `hmac-secret` shared-secret exchange
this slice does call), `ring` (ECDH key agreement for that same PIN
protocol, plus SHA-256; `ring`'s own long track record in the Rust
ecosystem — it underlies `rustls`, among many others — is why the
`ring` feature was chosen over `aws-lc-rs` here, needing no additional
system build toolchain beyond what Rust already requires), `num`/
`strum`/`nom`-family crates (CBOR/ASN.1 parsing support), and
`hidapi` itself (the one crate in this tree that actually touches
native code).

**Unsafe-code footprint, checked by grepping the vendored source**,
not assumed: `ctap-hid-fido2`'s own `src/*.rs` files contain **zero**
`unsafe` blocks — the entire CTAP2 protocol layer (CBOR encoding,
PIN/UV auth protocol, HMAC extension handling, response parsing) is
safe Rust. All the `unsafe` in this dependency chain is exactly where
it has to be: inside `hidapi`'s own FFI bindings to the native HID
API, which is unavoidable for real USB HID I/O from Rust. This is
also why `vault-auth` itself can keep `#![forbid(unsafe_code)]`
unchanged even though the workspace as a whole now depends
transitively on code containing `unsafe` — `forbid(unsafe_code)` is a
per-crate lint, not a transitive one, and `vault-auth` never depends
on `ctap-hid-fido2` at all (§12).

**Conclusion: the slice 1 recommendation holds**, with one correction
(libudev only, not libusb+libudev) and one gap it missed (no built-in
timeout) that this slice's own design has to account for rather than
assume away.

## 12. The `Ctap2Transport` boundary and the new `vault-webauthn-ctap2-hid` crate

`ctap-hid-fido2` is added as a dependency of a **new** crate,
`code/packages/rust/vault-webauthn-ctap2-hid`
(`coding_adventures_vault_webauthn_ctap2_hid`) — not of `vault-auth`.
`vault-auth` gains only a trait:

```rust
pub trait Ctap2Transport {
    fn get_hmac_secret_assertion(
        &self,
        request: &Ctap2AssertionRequest<'_>,
    ) -> Result<Ctap2AssertionResponse, Ctap2TransportError>;
}
```

`Ctap2AssertionRequest`/`Ctap2AssertionResponse`/`Ctap2TransportError`
are transport-agnostic plain data types living in `vault-auth`
alongside the trait — nothing in them mentions HID, USB, or
`ctap-hid-fido2`. `WebAuthnPrfAuthenticator` holds a `Box<dyn
Ctap2Transport + Send + Sync>`, supplied at construction time.

**Why a separate crate rather than adding the dependency straight to
`vault-auth`.** `vault-auth` is the crate every unlock factor in this
product goes through — `PasswordAuthenticator` and `TotpAuthenticator`
are pure computation with zero OS access today
(`required_capabilities.json` declares only `time`/`read` for the
wall clock TOTP needs). Giving `vault-auth` a native, hardware-I/O
dependency would mean every consumer of the password and TOTP factors
— which is to say every consumer of this crate, hardware key or not —
inherits `ctap-hid-fido2`'s build footprint (compiling `ring`,
`x509-parser`, `hidapi`'s C/native shim, …) and its runtime capability
profile (native FFI into the OS's HID stack). That is exactly the
shape `VLT-PM48-local-agent-ipc.md` already solved once for the local
agent: a protocol crate (`vault-pm-agent-protocol`) that stays free of
transport-specific dependencies, and a separate host crate
(`vault-pm-agent-host`) that implements the transport. This document
uses the identical split. `vault-webauthn-ctap2-hid`'s own
`required_capabilities.json` is the one manifest in this whole feature
that declares a real capability (`ffi: call` against `hidapi`'s native
library chain) — every other file this feature touches stays at "pure
computation" or "wall clock only."

`HidCtap2Transport` (the one type this new crate exports) is
stateless — `#[derive(Default, Clone, Copy)]` — and re-enumerates and
re-opens the device on every call rather than caching an open handle.
An unlock attempt is an occasional, human-paced operation, not a hot
loop, so the repeated-enumeration cost is immaterial, and never
holding native state between calls is what makes its `Send + Sync`
bound trivially and honestly true rather than an unchecked assertion
over unsynchronized native resources.

## 13. What `verify()` does now, and why it still refuses

`WebAuthnPrfAuthenticator::verify(credential)`'s new body, in order:

1. Reject an empty `credential` immediately (`AuthError::
   MalformedCredential`) — before any transport call, proven by a test
   that hands `verify()` a transport which panics if it's ever
   invoked.
2. Build a `Ctap2AssertionRequest`: `challenge = SHA-256(credential)`
   (freshness, feeds the eventual signature check once ECDSA lands);
   `hmac_secret_salt` derived **only** from registration-time data
   (`relying_party_id` + `credential_id`, domain-separated with
   `b"VLT05/webauthn-prf/hmac-secret-salt/v1"`) — **never** from
   `credential`. This is deliberate: the `hmac-secret` salt must be
   identical on every unlock attempt for the same registered
   credential so the derived secret (and therefore
   `key_contribution`) is reproducible, exactly the property
   `PasswordAuthenticator`'s Argon2id tag already has for a fixed
   password. If the salt depended on the (attempt-specific,
   ideally-random) `credential` challenge, the unlock key would change
   on every attempt and could never decrypt data sealed under a
   previous one. A dedicated test
   (`webauthn_prf_hmac_secret_salt_is_stable_across_attempts_and_
   independent_of_credential_bytes`) pins this.
3. Call `self.transport.get_hmac_secret_assertion(&request)`, mapping
   `Ctap2TransportError` to the three new `AuthError` variants (§14).
4. Check the response's rpId hash against `SHA-256(relying_party_id)`
   (constant-time compare, via the same `ct_eq` every other factor in
   this crate uses).
5. Check the response's `credential_id` matches the registered one.
6. Check the response's user-presence flag is set.
7. Check the response carries an `hmac-secret` output at all.
8. **Only if all seven checks above pass**, reach the final answer:
   `Err(AuthError::Unimplemented { backend: "ECDSA P-256
   assertion-signature verification (WebAuthn PRF)" })`.

Step 8 is unchanged in kind from slice 1 — `verify()` still never
returns `Ok(...)` — but it is reached from a materially different
place. Slice 1's `verify()` returned `Unimplemented` unconditionally,
for every input, without doing anything. Slice 2's `verify()` does
real hardware I/O and five real structural checks against the
response first, and *still* refuses at the end, because none of those
checks substitute for proving `response.signature` was produced by
the registered credential's private key over `response.auth_data ||
SHA-256(request.challenge)` — which needs ECDSA P-256, and no
primitive for that exists in this workspace (§7's survey is
unchanged: `argon2id`, `chacha20-poly1305`, `aes`, `rsa`, `ed25519`,
`x25519` all exist; nothing for the P-256 curve does). Accepting the
hardware's answer as `Ok(...)` at step 7 — "a device plugged in, this
credential id, physical touch, and it returned *something* under
`hmac-secret`" — without step 8's proof would mean trusting whatever
bytes answered rather than proving they came from the registered
authenticator. `vault-key-custody::TpmCustodian` draws the identical
line for the identical reason; this slice's addition is that the line
now sits one real hardware round trip later than it used to.

## 14. Touch timeout and failure-mode design

**Bounds.** `DEFAULT_TOUCH_TIMEOUT` is 30 seconds — matched to, not
invented ahead of, the ballpark FIDO2 authenticators commonly enforce
internally for a `GetAssertion` request. `with_touch_timeout` accepts
`MIN_TOUCH_TIMEOUT` (1s) through `MAX_TOUCH_TIMEOUT` (120s); a caller
asking for less has no realistic window for a human to react to a
blinking device, and a caller asking for more has stopped describing
an interactive unlock (`verify()`'s "clear, fast, non-hanging failure
mode" design requirement depends on this staying a bounded, human-
scale wait).

**Fast path when no hardware is present.** `FidoKeyHidFactory::create`
fails immediately — via a cheap HID enumerate, not a blocking read —
when zero (or more than one, ambiguously) devices are found. This
path never enters the timeout wait at all, so the passphrase-only
unlock path is not slowed down by "is there a hardware key plugged
in?" — confirmed for real (not asserted) by a test that measures
elapsed wall-clock time and asserts it stays under the configured
timeout.

**Bounded wait for the physical touch.** `ctap-hid-fido2`'s
`GetAssertion` call has no timeout parameter of its own (§11) — it
blocks on the underlying HID read until the device answers or its own
internal handling gives up, with no hook to bound that from the
public API. `HidCtap2Transport` wraps the call in a dedicated worker
thread and races it against `request.touch_timeout` with
`mpsc::Receiver::recv_timeout`, so `verify()` always returns control
to the caller within the configured window.

**The one honest trade-off this wrapper has.** There is no
cross-platform way to cancel a blocking native HID read from safe
Rust. When the timeout fires, the worker thread is left running
rather than killed; it exits on its own once the device eventually
answers (touched or not), at which point its result is silently
discarded. A single abandoned attempt's thread is bounded and
self-terminating — it is not a leak that grows without limit from one
timeout — but repeated timeouts against an unresponsive device do
accumulate live background threads until each resolves. This is
documented rather than hidden, in both the code (`HidCtap2Transport`'s
doc comment) and here, as a real limitation of building on top of
`ctap-hid-fido2`'s blocking convenience API rather than a lower-level
transport that exposes `hidapi`'s own read timeout directly — a
reasonable target for a future PR, not something this slice claims to
have solved perfectly.

**Error taxonomy.** Three new `AuthError` variants, deliberately
narrower than the raw text `ctap-hid-fido2` reports (which is
free-text `anyhow::Error`, not a typed CTAP2 status-code enum, and
therefore a channel this design does not trust with attacker- or
device-controlled bytes, consistent with every other `AuthError`
variant in this crate never interpolating input):

- `HardwareUnavailable` — no device answered, or more than one did
  and the transport can't disambiguate. Always reachable fast (see
  above).
- `HardwareTimeout` — a device was reached but didn't confirm a touch
  in time. **Also covers an authenticator that affirmatively declines
  the request.** CTAP2-over-HID has no signal that reliably
  distinguishes "touched and declined" from "never touched" on every
  authenticator; the small number of devices that do report an
  explicit denial (`CTAP2_ERR_OPERATION_DENIED` and similar) are
  still classified into this same variant by
  `vault-webauthn-ctap2-hid`'s `classify_ctap_error`, best-effort,
  via text matching on the error message (not by parsing a typed
  status code, which `ctap-hid-fido2`'s public API doesn't expose).
- `HardwareTransport { detail: &'static str }` — anything else: a HID
  I/O error, a malformed or unexpected CTAP2 response, or an
  unrecognized protocol-level failure. `detail` is always one of this
  crate's own static labels, never the original error text, exactly
  as `Unimplemented`'s `backend` field and every other `AuthError`
  variant already avoid echoing input.

**No secret material in any of the above.** None of the three new
variants, nor `classify_ctap_error`'s mapping, nor the `enable_log:
false` default this slice pins explicitly on `LibCfg` (`ctap-hid-
fido2`'s own debug tracing, which is off by default but was verified
by reading its source rather than assumed), ever logs or echoes the
`hmac-secret` output, the raw signature, or any other credential-
shaped bytes. The one place a genuinely secret 32-byte value exists in
this whole path (`Ctap2AssertionResponse::hmac_secret_output`) is
`Zeroizing`-wrapped from the moment `vault-webauthn-ctap2-hid`'s
`map_assertion` produces it, matching every other secret-carrying type
in this crate and in `vault-auth`.

## 15. CI wiring and testability

**CI wiring.** `ctap-hid-fido2` pins `hidapi`'s `linux-static-hidraw`
feature (§11), which links only against `libudev` on Linux. `.github/
workflows/ci.yml`'s existing Rust build job installs `libudev-dev` on
Linux runners now, following the same conditional-on-`needs_rust`
pattern already used for `libcairo2-dev` (Paint VM). **macOS and
Windows need no new CI step**: `hidapi`'s macOS backend links the
IOKit/CoreFoundation frameworks the Xcode toolchain (already present
on GitHub's macOS runners) ships by default, and its Windows backend
links `SetupAPI`/`hid.lib`, part of the standard Windows SDK the
existing MSVC setup step already provides for this workspace's other
Rust jobs. Neither needed an explicit package install to compile in
this slice's testing.

**Testability — what turned out to be real, non-mocked, and free.**
`ctap_hid_fido2::get_fidokey_devices()` and `FidoKeyHidFactory::
create` are both safe and fast to call with zero hardware attached —
confirmed, not assumed, by calling them directly in
`vault-webauthn-ctap2-hid`'s own test suite. That is the "real
integration compiles and calls the right APIs" verification the task
this slice implements explicitly asked for: no fake stands in for
`ctap-hid-fido2` anywhere in that crate's tests. The request/response
mapping functions (`build_hmac_secret_extension`, `map_assertion`,
`classify_ctap_error`) are pure and are unit-tested directly against
real `ctap-hid-fido2` types (`Assertion`, `Flags`) built by hand — the
real crate's own structs, not a stand-in for them.

**Testability — what needed a fake, and why that's the right
boundary.** `WebAuthnPrfAuthenticator::verify()`'s own logic (the
seven checks in §13, the salt-derivation stability property, the
error-mapping from `Ctap2TransportError` to `AuthError`) is unit-
tested in `vault-auth` against a small in-process `FakeTransport`,
covering every path §13 lists plus timeout/no-device/wrong-device/
wrong-relying-party/no-user-presence/no-hmac-secret-extension — the
exact matrix this slice's task asked for — without any device and
without depending on `ctap-hid-fido2` at all. This is the same split
`VLT-PM48`'s protocol/host crates already established: protocol logic
real-and-tested against a fake of the transport boundary, physical
transport thin-and-verified-for-real-wiring-but-not-for-hardware-
behavior.

**What genuinely cannot be tested in CI, and is not pretended to be.**
An actual physical touch. No software or virtual CTAP2 authenticator
crate was found in the Rust ecosystem at the time of this survey
(unlike Python's `python-fido2`, which ships `SoftCtap2Device`) —
`webauthn-authenticator-rs` (kanidm) was checked specifically for one
and doesn't carry a software authenticator implementation either. This
slice does not build one: `FakeTransport` already covers everything
`WebAuthnPrfAuthenticator::verify()` itself does with a transport's
response, and a full software CTAP2 state machine (CBOR command
parsing, PIN/UV auth protocol, credential storage) would only be
worth building to test `vault-webauthn-ctap2-hid`'s adapter code, which
is a thin, mostly-pure-function layer already covered per the two
paragraphs above. The literal physical touch — the one thing left
untested — is exactly the boundary `code/scripts/miri-twig-vm.sh`-style
manual verification exists for elsewhere in this workspace: real,
necessary, and honestly out of CI's reach.

**A real concurrency bug this slice's own testing found and fixed.**
Building `vault-webauthn-ctap2-hid`'s test suite surfaced a genuine
crash: running multiple `#[test]` functions that each called real
`ctap_hid_fido2` enumeration APIs, under the default parallel test
runner, reliably crashed the test binary with `SIGTRAP`. Adding a
`Mutex` around each individual call did **not** fix it — the crash
persisted even with calls serialized, which ruled out a same-process
data race as the sole cause. What did fix it: consolidating every
real-hardware call into a single `#[test]` function, so `libtest`
spawns exactly one OS thread for all of them. This points to `hidapi`'s
macOS backend (Core Foundation / IOKit) not tolerating entry from more
than one distinct OS thread across a process's lifetime, a stricter
constraint than ordinary mutual exclusion. `HID_ACCESS_LOCK` — the
`Mutex` added during this investigation — was kept in the shipped code
anyway, because it protects a **different, still-real** hazard: two
concurrent `verify()` calls in one long-lived process (plausible once
this transport is wired into `vault-pm-agent-host`, which serves
concurrent requests per `VLT-PM48`) would hit the same class of crash
in production without it, and serializing hardware access is also the
semantically correct behavior for a single physical USB device
regardless of the crash. `vault-webauthn-ctap2-hid`'s `CHANGELOG.md`
and the `HID_ACCESS_LOCK` doc comment carry the full account.

**A second real bug this document's own `/security-review` round
found and fixed.** `HID_ACCESS_LOCK` originally used a blocking
`lock()`. The reviewer traced the compounding failure mode that opens:
a device that never answers `GetAssertion` leaves its worker thread
holding the lock forever (the abandoned-thread trade-off above already
names this), but with a *blocking* lock, every **subsequent**
`verify()` call still spawns a fresh worker thread that also blocks
acquiring the same lock — accumulating one permanently-stuck thread
per attempt, unboundedly, for as long as attempts kept coming against
the same stuck device. The fix: `HID_ACCESS_LOCK` is now acquired with
`try_lock`, never a blocking `lock`. A contended attempt fails fast
with `Ctap2TransportError::Failed { detail: "another hardware
operation is already in progress" }` instead of queuing, so at most
one thread is ever blocked on real (or stuck) hardware I/O at a time.
`transport_fails_fast_without_queuing_when_hid_access_is_already_held`
pins this without touching real `hidapi` at all — holding the lock on
the test thread makes the contended path return before
`run_get_assertion` is ever reached, so the test is safe to run
alongside the one-thread-only real-hardware test in the same suite.
The same review also caught an un-zeroized stack copy of the
`hmac-secret` output in `map_assertion` (the extracted bytes were
copied into `Zeroizing` but the original local was never explicitly
wiped) and the absence of `#![forbid(unsafe_code)]` on this crate
(which has zero `unsafe` blocks, so the lint costs nothing to add and
turns a manual-inspection claim into a compiler-enforced one); both
are fixed. `vault-webauthn-ctap2-hid`'s `CHANGELOG.md` has the full
account of all three findings.

## 16. Explicitly still deferred (after slice 2)

- **ECDSA P-256 assertion-signature verification.** Unchanged from §7:
  no elliptic-curve signature primitive exists anywhere in this
  workspace. This is the one remaining piece standing between
  `verify()`'s current final `Err` and a real `Ok(...)`. Sized
  comparably to this workspace's existing `argon2id`/
  `chacha20-poly1305` primitive crates — its own PR, not a few lines
  bolted onto the hardware transport.
- **`YubikeyPrfCustodian` / any hardware-backed `KeyCustodian`.** §8's
  reasoning is unchanged and, if anything, reinforced: it needs both
  ECDSA (still missing) and the `hmac-secret` *wrap* direction, which
  is a different question from the *verify* direction this slice
  answers.
- **VLT06 policy composition.** Still nothing for a policy to
  reference, because `verify()` still cannot succeed.
- **A registration/enrollment ceremony.** Still out of scope — this
  slice only makes the verification-side type's hardware I/O real.
- **A software/virtual CTAP2 authenticator for CI.** Investigated
  (§15) and deliberately not built — the fake-transport boundary
  already covers what needs testing, and no existing Rust crate fills
  this gap the way `python-fido2`'s `SoftCtap2Device` does for Python.
- **A lower-level HID transport with a real read timeout.** §14 names
  this as the honest way to close the "abandoned worker thread"
  trade-off; not attempted here because it would mean bypassing
  `ctap-hid-fido2`'s convenience API for a `hidapi`-level integration,
  a larger and separately-reviewable change.
- **Windows admin-rights verification.** Still not checked against a
  real Windows machine (§11); carried forward as a known gap in this
  slice's own verification, not asserted as solved.

## 17. Acceptance gates (slice 2)

1. `ctap-hid-fido2` is re-verified against slice 1's recommendation
   (version, license, API surface, transitive dependency tree,
   unsafe-code footprint) before being added — §11.
2. `ctap-hid-fido2` is a dependency of `vault-webauthn-ctap2-hid`
   only; `vault-auth`'s `Cargo.toml` gains no new external dependency,
   only the workspace-internal `coding_adventures_sha256` path
   dependency it already needed for rpId-hash comparison — §12.
3. `WebAuthnPrfAuthenticator::verify()` performs a real CTAP2
   `GetAssertion` with the `hmac-secret` extension through its
   `Ctap2Transport`, and checks rpId hash, credential id, user
   presence, and `hmac-secret` presence before reaching its final
   `AuthError::Unimplemented` — §13, proven by
   `webauthn_prf_verify_still_refuses_after_a_correct_hardware_round_
   trip` and the wrong-rp/wrong-credential/no-presence/no-extension
   tests alongside it.
4. `verify()` never blocks past its configured `touch_timeout`
   (`MIN_TOUCH_TIMEOUT..=MAX_TOUCH_TIMEOUT`, default 30s), and returns
   fast when no hardware is present — §14, proven by a wall-clock
   timing assertion in `vault-webauthn-ctap2-hid`'s test suite.
5. No secret material (the `hmac-secret` output, the raw signature) is
   ever logged, echoed in an error, or left un-zeroized — §14.
6. `vault-webauthn-ctap2-hid`'s own test suite includes at least one
   real, non-mocked call into `ctap-hid-fido2`'s public API (device
   enumeration and `FidoKeyHidFactory::create`) that runs successfully
   with no hardware attached, proving the dependency wiring compiles
   and calls the real crate correctly — §15.
7. `cargo build --workspace`, `cargo test`, `cargo fmt --check`, and
   `cargo clippy --all-targets -- -D warnings` are green across the
   whole workspace, including the new `vault-webauthn-ctap2-hid` crate
   and `vault-auth`'s updated surface.
8. CI installs whatever native library `hidapi`'s selected feature set
   actually needs (`libudev-dev` on Linux; nothing extra on macOS/
   Windows) — §15.
9. `README.md`/`CHANGELOG.md` exist for `vault-webauthn-ctap2-hid` and
   are updated for `vault-auth`, and this document reflects what
   shipped versus what's still deferred (§16) — matching this
   document.

## 18. Citations

- FIDO Alliance, *Client to Authenticator Protocol (CTAP) 2.x* —
  `hmac-secret` extension (§11.2 in CTAP 2.1/2.2/2.3, "Authenticator
  API command extensions").
- W3C, *Web Authentication: An API for accessing Public Key
  Credentials Level 3* — `prf` extension (browser-facing name for the
  same CTAP2 capability); §7 "WebAuthn Extensions."
- RFC 9053 (COSE) — the public-key encoding `public_key_cose`
  captures; already the format `canonical-cbor`'s own doc comment
  names as a design driver.
- `code/specs/VLT03-vault-key-custody.md` §"`TpmCustodian` (scaffold)"
  — the precedent this document's `WebAuthnPrfAuthenticator` mirrors.
- `code/specs/VLT05-vault-auth.md` — the crate this PR extends.
- `code/specs/VLT-PM00-local-first-password-manager.md` §6 (reuse
  map), §24 (explicitly deferred decisions).
- `code/specs/VLT-PM48-local-agent-ipc.md` — precedent for "optional,
  additive, software path always authoritative" and for splitting a
  protocol crate from its transport (§12, §15 apply this directly to
  `vault-auth`/`vault-webauthn-ctap2-hid`).
- `code/specs/VLT-PM49-cli-external-import.md` §8/§9 — precedent for
  the "explicitly deferred, with concrete reasoning" structure this
  document's §6–§8 and §16 follow.
- `ctap-hid-fido2` v3.5.13 source (`github.com/gebogebogebo/
  ctap-hid-fido2`), and `hidapi` v2.6.6 source
  (`github.com/ruabmbua/hidapi-rs`) — read directly (not taken on
  faith from documentation) to confirm the `hmac-secret` API shape,
  the absence of a `GetAssertion` timeout parameter, the Linux
  `libudev`-only (not `libusb`) build requirement for the
  `linux-static-hidraw` feature, and the zero-`unsafe`-blocks claim
  about `ctap-hid-fido2`'s own protocol-layer code — §11, §14, §15.
