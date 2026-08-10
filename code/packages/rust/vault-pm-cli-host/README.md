# `coding_adventures_vault_pm_cli_host`

This crate is the native secret-input and entropy boundary for the local
password-manager CLI. It gives the eventual `vault-pm` executable two narrow,
auditable adapters:

- `ControllingTerminal` opens `/dev/tty` on Unix or `CONIN$`/`CONOUT$` on
  Windows, emits only fixed prompts, disables echo, reads one bounded line, and
  restores the original terminal mode before returning; and
- `OsEntropy` completely fills a caller-owned non-empty buffer using the
  repository `csprng` wrapper and maps operating-system details to one stable,
  payload-free failure.

Secret input never comes from process stdin, argv, an environment variable, a
configuration value, or a URL. Redirecting stdin therefore cannot inject a
master passphrase. New-vault collection performs two independent terminal
reads and compares them with the repository constant-time comparison primitive.
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

The accepted passphrase is non-empty and at most 1,024 bytes, matching the
portable-export application boundary. Prompt strings and every public error
are fixed and contain no secret, OS error, terminal path, user name, or caller
payload. This crate deliberately does not parse commands, choose storage,
persist configuration, calibrate Argon2id, or prepare vault bytes.

Seven Unix tests exercise stable diagnostics, bounds, constant-time
confirmation behavior, real OS entropy, pseudo-terminal hidden input and mode
restoration, oversized-line draining, and non-terminal refusal. Windows adds
three target-specific tests for console names and strict UTF-16 conversion;
cross-target Clippy validates the native Windows API surface from Unix.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli_host --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli_host --no-deps
```
