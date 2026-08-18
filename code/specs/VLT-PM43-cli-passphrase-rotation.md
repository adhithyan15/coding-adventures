# VLT-PM43 — Passphrase Rotation Without Re-encrypting Item Bodies

## Status

Normative Phase 1A contract for changing a vault's master passphrase.

Closes VLT-PM00 §23 item 10b and the §14.8 acceptance criterion it was filed
against:

> Password rotation rewraps the VRK without re-encrypting every item body.

That sentence had never been exercised, because nothing in the product
performed a rotation. VLT-PM42 §Status recorded the finding; this document is
the ceremony that answers it.

It does not close Phase 1A on its own. §23 item 10c — the `password generate`
contradiction between §14.4 and §23 — is unrelated and remains open.

## 1. What the criterion is actually asking for

§14.8's wording is a *performance and blast-radius* claim, not merely a
functional one. Read it as two separate promises:

1. **Functional.** A person who wants a different master passphrase can get
   one, and afterwards only the new passphrase opens the vault.
2. **Structural.** Getting one costs a fixed, small amount of work and touches
   a fixed, small number of bytes — regardless of whether the vault holds three
   logins or thirty thousand.

The second promise is the interesting one, and it is the one that is easy to
break by accident. A password manager that derived item encryption from the
master passphrase would have to decrypt and re-encrypt every record on every
password change: minutes of work, a window in which a crash leaves half the
vault under each key, and a strong incentive for a user never to rotate. That
is the design this product rejected in VLT-PM00 §8.1 before writing any code:

> The master password must not be the vault's long-lived root key.
> Initialization draws a random 256-bit Vault Root Key (`VRK`). A
> passphrase-derived key wraps the VRK. Changing the password replaces only the
> root-key wrap.

So the structural promise is already *true of the key hierarchy*. What was
missing is a ceremony that exercises it and a test that measures it. A property
nothing performs is a property nothing tests.

## 2. The layer the rotation lives at, and the layer it does not

The relevant chain as shipped, from `vault-pm-application`:

```text
   master passphrase
         |  argon2id(BootstrapV1.kdf: memory, iterations, lanes, 16-byte salt)
         v
   passphrase KEK  ── XChaCha20-Poly1305 ──▶ BootstrapV1.passphrase_root_wrap
         |                                     (AAD "VPM-ROOT-WRAP-v1" ‖ suite ‖ vault_id)
         v  unwraps
   VRK (random 256-bit)
         |  HKDF-SHA-256, salt = vault_id, closed ASCII purpose labels
         +──▶ locator key      — the opaque repository address
         +──▶ object wrap key  — wraps every per-object DEK
         +──▶ local state key  — the owner-private local secret
         +──▶ audit key
                    |
                    v
         per-object random DEK ──▶ one ObjectFrameV1 (item revision,
                                    catalog, certificate, commit, audit event)
```

Everything below the VRK is reached *only* through the VRK. The passphrase
appears in exactly one place on disk: the 32-byte ciphertext of
`BootstrapV1.passphrase_root_wrap`, plus the `Argon2idParametersV1` beside it.
No item revision, catalog, commit, device certificate, or audit event is bound
to the passphrase, directly or transitively.

Therefore a rotation is, in full:

1. derive a new KEK from the new passphrase with a **fresh random 16-byte
   salt** and the host's current Argon2id policy;
2. unwrap the VRK under the old KEK;
3. re-wrap the same VRK under the new KEK with a **fresh random 24-byte
   nonce**;
4. publish the new `BootstrapV1` — same `vault_id`, same
   `authority_public_key`, `generation + 1`, `previous_bootstrap = Some(old
   id)`, re-signed by the vault authority — and durably supersede the old one.

Steps 1–3 are one Argon2id derivation, one AEAD open, and one AEAD seal on 32
bytes. Step 4 writes two small records and deletes one. None of it is a
function of vault size. That is the O(1) claim, stated as an operation count
rather than as an aspiration.

### 2.1 There was no lower-layer primitive to expose

`VLT01-vault-sealed-store.md` describes KEK rotation without re-encrypting
bodies as a property of the *sealed store* family, and `vault-sealed-store` and
`vault-key-custody` are separate crates in this repository. `vault-pm` does not
build on them. Its envelope layer is `vault-pm-format`'s `ObjectFrameV1` and
`BootstrapV1`, its crypto is `vault-pm-application::crypto` and
`::initialize`, and its root wrap is created by `initialize::wrap_root_key` and
opened by `initialize::unwrap_root_key`. Those two private functions are the
entire pre-existing rotation machinery, and they were written for generation
zero and for unlock respectively.

So this contract adds a *workflow*, not a cryptographic capability. It
introduces no new algorithm, suite, label, AAD, key type, or wrap format. It
calls `wrap_root_key` with a different passphrase and a fresh salt, and it
re-signs the bootstrap with the authority key that
`ActiveStateV1::local_secret` already holds.

## 3. Command surface

```text
vault-pm [--vault NAME] passphrase rotate
```

`passphrase` is a new top-level noun consistent with §14.4's
`noun verb` shape (`item add`, `history restore`, `audit verify`). The verb is
`rotate` rather than `change` because the operation is defined by what it does
to the wrap, and because `password` is reserved by §14.4 for the generator.

No flag carries a passphrase, an old passphrase, a file, or a policy. §14.5's
prohibition is absolute: no master passphrase is accepted through argv, an
environment variable, command history, a URL, or config.

### 3.1 The interactive shell refuses it

`VLT-PM40-cli-interactive-shell.md` §4 refuses `init`, `vault`, `shell`, and
`--vault` because each would aim a retained authenticator somewhere the person
never authenticated. `passphrase` joins that list for a sharper reason: a shell
session **retains the passphrase it collected at the start**, and a successful
rotation invalidates exactly that value. A session that permitted a rotation
would either keep using a passphrase that no longer opens the vault — turning
every subsequent command into an authentication failure the person cannot
explain — or would have to silently adopt the new one, which is a retained
secret the session never prompted for and cannot re-confirm.

Refusing is not a limitation of the shell; it is the shell declining to hold a
credential whose meaning changed underneath it. A person rotates from a
one-shot invocation, which collects and discards its own secrets.

## 4. The ceremony

```text
  1  resolve the configured vault and acquire the cross-process writer lock
  2  collect the CURRENT passphrase                          (hidden prompt)
  3  unlock, finishing any interrupted publication first     (VLT-PM42)
  4  collect and confirm the NEW passphrase                  (hidden, twice)
  5  prepare, in memory:
        - unwrap the VRK under the current passphrase
        - prove it is the VRK this session is already using
        - build and authority-sign the next BootstrapV1
  6  publish the audit event                                 (audit-only commit)
  7  durably rotate:
        a. journal PendingRotation
        b. install the new bootstrap generation and advance "latest"
        c. delete the superseded generation record
        d. install the intended Active owner state
  8  report success; drop every key
```

Steps 2–4 are the host's; 5–7 are the application's.

### 4.1 The current passphrase is required, and it is required twice

Step 3 unlocks with it. Step 5 unwraps the VRK with it again, because an
`UnlockedVaultV1` retains the *derived subkeys* and not the VRK — deliberately,
since nothing else in the product needs the root. Rotation is the one operation
that does, so it pays one extra Argon2id derivation rather than lengthening the
lifetime of the root key in every other session. That is the same trade
VLT-PM42 §4 makes for recovery, for the same reason.

Step 5 then proves the two agree, without ever comparing key bytes: it derives
`V1Keys` from the freshly unwrapped VRK, opens `ActiveStateV1::local_secret`
with them, and requires the result to equal the session's local secret. An
AEAD that opens under the derived local-state key and yields the identical
owner secret is proof that the root behind it is the same root. A mismatch is
`IntegrityFailure`, not `AuthenticationFailed`: the passphrase authenticated
once already, so a disagreement here means persisted state is inconsistent.

### 4.2 The new passphrase is confirmed before anything durable happens

Step 4 uses the same collect-and-confirm host boundary `init` uses, with the
same constant-time comparison and the same `New vault passphrase:` /
`Confirm vault passphrase:` prompts. A mismatch fails before step 5, so a
mistyped new passphrase cannot become the only passphrase.

Rejecting a *weak* new passphrase is out of scope: this product has no
passphrase policy, and inventing one here would be a policy decision smuggled
in through a rotation ceremony.

### 4.3 Audit ordering

When the vault has entered its audit epoch, the rotation event is published
**before** the rotation takes effect, through the existing audit-only
publication path — the same `publish_audited_access` machinery every other
audited ceremony uses, over the same crash-resumable `PendingPublication`
journal.

The action is `AuditActionV1::PassphraseRotate`, label `passphrase_rotate`,
registry code 6. The event carries no item, revision, or selected identity, and
in particular carries no salt, no KDF parameters, no generation number, and no
bootstrap identifier: an audit chain is not a place to publish the shape of a
person's key material.

Failures are audited the way exports are:

| When the failure happens | Recorded as |
|---|---|
| the new passphrase cannot be collected or confirmed | `passphrase_rotate` / `failed`, then the host error |
| preparation fails (§4.1 mismatch, invalid policy) | `passphrase_rotate` / `failed`, then the error |
| the durable rotation fails after a published `succeeded` event | nothing further; see §5.3 |

Audit publication failure supersedes and withholds the original error, exactly
as `audited_export_portable_with_passphrase` does, because an unaudited effect
is worse than a reported one.

A vault that has not enabled auditing rotates with no event, as it performs
every other operation with no event.

## 5. Durability

### 5.1 The two durable stores this touches

- **Owner-private local state** (`LocalStateStore`) — one exact byte value per
  locator, replaced only by compare-exchange.
- **Bootstrap store** (`BootstrapStore`) — an immutable per-generation record
  keyed by bootstrap ID, plus a mutable `latest` pointer holding one ID.

The immutable repository is not touched by the rotation at all, except by the
audit event of §4.3, which is an ordinary commit under unchanged keys.

### 5.2 A new owner state, and why an implicit one will not do

`LocalVaultStateV1` gains a fourth variant:

```rust
PendingRotation {
    active: ActiveStateV1,   // the last stable state, bootstrap_id = old
    bootstrap: Vec<u8>,      // the exact signed next BootstrapV1
}
```

Its `intended_active` is derived, not stored: it is `active` with
`bootstrap_id` replaced by `id(bootstrap)`. Storing it would let the journal
disagree with itself.

Decoding validates every relation before the value exists: the bootstrap must
decode, verify under its own authority signature, name the same `vault_id`,
carry the same `authority_public_key` fingerprint the active state pins,
declare `generation = active_generation + 1`, and declare
`previous_bootstrap = Some(active.bootstrap_id)`.

The journal is not optional. Consider the alternative — put the new generation
first and update local state after:

```text
   put_generation(new)          ✔ latest = new bootstrap
   ✗ CRASH
   compare_exchange(active)     ✘ never ran; owner state still pins the OLD id
```

The next open loads the latest bootstrap, compares its ID against the pinned
`bootstrap_id`, and fails closed with `IntegrityFailure` — which is exactly
what that check is for, and exactly the wrong answer here. The vault would be
openable by neither passphrase, and the only way out would be to weaken the pin
check that provides rollback and fork detection. An explicit journal keeps the
pin check absolute.

### 5.3 The point of no return, and which way recovery goes

The journal write is the commit point. Before it, the vault is committed to the
old passphrase; at and after it, to the new one.

That gives every landing point a determinate answer:

| Crash lands | Durable state | Which passphrase opens the vault |
|---|---|---|
| before 7a | `Active` (old) | old |
| between 7a and 7b | `PendingRotation` | **new**, after recovery rolls forward |
| between 7b and 7c | `PendingRotation`, latest = new | **new**, after recovery rolls forward |
| between 7c and 7d | `PendingRotation`, old generation gone | **new**, after recovery rolls forward |
| after 7d | `Active` (new) | new |

**Recovery always rolls forward, and needs no passphrase.** This is the
property that makes the ceremony safe. Every remaining step is a function of
the journal alone: the exact signed bootstrap bytes are in it, the superseded
ID is in it, and the intended `Active` bytes are computed from it. Replay is
idempotent — `put_generation` accepts the identical already-installed
generation as success, `supersede_generation` is idempotent on an absent
record, and each compare-exchange re-reads and accepts the value it intended to
write.

Rolling *back* was considered and rejected. It would have to decide, without a
passphrase, whether the person still knows the old one; it would have to
un-install an immutable generation record; and it would have to explain why a
person who was just asked to type a new passphrase twice ends up with the old
one still in force. Rolling forward has none of those problems and one crisp
rule a person can hold: *once the machine accepted your new passphrase, it is
your passphrase.*

The recovery runs at the vault-open boundary, in the same place and by the same
rule VLT-PM42 established. `status` reports `recovery_required`; `doctor`
reports `recovery_required` with exit class 5; both stay read-only. Every
authenticated command finishes the rotation on the way in, then opens the
repaired vault through the ordinary strict open, and prints the same
payload-free `vault-pm: recovered an interrupted write` line on standard error.

A person who types the *old* passphrase into that first post-crash command gets
the rotation finished — recovery consumes no passphrase — and then a clean
`authentication required`, because the vault is now genuinely rotated. That is
accurate rather than confusing: the message names the state the vault is in.

### 5.4 Superseding the old wrap is part of the ceremony, not a cleanup

The bootstrap store keeps every installed generation under its own immutable
key, so advancing `latest` alone would leave the previous
`passphrase_root_wrap` — a wrap of the *same, unchanged* VRK under the *old*
KEK — sitting on disk indefinitely. Anyone who later obtained a copy of the
state directory and the old passphrase could unwrap the VRK from that record
and derive every subkey. The person's rotation would have accomplished nothing
against the one adversary they most likely had in mind.

So step 7c is normative. `BootstrapStore` gains:

```rust
fn supersede_generation(
    &self,
    locator: BootstrapLocator,
    superseded: BootstrapId,
) -> Result<(), BootstrapStoreError>;
```

with a hard refusal: an implementation must fail with `Conflict` if
`superseded` is the generation `latest` currently names. Deleting the live
bootstrap would brick the vault, and the guard makes that unreachable rather
than merely unintended. Deleting a record that is already gone is success.

This does not weaken the generation chain. `previous_bootstrap` still names the
predecessor by hash, so the chain remains linked and a rollback remains
detectable; what is removed is the ability to *re-open the vault through* a
retired credential. Nothing in the product reads a non-latest generation.

Two honest limits, stated rather than argued away:

- **Ordinary backups are out of scope.** A copy of the state directory taken
  before the rotation still contains the old wrap. That is inherent to backups
  of any encrypted store and is the same caveat VLT-PM00 §19.3 already carries
  for recovery artifacts.
- **Filesystem forensics are out of scope.** `supersede_generation` unlinks a
  file; it does not overwrite media. VLT-PM00 §7.2 already places media-level
  recovery outside the first production profile.

### 5.5 Key hygiene

`Zeroizing` containers already cover the passphrases, the derived KEK, the
unwrapped VRK, and both `V1Keys` sets, and each is dropped at the end of the
smallest scope that needs it. Specifically: the KEK derived from the *old*
passphrase lives only inside `unwrap_root_key` and is wiped when that call
returns, before the new wrap is computed. The VRK exists only for the span
between the unwrap and the re-wrap plus the §4.1 binding check.

The prepared rotation value carries the next bootstrap's *public signed bytes*
and nothing secret. It is safe to journal because it already is, by
construction, a public record.

## 6. Failure classes

| Situation | Application error | CLI exit |
|---|---|---|
| wrong current passphrase | `AuthenticationFailed` | 3 |
| new passphrase confirmation mismatch, or prompt unavailable | host `Invalid` | 2 |
| vault not initialized, or no configuration | `NotInitialized` | 2 |
| owner state is `PreparedInit` | `InvalidInput` | 2 |
| VRK does not bind to the session's local secret | `IntegrityFailure` | 6 |
| bootstrap store conflict, or a superseded-is-latest refusal | `IntegrityFailure` | 6 |
| another local writer won a compare-exchange | `ConcurrentHost` | 5 |
| store unavailable | `StorageUnavailable` | 7 |

Every one of them leaves the vault openable: by the old passphrase before the
journal is durable, by the new one after.

## 7. Verification

The acceptance gates below are the ones this contract is measured by. A gate
that only proves rotation "works" is not enough for §14.8; the structural half
needs its own evidence.

1. **No item body is re-encrypted.** Snapshot every byte of the encrypted
   object repository before the rotation and after it, and require the two
   trees to be *byte-identical* — same file set, same contents. This is the
   direct measurement of §14.8's clause. It is stated over the whole object
   root rather than over sampled records so that no future change can
   re-encrypt "just one" thing and pass.

   The rotation of a vault whose audit epoch is active publishes one audit
   commit, which does add objects. The gate is therefore taken over a
   pre-audit vault, where the rotation's repository footprint is exactly zero,
   and the audited case is covered separately by gate 2.

2. **The rotation itself is audited before it takes effect**, and the event's
   rendered row carries only closed field names.

3. **The old passphrase no longer opens the vault and the new one does**,
   proved through the real executable over a pseudo-terminal, across a process
   restart.

4. **A wrong current passphrase changes nothing.** Exit class 3, and the old
   passphrase still opens the vault afterwards.

5. **A `SIGKILL` inside the rotation is clean or resumable.** Using the
   VLT-PM41 harness against the real executable, at minimum at the two most
   dangerous landing points — inside the durable bootstrap swap and inside the
   supersession — the vault must come back openable by exactly one passphrase,
   never by both and never by neither, with `status` and `doctor` reporting a
   state the next ordinary command finishes.

6. **The unit layer covers the roll-forward recovery directly**, including
   replay idempotence and the refusal to delete the live generation.

## 8. What this contract deliberately does not add

- No recovery-code, escrow, or additional `recovery_wraps` entry. The field
  exists in `BootstrapV1` and is carried across the rotation unchanged.
- No KDF re-calibration verb. The rotation adopts the host's current Argon2id
  policy, which is how a person gets stronger parameters, but there is no way
  to change parameters *without* changing the passphrase.
- No authority-key rotation and no device re-enrollment. Those are separate
  operations on separate keys, and VLT-PM00 §15.4 owns them.
- No rotation of the VRK itself. Rotating the root would re-encrypt every body,
  which is the precise thing §14.8 requires this ceremony not to do.
- No non-interactive or scripted mode.

## 9. References

### Internal

- `VLT-PM00-local-first-password-manager.md` §8.1, §8.2, §14.4, §14.5, §14.8,
  §23 item 10b
- `VLT-PM05-application.md` §7, §8
- `VLT-PM15-operation-audit.md`
- `VLT-PM40-cli-interactive-shell.md` §4
- `VLT-PM41-cli-crash-fault-matrix.md`
- `VLT-PM42-cli-pending-publication-recovery.md`

### Code

- `code/packages/rust/vault-pm-application/src/rotate.rs`
- `code/packages/rust/vault-pm-application/src/state.rs`
- `code/packages/rust/vault-pm-application/src/initialize.rs`
- `code/packages/rust/vault-pm-application-storage-core/src/lib.rs`
- `code/packages/rust/vault-pm-audit/src/lib.rs`
- `code/packages/rust/vault-pm-cli/src/lib.rs`
- `code/programs/rust/vault-pm-cli/tests/local_cli_e2e.rs`
- `code/programs/rust/vault-pm-cli-drill/tests/crash_fault_matrix.rs`
