# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT11 (CLI transport)
  (`code/specs/VLT11-transports.md`).
- `parse_args(&[String]) -> Result<CliCommand, ParseError>` —
  pure parser. No I/O, no panic on user input. Bounded by
  `MAX_ARGV_LEN = 1024`, `MAX_ARG_LEN = 16 KiB`,
  `MAX_PATH_LEN = 4 KiB`, `MAX_INLINE_VALUE_LEN = 64 KiB`.
- `CliCommand` — non-exhaustive enum covering every documented
  subcommand: `Login` / `Unseal` / `Put` / `Get` / `List` /
  `Revoke` / `Share` / `Sync` / `Audit` / `Help` / `Version`.
- `SyncOp` (`Push` / `Pull` / `Status`) and `AuditOp`
  (`Verify` / `Tail`) — non-exhaustive enums for nested ops.
- `VaultPath` — newtype around the `<engine-mount>/<key>`
  string; rejects control characters at construction.
- `ParseError` — narrow variants:
  `NoSubcommand` / `UnknownSubcommand` / `BadArity` /
  `UnknownFlag` / `FlagMissingValue` / `ArgumentTooLong` /
  `TooManyArgs` / `ForbiddenChar`. `Display` impl never echoes
  the bad value (only lengths / positions / static labels), so
  a malicious caller cannot inject text into a downstream log
  via the error message.
- `CommandOutput` — non-exhaustive enum: `Message` /
  `Table { headers, rows }` / `Secret { bytes }`.
  `Secret` is treated specially by the renderer: text/JSON
  modes print only byte length; raw bytes require explicit
  `Format::SecretRaw` (which hex-encodes them rather than
  emitting raw bytes from the string-returning API).
- `Format` — `Text` / `Json` / `SecretRaw`.
- `render_output(&CommandOutput, Format) -> String` —
  hand-rolled formatter. JSON escaping (quote / backslash /
  control characters / `\uXXXX`) without pulling in serde so
  the crate stays dep-free.
- `CliDriver` trait — `Send`-friendly contract that the host
  binary implements. Default `dispatch(cmd)` routes each
  variant to a per-method handler; `Help` and `Version` are
  built-in defaults.
- `help_text()` — hand-rolled help that includes the security
  invariants (passphrases via stdin, no positional secret args,
  bounded inline value).
- 48 unit tests covering: bounds rejection (`MAX_ARGV_LEN` /
  `MAX_ARG_LEN` / `MAX_PATH_LEN` / `MAX_INLINE_VALUE_LEN`),
  empty argv → `NoSubcommand`, every documented subcommand's
  positive parse path, every `BadArity` rejection,
  `UnknownSubcommand` does NOT echo the bad token in the
  error, every `--flag` parser including `--version` numeric
  parse, control-char rejection in vault paths and recipients,
  user newline rejection in `login`, every `Help`/`Version`
  alias, table text + JSON rendering, secret redaction in text
  + JSON modes, secret-raw hex emission, and a
  `RecordingDriver` dispatch test that verifies every variant
  routes to the right handler.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.

### Security hardening (pre-merge review)

Six findings flagged before push, all fixed inline:

- **MEDIUM** — `MAX_ARG_LEN` (16 KiB) was tighter than the
  advertised `MAX_INLINE_VALUE_LEN` (64 KiB), so the latter was
  unreachable: anything > 16 KiB was rejected with the wrong
  bound name and the help text/spec/README claim of "inline
  ≤ 64 KiB" was wrong. Aligned `MAX_ARG_LEN = 64 KiB` so the
  per-subcommand caps fire as intended.
- **LOW** — JSON escaping did not handle U+2028 / U+2029, which
  are valid JSON but break JavaScript string literals. Output
  embedded in a `<script>` block could be terminated by an
  attacker-controlled identifier (sync peer ID, audit
  principal). Added explicit escapes.
- **LOW** — `--version` value parse failures returned
  `FlagMissingValue`, a misleading variant name. Added a
  `BadFlagValue { flag, reason }` variant; updated the parser
  and tests. Static-string-only payloads (no echo).
- **LOW** — Trojan Source / bidi-override defence: rejected
  U+202A–U+202E and U+2066–U+2069 plus the zero-width family
  (U+200B–U+200D, U+FEFF) in `VaultPath::new` and
  `validate_simple_field`. Without this, an attacker-supplied
  `share` recipient or vault path could render differently
  from how it parses, hiding the true target in audit logs and
  TUI displays. (CVE-2021-42574 class.)
- **INFO** — `Format` and `ParseError` were not
  `#[non_exhaustive]`. Future variants would be breaking
  changes. Added the attribute to both.
- **INFO** — `CommandOutput::Secret { bytes }` and
  `hex_encode` allocate non-zeroizing `Vec<u8>` / `String`.
  This is the documented design (the host is responsible for
  custody) but the README and rustdoc now state explicitly
  that hosts MUST treat these as sensitive and wipe before
  drop.

### Out of scope (future PRs)

- **Host binary** — `code/programs/rust/vault-cli/` will wire
  in the underlying layers and call `parse_args` /
  `dispatch` / `render_output`.
- **HTTP transport** — `vault-transport-http` (REST + WebSocket
  push). Same `CommandOutput` shape over a different wire.
- **gRPC transport** — `vault-transport-grpc`.
- **FUSE / env / k8s-csi / browser** transports — the
  optional adapters from VLT00 §VLT11.
- **Stdin handling for `put`** — host binary reads stdin when
  `inline_value` is `None`.
- **Passphrase prompting** — host binary uses
  `rpassword`-style hidden input on a TTY; a daemon mode uses
  authenticated IPC.
- **Localised error messages** — the current `Display` impl is
  English-only; a future i18n layer would wrap it.
