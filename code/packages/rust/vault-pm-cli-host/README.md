# `coding_adventures_vault_pm_cli_host`

This crate is the native secret-input and entropy boundary for the local
password-manager CLI. It gives the `vault-pm` executable four narrow,
auditable adapters:

- `ControllingTerminal` opens `/dev/tty` on Unix or `CONIN$`/`CONOUT$` on
  Windows, emits only fixed prompts, reads bounded item metadata under the
  existing echo mode or disables echo for secrets, restores the original
  terminal mode before returning, and supports exact-`yes` confirmation plus
  quoted/control-escaped direct secret delivery, and reads one bounded echoed
  command line for the foreground interactive shell; and
- `OsEntropy` completely fills a caller-owned non-empty buffer using the
  repository `csprng` wrapper and maps operating-system details to one stable,
  payload-free failure; and
- `write_portable_export` creates one explicit encrypted artifact destination
  without following or replacing an existing final path, synchronizes the
  bytes, and requests owner-only mode on Unix; and
- `read_portable_export` reads one non-empty regular artifact under an exact
  metadata and streaming byte ceiling before application authentication; and
- `read_attachment_source` and `read_external_import_source` read one
  non-empty regular *plaintext* source (an attachment, or a Bitwarden/CSV
  import file respectively) into a `Zeroizing` buffer under the same exact
  ceiling discipline, so a failed read leaves no copy of the person's
  plaintext in freed heap; and
- `clipboard` delivers one already-authorized secret to the platform clipboard
  and schedules a **verified** clear of it (VLT-PM46).

## The clipboard adapter

The clipboard is not reachable from portable Rust, `vault-pm` is a one-shot
process with nothing for a thirty-second timer to live in, and the clipboard is
a shared bus that other things write to. The module answers those three,
in order, with a pre-installed platform utility, a detached re-execution of this
same binary, and a clear that verifies before it wipes.

**The secret goes on a pipe, never in an argument.** `ps` and
`/proc/<pid>/cmdline` publish one process's command line to every account on the
host, and a commitment to a six-digit TOTP code is brute-forceable in
microseconds — so the value is written to the utility's standard input and the
clearer's delay, salt, and digest are written to a pipe. Every argument vector
in the module is a compile-time `&'static [&'static str]`, so there is no type
here a secret could be interpolated into.

**Utilities are resolved from `/usr/bin` and `/bin` only, and the file found
there must be a root-owned regular file with no group- or other-write bit.**
`PATH` is never consulted, because it is caller-controlled: resolving through it
would hand a live credential to the standard input of a program chosen by
whoever could prepend a directory. `/usr/local/bin` is excluded deliberately —
it is where locally-installed software lives and is group- or user-writable on a
meaningful fraction of real machines. The ownership check is what turns "it is
in a root-owned directory" from an assumption about the host into something the
module verifies: a symbolic link planted in `/usr/bin` would otherwise be
followed without comment. A utility that fails either test is not found and
`--copy` fails closed.

| Session | Write | Read | Clear |
|---|---|---|---|
| macOS | `pbcopy` | `pbpaste` | `pbcopy`, empty input |
| Wayland | `wl-copy` | `wl-paste --no-newline` | `wl-copy --clear` |
| X11 + `xclip` | `xclip -selection clipboard` | `… -o` | `…`, empty input |
| X11 + `xsel` | `xsel --clipboard --input` | `--output` | `--delete` |
| headless, Windows, anything else | — | — | — |

Wayland is chosen ahead of X11, because a Wayland session commonly also exports
`DISPLAY` for XWayland. A family is chosen only when all of its programs are
present, so `wl-copy` without `wl-paste` falls through rather than copying
something whose clear could never be verified. Windows fails closed: it ships
`clip.exe` but no console-mode clipboard *reader*, and this contract will not
perform a clear it cannot verify.

**The clear is verified, never unconditional.** The clearer reads the
clipboard, recomputes `SHA-256(salt || value)`, constant-time compares, and
wipes only on a match. An unconditional timed clear eats the paragraph a person
copied thirty seconds later, and does it in a way nobody would attribute to
their password manager. Two pending clears therefore need no coordination.

**The clearer is this same binary, re-executed and detached.** It receives the
delay, a fresh salt, and the commitment — never the value. It forks a second
time so it is orphaned to `init` and a long-lived interactive shell accumulates
no zombies, calls `setsid` so closing the terminal cannot cancel it, discards
its output, and arms `alarm(delay + 30)` so a wedged utility cannot leave it
resident. If the copy succeeds but the clearer cannot be spawned, the clipboard
is cleared immediately and the failure reported.

Every utility wait is bounded at five seconds *in time as well as in bytes* —
the read pipe is non-blocking and polled against a deadline, because on X11 and
Wayland a reader can wait forever on a selection owner that never answers, and
reading to end of file would be bounded in bytes and unbounded in time. Every
read is also capped at 4 KiB. Values must
be non-empty printable ASCII with no space, at most 1,024 bytes: that is what
makes the round trip a byte comparison, since whitespace and multi-byte
sequences are exactly what the read tools disagree about.

Tests drive tool selection, trusted-directory resolution, the value contract,
the parameter block, and the verified clear against an in-memory clipboard
double, so all of it runs on a headless CI runner. The real platform round trip
is opt-in behind `VAULT_PM_CLIPBOARD_E2E=1`, because running it would overwrite
the developer's own clipboard.

Secret input never comes from process stdin, argv, an environment variable, a
configuration value, or a URL. Redirecting stdin therefore cannot inject a
master passphrase. Interactive shell command lines are read from that same
controlling terminal, so a redirected stdin cannot inject a *command* into an
unlocked session either. A command line is not a secret, so it is read under the
terminal's ordinary echo mode; the only difference from item-metadata reads is
that a genuine end of input is reported as a value, letting a foreground shell
stop instead of failing. New-vault collection performs two independent terminal
reads and compares them with the repository constant-time comparison primitive.
Portable-export collection applies the same rule to two distinct fixed hidden
prompts rather than reusing the live vault passphrase.
Collected bytes are immediately wrapped in `Zeroizing<Vec<u8>>` and are never
available through `Debug`.

Unix opens `/dev/tty` without following links, verifies a character terminal,
and disables `ECHO` and `ECHONL` with an RAII termios guard. Windows opens the
attached console directly, verifies console handles, clears only
`ENABLE_ECHO_INPUT`, and converts console UTF-16 to UTF-8 bytes. Both paths
drain an oversized line before restoring echo, so unconsumed secret suffixes do
not become visible terminal input. Ordinary success, error, and panic-unwind
paths restore the captured mode; a force-kill that prevents destructors from
running is outside an in-process library's guarantees.

Accepted passphrases, login passwords, optional login notes, secure-note bodies, card numbers, card
verification codes, and API-key tokens are non-empty and at most 1,024 bytes.
Database passwords, TOTP Base32 seeds, and opaque-record hexadecimal payloads
use the same bound. That 1,024-byte line carries at most a 512-byte payload,
since hexadecimal spends two characters per byte. Echoed login,
secure-note, payment-card, API-key, database, and TOTP metadata have fixed
per-field bounds up to 2,048 UTF-8 bytes and reject control characters; only
username, billing postal code, scopes, API-key expiry, database name, and TOTP
issuer may be empty. Login URL count is a required canonical decimal value and
drives repeated required URL prompts; optional notes use a fixed hidden prompt
where an empty line means absence. PAN, CVV, API-key token, database password,
TOTP seed, and opaque-record payload use the same hidden wipe-on-drop input path
as other secrets. The opaque payload is hidden because an unknown schema offers
no way to show that any part of it is not a secret.
Prompt strings and every public error are fixed and
contain no secret, OS error, terminal path, user name, or caller payload.
This crate deliberately does not parse commands, choose vault storage, persist
configuration, calibrate Argon2id, or prepare vault bytes.

Interactive reveal never routes a secret through ordinary process stdout or
stderr. After the application has durably authorized the access, the adapter
reopens the controlling terminal and writes one `Secret: "..."` line. Debug
string escaping prevents stored control characters from becoming terminal
commands or counterfeit output; the escaped string and Windows UTF-16 buffer
are wipe-on-drop.

The escape itself is capacity-reserved, not grown incrementally:
`escaped_revealed_text` allocates its `Zeroizing<String>` at a provably
sufficient upper bound before writing a single byte of the secret, so the
buffer can never trigger `String`'s ordinary reallocate-and-free-the-old-copy
growth while it holds plaintext — the same discipline
`vault-pm-agent-protocol`'s `AgentRequest::encode` uses for binary framing,
applied here to Debug-escaped text. See VLT-PM05 §13.6.

Fourteen Unix tests exercise stable diagnostics, text/secret bounds, constant-time
confirmation behavior, real OS entropy, pseudo-terminal ordinary and hidden
input, mode restoration, oversized-line draining, non-terminal refusal,
create-new durable export persistence, bounded artifact reads, and
end-of-input-aware command-line reads.
Windows adds three target-specific tests for console names and strict bounded
UTF-16 conversion; cross-target Clippy validates the native Windows API
surface from Unix.

## The attachment file pair

`read_attachment_source` returns `Zeroizing` bytes and
`write_attachment_export` writes plaintext. The asymmetry is deliberate and it
is the whole security shape of the pair.

What the read holds is the *person's file*, not an already-encrypted artifact
like a portable export, so a refused or failed attach must not leave a copy of
it in freed heap. It bounds the metadata length before allocating and caps the
reader at one byte past the ceiling, so a file that grows between those two
observations cannot force an unbounded allocation. A path that will not open is
`InvalidAttachmentSource` — exit 2 — rather than a provider failure, because
exit 7 tells a person to retry later and retrying will not conjure the file.

On Unix the read also passes `O_NONBLOCK | O_NOCTTY`. The check that rejects a
FIFO cannot run until the open returns, and opening a FIFO for reading blocks
until a writer appears — so without the flag, naming a named pipe hung the
command instead of being refused.

The read itself is **exact**: one allocation of exactly the declared length,
`read_exact`, then a one-byte probe that must see end-of-file. `Zeroizing`
wipes the allocation it owns and only that one, so a vector holding plaintext
that reallocates leaves what it had already read in freed heap — and reserving
spare capacity does not help, because the reservation comes from a measurement
a concurrently-appended file has already invalidated. A file longer than it
measured is refused by the probe, a shorter one by `UnexpectedEof`. The point
is that reallocation is unreachable rather than unlikely.

What the write produces *is* plaintext: that is what an export is. So the care
is everywhere else. Create-new semantics, so an existing file, directory, or
symbolic link is never followed or replaced; owner-only mode at creation on
Unix; write, `fsync`; and removal of the incomplete file if either step fails,
because a half-written plaintext left behind by a failed export is a leak with
no owner.

Three residuals are written down rather than assumed away, because each is
true and none was obvious. `create_new` refusing a symbolic link, and
owner-only mode from the instant the file exists, are both Unix statements —
Windows resolves reparse points and `OpenOptions` exposes no mode there, which
matters more for this plaintext than for the encrypted portable artifact next
to it. The cleanup re-resolves the path rather than acting on the descriptor.
And a kill delivered *inside* the write leaves a partial file: the drill
brackets the whole call, so it proves both of its landing points clean and says
nothing about that one.

`AttachmentExportConfirmation` is a third confirmation sentence rather than a
reuse of the reveal or copy one. An export puts vault content into an ordinary
file this product will not track, clear, or know about again; neither existing
sentence says that, and a consent ceremony that misdescribes what it is
consenting to manufactures a record of an agreement nobody made.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli_host --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli_host --no-deps
```
