//! # `coding_adventures_vault_transport_cli` — VLT11 transport (CLI)
//!
//! ## What this crate is
//!
//! The **terminal interface** of the Vault stack:
//!
//! ```text
//!   $ vault login alice
//!   $ vault unseal
//!   $ vault put kv/shared/db-password 'hunter2'
//!   $ vault get kv/shared/db-password
//!   $ vault list kv/shared
//!   $ vault share kv/shared/db-password --with bob@x25519:base64-pubkey
//!   $ vault sync push
//!   $ vault sync pull
//!   $ vault audit verify
//!   $ vault revoke kv/shared/db-password --version 3
//! ```
//!
//! The crate splits the CLI cleanly into three pieces:
//!
//!   1. [`parse_args`] — turns a `&[String]` of command-line
//!      arguments into a [`CliCommand`] enum (typed,
//!      exhaustively-handled). Pure: no I/O, no global state,
//!      no panic on user input.
//!   2. [`CliDriver`] — a trait the host implements to actually
//!      do the work (call `LeaseManager::issue`, run a sync
//!      push, etc). The crate hands the parsed command through.
//!   3. [`render_output`] — turns a [`CommandOutput`] into a
//!      string for the terminal (text or JSON), so all
//!      transports get consistent formatting.
//!
//! Each piece is independently testable; the test suite covers
//! every subcommand's parser, every error path, and the
//! formatter.
//!
//! ## Why a library, not a binary
//!
//! The vault core is *headless*. The CLI binary lives in
//! `code/programs/rust/` and is a thin shim that constructs the
//! `Vault` (auth + policy + engines + leases + audit + sync) and
//! a host-side `CliDriver` that delegates to it. Putting the
//! parser/dispatch/formatter in a library lets:
//!
//!   * the integration test suite drive every subcommand without
//!     spawning a process,
//!   * the same dispatch logic wrap a daemon (where the CLI
//!     becomes IPC commands over a unix socket),
//!   * other transports (HTTP, gRPC, FUSE, browser) reuse the
//!     same `CommandOutput` shape so a downstream tool can
//!     post-process command output uniformly.
//!
//! ## Threat model (transport-tier)
//!
//! * **Untrusted shell history**: the CLI never accepts a
//!   passphrase as a positional argument — only via stdin / a
//!   pipe / an env var named in the documentation. This crate
//!   *parses* the flags but the host implementation is
//!   responsible for the actual prompting.
//! * **Argument injection**: positional arguments containing
//!   newlines / NULs / control characters are rejected at the
//!   parser, not the dispatcher. A `vault put` whose key
//!   contains `\n` cannot be smuggled into a downstream layer
//!   because [`parse_args`] surfaces it as
//!   [`ParseError::ForbiddenChar`].
//! * **Argv length**: a fixed cap on arg count and per-arg
//!   length means a malicious caller cannot exhaust memory just
//!   by handing a giant `argv`. Bound is generous (1024 args,
//!   16 KiB each) but real.
//! * **No silent shell expansion**: the parser does not glob,
//!   does not look up env vars, does not interpret `~` — the
//!   shell already did all that, and any of those at the parser
//!   would be a path-traversal hazard once we resolve to a real
//!   filesystem.
//! * **Diagnostic output never echoes secrets**: error messages
//!   and `--verbose` output describe *what kind* of value was
//!   wrong (length, character class) but never echo the value.
//!
//! ## Where it fits
//!
//! ```text
//!   ┌──────────────────────────────────────┐
//!   │  user / shell  / scripts             │
//!   └──────────────────┬───────────────────┘
//!                      │ argv
//!   ┌──────────────────▼───────────────────┐
//!   │  parse_args(argv) -> CliCommand      │  (this crate)
//!   └──────────────────┬───────────────────┘
//!                      │
//!   ┌──────────────────▼───────────────────┐
//!   │  CliDriver::dispatch(cmd) ->         │
//!   │     CommandOutput                    │  (host impls)
//!   └──────────────────┬───────────────────┘
//!                      │
//!   ┌──────────────────▼───────────────────┐
//!   │  render_output(output, format)       │  (this crate)
//!   └──────────────────┬───────────────────┘
//!                      │ stdout
//!   ┌──────────────────▼───────────────────┐
//!   │  user                                │
//!   └──────────────────────────────────────┘
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

// === Section 1. Bounds =====================================================
//
// All of these are wire-tier caps that protect downstream
// allocators from a malicious argv. They're generous (much
// larger than real-world usage) but finite.

/// Maximum number of arguments accepted on a single CLI call.
pub const MAX_ARGV_LEN: usize = 1024;
/// Maximum bytes per single argument. Aligned with
/// [`MAX_INLINE_VALUE_LEN`] so the per-argv pre-flight check
/// does not pre-empt the per-subcommand inline-value cap; per-
/// field bounds (e.g. [`MAX_PATH_LEN`]) are enforced inside the
/// subcommand parsers.
pub const MAX_ARG_LEN: usize = 64 * 1024;
/// Maximum bytes for a vault path (`<engine-mount>/<key>`).
pub const MAX_PATH_LEN: usize = 4 * 1024;
/// Maximum bytes for the inline value passed to `vault put`. Larger
/// values must come over stdin.
pub const MAX_INLINE_VALUE_LEN: usize = 64 * 1024;

// === Section 2. Errors =====================================================

/// All errors produced by the parser / formatter. The dispatcher
/// produces its own error type via the [`CliDriver`] trait.
///
/// `#[non_exhaustive]` so new failure modes (e.g. localised
/// hints) can land without breaking pattern matches at call
/// sites.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// Empty argv (no subcommand). The CLI must always have at
    /// least the program name + one subcommand.
    NoSubcommand,
    /// Unknown subcommand. The string is the offending token.
    UnknownSubcommand(String),
    /// Subcommand exists but caller passed too few/many positional
    /// arguments.
    BadArity {
        /// Subcommand name.
        cmd: &'static str,
        /// Brief description of the expected shape (e.g. "<path>
        /// <value>").
        expected: &'static str,
    },
    /// Caller passed an unknown flag.
    UnknownFlag(String),
    /// A flag that requires a value was given without one.
    FlagMissingValue(&'static str),
    /// A flag's value was present but unparseable. Used (for
    /// example) when `--version` got a non-numeric or
    /// out-of-range token. The static reason describes the
    /// expected shape; the bad value itself is never echoed.
    BadFlagValue {
        /// Flag name.
        flag: &'static str,
        /// Static description of the expected shape.
        reason: &'static str,
    },
    /// An argument exceeded a length cap. The cap is the static
    /// constant; the offending arg's *length* is reported, not
    /// its bytes (so error messages do not echo secrets).
    ArgumentTooLong {
        /// Name of the bound that was exceeded.
        bound: &'static str,
        /// The (cap, observed) pair, in bytes.
        cap_observed: (usize, usize),
    },
    /// argv length exceeded [`MAX_ARGV_LEN`].
    TooManyArgs(usize),
    /// An argument contained a forbidden character (newline, NUL,
    /// control character, etc). Position only; no echo.
    ForbiddenChar {
        /// Name of the field that contained the bad character.
        field: &'static str,
        /// 1-based index of the offending byte (so error messages
        /// can quote a position without revealing the value).
        position: usize,
    },
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoSubcommand => write!(f, "no subcommand given (try `vault help`)"),
            Self::UnknownSubcommand(c) => {
                // Don't echo arbitrary user input verbatim; show length only.
                write!(f, "unknown subcommand (length {} chars)", c.chars().count())
            }
            Self::BadArity { cmd, expected } => {
                write!(f, "bad arity for `{}` (expected: {})", cmd, expected)
            }
            Self::UnknownFlag(flag) => {
                write!(f, "unknown flag (length {})", flag.len())
            }
            Self::FlagMissingValue(flag) => {
                write!(f, "flag `{}` is missing its value", flag)
            }
            Self::BadFlagValue { flag, reason } => {
                write!(f, "flag `{}` has a bad value ({})", flag, reason)
            }
            Self::ArgumentTooLong {
                bound,
                cap_observed,
            } => write!(
                f,
                "argument too long ({}: cap = {} bytes, observed = {} bytes)",
                bound, cap_observed.0, cap_observed.1
            ),
            Self::TooManyArgs(n) => write!(
                f,
                "too many arguments ({} > MAX_ARGV_LEN = {})",
                n, MAX_ARGV_LEN
            ),
            Self::ForbiddenChar { field, position } => write!(
                f,
                "forbidden character in {} at byte {}",
                field, position
            ),
        }
    }
}

impl std::error::Error for ParseError {}

// === Section 3. Command enum ===============================================
//
// Every documented subcommand. New variants land non-breakingly
// because the enum is `#[non_exhaustive]`.

/// A vault path: `<engine-mount>/<key>` (e.g. `kv/shared/db-pw`).
/// We keep it as an opaque newtype so callers can't accidentally
/// pass a free-form string where a vault path is required.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultPath(String);

impl VaultPath {
    /// Construct, validating bounds and character class.
    pub fn new(s: impl Into<String>) -> Result<Self, ParseError> {
        let s = s.into();
        if s.is_empty() {
            return Err(ParseError::ForbiddenChar {
                field: "path",
                position: 0,
            });
        }
        if s.len() > MAX_PATH_LEN {
            return Err(ParseError::ArgumentTooLong {
                bound: "MAX_PATH_LEN",
                cap_observed: (MAX_PATH_LEN, s.len()),
            });
        }
        for (i, b) in s.bytes().enumerate() {
            if b == 0 || b == b'\n' || b == b'\r' || b == b'\t' || (b < 0x20 && b != b' ') {
                return Err(ParseError::ForbiddenChar {
                    field: "path",
                    position: i + 1,
                });
            }
        }
        // Bidi/invisible-character defence — see
        // `validate_simple_field` for the rationale (CVE-2021-42574).
        let mut byte_pos = 0usize;
        for c in s.chars() {
            let cp = c as u32;
            let is_bidi = matches!(cp, 0x202A..=0x202E | 0x2066..=0x2069);
            let is_invisible = matches!(cp, 0x200B..=0x200D | 0xFEFF);
            if is_bidi || is_invisible {
                return Err(ParseError::ForbiddenChar {
                    field: "path",
                    position: byte_pos + 1,
                });
            }
            byte_pos += c.len_utf8();
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One parsed CLI invocation. Each variant captures everything
/// the dispatcher needs to do its job; nothing is left as a
/// loose `&[String]` to be reparsed later.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CliCommand {
    /// `vault login <user>`
    Login {
        /// The principal logging in.
        user: String,
    },
    /// `vault unseal`
    Unseal,
    /// `vault put <path> [<value>]` — value via positional or
    /// stdin.
    Put {
        /// Vault path.
        path: VaultPath,
        /// Inline value, if provided. `None` → stdin.
        inline_value: Option<Vec<u8>>,
    },
    /// `vault get <path> [--version N]`
    Get {
        /// Vault path.
        path: VaultPath,
        /// Optional version number for engines that support it (KV-v2).
        version: Option<u32>,
    },
    /// `vault list <prefix>`
    List {
        /// Prefix (e.g. `kv/shared`).
        prefix: VaultPath,
    },
    /// `vault revoke <path> --version N`
    Revoke {
        /// Vault path.
        path: VaultPath,
        /// Version to revoke.
        version: u32,
    },
    /// `vault share <path> --with <recipient-id>` — adds a
    /// recipient to the wrap-set.
    Share {
        /// Vault path.
        path: VaultPath,
        /// Recipient identifier (engine-defined: `pk:<x25519>`,
        /// `user:<id>`, etc).
        recipient: String,
    },
    /// `vault sync push` / `vault sync pull` / `vault sync status`
    Sync {
        /// What to do.
        op: SyncOp,
    },
    /// `vault audit verify` / `vault audit tail`
    Audit {
        /// What to do.
        op: AuditOp,
    },
    /// `vault help`
    Help,
    /// `vault version`
    Version,
}

/// Subcommand for `vault sync`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyncOp {
    /// Push local state to the server.
    Push,
    /// Pull remote state from the server.
    Pull,
    /// Show current sync status (vector summary).
    Status,
}

/// Subcommand for `vault audit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuditOp {
    /// Walk the chain and verify every signature + prev_hash.
    Verify,
    /// Print the most recent N entries.
    Tail,
}

// === Section 4. Parser =====================================================

/// Parse a `&[String]` argv (excluding the program name) into a
/// [`CliCommand`]. Pure function — no I/O, no panic on bad input.
///
/// Apply the bounds [`MAX_ARGV_LEN`] / [`MAX_ARG_LEN`] up front
/// so a hostile caller cannot blow up our parser.
pub fn parse_args(argv: &[String]) -> Result<CliCommand, ParseError> {
    if argv.len() > MAX_ARGV_LEN {
        return Err(ParseError::TooManyArgs(argv.len()));
    }
    for a in argv {
        if a.len() > MAX_ARG_LEN {
            return Err(ParseError::ArgumentTooLong {
                bound: "MAX_ARG_LEN",
                cap_observed: (MAX_ARG_LEN, a.len()),
            });
        }
    }
    let (sub, rest) = argv.split_first().ok_or(ParseError::NoSubcommand)?;
    match sub.as_str() {
        "login" => parse_login(rest),
        "unseal" => parse_unseal(rest),
        "put" => parse_put(rest),
        "get" => parse_get(rest),
        "list" => parse_list(rest),
        "revoke" => parse_revoke(rest),
        "share" => parse_share(rest),
        "sync" => parse_sync(rest),
        "audit" => parse_audit(rest),
        "help" | "--help" | "-h" => Ok(CliCommand::Help),
        "version" | "--version" | "-V" => Ok(CliCommand::Version),
        _ => Err(ParseError::UnknownSubcommand(sub.clone())),
    }
}

fn require_no_unknown_flags(rest: &[String]) -> Result<(), ParseError> {
    for a in rest {
        if a.starts_with("--") || (a.len() == 2 && a.starts_with('-')) {
            return Err(ParseError::UnknownFlag(a.clone()));
        }
    }
    Ok(())
}

fn validate_simple_field(s: &str, field: &'static str) -> Result<(), ParseError> {
    for (i, b) in s.bytes().enumerate() {
        // Disallow control characters except literal space.
        if b == 0 || b == b'\n' || b == b'\r' || b == b'\t' || (b < 0x20 && b != b' ') {
            return Err(ParseError::ForbiddenChar {
                field,
                position: i + 1,
            });
        }
    }
    // CVE-2021-42574 ("Trojan Source"): reject Unicode bidi-
    // override codepoints (U+202A–U+202E, U+2066–U+2069) so a
    // share-recipient / login-user / vault-path that *renders*
    // differently from how it *parses* cannot slip past human
    // review of audit logs and TUI displays. Also reject the
    // zero-width space + BOM family for similar reasons.
    let mut byte_pos = 0usize;
    for c in s.chars() {
        let cp = c as u32;
        let is_bidi = matches!(cp, 0x202A..=0x202E | 0x2066..=0x2069);
        let is_invisible =
            matches!(cp, 0x200B..=0x200D | 0xFEFF);
        if is_bidi || is_invisible {
            return Err(ParseError::ForbiddenChar {
                field,
                position: byte_pos + 1,
            });
        }
        byte_pos += c.len_utf8();
    }
    Ok(())
}

fn parse_login(rest: &[String]) -> Result<CliCommand, ParseError> {
    if rest.len() != 1 {
        return Err(ParseError::BadArity {
            cmd: "login",
            expected: "<user>",
        });
    }
    let user = rest[0].clone();
    validate_simple_field(&user, "user")?;
    if user.is_empty() {
        return Err(ParseError::BadArity {
            cmd: "login",
            expected: "<user>",
        });
    }
    Ok(CliCommand::Login { user })
}

fn parse_unseal(rest: &[String]) -> Result<CliCommand, ParseError> {
    require_no_unknown_flags(rest)?;
    if !rest.is_empty() {
        return Err(ParseError::BadArity {
            cmd: "unseal",
            expected: "(no arguments)",
        });
    }
    Ok(CliCommand::Unseal)
}

fn parse_put(rest: &[String]) -> Result<CliCommand, ParseError> {
    match rest.len() {
        1 => {
            let path = VaultPath::new(rest[0].clone())?;
            Ok(CliCommand::Put {
                path,
                inline_value: None, // value will come from stdin
            })
        }
        2 => {
            let path = VaultPath::new(rest[0].clone())?;
            if rest[1].len() > MAX_INLINE_VALUE_LEN {
                return Err(ParseError::ArgumentTooLong {
                    bound: "MAX_INLINE_VALUE_LEN",
                    cap_observed: (MAX_INLINE_VALUE_LEN, rest[1].len()),
                });
            }
            // Inline values are kept as bytes — they may contain
            // arbitrary content (the user is putting a *secret*).
            // We don't validate against control chars here because
            // arbitrary binary content is legitimate.
            let inline_value = Some(rest[1].as_bytes().to_vec());
            Ok(CliCommand::Put {
                path,
                inline_value,
            })
        }
        _ => Err(ParseError::BadArity {
            cmd: "put",
            expected: "<path> [<value>]",
        }),
    }
}

fn parse_get(rest: &[String]) -> Result<CliCommand, ParseError> {
    let mut path: Option<VaultPath> = None;
    let mut version: Option<u32> = None;
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        if a == "--version" {
            let v = rest
                .get(i + 1)
                .ok_or(ParseError::FlagMissingValue("--version"))?;
            version = Some(v.parse::<u32>().map_err(|_| ParseError::BadFlagValue {
                flag: "--version",
                reason: "must be a non-negative integer in u32 range",
            })?);
            i += 2;
        } else if a.starts_with("--") {
            return Err(ParseError::UnknownFlag(a.clone()));
        } else {
            if path.is_some() {
                return Err(ParseError::BadArity {
                    cmd: "get",
                    expected: "<path> [--version N]",
                });
            }
            path = Some(VaultPath::new(a.clone())?);
            i += 1;
        }
    }
    let path = path.ok_or(ParseError::BadArity {
        cmd: "get",
        expected: "<path> [--version N]",
    })?;
    Ok(CliCommand::Get { path, version })
}

fn parse_list(rest: &[String]) -> Result<CliCommand, ParseError> {
    if rest.len() != 1 {
        return Err(ParseError::BadArity {
            cmd: "list",
            expected: "<prefix>",
        });
    }
    let prefix = VaultPath::new(rest[0].clone())?;
    Ok(CliCommand::List { prefix })
}

fn parse_revoke(rest: &[String]) -> Result<CliCommand, ParseError> {
    let mut path: Option<VaultPath> = None;
    let mut version: Option<u32> = None;
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        if a == "--version" {
            let v = rest
                .get(i + 1)
                .ok_or(ParseError::FlagMissingValue("--version"))?;
            version = Some(v.parse::<u32>().map_err(|_| ParseError::BadFlagValue {
                flag: "--version",
                reason: "must be a non-negative integer in u32 range",
            })?);
            i += 2;
        } else if a.starts_with("--") {
            return Err(ParseError::UnknownFlag(a.clone()));
        } else {
            if path.is_some() {
                return Err(ParseError::BadArity {
                    cmd: "revoke",
                    expected: "<path> --version N",
                });
            }
            path = Some(VaultPath::new(a.clone())?);
            i += 1;
        }
    }
    let path = path.ok_or(ParseError::BadArity {
        cmd: "revoke",
        expected: "<path> --version N",
    })?;
    let version = version.ok_or(ParseError::BadArity {
        cmd: "revoke",
        expected: "<path> --version N",
    })?;
    Ok(CliCommand::Revoke { path, version })
}

fn parse_share(rest: &[String]) -> Result<CliCommand, ParseError> {
    let mut path: Option<VaultPath> = None;
    let mut recipient: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        let a = &rest[i];
        if a == "--with" {
            let v = rest
                .get(i + 1)
                .ok_or(ParseError::FlagMissingValue("--with"))?
                .clone();
            validate_simple_field(&v, "recipient")?;
            recipient = Some(v);
            i += 2;
        } else if a.starts_with("--") {
            return Err(ParseError::UnknownFlag(a.clone()));
        } else {
            if path.is_some() {
                return Err(ParseError::BadArity {
                    cmd: "share",
                    expected: "<path> --with <recipient>",
                });
            }
            path = Some(VaultPath::new(a.clone())?);
            i += 1;
        }
    }
    let path = path.ok_or(ParseError::BadArity {
        cmd: "share",
        expected: "<path> --with <recipient>",
    })?;
    let recipient = recipient.ok_or(ParseError::BadArity {
        cmd: "share",
        expected: "<path> --with <recipient>",
    })?;
    Ok(CliCommand::Share { path, recipient })
}

fn parse_sync(rest: &[String]) -> Result<CliCommand, ParseError> {
    if rest.len() != 1 {
        return Err(ParseError::BadArity {
            cmd: "sync",
            expected: "push | pull | status",
        });
    }
    let op = match rest[0].as_str() {
        "push" => SyncOp::Push,
        "pull" => SyncOp::Pull,
        "status" => SyncOp::Status,
        _ => {
            return Err(ParseError::BadArity {
                cmd: "sync",
                expected: "push | pull | status",
            })
        }
    };
    Ok(CliCommand::Sync { op })
}

fn parse_audit(rest: &[String]) -> Result<CliCommand, ParseError> {
    if rest.len() != 1 {
        return Err(ParseError::BadArity {
            cmd: "audit",
            expected: "verify | tail",
        });
    }
    let op = match rest[0].as_str() {
        "verify" => AuditOp::Verify,
        "tail" => AuditOp::Tail,
        _ => {
            return Err(ParseError::BadArity {
                cmd: "audit",
                expected: "verify | tail",
            })
        }
    };
    Ok(CliCommand::Audit { op })
}

// === Section 5. Output =====================================================

/// What a [`CliDriver`] produces. Renderers turn this into a
/// terminal-ready string.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandOutput {
    /// A single human-readable line ("OK", "logged in as alice").
    Message(String),
    /// A sequence of rows for `list` / `audit tail` / etc.
    Table {
        /// Column headers.
        headers: Vec<String>,
        /// Data rows (`headers.len()` columns each).
        rows: Vec<Vec<String>>,
    },
    /// A `get`'s decrypted bytes — caller is responsible for not
    /// echoing them to a terminal that records history.
    ///
    /// **Custody warning**: the inner `Vec<u8>` is *not*
    /// zeroized on drop. Hosts that consume this variant MUST
    /// wrap or copy the bytes into a zeroizing container
    /// (`coding_adventures_zeroize::Zeroizing<Vec<u8>>`) before
    /// they drop, and MUST scrub any rendered string from
    /// [`render_output`] (e.g. via `Zeroizing<String>`) before
    /// dropping it. The crate stays dep-free by design; custody
    /// is the host's responsibility.
    Secret {
        /// The decrypted bytes. Renderers print only the byte
        /// length unless the format is `Format::SecretRaw`.
        bytes: Vec<u8>,
    },
}

/// Output format selector. `Text` prints a human-readable form;
/// `Json` prints a structured object suitable for piping to
/// `jq`. `SecretRaw` is only for `get`-style commands and emits
/// raw bytes (typically piped into a downstream tool).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    /// Human-readable.
    Text,
    /// Structured JSON for tooling.
    Json,
    /// Raw bytes; only meaningful for `Secret`.
    SecretRaw,
}

/// Render an output for the terminal. Always returns a `String`
/// for Text/Json, or a hex-escaped form for SecretRaw when the
/// caller can't take raw bytes (e.g. integration tests).
pub fn render_output(out: &CommandOutput, fmt: Format) -> String {
    match (out, fmt) {
        (CommandOutput::Message(m), Format::Text) => m.clone(),
        (CommandOutput::Message(m), Format::Json) => {
            format!("{{\"message\":\"{}\"}}", json_escape(m))
        }
        (CommandOutput::Message(_), Format::SecretRaw) => String::new(),
        (CommandOutput::Table { headers, rows }, Format::Text) => render_table_text(headers, rows),
        (CommandOutput::Table { headers, rows }, Format::Json) => render_table_json(headers, rows),
        (CommandOutput::Table { .. }, Format::SecretRaw) => String::new(),
        (CommandOutput::Secret { bytes }, Format::Text) => {
            // Default: do NOT echo the secret. Print the length.
            format!("<{} bytes; pipe with --format secret-raw to retrieve>", bytes.len())
        }
        (CommandOutput::Secret { bytes }, Format::Json) => {
            format!("{{\"secret_len\":{}}}", bytes.len())
        }
        (CommandOutput::Secret { bytes }, Format::SecretRaw) => {
            // Caller explicitly opted in. Hex-encode here for the
            // string-returning API; binary callers should grab
            // `bytes` directly off the `CommandOutput::Secret`.
            hex_encode(bytes)
        }
    }
}

fn render_table_text(headers: &[String], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    if !headers.is_empty() {
        out.push_str(&headers.join("\t"));
        out.push('\n');
    }
    for row in rows {
        out.push_str(&row.join("\t"));
        out.push('\n');
    }
    out
}

fn render_table_json(headers: &[String], rows: &[Vec<String>]) -> String {
    // Hand-rolled JSON to avoid pulling in serde — the strings
    // we encode are bounded (`MAX_ARG_LEN`-class) and we already
    // own the escaping helper.
    let mut out = String::new();
    out.push('[');
    for (ri, row) in rows.iter().enumerate() {
        if ri > 0 {
            out.push(',');
        }
        out.push('{');
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            // If headers run out, fall back to "col_<i>".
            let key = headers
                .get(i)
                .map(|s| s.as_str())
                .unwrap_or("col");
            out.push('"');
            out.push_str(&json_escape(key));
            out.push_str("\":\"");
            out.push_str(&json_escape(cell));
            out.push('"');
        }
        out.push('}');
    }
    out.push(']');
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // U+2028 (LINE SEPARATOR) and U+2029 (PARAGRAPH
            // SEPARATOR) are valid in JSON but break JavaScript
            // string literals. Escape them so JSON output
            // embedded in a `<script>` block or fed to
            // `eval()` cannot be terminated by an
            // attacker-controlled identifier (a sync peer ID,
            // an audit-log principal, …).
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

const HEX_LO: &[u8; 16] = b"0123456789abcdef";

fn hex_encode(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for byte in b {
        s.push(HEX_LO[(byte >> 4) as usize] as char);
        s.push(HEX_LO[(byte & 0x0f) as usize] as char);
    }
    s
}

// === Section 6. Driver trait ================================================

/// What the host binary implements. Each variant of [`CliCommand`]
/// turns into one method call. Defaulted to "not implemented yet"
/// so a host can tackle commands incrementally.
pub trait CliDriver {
    /// Engine-defined error type.
    type Error;

    /// Execute the parsed command. The default impl pattern-
    /// matches on the variant and dispatches to the per-method
    /// handlers below.
    fn dispatch(&mut self, cmd: CliCommand) -> Result<CommandOutput, Self::Error> {
        match cmd {
            CliCommand::Login { user } => self.login(user),
            CliCommand::Unseal => self.unseal(),
            CliCommand::Put {
                path,
                inline_value,
            } => self.put(path, inline_value),
            CliCommand::Get { path, version } => self.get(path, version),
            CliCommand::List { prefix } => self.list(prefix),
            CliCommand::Revoke { path, version } => self.revoke(path, version),
            CliCommand::Share { path, recipient } => self.share(path, recipient),
            CliCommand::Sync { op } => self.sync(op),
            CliCommand::Audit { op } => self.audit(op),
            CliCommand::Help => Ok(CommandOutput::Message(help_text().to_owned())),
            CliCommand::Version => {
                Ok(CommandOutput::Message(format!("vault {}", env!("CARGO_PKG_VERSION"))))
            }
        }
    }

    /// `vault login <user>` handler.
    fn login(&mut self, user: String) -> Result<CommandOutput, Self::Error>;
    /// `vault unseal` handler.
    fn unseal(&mut self) -> Result<CommandOutput, Self::Error>;
    /// `vault put` handler.
    fn put(
        &mut self,
        path: VaultPath,
        inline_value: Option<Vec<u8>>,
    ) -> Result<CommandOutput, Self::Error>;
    /// `vault get` handler.
    fn get(
        &mut self,
        path: VaultPath,
        version: Option<u32>,
    ) -> Result<CommandOutput, Self::Error>;
    /// `vault list` handler.
    fn list(&mut self, prefix: VaultPath) -> Result<CommandOutput, Self::Error>;
    /// `vault revoke` handler.
    fn revoke(&mut self, path: VaultPath, version: u32) -> Result<CommandOutput, Self::Error>;
    /// `vault share` handler.
    fn share(
        &mut self,
        path: VaultPath,
        recipient: String,
    ) -> Result<CommandOutput, Self::Error>;
    /// `vault sync (push|pull|status)` handler.
    fn sync(&mut self, op: SyncOp) -> Result<CommandOutput, Self::Error>;
    /// `vault audit (verify|tail)` handler.
    fn audit(&mut self, op: AuditOp) -> Result<CommandOutput, Self::Error>;
}

/// Built-in help text. Hand-rolled rather than generated so the
/// security-relevant invariants (no positional secret args, env
/// vars used for passphrases) are in the help itself.
pub fn help_text() -> &'static str {
    "vault — encrypted key/value vault\n\
     \n\
     USAGE:\n\
       vault <COMMAND> [args...]\n\
     \n\
     COMMANDS:\n\
       login <user>            authenticate (passphrase via stdin)\n\
       unseal                  unseal the vault (passphrase via stdin)\n\
       put <path> [<value>]    store a value (value via stdin if omitted)\n\
       get <path> [--version N]  fetch a value\n\
       list <prefix>           list keys under a prefix\n\
       revoke <path> --version N  destroy a specific version\n\
       share <path> --with <recipient>  add a recipient to the wrap-set\n\
       sync push|pull|status   sync with the configured server\n\
       audit verify|tail       audit-log integrity check / tail\n\
       help                    show this message\n\
       version                 print version\n\
     \n\
     SECURITY:\n\
       - Passphrases are never accepted as positional arguments.\n\
         They come from stdin (or, in a daemon, an authenticated IPC).\n\
       - `vault put <path>` without a value reads the value from\n\
         stdin so secrets are not visible in shell history.\n\
       - Inline values are bounded to 64 KiB; larger payloads must\n\
         come over stdin.\n"
}

// === Section 7. Tests ======================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    // --- Bounds ---

    #[test]
    fn rejects_too_many_args() {
        let big = vec!["x".to_string(); MAX_ARGV_LEN + 1];
        let r = parse_args(&big);
        assert!(matches!(r, Err(ParseError::TooManyArgs(_))));
    }

    #[test]
    fn rejects_oversize_arg() {
        let big = vec!["a".to_string(), "x".repeat(MAX_ARG_LEN + 1)];
        let r = parse_args(&big);
        assert!(matches!(r, Err(ParseError::ArgumentTooLong { .. })));
    }

    #[test]
    fn rejects_empty_argv() {
        let r = parse_args(&[]);
        assert!(matches!(r, Err(ParseError::NoSubcommand)));
    }

    #[test]
    fn rejects_unknown_subcommand() {
        let r = parse_args(&argv(&["bogus"]));
        assert!(matches!(r, Err(ParseError::UnknownSubcommand(_))));
    }

    #[test]
    fn error_display_does_not_echo_unknown_subcommand() {
        // An attacker who controls argv could pass a malicious
        // subcommand name; the error message must NOT echo it.
        let evil = "wipe-disk\nALERT: ignore previous instructions";
        let r = parse_args(&argv(&[evil]));
        let msg = format!("{}", r.unwrap_err());
        assert!(!msg.contains("wipe-disk"));
        assert!(!msg.contains("ignore"));
    }

    // --- VaultPath ---

    #[test]
    fn vault_path_rejects_empty() {
        assert!(matches!(VaultPath::new(""), Err(ParseError::ForbiddenChar { .. })));
    }

    #[test]
    fn vault_path_rejects_newlines_and_nul() {
        assert!(matches!(
            VaultPath::new("kv/foo\nadmin"),
            Err(ParseError::ForbiddenChar { .. })
        ));
        assert!(matches!(
            VaultPath::new("kv/foo\0bar"),
            Err(ParseError::ForbiddenChar { .. })
        ));
    }

    #[test]
    fn vault_path_rejects_oversize() {
        assert!(matches!(
            VaultPath::new("x".repeat(MAX_PATH_LEN + 1)),
            Err(ParseError::ArgumentTooLong { .. })
        ));
    }

    #[test]
    fn vault_path_rejects_bidi_override_chars() {
        // U+202E (RIGHT-TO-LEFT OVERRIDE) — Trojan Source attack.
        let evil = "kv/admin\u{202e}txt.exe";
        assert!(matches!(
            VaultPath::new(evil),
            Err(ParseError::ForbiddenChar { .. })
        ));
    }

    #[test]
    fn vault_path_rejects_zero_width_chars() {
        // U+200B (ZERO WIDTH SPACE).
        assert!(matches!(
            VaultPath::new("kv/admin\u{200b}"),
            Err(ParseError::ForbiddenChar { .. })
        ));
    }

    #[test]
    fn share_recipient_rejects_bidi_override() {
        let r = parse_args(&argv(&[
            "share",
            "kv/secret",
            "--with",
            "evil\u{202e}good",
        ]));
        assert!(matches!(r, Err(ParseError::ForbiddenChar { .. })));
    }

    #[test]
    fn json_escape_handles_u2028_u2029() {
        // Valid JSON, but break <script> contexts. Relevant if
        // CLI output is later embedded in JS.
        let out = CommandOutput::Message("a\u{2028}b\u{2029}c".into());
        let s = render_output(&out, Format::Json);
        assert!(s.contains("\\u2028"));
        assert!(s.contains("\\u2029"));
        assert!(!s.contains('\u{2028}'));
        assert!(!s.contains('\u{2029}'));
    }

    #[test]
    fn put_inline_value_accepts_64kib() {
        // The advertised inline cap is 64 KiB. After the
        // MAX_ARG_LEN/MAX_INLINE_VALUE_LEN reconciliation, a 64
        // KiB inline value must be accepted.
        let big = "x".repeat(MAX_INLINE_VALUE_LEN);
        let r = parse_args(&argv(&["put", "kv/k", &big]));
        assert!(r.is_ok(), "expected 64 KiB inline value to parse");
    }

    #[test]
    fn vault_path_accepts_normal_paths() {
        let p = VaultPath::new("kv/shared/db-pw").unwrap();
        assert_eq!(p.as_str(), "kv/shared/db-pw");
    }

    // --- login ---

    #[test]
    fn parses_login() {
        let r = parse_args(&argv(&["login", "alice"])).unwrap();
        assert_eq!(r, CliCommand::Login { user: "alice".into() });
    }

    #[test]
    fn login_requires_user() {
        assert!(matches!(
            parse_args(&argv(&["login"])),
            Err(ParseError::BadArity { .. })
        ));
        assert!(matches!(
            parse_args(&argv(&["login", "alice", "extra"])),
            Err(ParseError::BadArity { .. })
        ));
    }

    #[test]
    fn login_rejects_user_with_newline() {
        assert!(matches!(
            parse_args(&argv(&["login", "alice\nadmin"])),
            Err(ParseError::ForbiddenChar { .. })
        ));
    }

    // --- unseal ---

    #[test]
    fn parses_unseal() {
        assert_eq!(parse_args(&argv(&["unseal"])).unwrap(), CliCommand::Unseal);
    }

    #[test]
    fn unseal_rejects_extra_args() {
        assert!(matches!(
            parse_args(&argv(&["unseal", "extra"])),
            Err(ParseError::BadArity { .. })
        ));
    }

    // --- put ---

    #[test]
    fn parses_put_without_value() {
        let r = parse_args(&argv(&["put", "kv/secret"])).unwrap();
        match r {
            CliCommand::Put { path, inline_value } => {
                assert_eq!(path.as_str(), "kv/secret");
                assert!(inline_value.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn parses_put_with_value() {
        let r = parse_args(&argv(&["put", "kv/secret", "hunter2"])).unwrap();
        match r {
            CliCommand::Put { inline_value, .. } => {
                assert_eq!(inline_value.unwrap(), b"hunter2");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn put_rejects_oversize_inline_value() {
        let big = "x".repeat(MAX_INLINE_VALUE_LEN + 1);
        let r = parse_args(&argv(&["put", "kv/k", &big]));
        assert!(matches!(r, Err(ParseError::ArgumentTooLong { .. })));
    }

    // --- get ---

    #[test]
    fn parses_get_without_version() {
        let r = parse_args(&argv(&["get", "kv/secret"])).unwrap();
        match r {
            CliCommand::Get { version, .. } => assert_eq!(version, None),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn parses_get_with_version() {
        let r = parse_args(&argv(&["get", "kv/secret", "--version", "3"])).unwrap();
        match r {
            CliCommand::Get { version, .. } => assert_eq!(version, Some(3)),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn get_rejects_bad_version() {
        let r = parse_args(&argv(&["get", "kv/secret", "--version", "not-a-number"]));
        assert!(matches!(r, Err(ParseError::BadFlagValue { .. })));
    }

    #[test]
    fn get_rejects_negative_or_oversize_version() {
        for bad in &["-1", "4294967296", "  3  ", "0xff"] {
            let r = parse_args(&argv(&["get", "kv/secret", "--version", bad]));
            assert!(
                matches!(r, Err(ParseError::BadFlagValue { .. })),
                "expected BadFlagValue for {:?}",
                bad
            );
        }
    }

    #[test]
    fn get_rejects_missing_version_value() {
        let r = parse_args(&argv(&["get", "kv/secret", "--version"]));
        assert!(matches!(r, Err(ParseError::FlagMissingValue(_))));
    }

    #[test]
    fn get_rejects_unknown_flag() {
        let r = parse_args(&argv(&["get", "kv/secret", "--bogus", "5"]));
        assert!(matches!(r, Err(ParseError::UnknownFlag(_))));
    }

    // --- revoke ---

    #[test]
    fn parses_revoke() {
        let r = parse_args(&argv(&["revoke", "kv/secret", "--version", "7"])).unwrap();
        assert_eq!(
            r,
            CliCommand::Revoke {
                path: VaultPath::new("kv/secret").unwrap(),
                version: 7
            }
        );
    }

    #[test]
    fn revoke_requires_version() {
        let r = parse_args(&argv(&["revoke", "kv/secret"]));
        assert!(matches!(r, Err(ParseError::BadArity { .. })));
    }

    // --- share ---

    #[test]
    fn parses_share() {
        let r = parse_args(&argv(&[
            "share",
            "kv/secret",
            "--with",
            "pk:base64-thing",
        ]))
        .unwrap();
        assert_eq!(
            r,
            CliCommand::Share {
                path: VaultPath::new("kv/secret").unwrap(),
                recipient: "pk:base64-thing".into()
            }
        );
    }

    #[test]
    fn share_rejects_recipient_with_control_chars() {
        let r = parse_args(&argv(&["share", "kv/secret", "--with", "bob\nadmin"]));
        assert!(matches!(r, Err(ParseError::ForbiddenChar { .. })));
    }

    // --- sync / audit ---

    #[test]
    fn parses_sync_subops() {
        for (s, op) in &[("push", SyncOp::Push), ("pull", SyncOp::Pull), ("status", SyncOp::Status)] {
            assert_eq!(parse_args(&argv(&["sync", s])).unwrap(), CliCommand::Sync { op: *op });
        }
    }

    #[test]
    fn sync_rejects_unknown_op() {
        assert!(matches!(
            parse_args(&argv(&["sync", "wat"])),
            Err(ParseError::BadArity { .. })
        ));
    }

    #[test]
    fn parses_audit_subops() {
        for (s, op) in &[("verify", AuditOp::Verify), ("tail", AuditOp::Tail)] {
            assert_eq!(parse_args(&argv(&["audit", s])).unwrap(), CliCommand::Audit { op: *op });
        }
    }

    #[test]
    fn audit_rejects_unknown_op() {
        assert!(matches!(
            parse_args(&argv(&["audit", "wat"])),
            Err(ParseError::BadArity { .. })
        ));
    }

    // --- help / version ---

    #[test]
    fn parses_help_in_all_forms() {
        for form in &["help", "--help", "-h"] {
            assert_eq!(parse_args(&argv(&[form])).unwrap(), CliCommand::Help);
        }
    }

    #[test]
    fn parses_version_in_all_forms() {
        for form in &["version", "--version", "-V"] {
            assert_eq!(parse_args(&argv(&[form])).unwrap(), CliCommand::Version);
        }
    }

    #[test]
    fn help_text_documents_security_invariants() {
        let h = help_text();
        assert!(h.contains("Passphrases are never accepted as positional"));
        assert!(h.contains("stdin"));
    }

    // --- Output rendering ---

    #[test]
    fn render_message_text() {
        let out = CommandOutput::Message("logged in as alice".into());
        assert_eq!(
            render_output(&out, Format::Text),
            "logged in as alice"
        );
    }

    #[test]
    fn render_message_json_escapes_quotes_and_newlines() {
        let out = CommandOutput::Message("hello \"world\"\nbye".into());
        let s = render_output(&out, Format::Json);
        assert!(s.contains("hello \\\"world\\\""));
        assert!(s.contains("\\n"));
    }

    #[test]
    fn render_secret_text_does_not_echo_bytes() {
        let out = CommandOutput::Secret { bytes: b"hunter2".to_vec() };
        let s = render_output(&out, Format::Text);
        assert!(!s.contains("hunter2"));
        assert!(s.contains("7 bytes"));
    }

    #[test]
    fn render_secret_json_does_not_echo_bytes() {
        let out = CommandOutput::Secret { bytes: b"hunter2".to_vec() };
        let s = render_output(&out, Format::Json);
        assert!(!s.contains("hunter2"));
        assert!(s.contains("\"secret_len\":7"));
    }

    #[test]
    fn render_secret_raw_emits_hex() {
        let out = CommandOutput::Secret { bytes: vec![0xde, 0xad, 0xbe, 0xef] };
        let s = render_output(&out, Format::SecretRaw);
        assert_eq!(s, "deadbeef");
    }

    #[test]
    fn render_table_text_tabbed() {
        let out = CommandOutput::Table {
            headers: vec!["key".into(), "value".into()],
            rows: vec![vec!["a".into(), "1".into()], vec!["b".into(), "2".into()]],
        };
        let s = render_output(&out, Format::Text);
        assert!(s.contains("key\tvalue"));
        assert!(s.contains("a\t1"));
        assert!(s.contains("b\t2"));
    }

    #[test]
    fn render_table_json_object_per_row() {
        let out = CommandOutput::Table {
            headers: vec!["key".into(), "value".into()],
            rows: vec![vec!["a".into(), "1".into()]],
        };
        let s = render_output(&out, Format::Json);
        assert_eq!(s, "[{\"key\":\"a\",\"value\":\"1\"}]");
    }

    // --- CliDriver ---

    /// Test driver that records what was dispatched.
    struct RecordingDriver {
        calls: Vec<String>,
    }
    impl CliDriver for RecordingDriver {
        type Error = String;
        fn login(&mut self, user: String) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!("login:{}", user));
            Ok(CommandOutput::Message("ok".into()))
        }
        fn unseal(&mut self) -> Result<CommandOutput, Self::Error> {
            self.calls.push("unseal".into());
            Ok(CommandOutput::Message("ok".into()))
        }
        fn put(
            &mut self,
            path: VaultPath,
            inline_value: Option<Vec<u8>>,
        ) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!(
                "put:{}:{}",
                path.as_str(),
                inline_value.map(|v| v.len()).unwrap_or(0)
            ));
            Ok(CommandOutput::Message("ok".into()))
        }
        fn get(
            &mut self,
            path: VaultPath,
            version: Option<u32>,
        ) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!("get:{}:{:?}", path.as_str(), version));
            Ok(CommandOutput::Secret { bytes: b"fake".to_vec() })
        }
        fn list(&mut self, prefix: VaultPath) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!("list:{}", prefix.as_str()));
            Ok(CommandOutput::Table {
                headers: vec!["key".into()],
                rows: vec![],
            })
        }
        fn revoke(&mut self, path: VaultPath, version: u32) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!("revoke:{}:{}", path.as_str(), version));
            Ok(CommandOutput::Message("ok".into()))
        }
        fn share(
            &mut self,
            path: VaultPath,
            recipient: String,
        ) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!("share:{}:{}", path.as_str(), recipient));
            Ok(CommandOutput::Message("ok".into()))
        }
        fn sync(&mut self, op: SyncOp) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!("sync:{:?}", op));
            Ok(CommandOutput::Message("ok".into()))
        }
        fn audit(&mut self, op: AuditOp) -> Result<CommandOutput, Self::Error> {
            self.calls.push(format!("audit:{:?}", op));
            Ok(CommandOutput::Message("ok".into()))
        }
    }

    #[test]
    fn driver_dispatch_routes_each_command() {
        let mut d = RecordingDriver { calls: vec![] };
        for cmd in &[
            CliCommand::Login { user: "alice".into() },
            CliCommand::Unseal,
            CliCommand::Put {
                path: VaultPath::new("kv/k").unwrap(),
                inline_value: Some(b"v".to_vec()),
            },
            CliCommand::Get {
                path: VaultPath::new("kv/k").unwrap(),
                version: None,
            },
            CliCommand::List { prefix: VaultPath::new("kv").unwrap() },
            CliCommand::Revoke {
                path: VaultPath::new("kv/k").unwrap(),
                version: 1,
            },
            CliCommand::Share {
                path: VaultPath::new("kv/k").unwrap(),
                recipient: "bob".into(),
            },
            CliCommand::Sync { op: SyncOp::Push },
            CliCommand::Audit { op: AuditOp::Verify },
        ] {
            d.dispatch(cmd.clone()).unwrap();
        }
        assert_eq!(d.calls.len(), 9);
        assert!(d.calls[0].starts_with("login:alice"));
        assert!(d.calls[1] == "unseal");
        assert!(d.calls[2].starts_with("put:kv/k:1"));
        assert!(d.calls[3].starts_with("get:kv/k:None"));
    }

    #[test]
    fn driver_dispatch_routes_help_and_version_without_handler() {
        let mut d = RecordingDriver { calls: vec![] };
        let help = d.dispatch(CliCommand::Help).unwrap();
        match help {
            CommandOutput::Message(s) => assert!(s.contains("USAGE")),
            _ => panic!("expected message"),
        }
        let version = d.dispatch(CliCommand::Version).unwrap();
        match version {
            CommandOutput::Message(s) => assert!(s.starts_with("vault ")),
            _ => panic!("expected message"),
        }
    }
}
