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

## 9. Out of scope (this PR)

- Real CTAP2/WebAuthn hardware transport (§6, §7) — no `hidapi`/
  `ctap-hid-fido2` dependency is added.
- ECDSA P-256 signature verification (§6) — no elliptic-curve
  primitive exists in this workspace yet.
- `YubikeyPrfCustodian` / any hardware-backed `KeyCustodian` (§8).
- YubiKey HMAC-SHA1 challenge-response (§3) — ruled out as a design
  direction, not merely deferred.
- CI wiring for native hardware-I/O dependencies across the
  Linux/macOS/Windows build matrix (§5) — needed by the transport PR,
  not by this one.
- VLT06 policy composition rules that would let an operator require
  `webauthn_prf` as part of an unlock policy — the authenticator has
  to exist and actually work before a policy can reference it.
- A registration/enrollment ceremony (`vault-pm hardware-key add` or
  similar) — this PR ships the verification-side type only.

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

## 11. Citations

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
  protocol crate from its transport.
- `code/specs/VLT-PM49-cli-external-import.md` §8/§9 — precedent for
  the "explicitly deferred, with concrete reasoning" structure this
  document's §6–§8 follow.
