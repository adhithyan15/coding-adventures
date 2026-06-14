# VLT11 — Vault Transports

## Overview

The vault must be reachable from outside a Rust process. Three
transports cover almost every use case, plus a handful of
optional adapters. Each transport is a *thin* crate that
composes `{auth, policy, engines, leases, audit, sync}` and
exposes them; the vault core is **headless**.

| Transport                    | Crate                              | Status |
|------------------------------|------------------------------------|--------|
| Terminal CLI                 | `vault-transport-cli` (this PR)    | shipped |
| HTTP REST + WebSocket        | `vault-transport-http`             | future |
| gRPC                         | `vault-transport-grpc`             | future |
| FUSE filesystem              | `vault-transport-fuse`             | future |
| Subprocess env-var injector  | `vault-transport-env`              | future |
| Kubernetes CSI driver        | `vault-transport-k8s-csi`          | future |
| Browser WebExtension shim    | `vault-transport-browser`          | future |

This spec covers the **CLI transport**. Future per-transport
specs (`VLT11-http.md`, `VLT11-grpc.md`, etc) will describe the
others.

## CLI design

`vault put / get / list / share / sync / unseal / login` is the
documented surface. It subsumes `pass`-style workflows
(`pass insert`, `pass show`, `pass cp`) and doubles as the
control plane for a future daemon mode.

The implementation is split into a **library** crate that owns
parsing + dispatch + formatting, and a future **binary** crate
that wires in the underlying layers. This split is intentional:

- the integration test suite drives every subcommand without
  spawning a subprocess,
- a daemon-mode IPC will reuse the same parser by piping
  newline-delimited argv through a unix socket,
- other transports reuse the same `CommandOutput` shape so
  downstream tooling that consumes vault output uniformly works
  across CLI / HTTP / gRPC.

Implementation lives at
`code/packages/rust/vault-transport-cli/`.

## Library API

```rust
pub fn parse_args(argv: &[String]) -> Result<CliCommand, ParseError>;

pub enum CliCommand {       // #[non_exhaustive]
    Login { user: String },
    Unseal,
    Put    { path: VaultPath, inline_value: Option<Vec<u8>> },
    Get    { path: VaultPath, version: Option<u32> },
    List   { prefix: VaultPath },
    Revoke { path: VaultPath, version: u32 },
    Share  { path: VaultPath, recipient: String },
    Sync   { op: SyncOp },
    Audit  { op: AuditOp },
    Help, Version,
}

pub enum SyncOp  { Push, Pull, Status }   // #[non_exhaustive]
pub enum AuditOp { Verify, Tail }         // #[non_exhaustive]

pub trait CliDriver {
    type Error;
    fn dispatch(&mut self, cmd: CliCommand)
        -> Result<CommandOutput, Self::Error>;
    // … per-command handlers
}

pub enum CommandOutput {    // #[non_exhaustive]
    Message(String),
    Table   { headers: Vec<String>, rows: Vec<Vec<String>> },
    Secret  { bytes: Vec<u8> },
}

pub enum Format { Text, Json, SecretRaw }

pub fn render_output(out: &CommandOutput, fmt: Format) -> String;
```

## Subcommand surface

| Subcommand                              | Notes                                              |
|-----------------------------------------|----------------------------------------------------|
| `login <user>`                          | passphrase via stdin                               |
| `unseal`                                | passphrase via stdin                               |
| `put <path> [<value>]`                  | value via stdin if omitted; inline ≤ 64 KiB        |
| `get <path> [--version N]`              | engine decides whether `--version` is meaningful   |
| `list <prefix>`                         | returns a `Table`                                  |
| `revoke <path> --version N`             | engine-defined (KV-v2 soft-deletes)                |
| `share <path> --with <recipient>`       | recipient is engine-defined: `pk:<x25519>`, `user:<id>`, … |
| `sync push | pull | status`             | uses VLT10                                         |
| `audit verify | tail`                   | uses VLT09                                         |
| `help` / `--help` / `-h`                | built-in `CliDriver::dispatch`                     |
| `version` / `--version` / `-V`          | built-in                                           |

## Bounds (tested)

| Bound                  | Cap         | Why                                       |
|------------------------|-------------|-------------------------------------------|
| `MAX_ARGV_LEN`         | 1024        | per-call argument count                   |
| `MAX_ARG_LEN`          | 16 KiB      | per-arg byte cap                          |
| `MAX_PATH_LEN`         | 4 KiB       | a single vault path                       |
| `MAX_INLINE_VALUE_LEN` | 64 KiB      | larger values must come over stdin        |

## Threat model & test coverage

| Threat                                                            | Defence                                                    | Test                                                   |
|-------------------------------------------------------------------|------------------------------------------------------------|--------------------------------------------------------|
| Passphrase visible in shell history                               | passphrases never accepted as positional args              | `help_text_documents_security_invariants`              |
| Argument injection via newline / NUL / control char in path       | `VaultPath::new` rejects control chars                     | `vault_path_rejects_newlines_and_nul`                  |
| Argument injection via control char in `login` user / share recipient | parser rejects control chars on those fields           | `login_rejects_user_with_newline`, `share_rejects_recipient_with_control_chars` |
| Error message echoes attacker-controlled token (log injection)    | `Display` for `ParseError` never echoes the value          | `error_display_does_not_echo_unknown_subcommand`       |
| `argv` length amplification                                       | `MAX_ARGV_LEN`, `MAX_ARG_LEN` rejected up front            | `rejects_too_many_args`, `rejects_oversize_arg`        |
| Inline value too large                                            | `MAX_INLINE_VALUE_LEN` rejection                           | `put_rejects_oversize_inline_value`                    |
| `--version` arg parsed as bytes                                   | strict `u32::parse`; non-numeric → `FlagMissingValue`      | `get_rejects_bad_version`                              |
| Unknown flag silently passed through                              | parser rejects with `UnknownFlag`                          | `get_rejects_unknown_flag`                             |
| Secret bytes echoed to terminal / shell history                   | `render_output(Secret, Text|Json)` prints only length      | `render_secret_text_does_not_echo_bytes`, `render_secret_json_does_not_echo_bytes` |
| Secret bytes accidentally piped to non-secret-aware tool          | requires explicit `Format::SecretRaw` opt-in (hex-encoded) | `render_secret_raw_emits_hex`                          |
| JSON output injection via embedded quotes/newlines                | hand-rolled `json_escape`                                  | `render_message_json_escapes_quotes_and_newlines`      |
| Help / version aliases inconsistent across tools                  | three forms each (`help`/`--help`/`-h`)                    | `parses_help_in_all_forms`, `parses_version_in_all_forms` |
| Empty argv panics the parser                                      | returns `NoSubcommand`                                     | `rejects_empty_argv`                                   |
| Driver does not handle every variant                              | exhaustive `dispatch` match + per-variant test             | `driver_dispatch_routes_each_command`                  |
| **JSON output broken in `<script>` context (U+2028/U+2029)**      | `json_escape` emits ` ` / ` `                    | `json_escape_handles_u2028_u2029`                      |
| **Trojan Source / bidi-override (CVE-2021-42574)**                | reject U+202A–U+202E, U+2066–U+2069, U+200B–U+200D, U+FEFF | `vault_path_rejects_bidi_override_chars`, `share_recipient_rejects_bidi_override` |
| Wrong error variant on bad `--version` value                      | `BadFlagValue { flag, reason }` (static-only payload)      | `get_rejects_bad_version`, `get_rejects_negative_or_oversize_version` |
| `MAX_ARG_LEN` (tighter) made `MAX_INLINE_VALUE_LEN` (looser) unreachable | `MAX_ARG_LEN = 64 KiB` aligned with `MAX_INLINE_VALUE_LEN` | `put_inline_value_accepts_64kib`                  |
| Future `Format` / `ParseError` variants are breaking              | `#[non_exhaustive]` on both                                | structural                                             |
| Help / Version need a host handler                                | `dispatch` provides built-in defaults                      | `driver_dispatch_routes_help_and_version_without_handler` |

## Out of scope (future PRs / sibling crates)

- **Host binary** — `code/programs/rust/vault-cli/` will
  construct the underlying layers and a host-side `CliDriver`
  that delegates.
- **HTTP transport** — `vault-transport-http` with REST +
  WebSocket push for sync; composes VLT05 (auth-as-middleware)
  + VLT06 (policy-as-middleware) + VLT08 (engines exposed as
  paths). Native TLS.
- **gRPC transport** — same surface, different wire.
  Reflection enabled so language bindings auto-generate.
- **FUSE / env-var injector / k8s-csi / browser** — optional
  adapters per VLT00 §VLT11.
- **Stdin handling** — the host binary reads stdin when `Put`'s
  `inline_value` is `None` (and prompts for passphrases on a
  TTY).
- **Localisation** — `Display` impls are English-only; a future
  i18n layer would wrap them.

## Citations

- VLT00-vault-roadmap.md — VLT11 placement.
- HashiCorp Vault CLI — `vault put / get / list / kv` shape.
- 1Password CLI — `op signin / vault list / item get` shape.
- `pass` (zx2c4) — `pass insert / pass show / pass cp` for the
  pass-style workflow surface this subsumes.
- VLT07-vault-leases, VLT08-vault-dynamic-secrets,
  VLT09-vault-audit-log, VLT10-vault-sync-engine — the layers
  the CLI's `CliDriver` ultimately delegates to.
