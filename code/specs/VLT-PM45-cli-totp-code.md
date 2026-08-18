# VLT-PM45 — Audited TOTP Code Display

## Status

Normative Phase 1B contract for computing and disclosing the *current* one-time
code for one stored TOTP item, through the local CLI and controlling terminal.

`VLT-PM00-local-first-password-manager.md` §23 item 11 bundles four daily-use
conveniences — "password generator, TOTP display, clipboard, attachments and
packing". `VLT-PM44-cli-password-generate.md` shipped the generator. This
document is the **TOTP display**, and only the display. Clipboard delivery and
attachments remain separate ceremonies.

Depends on: VLT05 (`vault-auth`), VLT-PM15, VLT-PM22, VLT-PM25, VLT-PM29,
VLT-PM40.

## 1. Why this slice exists

`VLT-PM29-cli-totp-create.md` shipped the ability to *store* a TOTP seed, and
its §1 explicitly excluded "code generation/display". VLT-PM25 shipped an
audited reveal of the raw Base32 seed, which exists so a person can
re-provision a second device.

Neither of those is the reason anyone puts a TOTP seed in a password manager.
The reason is the six digits on the screen right now. Until this command
exists, the product can accept a shared secret and hand it back, and can do
nothing with it — which is the exact shape of a feature that looks finished
from the outside and is not.

Out of scope, and each for its own reason:

- **A live refreshing display.** Deferred to a follow-up contract; see §8.
- **Clipboard delivery.** No clipboard adapter existed anywhere in this product
  when this document was written; `--copy` was recognized and refused. See
  §2.3, and `VLT-PM46-cli-clipboard.md`, which has since supplied one.
- **`otpauth://` parsing, QR scanning, HOTP counters, issuer discovery.**
  VLT-PM29 §1 excluded them from creation and this document does not reopen
  them: a command that reads a stored record cannot be the place where new
  provisioning formats are introduced.
- **Clock correction.** See §4.4.
- **Verification of a code the person types in.** `vault-auth` already has
  `TotpAuthenticator::verify_at_time`. Nothing in the password-manager product
  is a TOTP *verifier*, and this command does not make it one.
- **Codes from a historical revision or a losing conflict candidate.** A TOTP
  code is a fact about *now*; a historical seed produces a code only if the
  seed never changed, in which case the current revision produces the same one.
  A conflicted item is refused outright (§6).

## 2. Command surface

```text
vault-pm [--vault NAME] totp code ITEM (--reveal | --copy)
```

### 2.1 Grammar

`ITEM` is the existing uppercase canonical item selector, parsed by
`ItemId::from_user_string`. The optional leading `--vault NAME` selector is
command-scoped and never rewrites `default_vault`, exactly as VLT-PM22
established; unlike `password generate`, this command *does* open a vault, so
the selector is meaningful and is accepted.

The verb is closed: `totp` followed by exactly `code`, followed by exactly one
item selector, followed by exactly one output flag. A missing item, a lowercase
item, a second item, an unknown flag, a repeated flag, both output flags, a
missing output flag, or any option-like argument fails as `invalid command`
before host preparation. The command accepts no secret, no revision, no output
destination, no provider option, no confirmation flag, and no bypass.

### 2.2 The output flag is required

VLT-PM00 §14.4 originally wrote the tail as `[--copy|--reveal]`. This contract
narrows the brackets to parentheses and amends §14.4 to match, for the same
reason VLT-PM44 §2 gave for the generator: the interesting half of this
command's output is a live credential, and a default that printed it would put
it into shell history, terminal scrollback, `tee` pipelines, and CI logs the
first time anyone redirected the command. There is no third arm and in
particular no "print to standard output" arm.

The two flags are not symmetric in what they cost to get wrong, and requiring
the person to name one means a delivery is never chosen on their behalf.

### 2.3 `--copy` is recognized and refused

`--copy` parses, and then fails with the stable `unsupported` class (exit 8)
**before any prompt, unlock, clock reading, or entropy reservation**. No
clipboard adapter exists in this product; `clipboard_clear_seconds` is a
configuration value with no writer behind it. VLT-PM00 §14.4 documents the
flag, so a person who types it deserves "not yet" rather than "invalid
command", and deserves it without first typing their master passphrase.

This is deliberately identical to VLT-PM44 §2.3. When a clipboard adapter
lands, it lands once, and both commands stop refusing on the same day.

**Amended by `VLT-PM46-cli-clipboard.md`.** The adapter landed, once, and both
commands stopped refusing on the same day. The check keeps its position — still
before any prompt, unlock, clock reading, or entropy reservation — and still
returns `unsupported` (exit 8); only its condition narrowed from "always" to
"this host has no clipboard". Everything else in this document is untouched,
and that is the point: §3's ceremony, §3.1's outcome table, §4.1's two clock
readings, and §5.2's non-secret validity line are identical under `--copy`. The
one visible change is the confirmation prompt, which reads "Copy secret to this
system's clipboard?" rather than naming a terminal the value is not going to.
The clear delay is the selected vault's configured `clipboard_clear_seconds`,
which this command — unlike `password generate` — already holds, because it
opened the vault.

## 3. The audit decision

**This command publishes a full `ItemRead` audit event under the same
publish-before-release discipline as `item reveal`.** It is not a lighter
ceremony. That is not a judgement call this document had to make; VLT-PM15 §2
already made it, in a sentence that names this exact operation:

> An `item show`, future secret reveal, clipboard copy, autofill, history show,
> attachment export, **TOTP display**, browser fill, or API retrieval is an
> access.

The tempting argument for a lighter treatment is real and worth writing down so
that it is visibly rejected rather than merely unconsidered: a six-digit code is
valid for about thirty seconds and, unlike the seed, does not let whoever sees
it produce the *next* code. Its blast radius is genuinely smaller than the
seed's.

That argument is about the **consequence** of a disclosure, and the audit trail
is a record of the **fact** of one. Three things follow:

1. **An access log whose completeness depends on how long the disclosed value
   stays useful is not an access log.** The question the trail must answer is
   "which items were read on this vault, when, by which device" — and it must
   answer it for every read, or a reader cannot tell a complete history from a
   filtered one.
2. **A TOTP code is the second factor.** An attacker who already has the
   password needs exactly this and nothing else. "Short-lived" describes a
   limit on their convenience, not on whether the disclosure happened.
3. **The read is item-scoped and revision-bound.** Producing the code requires
   decrypting the current live revision of a specific item. That is the same
   traversal `item reveal` performs, and there is no version of it that touches
   less.

### 3.1 Event shape

One item-scoped `AuditActionV1::ItemRead` event, with VLT-PM25 §4's outcomes
unchanged:

| Situation | Outcome | Selected revision |
|---|---|---|
| refusal at the confirmation prompt, or a host failure collecting the answer | `Denied` | none |
| missing, tombstoned, or conflicted item | `Failed` | none |
| item is not a TOTP record | `Failed` | exact current revision |
| stored TOTP parameters this build cannot compute (§6.1) | `Failed` | exact current revision |
| code computed | `Succeeded` | exact current revision |

The event contains **no code value, no digit count, no period, no remaining
validity, no algorithm, no label, no issuer, no secret length, no confirmation
answer, no terminal identity, no vault name, no device name, no provider
detail, no path, and no arbitrary error text**. It records that a code was
viewed, not what it was — the same rule every other reveal ceremony follows.

The remaining-validity figure is deliberately excluded even though it is not
secret on its own (§5.2). An audit row already carries an advisory timestamp;
adding the period phase would let a reader of the chain recompute the exact
step boundary the code belonged to, which is one step closer to the code than
the chain needs to be.

Audit publication failure withholds both the code and the original operation
error, retaining the ordinary exact recovery journal.

### 3.2 Confirmation ceremony

Unchanged from VLT-PM25 §3. After unlock, the host writes the fixed prompt

```text
Reveal secret on this terminal? Type yes to continue:
```

and only exact lowercase `yes` authorizes release. Empty input, EOF, other
text, terminal unavailability, or validation failure never authorizes it.
Explicit refusal publishes `Denied`; a host failure while collecting the answer
also publishes `Denied` before the payload-free host error is returned.

Wrong-passphrase and pre-authentication time/entropy failures release no code
and do not claim that an item access occurred.

## 4. Time

TOTP is a function of the clock, so the clock is part of this contract rather
than an implementation detail.

### 4.1 Two readings, not one

The command takes the wall clock **twice**, and they are not interchangeable:

| Reading | Taken | Used for |
|---|---|---|
| audit timestamp | before authentication, alongside the audit randomness | the `ItemRead` event's advisory timestamp |
| code time | after unlock and after confirmation, immediately before computation | the TOTP step and the remaining-validity figure |

The first reading is reserved pre-authentication because VLT-PM15's ceremony
requires the complete advisory time and randomness for an attempt to exist
before the attempt does.

The second reading exists because the first one is **stale by construction**.
Between them sit an Argon2id key derivation and a human being reading a prompt
and typing three letters. Several seconds is the ordinary case and a whole
thirty-second period is entirely reachable. A code computed from the
pre-authentication reading would therefore routinely be the *previous* code —
correct-looking, six digits, and rejected by the site. Using one reading for
both purposes is the single most likely way for this command to be subtly and
intermittently wrong, so the two readings are named separately here.

### 4.2 Step derivation

Per RFC 6238 with `T0 = 0`:

```text
unix_seconds = floor(code_time_ms / 1000)
T            = floor(unix_seconds / period)
code         = HOTP(seed, T) mod 10^digits, zero-padded to digits
```

`period`, `digits`, and the HMAC algorithm come from the stored record, never
from a flag. VLT-PM29 §2 already constrains them at creation (algorithm in
`SHA1`/`SHA256`/`SHA512`, digits in `{6, 8}`, period in `1..=3600`); §6.1 says
what happens when a record nevertheless arrives carrying something else.

Milliseconds convert to seconds by flooring, which is the same truncation every
TOTP client performs and keeps the boundary at the second, not at a fraction of
one.

### 4.3 The system clock is the time source, and that is stated

The code time is the host's ordinary wall clock, through the same
`CliHost::now_ms` seam every other command uses. There is no NTP query, no
time-server round trip, no stored offset, and no drift estimate.

This matches how every real TOTP client behaves, and it means **TOTP
correctness depends on the host clock being reasonably accurate**. A host whose
clock is more than a period out of step will produce codes the far side
rejects, and this command cannot detect that: it has no correct answer to
compare against, and inventing one would require exactly the network round trip
a local-first product declines to make.

The failure is at least legible — every code is rejected, rather than some — so
it is documented here and surfaced nowhere at runtime. Extending
`vault-pm doctor` with a clock-sanity reading is a reasonable follow-up and is
not in this slice.

### 4.4 No skew window

`vault-auth::TotpAuthenticator` carries a `window` parameter for accepting a
code that arrives a step early or late. That is a *verifier's* tolerance. A
generator has no use for it: there is exactly one current step, and offering a
neighbouring one would be offering a code that is already spent or not yet
live. This command computes the window-free current step.

## 5. Output

### 5.1 The code goes only to the controlling terminal

The computed code is delivered through the unchanged VLT-PM00 §14.6 reveal
path: the native host reopens the controlling terminal or attached console and
writes one quoted, control-escaped line. It never enters `CliOutput`, process
standard output, process standard error, argv, standard input, an environment
variable, configuration, a file, a URL, a `Debug` rendering, or a cloneable
string owned by the command result.

The code is zero-padded to the record's digit count, because `042311` and
`42311` are different strings to paste and only one of them is the code.

### 5.2 The validity window goes to ordinary standard output

Standard output on success carries exactly one line:

```text
Code valid for N more seconds
```

where `N = period - (unix_seconds mod period)`, an integer in `1..=period`.

This is on stdout, not the terminal channel, because **it is not secret**. It
is a function of the clock and the stored period alone; anyone who can read the
clock can compute it, and it discloses nothing about the seed or the code. The
opposite arrangement — hiding a public number on the private channel — would
make the command's non-secret output invisible to a script and buy nothing.

`N` exists because a person who receives `123456` with two seconds left will
type it into a form that rejects it and will blame the vault. Reporting the
window lets them decide to wait rather than discovering it after the fact.

### 5.3 Ordering, and the honest imprecision in `N`

The successful order is fixed: compute → publish the `ItemRead` event durably →
write the code to the terminal → return the standard-output line. Publication
before release is VLT-PM15's rule and is unchanged here.

`N` is computed from the code time, which is read *before* publication. The
disk write and the terminal write therefore sit between the measurement and the
person, so `N` can be a few milliseconds optimistic. This is documented rather
than compensated. Compensating would mean reading the clock again after the
secret was already released, and a clock failure at that point would be a
failure the command cannot honestly report — the code is out, the event says
`Succeeded`, and there is nothing to roll back. A number that is generous by
milliseconds is a better trade than an error path that exists only to refine
it.

### 5.4 No automatic waiting

When `N` is small the command reports the small number and returns. It does not
sleep until the next step, and it does not silently hand back the *next* code.

Sleeping would hold an unlocked vault open, with decrypted seed material live
in the process, for a duration chosen by nobody. Returning the next step's code
would mean returning a code that is not valid yet, which is worse than
returning one that is nearly expired. Re-running the command is one line of
shell and re-authorizes the access — which, given §3, is the correct thing for
a second disclosure to do.

## 6. Errors

| Situation | Class | Exit |
|---|---|---|
| malformed grammar, refusal at the prompt, or a non-TOTP item | invalid | 2 |
| wrong passphrase | locked | 3 |
| missing or tombstoned item | not found | 4 |
| current conflict on the item | conflict | 5 |
| authenticated corruption | integrity | 6 |
| time, entropy, terminal, audit publication, or terminal write unavailable | provider | 7 |
| `--copy`, unsupported platform, or uncomputable stored parameters (§6.1) | unsupported | 8 |

A non-TOTP item is `invalid` rather than `not found` for the same reason
VLT-PM25 §6 makes a field/schema mismatch invalid: the item exists and was
read, and the request was the wrong shape for it.

### 6.1 Stored parameters this build cannot compute

The record's `algorithm` is a `String` and its `digits`/`period` are integers.
VLT-PM29 validates them at the CLI input boundary, but the *codec* does not, so
a record can reach this command carrying anything a future writer or a portable
import produced.

Such a record is not corrupt — it decoded and authenticated cleanly — so
`integrity` would be a lie. It is a capability this build does not have, so the
class is `unsupported` (exit 8), and the audit event is `Failed` bound to the
exact current revision. The computation fails closed; there is no fallback to
`SHA1`, no clamp of the digit count, and no substituted period. Silently
computing a code under parameters other than the ones stored would produce six
plausible digits that are simply wrong, which is the worst available outcome.

## 7. Reuse: `vault-auth` owns RFC 6238

The RFC 6238 engine is **not** written for this slice. VLT-PM00 §6's reuse map
already assigns "password and TOTP factors" to `vault-auth`, and
`vault-auth::TotpAuthenticator` already implements RFC 4226 dynamic truncation
and RFC 6238 step derivation with `T0 = 0`.

This contract requires two closures in that package, both of which its own
source already anticipated in comments:

1. **Algorithm parameterisation.** The authenticator was hard-wired to
   HMAC-SHA-1. VLT-PM29 stores `SHA1`, `SHA256`, or `SHA512`, so the
   authenticator gains an explicit algorithm selector. It is a required
   constructor argument rather than a defaulted one: a security-relevant
   parameter with a silent default is a parameter that gets silently wrong.
2. **A zero-padded rendering.** The authenticator returned an integer. Display
   needs the padded string, and it needs it in a wipe-on-drop buffer rather
   than a plain `String`, so the rendering belongs beside the computation
   rather than in a caller.

A defect found while closing them is fixed in the same change: the modulus was
computed as `10u32.pow(digits)` while `digits` was permitted up to 10, and
`10^10` exceeds `u32::MAX`. That path panicked in debug builds and wrapped in
release ones. The modulus is now computed in `u64`. This product never reaches
it — VLT-PM29 caps digits at 8 — but a shared engine that panics on a
documented-legal argument is a defect regardless of who calls it.

`vault-pm-application` gains a dependency on `vault-auth` and computes the code
**inside the application boundary**. The decoded seed bytes never cross into
CLI orchestration; what crosses is the finished code. This is stricter than
`item reveal totp-secret`, which by its nature must hand the seed out, and it
is the reason this command is not implemented as "reveal the seed, then compute
in the CLI".

## 8. Deferred: the live refreshing display

An interactive display that redraws the code as each step turns over is a
genuinely useful thing and is **explicitly deferred**, not overlooked.

It is a larger contract than this one, and the reasons are structural rather
than a matter of effort. A loop that holds the vault unlocked across many steps
has to answer to VLT-PM40's idle-lock bound; it must decide whether every
redraw is a fresh `ItemRead` (an audit chain that grows without bound while a
person watches a screen) or whether one authorization covers a session (a new
concept the audit vocabulary does not currently have); it needs terminal raw
mode, redraw, resize, and interrupt handling that no other command in this
product needs; and it must define what happens to the displayed code when the
idle bound expires underneath it.

Each of those is a decision with a defensible answer and none of them is
implied by this document. A one-shot command that prints the current code and
its remaining validity is complete, correct, and useful on its own, and it is
the whole of this slice.

## 9. Acceptance gates

The slice is complete only when tests prove:

1. the grammar accepts exactly `totp code ITEM (--reveal|--copy)`, with and
   without the leading named-vault selector, and rejects a missing item, a
   lowercase item, extra arguments, unknown flags, repeated flags, both output
   flags, and a missing output flag;
2. `--copy` returns `unsupported` with no prompt, no unlock, no clock reading,
   and no audit event;
3. the TOTP computation reproduces the **RFC 6238 Appendix B test vectors** for
   SHA-1, SHA-256, and SHA-512 at every published timestamp, in the published
   digit width, and the six-digit truncation of the same vectors;
4. the code changes exactly at the period boundary — equal across an entire
   step, different across the transition, and identical again one full period
   later — verified at the exact boundary second and at both adjacent seconds;
5. zero-padding is preserved for a step whose truncation has leading zeroes;
6. the reported remaining validity is `period - (unix_seconds mod period)` for
   every second of a period, is never `0`, and is never greater than `period`;
7. refusal and confirmation-input failure publish `Denied` before their errors
   and release no code;
8. non-TOTP, missing, tombstoned, and conflicted selections publish `Failed`
   with no code, and an uncomputable stored algorithm publishes `Failed` and
   returns `unsupported`;
9. success publishes exactly one `ItemRead` `Succeeded` bound to the exact
   current revision, before host delivery, and survives restart;
10. the audit rows contain no code, algorithm, digit count, period, remaining
    validity, label, or issuer, and the code appears in no `Debug` rendering;
11. the real PTY executable receives the code only on `/dev/tty` while captured
    standard output contains only the validity line, and two runs inside one
    period return the same code; and
12. formatting, Clippy, rustdoc, and the package tests of the affected
    dependency closure pass.
