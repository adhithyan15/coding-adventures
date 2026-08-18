# Changelog

## Unreleased

- **Added `clipboard`**, the platform clipboard adapter with a verified timed
  clear, specified by `VLT-PM46-cli-clipboard.md`. `VLT-PM00` §14.6 has always
  called `--copy` the *preferred* secret-output mode and `VLT-PM07` has always
  carried `clipboard_clear_seconds`; neither had an implementation behind it.

  **Nothing secret is ever an argument.** The value is written to a platform
  utility's standard input, and the detached clearer's delay, salt, and digest
  are written to a pipe. `ps` and `/proc/<pid>/cmdline` publish one process's
  arguments to every account on a host, and a commitment to a six-digit TOTP
  code is brute-forceable in microseconds, so this is the constraint the whole
  module is shaped around rather than a refinement of it. Every argument vector
  is a `&'static [&'static str]` chosen at compile time, so there is no type in
  the module a secret could be interpolated into.

  **Utilities are resolved from `/usr/bin` and `/bin` only; `PATH` is never
  consulted.** `PATH` is caller-controlled, so resolving through it would hand
  a live credential to the standard input of a program chosen by anyone who
  could prepend a directory. `/usr/local/bin` is excluded deliberately: it is
  where locally-installed software lives and is group- or user-writable on a
  meaningful fraction of real machines. A `wl-copy` installed only under a Nix
  profile is therefore not found and `--copy` fails closed, which is the
  correct direction to fail on a trust question.

  Four families are supported — macOS `pbcopy`/`pbpaste`, Wayland
  `wl-copy`/`wl-paste`, X11 `xclip`, and X11 `xsel` — chosen with Wayland
  ahead of X11, because a Wayland session commonly also exports `DISPLAY` for
  XWayland. A family is chosen only when *all* of its programs are present, so
  a host with `wl-copy` and no `wl-paste` falls through rather than copying
  something whose clear could never be verified. Windows fails closed: it ships
  `clip.exe` but no console-mode clipboard *reader*, and a clear that cannot be
  verified is not one this contract will perform.

  **The clear is verified, never unconditional**, which is §14.6's own
  qualifier — "when the platform can prove it still owns that value". The
  clearer reads the clipboard, recomputes `SHA-256(salt || value)`, and
  constant-time compares before wiping anything. An unconditional timed clear
  is a data-loss bug wearing a security feature's clothes: thirty seconds is
  long enough to paste a password and then copy a paragraph of your own, and
  wiping that paragraph is impossible to attribute to the password manager that
  did it. Two pending clears need no coordination — the first wakes, sees a
  digest that is not its own, and leaves the second secret alone.

  **The timed clear survives process exit by re-executing this same binary.**
  The child gets the delay, a fresh salt, and the commitment, and never the
  value. `VLT-PM46` §4.3 states the honest limit — a six-digit code has 10^6
  preimages and a salt does not change that — and then why it is acceptable:
  the child exists for exactly the window in which the clipboard already holds
  the secret, readable by any process in the session with no privilege at all.
  It forks a second time so the grandchild is orphaned to `init` and the
  long-lived `vault-pm shell` accumulates no zombies, calls `setsid` so closing
  the terminal window cannot cancel a pending clear, sends its output to
  `/dev/null`, and arms `alarm(delay + 30)` so a wedged utility cannot leave it
  resident. If the clipboard write succeeds and the clearer cannot be spawned,
  the clipboard is cleared immediately and the failure is reported: a copy
  whose clear was never scheduled leaves a secret there forever while the
  person believes a timeout is running.

  Every wait on a utility is bounded at five seconds and every read is bounded
  at 4 KiB, so no clipboard path can hang a terminal or force an allocation.
  Accepted values are non-empty printable ASCII with no space, at most 1,024
  bytes — the contract that makes the round trip a byte comparison, since
  whitespace and multi-byte sequences are exactly what the read tools disagree
  about, and a disagreement there becomes "the clear silently never fires".
- Added `TextPrompt::SecretCopyConfirmation` and
  `ControllingTerminal::confirm_secret_copy`. Same exact-lowercase-`yes` rule
  as the reveal confirmation, different sentence: asking "reveal secret on this
  terminal?" and then putting the value somewhere every process in the session
  can read would manufacture a record of an agreement nobody made.
- Added six payload-free diagnostics: `ClipboardUnavailable`,
  `ClipboardValueUnsupported`, `ClipboardWriteFailed`, `ClipboardReadFailed`,
  `ClipboardClearScheduleFailed`, and `InvalidClipboardClearRequest`.
- Added a dependency on `rust/sha256` for the clipboard commitment, and
  declared the new `env:read`, `fs:read`, `proc:exec`, `proc:fork`,
  `proc:signal`, `stdin:read`, and `time:sleep` capabilities.

- Add `ControllingTerminal::read_command_line`, a bounded echoed line reader for
  the foreground interactive shell. It uses the same controlling terminal every
  prompt uses, so a redirected standard input can supply neither a secret nor a
  command; it writes the fixed compile-time `vault-pm> ` prompt; and it reports
  a real end of input as `Ok(None)` rather than a failure so a session can end
  cleanly, while every other read failure stays closed. Line content is bounded
  at 1,024 bytes, must be valid UTF-8, and must contain no control characters.
- Factor the Unix controlling-terminal open into one helper shared by the
  secret, text, command-line, and reveal paths, and split the bounded line
  reader into an end-of-input-aware form. The secret reader's behaviour is
  unchanged: end of input still fails closed.

- Add a fixed hidden wipe-on-drop opaque-record payload prompt, collecting the
  whole canonical-CBOR payload as lowercase hexadecimal under the existing
  1,024-byte secret-line bound. It is hidden rather than echoed because an
  opaque record's schema is unknown, so no part of its payload can be shown to
  be non-secret.
- Add a fixed canonical URL-count prompt, repeated required URL input, and
  optional hidden wipe-on-drop login-notes input for complete login forms.
- Add fixed bounded TOTP label, issuer, algorithm, digits, and period prompts
  plus hidden wipe-on-drop Base32 seed input.
- Add fixed bounded database label, engine, host, port, database, and username
  prompts plus hidden wipe-on-drop password input.
- Add fixed bounded API-key label, service, scope, and expiry prompts plus a
  hidden wipe-on-drop token prompt.
- Add fixed bounded payment-card metadata prompts plus hidden PAN and CVV
  prompts, retaining controlling-terminal input and wipe-on-drop ownership.
- Add exact-`yes` controlling-terminal confirmation and direct quoted,
  control-escaped secret delivery without routing values through process
  stdout or stderr.
- Wipe the temporary escaped string and Windows UTF-16 console buffer after
  every terminal disclosure attempt.
- Add a bounded regular-file portable-artifact reader and reuse the fixed
  hidden import-passphrase prompt through CLI composition.
- Add fixed hidden portable-export passphrase and confirmation prompts with
  constant-time comparison.
- Add create-new durable encrypted-artifact persistence with Unix mode `0600`,
  no final-path replacement, and best-effort incomplete-file cleanup.
- Add fixed secure-note title and hidden body prompts, reusing the bounded,
  echo-restoring, wipe-on-drop controlling-terminal input boundary.

## 0.1.0

- Add fixed-prompt echoed UTF-8 login metadata collection with per-field
  bounds, control rejection, and optional username/URL handling.
- Add a fixed hidden login-password prompt and byte-accurate Windows UTF-8
  bounds.
- Add fixed-prompt, bounded secret collection from the controlling terminal on
  Unix and the attached console on Windows.
- Add echo restoration, oversized-line draining, zeroizing ownership, and
  constant-time new-passphrase confirmation.
- Add a stable, payload-free OS entropy adapter over `csprng`.
