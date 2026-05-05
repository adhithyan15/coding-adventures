# `coding_adventures_vault_transport_cli` — VLT11 transport (CLI)

Terminal interface for the Vault stack. Library crate — the
binary lives in `code/programs/rust/` and is a thin shim that
constructs the underlying layers (auth, policy, engines, leases,
audit, sync) and a host-side `CliDriver` that delegates.

The crate splits into three pieces:

1. **`parse_args`** — pure parser from `&[String]` → `CliCommand`.
2. **`CliDriver`** — trait the host implements to do the work.
3. **`render_output`** — turns a `CommandOutput` into a string
   (text or JSON) for the terminal.

## Quick example

```rust
use coding_adventures_vault_transport_cli::{
    parse_args, render_output, CliCommand, CliDriver, CommandOutput,
    Format, SyncOp, AuditOp, VaultPath,
};

let argv: Vec<String> = std::env::args().skip(1).collect();
let cmd = parse_args(&argv).map_err(|e| e.to_string())?;

let mut driver = MyVaultDriver::new(/* … */);
let out = driver.dispatch(cmd)?;
println!("{}", render_output(&out, Format::Text));
```

## Subcommand surface

| Subcommand                              | Purpose                                            |
|-----------------------------------------|----------------------------------------------------|
| `login <user>`                          | authenticate (passphrase via stdin)                |
| `unseal`                                | unseal the vault (passphrase via stdin)            |
| `put <path> [<value>]`                  | store a value (value via stdin if omitted)        |
| `get <path> [--version N]`              | fetch a value                                      |
| `list <prefix>`                         | list keys under a prefix                           |
| `revoke <path> --version N`             | destroy a specific version                         |
| `share <path> --with <recipient>`       | add a recipient to the wrap-set                    |
| `sync push | pull | status`             | sync with the configured server                    |
| `audit verify | tail`                   | audit-log integrity check / tail                   |
| `help` / `--help` / `-h`                | show help                                          |
| `version` / `--version` / `-V`          | print version                                      |

## Threat model

- **Untrusted shell history.** Passphrases are never accepted as
  positional arguments. The CLI reads them from stdin.
- **Argument injection.** Positional arguments containing
  newlines / NULs / control characters are rejected at the
  parser, surfacing as `ParseError::ForbiddenChar` with a byte
  position — never an echo of the bad value.
- **Diagnostic output never echoes bytes.** `Display` for
  `ParseError` reports lengths and field names, not contents.
  An attacker passing `vault wipe-disk\nALERT: ...` cannot use
  the error message to inject text into a downstream log.
- **Bounded argv.** `MAX_ARGV_LEN = 1024`, `MAX_ARG_LEN = 16
  KiB`, `MAX_PATH_LEN = 4 KiB`, `MAX_INLINE_VALUE_LEN = 64
  KiB`. Larger payloads must come over stdin.
- **No silent shell expansion.** The parser does not glob, does
  not look up env vars, does not interpret `~`. The shell
  already handled all that.
- **`CommandOutput::Secret` does not print bytes by default.**
  Text + JSON renderers print only byte length; raw bytes
  require an explicit `Format::SecretRaw` opt-in (which
  hex-encodes them rather than emitting raw bytes from the
  string-returning API).

## What this crate is NOT

- Not a binary. The terminal binary lives in
  `code/programs/rust/`.
- Not the auth/policy/engine/lease/audit/sync logic — the
  `CliDriver` trait delegates to those layers.
- Not a daemon. The same parser will be reused by a future
  `vault-transport-daemon` over a unix socket; same
  `CommandOutput` shape, different wire.

## Capabilities

None — pure parser + formatter + trait. See
`required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT11-transports.md`](../../../specs/VLT11-transports.md).
