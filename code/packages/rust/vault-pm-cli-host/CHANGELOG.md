# Changelog

## Unreleased

- **Fixed: `escaped_revealed_text` no longer reallocates while holding a
  secret.** `item reveal` and `conflict reveal` (via `write_revealed_text`)
  used to build the quote/control-escaped terminal text with
  `Zeroizing::new(format!("{value:?}"))`. `format!`'s `String` starts empty
  and grows via `push`/`push_str` as `Debug` escapes each character, using
  ordinary incremental-growth reallocation: once a write would exceed
  capacity, the buffer `memcpy`s the plaintext already written into a larger
  allocation and frees the old one through the global allocator *without
  scrubbing it first*. `Zeroizing` only wipes the final allocation it ends up
  holding, so every intermediate allocation the buffer reallocated out of
  along the way left a stale, unwiped copy of the secret in freed heap — the
  same reallocation-leaves-a-stale-copy pattern already found and fixed in
  `vault-pm-agent-protocol`'s `AgentRequest::encode`.

  `escaped_revealed_text` now reserves capacity once, before any byte of the
  secret is read — `2 + 6 * value.len()`, a provably sufficient upper bound
  on Debug-escaping's worst per-input-byte expansion (a lone ASCII control
  byte escaping to `\u{xx}`) — and writes the real, unmodified `Debug`
  formatter's output into that buffer via `write!`, so no reallocation can
  occur while a copy of the secret is resident. A `debug_assert_eq!` on the
  final capacity turns "the bound stayed sufficient" into a standing
  invariant. See VLT-PM05 §13.6 for the full design account, and
  `escaped_revealed_text_never_reallocates_a_buffer_already_holding_a_secret`
  for the regression test (mirrors `vault-pm-agent-protocol`'s
  `encode_never_reallocates_a_buffer_already_holding_a_secret`).

- **Added `read_external_import_source`** (`VLT-PM49-cli-external-import.md`),
  the filesystem half of `vault-pm import bitwarden`/`import csv`. Same
  exact-length-read shape as `read_attachment_source` and for the same
  reason: a Bitwarden or browser-CSV export *is* the person's plaintext
  secrets rather than an already-encrypted artifact, so the returned buffer
  is `Zeroizing`, opens with `O_NONBLOCK | O_NOCTTY` on Unix, and never
  reallocates mid-read. Reuses the existing `InvalidImportSource`/
  `ImportReadFailed` error variants rather than adding new ones, since "the
  import source was empty/not-a-file/over-bound/unreadable" is exactly what
  those already mean.

- **Added `read_attachment_source` and `write_attachment_export`**, the
  filesystem halves of `VLT-PM47-cli-attachments.md`. The read returns
  `Zeroizing` bytes because what it holds is the person's file rather than an
  already-encrypted artifact, so a refused or failed attach leaves no copy in
  freed heap; it bounds the metadata length before allocating and caps the
  reader at one byte past the ceiling, so a file that grows between the two
  cannot force an unbounded allocation. A path that will not open is
  `InvalidAttachmentSource` and exit 2 rather than a provider failure: exit 7
  tells a person to retry later, and retrying will not conjure the file.

  The write refuses to replace an existing destination, creates owner-only on
  Unix, `fsync`s, and removes the incomplete file if anything fails — a
  half-written plaintext left behind by a failed export is a leak with no
  owner.

- `read_attachment_source` opens with `O_NONBLOCK | O_NOCTTY` on Unix, because
  the check that rejects a FIFO cannot run until the open returns and opening a
  FIFO for reading blocks until a writer appears — naming a named pipe used to
  hang the command rather than be refused.

- **The read is now exact.** `Zeroizing` wipes the allocation it owns and only
  that one, so a vector holding plaintext that reallocates leaves the bytes
  already read in freed heap. Reserving extra capacity did not fix that,
  because the reservation comes from a measurement a concurrently-appended file
  has already invalidated — a hundred-byte file that grows to a megabyte during
  the read reallocates repeatedly whatever the ceiling is, and the result was
  still *accepted*, since the only length check compared it against the 16 MiB
  ceiling. One allocation of exactly the declared length, `read_exact`, and a
  one-byte probe that must see end-of-file: a file longer than it measured is
  refused by the probe and a shorter one by `UnexpectedEof`, and reallocation is
  unreachable rather than unlikely.

- `write_attachment_export` documents what its two guarantees are worth per
  platform, that its cleanup is by path rather than by descriptor, and that a
  kill *inside* the write still leaves a partial file. All three were true
  before and none was written down, which is the same as assuming them.

- **Added `TextPrompt::AttachmentExportConfirmation`** and
  `ControllingTerminal::confirm_attachment_export`. A third sentence for the
  same reason there is a second: an export puts vault-held content into a
  plaintext file, neither of the other two prompts says so, and a consent
  ceremony that misdescribes what it is consenting to manufactures a record of
  an agreement nobody made.

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

  **Utilities are resolved from `/usr/bin` and `/bin` only, must be root-owned
  regular files with no group- or other-write bit, and `PATH` is never
  consulted.** The ownership test is what makes the directory rule more than an
  assumption about the host: a symbolic link planted in a trusted directory
  would otherwise be followed silently, and so would an image where `/usr/bin`
  is not in fact root's. `PATH` is caller-controlled, so resolving through it would hand
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
  at 4 KiB *and* at the same five-second deadline, so no clipboard path can hang
  a terminal or force an allocation. The time bound on the read is not
  redundant: on X11 and Wayland the selection is served on demand by whichever
  process owns it, so a reader can stall below the byte ceiling forever waiting
  on an owner that never answers — and the consequence would be a verified clear
  that silently never fires. The detached clearer also resets `SIGALRM` to its
  default disposition before arming its watchdog, because `execve` preserves an
  *ignored* signal and an inherited `SIG_IGN` would have made the watchdog a
  silent no-op.
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
