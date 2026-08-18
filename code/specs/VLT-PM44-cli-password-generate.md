# VLT-PM44 — CLI Password Generation

## Status

Normative Phase 1B contract for minting a new password that the vault never
stores, through the local CLI and controlling terminal.

`VLT-PM00-local-first-password-manager.md` §23 item 11 bundles four daily-use
conveniences — "password generator, TOTP display, clipboard, attachments and
packing". This document is the generator, and only the generator. TOTP display,
clipboard delivery, and attachments remain separate ceremonies.

§23 item 10c settled which phase the generator belongs to. It did not settle
what the generator *does*: §14.4 names the signature

```text
vault-pm password generate [policy flags] [--copy|--reveal]
```

and says nothing about how strong the result must be or where its randomness
comes from. A password manager whose generator is underspecified in exactly
those two places is a password manager with a latent vulnerability, because
"policy flags" is precisely the surface through which a person can ask for a
password that is too weak to be worth storing. §3 and §4 below close that gap,
and they are the load-bearing half of this contract.

## 1. What this command is, and what it is not

This is a **generate-only** command. It creates one password, delivers it once,
and forgets it. It opens no vault, unlocks nothing, reads no item, writes no
item, and publishes no audit event. It is usable on a machine where
`vault-pm init` has never run.

That is a deliberate scoping choice, not an omission, and three of this
product's own rules force it:

1. **There is nothing to audit.** `VLT-PM15-operation-audit.md` §2 defines the
   audit boundary as "one user-visible application action" against a vault, and
   names the exceptions explicitly: "Locked `status`, help, and grammar
   rejection reveal no vault content and do not require an event." A generation
   reveals no vault content either — there is no vault in the picture. Worse, a
   vault-scoped event *would* be a new disclosure: an audit chain that recorded
   "a password was generated at 14:02" tells a later reader of the chain
   something the vault did not previously know, and correlates that instant
   with whichever item is created next. The audit trail is supposed to make
   vault operations attributable, not to accumulate a diary of things that
   never touched the vault.
2. **An unlock would be a lie about what happened.** Every command that calls
   `authenticated_access` collects the master passphrase. Requiring one here
   would mean prompting for the vault's most valuable secret in order to
   perform an operation that never opens the vault — training the person to
   type the master passphrase at prompts that do not need it, which is the
   habit every phishing attack against a password manager depends on.
3. **`init` must not be a prerequisite.** The single most common moment to want
   a generated password is while signing up for something, which is frequently
   before the vault exists.

**Deferred, deliberately.** `password generate` does not offer to store its
result into a new or existing item. That would make it a mutation — vault
access, unlock, audit event, publication ordering, conflict handling, the
entire `item add` ceremony — with a generated value substituted for one prompt.
It is a strictly larger contract than this one and belongs in its own document.
The composition that exists today is that a person reveals a generated password
and pastes it into the `item add login` password prompt, which is one extra
step and requires nothing new.

## 2. Command surface

```text
vault-pm password generate [policy flags] (--reveal | --copy)
```

where the policy flags are exactly:

```text
--length N
--no-lowercase
--no-uppercase
--no-digits
--no-symbols
--exclude-ambiguous
```

### 2.1 Closed grammar

`generate` is the only verb under the `password` noun. Every flag above may
appear at most once, in any order, and no other token is accepted. A repeated
flag, an unknown flag, a positional argument, an `--flag=value` spelling, or a
missing `--length` value fails before any host work with the invalid class.

`--length N` takes exactly one following argument: one to three ASCII decimal
digits with no sign, no underscore, no whitespace, and no leading zero. `007`
is rejected rather than read as seven, because a grammar that accepts two
spellings of one number is a grammar where a typo can silently mean something
else.

### 2.2 No vault selector

The leading `--vault NAME` selector is **rejected** for this command, exactly as
it is for `init`, `vault create`, and `help`. VLT-PM00 §14.4 permits the
selector on commands "that operate on an existing vault", and this command
operates on none. Accepting and ignoring it would be a small false statement —
the person would have named a target that had no effect on anything.

The interactive shell binds one vault at session start and prefixes every
delegated command with that selector. Because this command refuses the
selector, the shell must delegate `password` **without** prefixing it. That is
the shell's job, not a reason to weaken the grammar: the generator is useful
inside a session, so refusing the verb in the shell would be worse than
teaching the shell that one verb is vault-free. A session's retained
authenticator is not consulted, which is correct — nothing here authenticates.

### 2.3 Exactly one output mode

One of `--reveal` or `--copy` is **required**. Neither, or both, is invalid.

There is no default output mode and no plain-stdout mode, because VLT-PM00
§14.6 makes ordinary output redacted and this command has nothing *but* a
secret to say. A `password generate` that printed to stdout by default would
put a live credential into shell history files, terminal scrollback,
`tee` pipelines, and CI logs the first time anyone redirected it.

`--copy` is **recognized and refused** in this slice, with the unsupported
class and the fixed message `vault-pm: unsupported capability`. No clipboard
adapter exists anywhere in this product — `clipboard_clear_seconds` is a
configuration value with no writer behind it — and building one is §23 item
11's clipboard ceremony, not this one. Recognizing the flag and refusing it is
better than treating it as an unknown token: §14.4 documents `--copy` as part
of this command's signature, so a person who reads the spec and types it
deserves "not yet", not "invalid command". When the clipboard ceremony lands,
this refusal becomes a delivery path and nothing else about this document
changes.

**Amended by `VLT-PM46-cli-clipboard.md`.** The clipboard ceremony has landed
and `--copy` is a delivery path, exactly as the paragraph above predicted:
nothing else in this document changed. The refusal did not disappear, it
narrowed — the check still happens in the same place, before any prompt, and
still returns `unsupported` (exit 8), but its condition is now "this host has
no clipboard" rather than "always". The delivery uses the product default
`clipboard_clear_seconds` of 30 rather than a configured value, because §1 of
this document forbids this command from resolving the platform layout or
opening a config file; VLT-PM46 §6 records that consequence.

## 3. Character classes and the alphabet

Four classes are selected by default. Each `--no-*` flag removes one. Removing
all four is invalid.

| Class | Members | Size |
|---|---|---:|
| lowercase | `abcdefghijklmnopqrstuvwxyz` | 26 |
| uppercase | `ABCDEFGHIJKLMNOPQRSTUVWXYZ` | 26 |
| digits | `0123456789` | 10 |
| symbols | `!#$%&()*+,-.:;<=>?@[]^_{|}~` | 27 |

The symbol set is the 32 printable US-ASCII punctuation characters minus `"`,
`'`, `` ` ``, `\`, and `/`; the space character is printable but is not
punctuation and was never a candidate. The exclusions are not aesthetic. Quote and backslash characters are the
ones that most often survive a round trip through a shell, a CSV import, a JSON
blob, or a hand-written SQL string as *something other than themselves*; a
generated password that a downstream system silently mangles is a password the
person can no longer log in with, and they will not know why. Space is excluded
because it is invisible at both ends of a value and is stripped by a large
fraction of login forms.

`--exclude-ambiguous` removes the six characters that are routinely misread
when a password is copied off a screen or read aloud:

| Removed | Class | Confused with |
|---|---|---|
| `0` | digits | `O` |
| `1` | digits | `l`, `I` |
| `I` | uppercase | `1`, `l` |
| `O` | uppercase | `0` |
| `l` | lowercase | `1`, `I` |
| `|` | symbols | `l`, `I`, `1` |

Every removed character belongs to exactly one class, so the flag can never
empty a selected class. The resulting sizes are lowercase 25, uppercase 24,
digits 8, symbols 26; all four classes selected gives 83 rather than 89.

### 3.1 No forced class inclusion

Every character is drawn independently and uniformly from the whole selected
alphabet. The generator does **not** guarantee that at least one character of
each selected class appears.

This is the security-relevant choice it looks like, and it is made in the
strong direction. A "must contain at least one digit" rule is a constraint on
the output, and constraining the output of a uniform sampler always removes
entropy — the sampler is no longer uniform over |alphabet|^length, so the
entropy claim in §4 would stop being true and would have to be replaced by a
smaller number that is much harder to state exactly. Since the floor in §4 is
enforced against that claim, an inflated claim would let an under-strength
policy through. Independent uniform sampling is the one construction whose
strength can be stated exactly, so it is the one whose strength can be
enforced exactly.

The practical cost is nil at the default: the chance that a 24-character draw
from the 89-character alphabet omits the digits entirely is
`(79/89)^24 ≈ 5.7 × 10^-2`… which is *not* nil, and stating it honestly is the
point. A person facing a site that demands a digit re-runs the command, or
narrows the alphabet with `--no-symbols` so the site accepts the result at all.
A "minimum one of each class" flag is a reasonable later addition; it belongs
with an entropy accounting that is correct for constrained sampling, and this
slice does not pretend to have one.

## 4. The entropy floor and the randomness source

### 4.1 Where the randomness comes from

Every byte consumed by this command comes from the operating-system CSPRNG,
reached through the path this product already uses for vault key material:

```text
password generate
  └─ CliHost::fill_entropy
       └─ coding_adventures_vault_pm_cli_host::OsEntropy::fill
            └─ coding_adventures_csprng::fill_random
                 └─ getrandom(2) / getentropy(2) / BCryptGenRandom
```

No general-purpose RNG is acceptable here and none is used. `coding_adventures_csprng`
is a fail-closed wrapper with no fallback chain: if the OS source is
unavailable it returns an error, which this command surfaces as the provider
class rather than substituting a weaker source. A generated password is
long-lived key material for whatever it protects, and the same reasoning that
makes a vault root key come from the OS CSPRNG makes this come from the OS
CSPRNG.

### 4.2 Uniform selection without modulo bias

Naïvely reducing a random word modulo the alphabet size is biased toward the
low end of the alphabet, and although the bias is tiny for a 64-bit word it is
avoidable at no cost, so it is avoided.

Randomness is consumed as 8-byte big-endian words. For an alphabet of size `n`,
the acceptance region is `floor(2^64 / n) * n`, the largest multiple of `n`
that fits in a 64-bit word. A word inside the region selects
`alphabet[word mod n]`; a word outside it is **discarded** and the next word is
read. This is exactly uniform, not approximately uniform.

The command therefore reserves `(length + 8) * 8` bytes in one call: one word
per character plus eight spare words. The reserve is not decoration. Discarding
is what makes the sampler exact, and a sampler that can discard must be able to
run out. With `n ≤ 89` the chance that any one word is discarded is at most
`88 / 2^64 ≈ 4.8 × 10^-18`, so exhausting eight spares is far below any
probability worth naming; if it ever happened the command **fails** with the
provider class rather than falling back to a biased draw. Failing closed on an
impossible event is cheap. Silently becoming biased on it is not.

### 4.3 The floor

**The minimum is 80 bits of entropy, and the command refuses to generate below
it.**

The entropy of one generated password is exactly `length × log2(|alphabet|)`
bits, which §3.1 is what buys. The refusal is evaluated as an exact integer
comparison — `|alphabet|^length ≥ 2^80` — rather than in floating point,
because a security boundary decided by rounding is a security boundary that
differs between platforms.

80 bits is chosen against the threat this product actually faces, which is not
online guessing but **offline** attack on a credential database that leaked
from the far end. Sites that store passwords under a fast unsalted hash are
attacked today at roughly `10^12`–`10^14` guesses per second on commodity GPU
farms. `2^80 ≈ 1.2 × 10^24`, which is about 380 years at `10^14/s`; the next
power of ten of attacker capability still leaves decades. Below 80 the margin
collapses quickly rather than degrading gracefully — 64 bits is under a day at
`10^14/s` — which is what makes 80 a floor rather than a preference.

The floor is not the recommendation. The default is 24 characters over all four
classes, which is **155 bits**, comfortably past the 128-bit bar at which the
password stops being the weakest part of any system it is typed into. The floor
exists only to catch policies a person deliberately narrowed, and it is
deliberately set low enough that it never argues with a reasonable request.

Minimum lengths implied by the floor, for the policies people actually ask for:

| Policy | Alphabet | Minimum length | Bits at that length |
|---|---:|---:|---:|
| all four classes | 89 | 13 | 84.2 |
| all four, `--exclude-ambiguous` | 83 | 13 | 82.9 |
| `--no-symbols` | 62 | 14 | 83.4 |
| `--no-digits --no-symbols` | 52 | 15 | 85.5 |
| `--no-uppercase --no-symbols` | 36 | 16 | 82.7 |
| lowercase only | 26 | 18 | 84.6 |
| digits only (a PIN) | 10 | 25 | 83.0 |
| digits only, `--exclude-ambiguous` | 8 | 27 | 81.0 |

Two rows are worth reading twice. A 12-character password using every class is
77.7 bits and **is refused**, one character short; the deficit is real and the
remedy costs one keystroke. And "digits only" needs 25 of them, which is the
floor correctly reporting that a numeric PIN is not a password — the command
will produce one, but only at a length that actually earns the name.

There is no override flag in this slice. A site with a hostile maximum-length
cap is a real problem and deserves a real answer — a flag that names the cap,
says out loud how many bits the result has, and refuses to pretend otherwise —
and that answer is a later addition. Shipping the escape hatch before the floor
would mean shipping only the escape hatch.

Length is additionally bounded to `1 ≤ N ≤ 128`. The upper bound exists so the
one entropy reservation this command makes is bounded (at most 1088 bytes), not
because 128 characters is meaningful.

## 5. Ceremony and ordering

```text
1. parse and validate the policy      -- no host call yet
2. refuse if the policy is under the floor
3. host.confirm_secret_reveal()       -- exact lowercase `yes`
4. host.fill_entropy(reserve)         -- OS CSPRNG, one call
5. generate                           -- uniform draw, §4.2
6. host.write_revealed_text(password) -- controlling terminal only
7. wipe
```

The order is the contract, not an implementation detail.

**Validation precedes every host call** (steps 1–2 before 3), so a request that
was never going to be honoured costs no prompt, no terminal interaction, and no
entropy.

**Confirmation precedes generation** (step 3 before 4–5). VLT-PM00 §14.6
requires an interactive TTY confirmation before `--reveal`, and this command
uses the same fixed prompt and the same exact-`yes` rule as
`VLT-PM25-cli-secret-reveal.md` §3 rather than inventing a second confirmation
ceremony with its own wording. Putting it first has a property the item-reveal
ordering cannot have: on refusal **no password is ever created**. There is no
secret to wipe because none existed. A generator that minted a value and then
discarded it on refusal would be strictly worse for no benefit, since unlike
`item reveal` there is no audit event that needs to be published before the
answer is known.

**Delivery is terminal-only** (step 6). The generated password never enters
`CliOutput`, process stdout, process stderr, argv, an environment variable, a
configuration file, a `Debug` rendering, or any cloneable value owned by the
command result. It is written by the same host adapter `item reveal` uses,
which reopens the controlling terminal directly, quotes and control-escapes the
value so it cannot inject terminal control sequences, and wipes its temporary
buffers. One line is written:

```text
Secret: "QUOTED AND ESCAPED VALUE"
```

Ordinary stdout and stderr are empty on success. Nothing echoes the requested
length, the alphabet, or the entropy bits — those are properties of a live
credential and belong in this document, not in a transcript.

**Wiping** (step 7). The entropy reserve and the generated password are both
held in wipe-on-drop buffers for their whole lifetime and are wiped when the
command ends, including on the failure paths. The password string is allocated
once at its exact final capacity so that appending a character can never
reallocate and strand an unwiped copy of a prefix on the heap.

A terminal write that fails after step 6 began is reported with the provider
class. Unlike `item reveal`, there is no truthfulness problem to resolve: no
event claimed anything, and the value is wiped either way.

## 6. Failure classes

| Situation | Class | Exit |
|---|---|---:|
| unknown verb, unknown/repeated flag, bad `--length` spelling, missing or doubled output mode, `--vault` selector, all classes disabled, length outside 1–128 | invalid | 2 |
| policy below the 80-bit floor | invalid | 2 |
| confirmation answered with anything but `yes` | invalid | 2 |
| `--copy` | unsupported | 8 |
| OS CSPRNG unavailable | provider | 7 |
| entropy reserve exhausted by discards (§4.2) | provider | 7 |
| controlling terminal unavailable or unwritable | provider | 7 |
| no audited terminal adapter for the platform | unsupported | 8 |

The floor refusal carries its own fixed, payload-free message —
`vault-pm: password policy below the minimum entropy floor` — rather than the
generic invalid-command text. It is the one rejection in this command that a
person will hit while doing something entirely reasonable, and "invalid
command" would send them looking for a typo that is not there. The message
still names no length, alphabet, or bit count: it says which rule was broken,
and this document says what the rule is.

## 7. Where the code lives

The generator's policy and sampler are a **pure** library,
`coding_adventures_vault_pm_password_policy`, with no clock, storage, terminal,
process, or entropy source of its own. It validates a policy, states how many
bytes that policy needs, and turns caller-supplied bytes into a password. It
cannot generate anything by itself, which is what makes every property in §3
and §4 testable against fixed byte vectors rather than against a random source.

The CLI owns the two things that are not pure: reserving randomness from the
host, and delivering the result to the controlling terminal. The application
layer is not involved at all, for the reasons in §1.

## 8. Acceptance gates

The slice is complete only when tests prove:

1. the grammar accepts exactly the flags in §2 and rejects unknown flags,
   repeated flags, positional arguments, `--flag=value`, a missing `--length`
   value, non-canonical numbers, neither output mode, both output modes, and a
   leading `--vault` selector;
2. the alphabet for every class combination is exactly the table in §3, and
   `--exclude-ambiguous` removes exactly the six characters in §3, leaving
   every selected class non-empty;
3. the floor accepts and refuses exactly at the boundary in §4.3, checked on
   both sides of every row of that table, by exact integer comparison;
4. generation from a fixed byte vector is deterministic and reproducible, every
   output character is in the selected alphabet, no excluded character appears,
   and the output has exactly the requested length;
5. a word in the rejection region is discarded rather than reduced, and an
   exhausted reserve fails rather than falling back;
6. over many samples the output is not obviously non-random: every alphabet
   member appears, no member is wildly over-represented, and consecutive
   outputs do not repeat or cycle;
7. `--reveal` delivers only through the terminal adapter, with empty stdout and
   stderr, and a refused confirmation delivers nothing at all;
8. `--copy` fails unsupported, and an unavailable CSPRNG fails provider, with
   no value delivered in either case;
9. the real executable, driven under a pseudo-terminal, receives the password
   on `/dev/tty` while its captured stdout stays empty, refuses an
   under-strength policy, and never writes a generated value to any ordinary
   stream; and
10. formatting, Clippy, rustdoc, and the policy/CLI/executable test suites pass.

## 9. References

### Internal

- `VLT-PM00-local-first-password-manager.md` §2.1, §14.4, §14.6, §14.7, §23
  items 10c and 11 — command surface, secret-output policy, exit classes, and
  the phase resolution this document implements.
- `VLT-PM15-operation-audit.md` §2 — the audit boundary, and the named
  exceptions that put this command outside it.
- `VLT-PM25-cli-secret-reveal.md` §3, §5 — the confirmation prompt and the
  terminal delivery adapter reused unchanged.
- `VLT-PM40-cli-interactive-shell.md` — the session host that must delegate
  this verb without a vault selector.

### Code

- `code/packages/rust/vault-pm-password-policy` — the pure policy and sampler.
- `code/packages/rust/vault-pm-cli` — grammar, ceremony, and composition.
- `code/packages/rust/vault-pm-cli-host` — `OsEntropy`, `confirm_secret_reveal`,
  `write_revealed_text`.
- `code/packages/rust/csprng` — the operating-system CSPRNG wrapper.
