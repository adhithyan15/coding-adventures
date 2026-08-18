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
  metadata and streaming byte ceiling before application authentication.

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

Fourteen Unix tests exercise stable diagnostics, text/secret bounds, constant-time
confirmation behavior, real OS entropy, pseudo-terminal ordinary and hidden
input, mode restoration, oversized-line draining, non-terminal refusal,
create-new durable export persistence, bounded artifact reads, and
end-of-input-aware command-line reads.
Windows adds three target-specific tests for console names and strict bounded
UTF-16 conversion; cross-target Clippy validates the native Windows API
surface from Unix.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli_host --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli_host --no-deps
```
