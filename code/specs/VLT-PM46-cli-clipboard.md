# VLT-PM46 — Host Clipboard Delivery with a Verified Timed Clear

## Status

Normative Phase 1B contract for delivering one already-authorized secret to the
platform clipboard instead of to the controlling terminal, and for clearing that
clipboard value again after the configured timeout.

`VLT-PM00-local-first-password-manager.md` §23 item 11 bundles four daily-use
conveniences — "password generator, TOTP display, clipboard, attachments and
packing". `VLT-PM44-cli-password-generate.md` shipped the generator and
`VLT-PM45-cli-totp-code.md` shipped the TOTP display. Both of them parse
`--copy` and then refuse it with the `unsupported` class, because no clipboard
adapter existed anywhere in this product. This document is the **clipboard
adapter**, and the wiring that makes those two `--copy` modes stop refusing.
Attachments and packing remain a separate ceremony.

Depends on: VLT-PM00 §14.6, §23 item 7c-3; VLT-PM07; VLT-PM08; VLT-PM15;
VLT-PM25; VLT-PM40; VLT-PM44; VLT-PM45.

## 1. Why this slice exists, and what it is *not*

VLT-PM00 §14.6 says `--copy` is the **preferred** secret-output mode, and §7's
config schema has carried `clipboard_clear_seconds` since VLT-PM07 — a value
with a validator, a default, a round-trip test, and **no writer**. Two shipped
commands document a flag that fails. A product whose preferred output path is
the one path that does not exist is not a product with a missing feature; it is
a product whose documentation is wrong.

This slice changes exactly one thing: **the final delivery step of a disclosure
that has already been authorized.** It is emphatically not a new disclosure
path. §3 states that as a rule and §9 gates it with tests, because "we added a
way to get the secret out" is precisely the change that quietly escapes an audit
ceremony.

Out of scope, each for its own reason:

- **`item reveal --copy`, `item show --copy`, `history show --copy`.** VLT-PM00
  §14.4 names those signatures, but they are VLT-PM25's ceremonies, not this
  one's. The adapter this document specifies is general; wiring it into a third
  and fourth command is a grammar change to those commands and belongs with
  them. §8.1 records what such a slice would have to add.
- **Clipboard *reading* as a product feature.** The adapter reads the clipboard
  only to decide whether it is still allowed to clear it (§5.2). Nothing in this
  product imports, parses, or stores a clipboard value.
- **Attachments and packing.** §23 item 11's remaining half.
- **Windows.** §4.4 explains why V1 fails closed there rather than shipping a
  clear it cannot verify.
- **A clipboard *manager* integration** (`clipmenu`, KDE Klipper, macOS
  clipboard history apps). §7.4 states plainly that this product cannot defeat
  one, and that saying so is better than implying otherwise.

## 2. Command surface

No new user-facing verb, and no change to either shipped grammar. `--copy`
already parses on both commands:

```text
vault-pm password generate [policy flags] (--reveal | --copy)
vault-pm [--vault NAME] totp code ITEM (--reveal | --copy)
```

What changes is that `SecretOutputMode::Copy` now has a behaviour instead of a
refusal.

### 2.1 One new command, and it is not for people

```text
vault-pm clipboard clear
```

This is the second half of `--copy`: the process `vault-pm` spawns *on itself*
to perform the timed clear after the original one-shot process has exited (§4).
It reads its parameters — a delay, a random salt, and a commitment to the copied
value — from **process standard input**, and it takes no arguments.

It is listed in `USAGE` rather than hidden. A closed grammar with a secret verb
in it is not a closed grammar, and this product's whole posture is that the
surface is exactly what the usage text says it is. Typed by hand with no input
it reads zero bytes, fails the exact-length check in §4.3, and exits `2`. It
opens no vault, resolves no platform layout, takes no writer lock, prompts for
nothing, discloses nothing, and publishes no audit event.

### 2.2 Why standard input, in a product that refuses standard input

`VLT-PM08-cli-host.md` establishes that secret input never comes from process
stdin — a redirected stdin must not be able to inject a master passphrase. That
rule is untouched: **no passphrase, record secret, or copied value is ever read
from stdin.** What `clipboard clear` reads from stdin is a 73-byte fixed-length
parameter block containing a delay, a salt, and a SHA-256 commitment.

Standard input is used here *because* argv is visible. On a shared host any user
can run `ps` and read another process's command line. A commitment to a
six-digit TOTP code in argv would be brute-forced in microseconds, so a design
that passed the verification material on the command line would leak exactly the
secret it exists to protect. The parent writes the block into an anonymous pipe
it created; nothing about it is observable to a third party.

## 3. The ceremony is unchanged; only the channel differs

**`--copy` is not a new disclosure path.** For both commands, every step of the
existing ceremony happens, in the existing order, with the existing audit
consequences. The only difference is the last one:

| Step | `--reveal` | `--copy` |
|---|---|---|
| grammar, item resolution | identical | identical |
| audit time/randomness reservation (`totp code`) | identical | identical |
| unlock | identical | identical |
| interactive confirmation | required | required (§3.1) |
| durable `ItemRead` publication before release (`totp code`) | identical | identical |
| final delivery | controlling terminal | platform clipboard |
| non-secret stdout (`totp code` validity line) | identical | identical |

In particular, for `totp code --copy` the event is the same item-scoped
`AuditActionV1::ItemRead`, with VLT-PM45 §3.1's outcome table unchanged, and the
event still records **that** a code was viewed and never the code. Refusal at
the prompt still publishes `Denied`. `password generate --copy` still publishes
nothing, because VLT-PM44 §1 established that minting a password that no vault
ever stores is not a vault access.

### 3.0 The disclosure intent stays `InteractiveReveal`, and a trap is disarmed

`SecretDisclosureIntentV1` has a `Clipboard` variant, and `--copy` deliberately
does **not** use it. The variant would name the channel more precisely, but this
contract's entire claim is that the ceremony is the same one, and the audit
trail records the *fact* of an access rather than the door it left by — the same
reasoning VLT-PM45 §3 used to reject a lighter treatment for a short-lived code.
A reader of the chain learns that an item was read, which is what the chain is
for.

That variant was, however, a trap, and this is the slice that would have sprung
it: it authorized **unconditionally**, with no confirmation flag, and was
reachable only from tests. The obvious "improvement" — switching `--copy` to the
intent that names clipboards — would have silently deleted the application-layer
confirmation gate while looking like an increase in fidelity. It now carries
`confirmed` and enforces it exactly as `InteractiveReveal` does. A destination
is not an authorization, and a secret placed where every process in the session
can read it is not the disclosure that needs *less* consent.

### 3.1 The confirmation prompt tells the truth

The reveal prompt reads:

```text
Reveal secret on this terminal? Type yes to continue:
```

Reusing it for a clipboard copy would be a false statement to the person being
asked to consent — the secret is not going to that terminal, it is going
somewhere that other processes in their session can read. So `--copy` gets its
own fixed prompt:

```text
Copy secret to this system's clipboard? Type yes to continue:
```

Same ceremony, same exact-lowercase-`yes` rule, same `Denied` outcome on refusal
or on a host failure collecting the answer. Only the sentence differs, and it
differs because the two sentences describe different consequences. A consent
ceremony that misdescribes what it is consenting to is worse than no ceremony,
because it manufactures a record of an agreement nobody made.

### 3.2 Availability is checked first, before anything is spent

For both commands, `--copy` first asks the host whether a clipboard delivery is
possible at all (§4.1's detection, which spawns nothing and reads no clipboard).
If it is not, the command fails with `unsupported` (exit 8) **before any prompt,
unlock, clock reading, entropy reservation, or audit event** — exactly where the
old blanket refusal sat.

This is the same argument VLT-PM44 §2.3 and VLT-PM45 §2.3 made for the refusal
itself: a person who asked for a delivery this host cannot perform deserves to
be told immediately, not after typing their master passphrase. The position of
the check is preserved on purpose; only its condition narrowed, from "always"
to "when this host has no clipboard".

## 4. The adapter

### 4.1 The write is a pre-installed platform utility, fed on standard input

The clipboard is not reachable from portable Rust, and this repository's
standing rule is minimal, dependency-free platform integration rather than
third-party FFI. The adapter therefore spawns a utility the platform already
ships or the person already installed, and writes the value to **that process's
standard input**.

The secret is **never** passed as a command-line argument. `ps`, `/proc/*/cmdline`,
process accounting, and any auditd rule watching execve see the tool name and
its fixed flags and nothing else. This is not a refinement; it is the reason the
adapter is shaped this way, and §9 gates it with a test that asserts the exact
argument vector.

| Session | Write | Read | Clear |
|---|---|---|---|
| macOS | `pbcopy` | `pbpaste` | `pbcopy` with empty input |
| Wayland (`WAYLAND_DISPLAY` set) | `wl-copy` | `wl-paste --no-newline` | `wl-copy --clear` |
| X11 (`DISPLAY` set), `xclip` present | `xclip -selection clipboard` | `xclip -selection clipboard -o` | `xclip -selection clipboard` with empty input |
| X11 (`DISPLAY` set), `xsel` present | `xsel --clipboard --input` | `xsel --clipboard --output` | `xsel --clipboard --delete` |
| anything else | — | — | — |

Selection is ordered: Wayland before X11 (a Wayland session commonly also
exports `DISPLAY` for XWayland, and the native selection is the correct one),
and `xclip` before `xsel` only because it is the more commonly installed of two
equivalent tools.

### 4.2 Tools are resolved from a fixed trusted directory list, not `PATH`

A tool is used only if it is found in `/usr/bin` or `/bin`, probed in that
order, **and only if the file found there is a root-owned regular file with no
group- or other-write bit**. `PATH` is never consulted.

The second half of that sentence exists because the first half alone would be a
claim about a host's layout rather than something this product checks. A
symbolic link planted in a trusted directory, an image where `/usr/bin` is not
in fact root-owned, or a root-owned binary a group member may replace would each
satisfy "it is in `/usr/bin`" while defeating the point of saying so. The probe
therefore does not follow links and inspects owner and mode. A narrow
time-of-check/time-of-use window remains before the `execve`; winning it
requires the ability to replace a file inside a root-owned, non-world-writable
directory, which is already root.

`PATH` is caller-controlled. Resolving through it would mean that anyone who can
prepend a directory to `PATH` — an inherited environment, a compromised shell
profile, a wrapper script — gets handed a live credential on the standard input
of a program of their choosing. That is a complete compromise of this feature
delivered through a mechanism nobody would think to audit. The two directories
above are root-owned on every mainstream Unix. `/usr/local/bin` is deliberately
excluded: it is the conventional home of locally-installed software and is
group- or user-writable on a meaningful fraction of real machines, which is
exactly the property that disqualifies it.

The cost is stated rather than hidden: a `wl-copy` installed only under
`/usr/local/bin`, `/opt`, or a Nix profile is **not found**, and `--copy` fails
closed with `unsupported` on that host. Failing closed on a trust question is
the correct direction to fail.

### 4.3 The timed clear survives process exit by re-executing this same binary

`vault-pm` is a one-shot process (VLT-PM00 §14.5): it prompts, does one thing,
wipes, and exits. A clear that must happen thirty seconds later therefore has
nothing to happen *in*. Three mechanisms were considered:

| Mechanism | Rejected because |
|---|---|
| clear inside the interactive shell only (VLT-PM40) | makes the product's *preferred* output mode work only in its *optional* session mode; the one-shot invocation is the common case |
| block the foreground process for the delay | holds the terminal hostage for the configured timeout, and a `Ctrl-C` — the obvious response — cancels the clear, so the failure mode is "the secret stays in the clipboard exactly when the person was impatient" |
| spawn a detached process that performs the clear | **chosen** |

A detached background process holding secret material in its own memory is a
real cost and is the obvious objection to the chosen mechanism. **This design
does not hold the secret.** The child is given three things:

```text
delay_seconds : u32       the configured timeout
salt          : [u8; 32]  fresh OS-CSPRNG bytes, per copy
digest        : [u8; 32]  SHA-256(salt || copied value)
```

It never receives the value. The strongest statement available about the digest
is worth making precisely, because a six-digit TOTP code has only 10^6
preimages and a salt does not change that: **an attacker who reads the child's
memory can recover a low-entropy copied value.** What that attacker gains is
nothing, because for the entire lifetime of that child the same value is sitting
in the system clipboard, readable by any process in the same session with no
memory-reading privilege at all. The child exists for exactly the window during
which the clipboard already holds the secret, and it exits at the moment it
stops holding it. It adds no exposure window, and the salt keeps two copies of
the same password from producing the same digest, so a leaked digest is not an
oracle for "is this credential still in use".

The child is `std::env::current_exe()` — this same binary, re-executed as
`vault-pm clipboard clear`. It is not a sibling helper binary discovered next to
the executable: that would introduce a directory-lookup step an attacker could
target, to launch a program of their choosing with a commitment on its standard
input. Re-executing the file that is already running introduces no lookup at
all.

The parameter block is 73 bytes and is checked exactly:

```text
offset  0  4   magic   "VPMC"
offset  4  1   version 0x01
offset  5  4   delay   big-endian u32, 1..=3600
offset  9  32  salt
offset 41  32  digest
```

Any other length, magic, version, or an out-of-range delay is
`invalid clipboard clear request` (exit 2). The delay bound is VLT-PM07's own
`clipboard_clear_seconds` range, restated at the process boundary because a
boundary that trusts its input is not a boundary.

The child is fully detached: it `fork`s once more and lets the intermediate exit
immediately, so the grandchild is orphaned to `init` and the original process
leaves no zombie behind — which matters because `vault-pm shell` (VLT-PM40) is
long-lived and would otherwise accumulate one per copy. It calls `setsid`, so
closing the terminal window does not `SIGHUP` the pending clear away. Its
standard output and standard error are `/dev/null`, so it can never write to the
terminal it was launched from. And it arms `alarm(delay + 30)` before doing
anything else, so a wedged clipboard utility cannot leave it resident: the
kernel kills it, unconditionally, at a bounded time.

### 4.4 Windows fails closed in V1, and that is a decision

Windows ships `clip.exe`, which writes the clipboard from standard input. It
ships no console-mode clipboard *reader*. §5 requires reading the clipboard back
before clearing it, so a Windows implementation built on `clip.exe` could offer
either an unverified clear — which §5.1 rejects — or no clear at all, which
VLT-PM00 §14.6 promises against.

`--copy` on Windows therefore fails with `unsupported` (exit 8), and everything
else on Windows is untouched. A Win32 `OpenClipboard`/`GetClipboardData`
implementation would close this properly and is the natural follow-up (§8.2).
Shipping half a ceremony on a platform in order to say the platform is supported
is the failure mode this document is avoiding.

## 5. Clearing, and the value someone else put there

### 5.1 The clear is verified, never unconditional

VLT-PM00 §14.6 does not say "clear the clipboard after the timeout". It says
`--copy` "clears an owned clipboard value after the configured timeout **when
the platform can prove it still owns that value**". That qualifier is the whole
of this section.

An unconditional timed clear is a data-loss bug wearing a security feature's
clothes. Thirty seconds is long enough to copy a password, paste it, and then
copy a paragraph of your own; wiping that paragraph is a product that eats the
user's work and is impossible to attribute to the password manager that did it.

So the clear is conditional. Before clearing, the child reads the clipboard,
recomputes `SHA-256(salt || current_value)`, and compares it to the digest it
was given with a constant-time comparison. It clears **only** on a match.

| Clipboard at the deadline | Action |
|---|---|
| still exactly the copied value | cleared |
| something the person copied afterwards | left alone |
| already emptied by the person or by another tool | left alone |
| unreadable (session ended, tool now missing, tool wedged) | left alone |

The comparison is against a digest rather than the value for §4.3's reason, and
it is constant-time because a comparison against a secret-derived constant
should be, even where the timing channel is uninteresting — the repository has a
constant-time primitive and there is no reason to hand-roll the interesting
mistake later.

One trailing newline is trimmed from the read value before hashing, because the
clipboard read tools do not agree on whether one is present.

### 5.2 Reading the clipboard is not a disclosure

The child reads a value it may not clear, so it reads values the ceremony never
authorized it to see. This is not a new disclosure path, for three reasons: the
value never leaves that process, the process has no terminal, no standard
output, no vault, and no audit chain to write to; and the read is discarded into
a wipe-on-drop buffer within microseconds. It is bounded at 4,096 bytes, which
is four times the largest value this product will ever copy, so a large
clipboard cannot force an allocation and cannot deadlock the reader.

### 5.3 A second copy does not disarm the first clear

If a person copies two secrets inside one timeout, two children exist. The first
one wakes, finds a value whose digest does not match its own, and leaves it
alone; the second one wakes later and clears. The second secret is cleared, the
first one already isn't there, and no schedule has to be cancelled or
deduplicated. The verified-clear rule makes the concurrent case fall out
without any coordination between processes, which is the reason to prefer it
over a "cancel the previous timer" design that would need shared state.

### 5.4 If the clear cannot be scheduled, the copy is undone

If the write to the clipboard succeeds but the child cannot be spawned, the
adapter immediately clears the clipboard and reports the failure. A copy whose
clear was never scheduled is a secret left in the clipboard forever, and it is
worse than a failed command because the person believes a timeout is running.

## 6. Which timeout, for which command

| Command | `clipboard_clear_seconds` from |
|---|---|
| `totp code --copy` | the selected vault's config entry (VLT-PM07) |
| `password generate --copy` | the compile-time default, 30 |

`totp code` opens a vault, so its configured value is already in hand at the
point of delivery and is used.

`password generate` **cannot** read config, and that is a load-bearing property
rather than an oversight. VLT-PM44 §1 established that it resolves no platform
layout, takes no writer lock, and works on a machine where `init` has never run
— which is the most common moment to want a generated password. Reading
`clipboard_clear_seconds` would require resolving the layout and loading a
config file that may not exist. The product default, 30 seconds, is used
instead, and the consequence is stated rather than hidden: a person who
configured 120 seconds for their vault gets 30 for a generated password. A
follow-up that gives the generator an optional best-effort config read is
reasonable and is not this slice.

## 7. What this feature cannot do

### 7.1 The clipboard is a shared bus

Every process in the person's session can read the clipboard for as long as the
value is there. That is what a clipboard is. `--copy` is preferred over
`--reveal` because a terminal-visible secret persists in scrollback, in `script`
logs, and over the shoulder indefinitely, while a clipboard value has a bounded
lifetime — not because the clipboard is private.

### 7.2 X11 and Wayland selections are owned by a resident process

On X11 and Wayland the selection is not a buffer the display server stores; it
is served on demand by the process that owns it. `xclip`, `xsel`, and `wl-copy`
therefore fork and stay resident holding the value in **their** memory until
something else takes the selection. That is inherent to the protocol and is not
introduced by this design — it is equally true of every other program that has
ever put anything on an X11 clipboard — but it means the value lives in one more
process than the macOS path uses. It is documented here because a reader
comparing the platforms deserves to know they are not identical.

### 7.3 A failed clear is silent, and that is a deliberate trade

The detached clearer has no terminal, no standard output, no vault, and no
audit chain to write to — all of which §5.2 relies on to argue that its
clipboard read is not a disclosure. The consequence is that "the clipboard was
cleared" and "the clipboard could not be cleared" look identical from outside:
a transient tool failure, a session that ended, or a selection owner that never
answers all produce a secret that stays on the clipboard with nobody told.

The obvious repair — report the outcome somewhere — is refused twice over.
Writing to the terminal would mean writing to a terminal the process no longer
owns and a person may have handed to something else. Publishing an audit event
would mean a detached background process holding a vault open, which is the one
thing §2.1 says it must never do.

The exit status is also deliberately uninformative: it does not distinguish
"cleared" from "left alone", because a local user who could observe that
difference would have an oracle for whether a particular value is still on the
clipboard. Silence here is the same choice as constant-time comparison in
§5.1 — refusing to leak the one bit that the comparison exists to protect.

What remains is that the guarantee is best-effort, and this paragraph is where
that is written down rather than implied.

### 7.4 A clipboard manager defeats the clear, and cannot be stopped

If a clipboard history manager is running, it captured the value the moment it
was copied and keeps its own copy. Clearing the system clipboard does not reach
into that history. This product cannot detect one, cannot opt out of one, and
does not pretend to. On such a host `--reveal` is the safer mode, and this is
the sentence that says so.

## 8. Deferred

### 8.1 `--copy` on the reveal commands

`item reveal`, `item show --field`, and `history show` all have `--copy` in
VLT-PM00 §14.4. Wiring them is small — the adapter is general — but each is a
grammar change to a VLT-PM25 ceremony, each must decide what a non-UTF-8 or
non-printable stored secret means for a clipboard whose contract is printable
ASCII (§9 gate 3), and `item show --field` has an output-mode question this
document has no standing to answer. They belong with those commands.

### 8.2 Windows

§4.4. A Win32 `OpenClipboard`/`GetClipboardData`/`SetClipboardData`
implementation, in-process rather than through a utility, closes both the write
and the read and needs no `clip.exe`.

### 8.3 Native ownership tests instead of a content digest

X11 and Wayland can answer "do I still own the selection" natively, and macOS
has `NSPasteboard.changeCount`. Each is a stronger ownership test than a content
digest and each is platform-specific. The digest is chosen for V1 because it is
one mechanism that works identically everywhere, and because it is strictly
*more* conservative: it declines to clear a value it did not write even when it
still holds ownership.

## 9. Acceptance gates

The slice is complete only when tests prove:

1. tool selection follows §4.1's table exactly for every session combination,
   including Wayland winning over a simultaneously-exported `DISPLAY`, and
   returns unavailable for a headless session, an unknown platform, and a
   session whose tools are absent;
2. a tool present only outside `/usr/bin` and `/bin` — including one on `PATH`
   and one in `/usr/local/bin` — is never selected, and inside those
   directories a symbolic link, a directory, and a file this process owns are
   each refused while a real root-owned binary is accepted;
3. the value contract is enforced: empty, over-long, non-ASCII, space-bearing,
   and control-bearing values are refused before anything is spawned;
4. the spawned argument vector for the write, the read, the clear, and the
   detached clearer contains no byte of the secret and no byte of the digest,
   and the secret reaches the tool only on its standard input;
5. the parameter block round-trips, and a wrong magic, wrong version, short
   block, long block, zero delay, and over-range delay are each refused;
6. the verified clear clears on a digest match, does not clear on a mismatch,
   does not clear when the read fails, and tolerates one trailing newline;
6a. every wait on a clipboard utility is bounded in time as well as in bytes: a
   tool that never exits, and a reader that emits a few bytes below the ceiling
   and then stalls, are both killed on the deadline rather than awaited;
7. the digest depends on the salt — the same value under two salts produces two
   digests;
8. `password generate --copy` and `totp code --copy` fail with `unsupported`
   before any prompt when the host has no clipboard, and the failure costs no
   scripted prompt, no unlock, and no audit event;
9. `--copy` on both commands, on a host that has one, performs the confirmation
   ceremony with the §3.1 prompt, delivers to the clipboard adapter and never to
   the terminal reveal path, and `totp code --copy` still publishes exactly one
   `ItemRead` `Succeeded` bound to the exact current revision and still prints
   only the non-secret validity line on standard output;
10. refusal at the `--copy` prompt publishes `Denied` and copies nothing;
11. `totp code --copy` uses the selected vault's configured
    `clipboard_clear_seconds` and `password generate --copy` uses the product
    default;
12. `vault-pm clipboard clear` is present in `USAGE`, opens no vault, and exits
    `2` on an empty or malformed parameter block;
13. the real-process executable on a headless host returns `unsupported` for
    both `--copy` commands and writes nothing to `/dev/tty`; and
14. formatting, Clippy, rustdoc, and the package tests of the affected
    dependency closure pass.
