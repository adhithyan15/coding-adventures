# VLT00 — Vault Roadmap

## Purpose

The Vault stack is a **generic, layered library for building any kind
of secrets-storage product**. It is not itself a product. Two
canonical use cases drive the design — and the explicit goal is that
*both* of them can be built on the same primitives without forking the
stack:

1. **End-user password manager** — a Bitwarden / 1Password /
   KeePassXC-class app, where a human types a master password (plus
   maybe a YubiKey, plus maybe a passkey) and sees their logins,
   secure notes, credit cards, TOTP seeds, attachments, and shared
   vaults across multiple devices.
2. **Machine secrets store** — a HashiCorp Vault / AWS Secrets
   Manager / Doppler-class system, where workloads (CI runners,
   Kubernetes pods, EC2 instances, GitHub Actions jobs) authenticate
   via cloud IAM / service-account JWTs / mTLS / AppRole and fetch
   API keys, dynamic database credentials, short-lived certs, and
   transit-encrypted blobs.

Both can be built on top of the same package set if the seams are cut
correctly. This document maps those seams.

### Why one stack for both

A naive read says: password managers and machine-secret vaults are
unrelated. Look closer:

- Both are **AEAD over a KV store**. The body cipher is the same.
- Both **wrap a per-record DEK under a higher-level key**. The
  envelope construction is the same.
- Both need **pluggable authentication**: a password manager wants
  password + TOTP + WebAuthn; a machine vault wants mTLS + IAM-signed
  requests + Kubernetes JWTs. Both reduce to "verify a credential,
  emit an authenticated session."
- Both need **pluggable policy**: "Alice may read this item" /
  "service-X may issue DB creds for prod-postgres."
- Both need **leases / TTLs / revocation** — password managers for
  shared-item revocation; machine vaults for short-lived dynamic
  credentials.
- Both need **audit logs**.
- Both need **a transport surface**: password managers ship a CLI +
  REST + native UIs; machine vaults ship REST + gRPC + CLI + SDKs.

What differs is *which* plugins you compose. A password manager
composes `{password+WebAuthn auth, simple-RBAC policy, opaque-blob
sync, file/cloud-blob transport}`. A machine vault composes
`{IAM/JWT/AppRole auth, HCL-or-Cedar policy, lease manager, dynamic-
secret engines, HTTP+gRPC transport}`. Same Lego bricks, different
build.

### Reference targets

Concrete products that should be implementable on this stack without
hacks:

| Class                      | Reference targets                                                |
|----------------------------|------------------------------------------------------------------|
| File-based, single-user    | KeePassXC, age, pass, gpg-encrypted SQLite                       |
| Cloud-synced E2EE pwd mgr  | Bitwarden, 1Password, Proton Pass, Standard Notes                |
| Browser / OS-integrated    | Apple iCloud Keychain, Chrome / Edge / Firefox password managers |
| Server-mediated machine    | HashiCorp Vault, CyberArk Conjur, Doppler, Infisical, Akeyless   |
| Cloud-native KMS + secrets | AWS Secrets Manager + KMS, GCP Secret Manager, Azure Key Vault   |
| GitOps secrets             | Mozilla SOPS, Bitnami Sealed Secrets                             |
| Identity-only / cert-based | SPIFFE/SPIRE, Teleport, smallstep step-ca                        |

The roadmap below names which layers each class needs.

## Layer map

```text
  ┌──────────────────────────────────────────────────────────────┐
  │  Application (your Bitwarden clone, your Vault clone, …)     │
  └──────────────────────────────────────────────────────────────┘
                                 │
  Distribution tier ─────────────┼──────────────────────────────
  ┌──────────────────────────────────────────────────────────────┐
  │  VLT15  import / export — 1Password, Bitwarden, KeePass, CSV │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT14  attachments — streamable encrypted blobs             │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT13  encrypted search index                               │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT12  revision history / version stream                    │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT11  transports — HTTP / gRPC / CLI / library / FUSE      │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT10  sync engine — E2EE deltas, multi-device              │
  └──────────────────────────────────────────────────────────────┘
                                 │
  Channel tier ──────────────────┼──────────────────────────────
  ┌──────────────────────────────────────────────────────────────┐
  │  VLT-CH vault-secure-channel — Signal-protocol-style:        │
  │         X3DH initial agreement + Double Ratchet per-message  │
  │         (continuous key rotation; channel takeover does not  │
  │         compromise past or future messages)   ◄── SHIPPED    │
  └──────────────────────────────────────────────────────────────┘
                                 │
  Lifecycle tier ────────────────┼──────────────────────────────
  ┌──────────────────────────────────────────────────────────────┐
  │  VLT09  audit log — tamper-evident hash-chained              │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT08  dynamic-secret engines — DB / AWS-STS / PKI / SSH /  │
  │         transit (encryption-as-a-service) / TOTP / KV-v2     │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT07  leases — TTL, renewal, revocation, response-wrapping │
  └──────────────────────────────────────────────────────────────┘
                                 │
  Identity tier ─────────────────┼──────────────────────────────
  ┌──────────────────────────────────────────────────────────────┐
  │  VLT06  policy engine — pluggable: RBAC / HCL / Cedar / Rego │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT05  auth — pluggable: password+OPAQUE, TOTP, WebAuthn,   │
  │         passkeys, FIDO2-PRF, OIDC, JWT, mTLS, AWS-STS, GCP,  │
  │         Azure-MI, K8s-SA, AppRole, Shamir quorum, SSO        │
  └──────────────────────────────────────────────────────────────┘
                                 │
  Crypto foundation tier ────────┼──────────────────────────────
  ┌──────────────────────────────────────────────────────────────┐
  │  VLT04  recipients — multi-recipient DEK wrap (sharing,      │
  │         devices, recovery, KMS, age-style)                   │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT03  key custody — passphrase / OS keystore / TPM /       │
  │         Secure Enclave / YubiKey-PRF / HSM / Cloud KMS       │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT02  typed records — codecs for logins / notes / cards /  │
  │         TOTP seeds / SSH keys / certs / arbitrary JSON       │
  ├──────────────────────────────────────────────────────────────┤
  │  VLT01  sealed store — per-record envelope AEAD  ◄── SHIPPED │
  ├──────────────────────────────────────────────────────────────┤
  │  storage-core — opaque KV with CAS trait                     │
  │    backends: InMemory ◄── SHIPPED, STR01 file ◄── SHIPPED,   │
  │              S3 / GDrive / WebDAV / git ◄── future           │
  └──────────────────────────────────────────────────────────────┘
                                 │
  Primitives ────────────────────┼──────────────────────────────
  ┌──────────────────────────────────────────────────────────────┐
  │  csprng, chacha20-poly1305, argon2id, hkdf, hmac, blake2b,   │
  │  sha-256, sha-512, ed25519, x25519, ct-compare, zeroize, …   │
  └──────────────────────────────────────────────────────────────┘
```

### Dependency rule

A layer reads only the public API of layers beneath it. Reaching
around a layer to talk to `storage-core` directly bypasses envelope
encryption — banned. The single intentional exception is VLT15
(import/export), which touches plaintext during a user-initiated
ceremony with explicit warnings.

### Ciphertext-only storage promise

A `storage-core` backend (any implementation: in-memory, file,
S3, Google Drive, WebDAV, git, future cloud) sees only:

- the `(namespace, key)` slot identifier (opaque bytes),
- the **ciphertext** plus AEAD tag of the envelope-encrypted
  record (produced by VLT01),
- a small set of non-secret metadata fields (revision,
  content-type, timestamps).

It never sees plaintext. So **anyone with raw backend access — a
stolen laptop, a compromised cloud-provider account, a subpoena
served on the sync server — learns nothing about vault contents
without the master key.** This is the central security
guarantee of the storage-agnostic design and is preserved by
*every* backend in the family.

The first two backends — `InMemoryStorageBackend` (in
`storage-core` itself, for tests) and `FsStorageBackend` (STR01,
local-disk with atomic write+rename+fsync) — make the
storage-agnosticism property concrete. Future cloud backends
(S3, Google Drive, WebDAV, git, IPFS, SQLite) implement the
same trait without changing the property.

## Reading guide

For each layer below: **what it does**, **why it exists** (which
reference target needs it, and why it can't live in the layer above
or below), **what it depends on**, **trait sketch** where the
abstraction shape is the point of the spec.

### VLT01 — Sealed store ✅ shipped

Per-record envelope encryption. One KEK, one fresh DEK per record
wrapped under the KEK, body AEAD bound to `(namespace, key)` via AAD.
KEK rotation without re-encrypting bodies. Sits directly on
`storage-core`. Spec: [VLT01-vault-sealed-store.md](./VLT01-vault-sealed-store.md).

The KEK is currently derived from a single password via Argon2id.
The next layers generalise that.

### VLT02 — Typed records ✅ shipped

VLT01 stores `Vec<u8>` plaintext. Real apps need typed records.

Defines a `VaultRecord` trait: typed struct ↔ canonical bytes
(canonical CBOR). Each record carries a `content_type`
(`"vault/login/v1"`, `"vault/note/v1"`, `"vault/card/v1"`,
`"vault/totp/v1"`, `"vault/ssh-key/v1"`, `"vault/x509-cert/v1"`,
`"vault/api-key/v1"`, `"vault/db-credential/v1"`,
`"vault/custom/v1"`). Schema migration is a codec concern, handled
on read. Apps register custom types; the vault treats unknown types
as opaque so it never crashes on them.

Why it matters for both reference targets: both classes of vault
have first-class records that aren't just bytes. A password manager
needs `Login`. A machine vault needs `DatabaseCredential` with
`{username, password, host, port, database, lease_id}`.

Depends on: VLT01.

### VLT03 — Key custody ✅ shipped (PassphraseCustodian + TpmCustodian scaffold)

The KEK has to come from somewhere. VLT01's "Argon2id over a
password" is one source; many real systems use others. This is the
pluggability point.

**TPM-first / hardware-preferred (refinement, 2026-05-04):** when
the host has a hardware custodian available, the vault refuses to
fall back to a software passphrase custodian unless the caller
explicitly opts in. The `KeyCustodian` trait reports
`CustodianCaps { hardware_bound, extractable, requires_user_presence,
remote }`; the `select_custodian(candidates, force_software)`
helper picks the first hardware-bound candidate by default;
`assert_no_hardware_bypass(candidates, host_has_hw,
force_software)` is the boot-time advisory check. If
`force_software=true` is set on a hardware-only candidate list,
the helper returns `NoSoftwareCandidate` rather than silently
returning a hardware custodian — failing closed against the
"asked for software, got hardware" footgun.

The headline invariant: **no extractable key material lives in
process heap when a hardware custodian is in play** — wrap /
unwrap operations cross the hardware boundary, and the unwrapped
key is held only briefly inside `Zeroizing<…>`. Side-channel
attack surface (cold-boot RAM scraping, debugger attaches, swap
files, core dumps) is correspondingly reduced.

The first PR ships `PassphraseCustodian` (Argon2id +
XChaCha20-Poly1305 wrap) plus a `TpmCustodian` scaffold that
reports the right capability shape so `select_custodian` can
already make TPM-first decisions; the platform-specific TPM 2.0
backends (`tss-esapi` on Linux, TBS on Windows, Secure Enclave on
macOS) land in follow-up PRs.

```rust
trait KeyCustodian {
    fn capabilities(&self) -> CustodianCaps;     // hardware-bound? extractable? requires user presence?
    fn wrap(&self, label: &Label, key: &Key) -> Result<WrappedKey>;
    fn unwrap(&self, label: &Label, wrapped: &WrappedKey) -> Result<Key>;
}
```

First-party implementations:

| Custodian               | Wraps under...                                 | Use case                           |
|-------------------------|------------------------------------------------|------------------------------------|
| `PassphraseCustodian`   | Argon2id-derived KEK                           | KeePassXC, age, default vault      |
| `OSKeystoreCustodian`   | macOS Keychain / Windows DPAPI / libsecret     | Chrome / Firefox / native apps     |
| `SecureEnclaveCustodian`| Apple Secure Enclave (`SecKey`, biometric-gated)| Touch ID / Face ID unlock         |
| `TpmCustodian`          | TPM 2.0 storage hierarchy                       | Windows Hello, Linux + TPM         |
| `YubikeyHmacCustodian`  | YubiKey HMAC-SHA1 challenge-response            | KDBX YubiKey unlock                |
| `YubikeyPrfCustodian`   | FIDO2 PRF / hmac-secret extension               | Bitwarden / 1Password YubiKey unlock|
| `Pkcs11Custodian`       | PKCS#11 HSM (CloudHSM, SoftHSM, smartcards)    | Enterprise HSM                     |
| `AwsKmsCustodian`       | AWS KMS Encrypt/Decrypt                         | HashiCorp Vault auto-unseal, AWS SM|
| `GcpKmsCustodian`       | GCP Cloud KMS                                  | Google Secret Manager              |
| `AzureKvCustodian`      | Azure Key Vault Managed HSM                     | Azure Key Vault                    |
| `DistributedFragmentCustodian` | k-of-n shard reconstruction (Shamir)     | Vault unseal quorum, "no single human holds the key" |

`CustodianCaps` answers questions like: can the wrapped key be
extracted (matters for the recipient layer)? Is user presence
required on every unwrap (gates UX)? Is the key bound to a hardware
secret (gates threat model)?

Depends on: VLT01. Used by: VLT04, VLT05.

### VLT04 — Recipients ✅ shipped (PassphraseRecipient + X25519Recipient)

VLT01 wraps each DEK under exactly one KEK. That model breaks the
moment you want any of:

- **Sharing.** Alice creates an item; Bob reads it. The server is
  zero-knowledge, so Alice must re-wrap the DEK under a key Bob owns.
- **Multi-device.** Laptop, phone, browser extension — each unwraps
  the vault key with its own device key. Adding a phone is one
  asymmetric wrap, not a re-encrypt.
- **Recovery.** Optional recovery key (printed at signup) is just
  another recipient on every wrap.
- **Multi-KMS / GitOps.** SOPS encrypts a file's DEK to a list of
  recipients (AWS KMS *and* GCP KMS *and* age public keys). Same
  shape.
- **Sealed Secrets.** A k8s controller's RSA pubkey is a recipient.

Lift the wrap layer into a `Recipient` trait (age's terminology):

```rust
trait Recipient {
    fn wrap(&self, file_key: &Key) -> Result<WrappedFileKey>;
    fn try_unwrap(&self, identity: &Identity, wrapped: &WrappedFileKey)
        -> Result<Option<Key>>;        // None = "not for me"; Err = "for me but failed"
}
```

First-party recipients: passphrase (Argon2id), X25519 pubkey
(crypto_box / ECDH+HKDF+ChaCha20Poly1305), RSA-OAEP-2048,
KMS-CMK (AWS / GCP / Azure), YubiKey-PRF, plugin recipients
(out-of-process wrap/unwrap binaries, age-plugin-* compatible).

A vault record's wrap-set is `Vec<WrappedFileKey>`. Adding a
grantee = one more wrap operation. KEK rotation = re-wrap the
DEK under a new KEK and append; old KEK's wrap is retired.

Depends on: VLT01, VLT03 (custodians can be recipients), VLT02
(so recipient lists can themselves be records).

### VLT05 — Authentication ✅ shipped (Password + TOTP)

Both reference classes need pluggable authentication, and the set
of factors is **wide**. Bitwarden alone supports password +
TOTP + WebAuthn + Duo + Email + FIDO2-PRF. HashiCorp Vault
supports tokens + AppRole + Userpass + LDAP + OIDC + AWS-STS +
GCP-JWT + Azure-MI + K8s-SA + GitHub + JWT + TLS + Kerberos +
Okta + Cloud Foundry. There is no fixed set; this layer is a
plugin host.

```rust
trait Authenticator {
    fn name(&self) -> &str;
    fn challenge(&self, principal: &Principal) -> Result<Challenge>;
    fn verify(&self, challenge: &Challenge, response: &Response)
        -> Result<AuthAssertion>;
    fn key_contribution(&self, assertion: &AuthAssertion)
        -> Option<KeyContribution>;
}
```

The two operating modes that fall out of the survey:

- **Gate mode** — does this principal pass? The factor produces an
  authenticated session token but no key material. TOTP, SMS,
  email-OTP, WebAuthn-2FA work this way.
- **Bind mode** — the factor *also contributes* key material to the
  unlock. The KDF input set widens: `derive(password, secret_key,
  yubikey_prf_secret, …)`. 1Password's Secret Key is a bind-mode
  factor. FIDO2-PRF is a bind-mode factor. This is what makes
  hardware-bound vaults genuinely hardware-bound rather than
  "hardware as MFA."

First-party authenticators (in rough buildout order):

| Authenticator               | Mode      | Notes                                                                |
|-----------------------------|-----------|----------------------------------------------------------------------|
| `PasswordAuthenticator`     | bind      | Argon2id; for VLT01-style local vaults                                |
| `OpaqueAuthenticator`       | bind      | Server-side aPAKE — server never holds an offline-attackable verifier |
| `Srp6aAuthenticator`        | bind      | Older PAKE; needed for Proton / 1P compatibility                      |
| `TotpAuthenticator`         | gate      | RFC 6238                                                              |
| `HotpAuthenticator`         | gate      | RFC 4226                                                              |
| `EmailOtpAuthenticator`     | gate      | Pluggable mailer transport                                            |
| `SmsOtpAuthenticator`       | gate      | Pluggable SMS transport                                               |
| `WebAuthnAuthenticator`     | gate      | RFC 9580 / WebAuthn L3 — signature on challenge                       |
| `WebAuthnPrfAuthenticator`  | bind      | FIDO2 hmac-secret / PRF — derives key contribution                    |
| `PasskeyAuthenticator`      | gate+bind | Resident credential; passwordless flow                                |
| `OidcAuthenticator`         | gate      | OIDC / JWT, JWKS-verified                                             |
| `MtlsAuthenticator`         | gate      | Client X.509 cert validated against trust anchor                      |
| `AwsStsAuthenticator`       | gate      | Vault-style: client sends pre-signed `GetCallerIdentity`              |
| `GcpJwtAuthenticator`       | gate      | Signed metadata-server JWT                                            |
| `AzureMiAuthenticator`      | gate      | Azure managed identity token                                          |
| `KubernetesSaAuthenticator` | gate      | SA token verified against API-server JWKS                             |
| `AppRoleAuthenticator`      | gate      | role_id + secret_id; secret_id often delivered via response-wrapping  |
| `ShamirQuorumAuthenticator` | bind      | k-of-n share reconstruction (Vault-style unseal)                      |
| `DuoAuthenticator`          | gate      | Push notification                                                     |
| `LdapAuthenticator`         | gate      | LDAP bind                                                             |
| `KerberosAuthenticator`     | gate      | SPNEGO                                                                |

Authenticators **compose**. A principal may be required to satisfy
`{password, AND (TOTP OR WebAuthn-PRF), AND fresh-IP-policy}`.
Composition rules live one layer up (VLT06 policy) so the
authenticator trait stays atomic.

The "key contribution" output of each bind-mode authenticator feeds
HKDF to derive the next layer's key:

```text
   derive_unlock_key = HKDF(
       ikm = password_kdf_output || secret_key || webauthn_prf_secret || …,
       salt = vault_id,
       info = "vault/v1/unlock",
   )
```

Ordering and presence of contributions is part of the vault
manifest (so the same set of factors always derives the same key).

Depends on: VLT01, VLT03 (custodians are storage for credential
material). Used by: every transport.

### VLT06 — Policy engine ✅ shipped (SimpleRbacEngine + decorators)

Authentication says *who*. Policy says *what they can do*. This
layer is also pluggable — one project's "RBAC with three roles" is
another project's "HashiCorp HCL with capability strings" is
another's "Cedar" or "Rego."

```rust
trait PolicyEngine {
    fn decide(&self, ctx: &PolicyContext) -> Decision;
}

struct PolicyContext<'a> {
    principal: &'a Principal,
    action:    &'a Action,            // read / write / delete / list / decrypt / sign / issue / …
    resource:  &'a ResourceRef,        // (namespace, key) or (engine, role, …)
    factors:   &'a [AuthAssertion],    // which factors backed the session
    time:      Instant,
    metadata:  &'a Metadata,           // IP, device_id, geo, …
}

enum Decision { Allow, Deny(Reason), AllowWith(Constraints) }
```

First-party engines: `SimpleRbacEngine` (roles × resources;
fits a Bitwarden-class app); `HclEngine` (HashiCorp's `path
"secret/data/*" { capabilities = [...] }` syntax); `CedarEngine`
(AWS Cedar); `RegoEngine` (OPA bindings). Decorators add quorum
("two admins must approve"), time-of-day, MFA-required-for-action,
break-glass auditing.

Depends on: VLT05 (factor list flows in).

### VLT-CH — Secure channel ✅ shipped

Sits between identity (VLT05 produces the identity keys) and
distribution (VLT11 transports carry the wire bytes). The
**continuous-key-rotation** layer the user asked for: channel
takeover at time T does not compromise messages sent before T
(forward secrecy), and after the next DH ratchet step does not
compromise messages sent after T either (post-compromise
security).

Composes the already-shipped `coding_adventures_x3dh` (Signal-
style initial key agreement using identity + signed-prekey +
optional one-time-prekey) and `coding_adventures_double_ratchet`
(per-message DH ratchet + KDF chain) crates into one ergonomic
wrapper:

```rust
ChannelInitiator::open(my_identity, peer_bundle, plaintext, aad)
    -> (Channel, FirstMessage)
ChannelResponder::accept(first_msg, my_identity, my_spk, my_opk?,
                         sender_ik_pub, aad)
    -> (Channel, plaintext)
Channel::send(plaintext, aad)    -> wire bytes
Channel::receive(wire, aad)      -> plaintext
```

Wire format:
- First message: `"C1" || ek_pub(32) || dr_header(40) || ct_len(4 BE) || ct`
- Subsequent: `"CN" || dr_header(40) || ct_len(4 BE) || ct`

Caller-supplied AAD is passed through to the ratchet AEAD so
the ciphertext is bound to application context (e.g.
`vault_id || record_id`).

Out of scope for v1: PreKeyBundle distribution (a server / sync
concern, VLT10 territory), sealed-sender / metadata-private
envelopes, multi-device fan-out orchestration. Spec:
[VLT-CH-vault-secure-channel.md](./VLT-CH-vault-secure-channel.md).

### VLT07 — Leases

Every machine-vault feature with TTLs eventually wants the same
thing: a lease manager. Issue a lease, attach a TTL, attach a
revocation hook, optionally renew, optionally chain (parent
revocation propagates to children).

This is also where **response wrapping** lives — a single-shot,
TTL-bound capability token that yields a payload exactly once.
Vault's killer primitive for "secure introduction": broker fetches a
wrap token, hands it to a workload, workload exchanges it for the
real secret without the broker ever seeing plaintext. Crypto-wise
it's just a random ID indexing a temporary cubbyhole; the value
comes from the lifecycle guarantees.

```rust
trait LeaseManager {
    fn issue(&self, payload: LeasePayload, ttl: Duration,
             on_revoke: RevokeHook) -> Result<LeaseId>;
    fn renew(&self, id: &LeaseId, extra: Duration) -> Result<()>;
    fn revoke(&self, id: &LeaseId) -> Result<()>;
    fn lookup(&self, id: &LeaseId) -> Result<LeaseInfo>;
}
```

A background reaper revokes expired leases (via the hook).

Depends on: VLT01 (cubbyhole is just a sealed namespace), VLT06
(revocation requires policy).

### VLT08 — Dynamic-secret engines

The machine-vault soul. A *secret engine* is a plugin that
produces secrets on demand (instead of holding them at rest).
Issuing a secret = run the engine's mint operation, wrap the result
in a lease (VLT07), return.

```rust
trait SecretEngine {
    fn mount_path(&self) -> &str;
    fn mint(&self, role: &Role, ctx: &MintContext) -> Result<MintedSecret>;
    fn revoke(&self, secret_ref: &SecretRef) -> Result<()>;
    fn rotate_root(&self) -> Result<()>;
}
```

First-party engines:

- **`KvV2Engine`** — versioned static KV (Bitwarden's primary use).
- **`DatabaseEngine`** — connect to a DB (Postgres / MySQL / Mongo /
  Redis / Cassandra), `CREATE USER ... LIMITED TO ROLE ...`, return
  `{username, password}`, register revocation that drops the user.
- **`AwsEngine`** — STS AssumeRole or IAM user-provisioning; return
  ephemeral AWS credentials.
- **`GcpEngine`** — service-account-key issuance with TTL.
- **`AzureEngine`** — service-principal credential issuance.
- **`PkiEngine`** — internal CA: issue X.509 certs, sign CSRs,
  revoke (CRL + OCSP).
- **`SshEngine`** — issue short-lived SSH client/host certs against
  a CA keypair.
- **`TransitEngine`** — encryption-as-a-service: `encrypt(pt) →
  ct`, `decrypt(ct) → pt`, `sign(msg) → sig`, with the key never
  leaving the engine. AWS-KMS-Encrypt / GCP-KMS Encrypt are the
  same shape externally.
- **`TotpEngine`** — store TOTP seeds on behalf of users (so the
  vault is also their authenticator app).
- **`KubernetesEngine`** — issue scoped K8s service-account tokens.

Each first-party engine is its own crate (`vault-engine-database`,
`vault-engine-pki`, …). Apps include only what they need.

Depends on: VLT01, VLT02, VLT06, VLT07.

### VLT09 — Audit log

Append-only, tamper-evident, hash-chained. Each entry contains
`prev_hash = blake2b(prev_entry || entry_body)`, signed by the
issuer's device key (Ed25519). Audit entries are themselves sealed
records (encrypted under the vault's master key), so a malicious
sync server sees hashes and signatures but not the content — yet
can verify chain integrity.

Pluggable sink: local file, syslog, S3, Sigsum / Trillian
transparency log, Splunk.

Depends on: VLT01, VLT05 (signing key identity).

### VLT10 — Sync engine

Multi-device E2EE sync. Server stores opaque ciphertext + revision
metadata. Per-record version vectors; merge policy is *last-writer-
wins per record* by default with conflicts surfaced for app-level
resolution; CRDT mode opt-in for fields that admit one (e.g.
"tags" as an OR-set).

The server-side companion is a thin reference implementation
(in-memory or sqlite-backed) so the client tests have something to
talk to without booting cloud infrastructure. Real deployments
would put this behind nginx + Postgres + S3; the protocol stays the
same.

Wire shape: `(namespace, key, revision, sealed_record_bytes)` plus
the wrap-set from VLT04. The server cannot tell what kind of record
anything is.

Depends on: VLT01, VLT04, VLT05 (server-side auth), VLT06 (server-
side authorization). Used by VLT11's transports.

### VLT11 — Transports

The vault must be reachable from *outside* a Rust process. Three
transports cover almost every use case:

- **`vault-transport-cli`** — `vault put / get / list / share / sync
  / unseal / login`. The terminal interface. Subsumes `pass`-style
  workflows. Doubles as the daemon control plane.
- **`vault-transport-http`** — REST API, Vault-style + Bitwarden-
  style endpoints. Composes VLT05 (auth-as-middleware) + VLT06
  (policy-as-middleware) + VLT08 (engines exposed as paths). Native
  TLS. WebSocket extension for sync push.
- **`vault-transport-grpc`** — same surface, different wire. Useful
  for SDK clients. Reflection enabled so language bindings are
  auto-generated.

Optional adapters:

- **`vault-transport-fuse`** — mount the vault as a filesystem; each
  file read decrypts on demand. Vault Agent style.
- **`vault-transport-env`** — wrap a subprocess, inject decrypted
  secrets as env vars, scrub on exit.
- **`vault-transport-k8s-csi`** — Kubernetes CSI driver so a pod
  mounts a vault path as a tmpfs volume.
- **`vault-transport-browser`** — WebExtension messaging shim for a
  password-manager browser extension.

Each transport is a thin crate that composes `{auth, policy,
engines, leases, audit}` and exposes them. The vault core is
*headless*; transports are deliberately interchangeable.

Depends on: everything in tiers Crypto/Identity/Lifecycle.

### VLT12 — Revision history

Already specified in original VLT00 v1; carries forward unchanged.
Every `put` archives the prior ciphertext to a sibling history
list. Retention policy is per-namespace. `restore(ns, key, rev)`
brings back an old revision as a new write.

Depends on: VLT01.

### VLT13 — Encrypted search

Local trigram or BM25 index, encrypted under the vault's master
key, synced as a set of vault records. Per-schema field
declarations (VLT02 specifies which fields are searchable). No
server-side search in v1; if needed later, layer SSE on top.

Depends on: VLT01, VLT02.

### VLT14 — Attachments

Streamable encrypted blobs. Per-blob DEK; chunked AEAD framing
(64 KiB chunks, counter-nonce, like age v1). Parent record stores
the blob reference; blob lives in `__vault_blobs__`. Stream API on
read so memory usage stays bounded. Sync upload is a transport
concern (VLT11).

Depends on: VLT01, VLT04, VLT11.

### VLT15 — Import / export

`.1pux`, Bitwarden JSON, KeePassXC `.kdbx`, LastPass CSV, Chrome /
Firefox CSV exports, age files, gpg files, SOPS files. Output: a
versioned portable JSON bundle. Ceremony is explicitly user-driven
because plaintext is touched.

Each importer is a tiny sibling crate (`vault-import-1password`,
`vault-import-bitwarden`, `vault-import-keepass`, …). Depends on
VLT02 (target schemas) and VLT04 (recipient list at export).

## Buildout per reference target

Cross-reference of which layers each target needs:

```text
                                | crypto |  identity |  lifecycle |  distribution
                                | 01 02 03 04 | 05 06 | 07 08 09 | 10 11 12 13 14 15
KeePassXC-class (file, 1 user)  |  ●  ●  ●  ●  |       |          |        ●
age / SOPS / Sealed Secrets     |  ●     ●  ●  |       |          |        ●     ● ●
Bitwarden-class (E2EE pwd mgr)  |  ●  ●  ●  ●  |  ●  ●  |        ●  |  ●  ●  ●  ●  ●  ●
HashiCorp Vault-class (machine) |  ●  ●  ●     |  ●  ●  |  ●  ●  ●  |        ●
AWS Secrets Manager-class       |  ●  ●  ●     |  ●  ●  |  ●     ●  |        ●
SPIFFE/Teleport-class (cert)    |  ●     ●     |  ●  ●  |     ●     |        ●
```

Reading: a Bitwarden clone is sealed-store + records + custody +
recipients + auth + policy + audit + sync + transports + history +
search + attachments + import/export. A Vault clone trades sync /
search / attachments / import-export for leases + dynamic engines
on the same `auth + policy + transports + audit` rails.

## Primitives inventory

What's already in the repo:

| Primitive               | Crate                | Notes                                  |
|-------------------------|----------------------|----------------------------------------|
| ChaCha20-Poly1305       | `chacha20-poly1305`  | + XChaCha20 variant                    |
| Argon2id / -d / -i      | `argon2id` etc.      | RFC 9106                               |
| HKDF                    | `hkdf`               | RFC 5869                               |
| HMAC                    | `hmac`               |                                        |
| BLAKE2b                 | `blake2b`            |                                        |
| SHA-256 / SHA-512       | `sha256` / `sha512`  |                                        |
| Ed25519                 | `ed25519`            |                                        |
| X25519                  | `x25519`             |                                        |
| CSPRNG                  | `csprng`             |                                        |
| Constant-time compare   | `ct-compare`         |                                        |
| Zeroize                 | `zeroize`            |                                        |
| KV w/ CAS               | `storage-core`       |                                        |
| Sealed envelope store   | `vault-sealed-store` | shipped (VLT01)                        |

Greenfield primitives needed:

| Primitive                       | Likely crate                | Used by         |
|---------------------------------|------------------------------|-----------------|
| AES-256-GCM                     | `aes-gcm`                   | VLT01 alt cipher (interop), VLT08 transit |
| RSA-OAEP-2048                   | `rsa`                       | VLT04 (1P-style sharing, Sealed Secrets)  |
| OPAQUE aPAKE                    | `opaque-pake`               | VLT05                                     |
| SRP-6a                          | `srp6a`                     | VLT05 (1P / Proton interop)               |
| WebAuthn server (Level 3)       | `webauthn-rs`-style         | VLT05                                     |
| FIDO2 PRF / hmac-secret         | `fido2-prf`                 | VLT05 bind-mode                           |
| TOTP / HOTP                     | `totp` / `hotp`             | VLT05, VLT08                              |
| Shamir's Secret Sharing         | `shamir`                    | VLT03 distributed-fragment, VLT05 quorum  |
| Canonical CBOR                  | `canonical-cbor`            | VLT02                                     |
| Streaming AEAD (chunked frame)  | `aead-stream`               | VLT14                                     |
| Trigram / BM25 index            | `vault-search-index`        | VLT13                                     |
| OPA / Rego embedding            | `rego`                      | VLT06 optional                            |
| Cedar policy engine             | `cedar` (AWS)               | VLT06 optional                            |
| HCL parser                      | `hcl`                       | VLT06 (Vault-compatible)                  |
| X.509 issuance                  | `x509-issuer`               | VLT08 PKI engine                          |
| SSH cert issuance               | `ssh-cert-issuer`           | VLT08 SSH engine                          |
| Postgres / MySQL / Mongo client | reuse ecosystem crates      | VLT08 database engine                     |
| AWS / GCP / Azure SDK shims     | `vault-engine-aws` etc.     | VLT08 + VLT03 KMS custody                 |
| OPAQUE-3DH transcript helpers   | inside `opaque-pake`        | VLT05                                     |

The greenfield list is long but each item is small in isolation,
and most have respected open-source references to draw from.

## Auth flexibility — the picture in one diagram

The user explicitly called out: "I want the auth mechanisms for the
vault itself to be flexible. I want to be able to require things
like a Yubikey, even be able to use things like Passkeys or any
sort of an external service like SMS two factor or Authentication
One Time Code apps, etc."

That falls out of VLT05 + VLT06 like this:

```text
    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
    │ Authenticator│    │ Authenticator│    │ Authenticator│   (any number,
    │  password    │    │  WebAuthn    │    │  TOTP        │   any kinds —
    │ (bind mode)  │    │ (PRF, bind)  │    │ (gate)       │   plug-in trait)
    └──────┬───────┘    └──────┬───────┘    └──────┬───────┘
           │ KeyContribution   │ KeyContribution   │ AuthAssertion only
           ▼                   ▼                   ▼
    ┌──────────────────────────────────────────────────────┐
    │  HKDF combine (bind-mode contributions only)         │  → unlock key
    │  Policy.require (all factors → AuthAssertions)       │  → admit / deny
    └──────────────────────────────────────────────────────┘
```

A site admin's policy file declares the required factor *graph*:

```hcl
policy "vault-unlock" {
    require all_of {
        password,
        any_of { webauthn_prf, sms_otp },   # YubiKey OR SMS fallback
        if device.untrusted {
            require email_otp,              # extra step on new devices
        }
    }
}
```

The policy engine enforces the graph; the auth layer supplies the
factor implementations. Adding a new factor (Authy push, Duo,
WeChat scan-QR, smartcard) = registering one more `Authenticator`
implementation. Apps swap factors without touching the vault core.

## Access surfaces — the picture in one diagram

```text
       ┌───────────────────────────────────────────────────┐
       │  Application (Bitwarden clone, Vault clone, …)    │
       └────────┬───────────┬───────────┬──────────────────┘
                │           │           │
        ┌───────▼───┐ ┌─────▼─────┐ ┌──▼───────────┐
        │  CLI      │ │  HTTP     │ │  gRPC        │   (VLT11)
        │  binary   │ │  server   │ │  server      │
        └───────┬───┘ └─────┬─────┘ └──┬───────────┘
                │           │          │
                └───────────┼──────────┘
                            ▼
              ┌─────────────────────────────────┐
              │   vault core (headless)         │
              │   composes 01..10 layers        │
              └─────────────────────────────────┘
```

Same vault, three transports. Local-only deployment: only the CLI
binary is built. Daemon deployment: HTTP + CLI control plane (like
`gpg-agent` / `ssh-agent` / `vault-agent`). Server deployment: HTTP
+ gRPC + CLI + browser-extension messaging. The vault core has no
opinion.

## Cross-cutting concerns

### Threat model (whole stack)

- Adversary reads the entire at-rest database.
- Adversary observes all network traffic.
- Adversary controls the sync / API server (honest-but-curious at
  best, malicious at worst).
- Adversary does **not** control a client device while the vault is
  unsealed.
- Adversary cannot exhaust the user's KDF parameters offline (we
  enforce ceilings; we tune for ≥250 ms unseal on modern hardware).

### No key material in logs — ever

Inherited from VLT01. Tests assert it by pattern-matching debug
impls of key-holding types.

### Errors come from literals

No layer derives an error message from persisted bytes. A malicious
server can't inject error strings.

### Testing baseline

Every layer ships with: round-trip, tamper-detection, boundary,
rotation/migration, cross-instance tests. Server-touching layers
ship a reference in-memory server so client tests are hermetic.

## What lives in the application, not the vault

Belongs to whatever product is being built on top, not to the vault
primitive:

- UI shell (TUI / GUI / mobile app / browser extension UI).
- Autofill & form detection.
- OS biometric integration glue (calling the custodian; the
  custodian is in VLT03, but "Touch ID prompt UX" is product code).
- Phishing-resistant URL matching heuristics.
- Password generators (sibling crate; not core).
- Breach monitoring (HaveIBeenPwned API integration).
- Organisation billing, SSO admin UX, SCIM provisioning.

These are real features; they're just not primitives.

## Milestone plan

```text
foundation primitives
[x] shamir       Shamir's Secret Sharing over GF(2^8)        (SSS01)
[x] canonical-cbor   RFC 8949 §4.2.3 deterministic codec    (CBR01)

storage backends
[x] storage-core           opaque KV trait + InMemoryBackend (built-in)
[x] storage-fs (STR01)     local-disk backend with atomic write+rename+fsync
[ ] storage-s3             AWS / S3-compatible
[ ] storage-gdrive         Google Drive
[ ] storage-webdav         WebDAV
[ ] storage-git            git (commits as ciphertext blobs)
[ ] storage-sqlite         single-file SQLite

vault stack
[x] VLT01  sealed store
[x] VLT02  typed records
[x] VLT03  key custody (PassphraseCustodian + TpmCustodian scaffold)
[x] VLT04  recipients (PassphraseRecipient + X25519Recipient)
[x] VLT05  auth (PasswordAuthenticator + TotpAuthenticator)
[x] VLT06  policy engine (SimpleRbacEngine + AllOf/AnyOf/RequireFactor/TimeBound)
[x] VLT-CH secure channel (X3DH + Double Ratchet, continuous key rotation)
[ ] VLT07  leases
[ ] VLT08  dynamic-secret engines (KV-v2 first; database, PKI, AWS, transit follow)
[ ] VLT09  audit log
[ ] VLT10  sync engine
[ ] VLT11  transports (CLI, HTTP, gRPC; FUSE/CSI/env later)
[ ] VLT12  revision history
[ ] VLT13  encrypted search
[ ] VLT14  attachments
[ ] VLT15  import / export
```

Per-layer specs already landed:

- [SSS01-shamir-secret-sharing.md](./SSS01-shamir-secret-sharing.md)
- [CBR01-canonical-cbor.md](./CBR01-canonical-cbor.md)
- [STR01-storage-fs-backend.md](./STR01-storage-fs-backend.md)
- [VLT01-vault-sealed-store.md](./VLT01-vault-sealed-store.md)
- [VLT02-vault-records.md](./VLT02-vault-records.md)
- [VLT03-vault-key-custody.md](./VLT03-vault-key-custody.md)
- [VLT04-vault-recipients.md](./VLT04-vault-recipients.md)
- [VLT05-vault-auth.md](./VLT05-vault-auth.md)
- [VLT06-vault-policy.md](./VLT06-vault-policy.md)
- [VLT-CH-vault-secure-channel.md](./VLT-CH-vault-secure-channel.md)

**First useful checkpoint** (after VLT05 + VLT06 + the CLI half of
VLT11): a local single-user vault with pluggable auth factors,
unlocked from the terminal. Functionally equivalent to `pass` but
on the new stack.

**Second useful checkpoint** (after VLT07 + VLT08 KV-v2 + VLT09 +
HTTP transport): a server-mediated machine vault — minimal
HashiCorp Vault clone that stores static KV, runs leases, audits.

**Third useful checkpoint** (after VLT10 + remaining VLT11 + VLT12
+ VLT13 + VLT14): a usable end-user password manager — minimal
Bitwarden clone with multi-device sync, history, search, and
attachments.

**Fourth checkpoint** (after VLT08 dynamic engines + VLT15): both
products at near-feature-parity.

Each milestone is independently shippable: at any of these
checkpoints there's a real working product on top of the stack.
