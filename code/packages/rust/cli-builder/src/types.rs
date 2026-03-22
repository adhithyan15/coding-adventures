// types.rs -- Output types and spec schema types
// =================================================
//
// This module has two halves:
//
//   1. **Spec schema** — `serde`-deserializable structs that mirror the JSON
//      structure described in §2 of the spec. These are what `spec_loader.rs`
//      produces after reading and validating a JSON file.
//
//   2. **Parser outputs** — `ParseResult`, `HelpResult`, `VersionResult`, and
//      the `ParserOutput` enum that wraps them. These are what `parser.rs`
//      returns to callers.
//
// # Why keep schema and output types in one file?
//
// Many spec types (e.g. `FlagDef`) are referenced directly in `ParseResult`
// during default-value population. Co-locating them avoids a dependency tangle
// between modules.

use std::collections::HashMap;
use serde::Deserialize;
use serde_json::Value;

// ===========================================================================
// Spec schema types (§2)
// ===========================================================================

/// The top-level CLI specification document (§2.1).
///
/// This is the root object deserialized from a JSON spec file. Every other
/// type in the schema hangs off this one.
#[derive(Debug, Clone, Deserialize)]
pub struct CliSpec {
    /// Spec format version. Must be `"1.0"`.
    pub cli_builder_spec_version: String,

    /// Program name as invoked (e.g. `"ls"`, `"git"`).
    pub name: String,

    /// Human-readable display name for help output. Optional.
    #[serde(default)]
    pub display_name: Option<String>,

    /// One-line description. Shown in help.
    pub description: String,

    /// Version string. If present, `--version` is auto-enabled.
    #[serde(default)]
    pub version: Option<String>,

    /// Parsing mode: `"posix"`, `"gnu"`, `"subcommand_first"`, or `"traditional"`.
    /// Defaults to `"gnu"`.
    #[serde(default = "default_parsing_mode")]
    pub parsing_mode: String,

    /// Control auto-injection of `--help` and `--version`.
    #[serde(default = "default_builtin_flags")]
    pub builtin_flags: BuiltinFlags,

    /// Flags valid at every nesting level.
    #[serde(default)]
    pub global_flags: Vec<FlagDef>,

    /// Flags valid only at root level.
    #[serde(default)]
    pub flags: Vec<FlagDef>,

    /// Positional arguments at root level.
    #[serde(default)]
    pub arguments: Vec<ArgumentDef>,

    /// Subcommands (recursive).
    #[serde(default)]
    pub commands: Vec<CommandDef>,

    /// Mutually exclusive flag groups at root level.
    #[serde(default)]
    pub mutually_exclusive_groups: Vec<ExclusiveGroup>,
}

fn default_parsing_mode() -> String {
    "gnu".to_string()
}

fn default_builtin_flags() -> BuiltinFlags {
    BuiltinFlags { help: true, version: true }
}

/// Controls whether `--help` / `--version` are auto-injected (§2.1).
#[derive(Debug, Clone, Deserialize)]
pub struct BuiltinFlags {
    /// Inject `--help` / `-h`. Default: `true`.
    #[serde(default = "bool_true")]
    pub help: bool,
    /// Inject `--version`. Default: `true`.
    #[serde(default = "bool_true")]
    pub version: bool,
}

fn bool_true() -> bool {
    true
}

/// A flag definition (§2.2).
///
/// At least one of `short`, `long`, or `single_dash_long` must be present.
#[derive(Debug, Clone, Deserialize)]
pub struct FlagDef {
    /// Unique ID within the scope. Used as the key in `ParseResult.flags`.
    pub id: String,

    /// Single-character short form without the `-` prefix (e.g. `"l"`).
    #[serde(default)]
    pub short: Option<String>,

    /// Long form without the `--` prefix (e.g. `"long-listing"`).
    #[serde(default)]
    pub long: Option<String>,

    /// Multi-character single-dash name (e.g. `"classpath"` → `-classpath`).
    #[serde(default)]
    pub single_dash_long: Option<String>,

    /// Human-readable description. Shown in help output.
    pub description: String,

    /// Value type: `"boolean"`, `"string"`, `"integer"`, `"float"`,
    /// `"path"`, `"file"`, `"directory"`, or `"enum"`.
    #[serde(rename = "type")]
    pub flag_type: String,

    /// Whether this flag must be present. Default: `false`.
    #[serde(default)]
    pub required: bool,

    /// Default value when the flag is absent and `required` is `false`.
    #[serde(default)]
    pub default: Option<Value>,

    /// Shown in help for non-boolean flags: `--output=VALUE`.
    #[serde(default)]
    pub value_name: Option<String>,

    /// Valid values when `flag_type` is `"enum"`.
    #[serde(default)]
    pub enum_values: Vec<String>,

    /// IDs of flags that cannot be used alongside this one.
    #[serde(default)]
    pub conflicts_with: Vec<String>,

    /// IDs of flags that must also be present when this flag is used.
    #[serde(default)]
    pub requires: Vec<String>,

    /// This flag is required unless at least one of these flag IDs is present.
    #[serde(default)]
    pub required_unless: Vec<String>,

    /// If `true`, the flag may appear multiple times; result is an array.
    #[serde(default)]
    pub repeatable: bool,
}

/// A positional argument definition (§2.3).
#[derive(Debug, Clone, Deserialize)]
pub struct ArgumentDef {
    /// Unique ID within the scope. Used as the key in `ParseResult.arguments`.
    pub id: String,

    /// Display name in help (e.g. `"FILE"`, `"DEST"`).
    /// Accepts `display_name` (preferred) or `name` (backward compatibility).
    #[serde(alias = "name")]
    pub display_name: String,

    /// Human-readable description.
    pub description: String,

    /// Value type.
    #[serde(rename = "type")]
    pub arg_type: String,

    /// Whether at least one value must be provided. Default: `true`.
    #[serde(default = "bool_true")]
    pub required: bool,

    /// Whether multiple values may be provided. Default: `false`.
    #[serde(default)]
    pub variadic: bool,

    /// Minimum count when `variadic` is `true`. Default: `1`.
    #[serde(default = "default_variadic_min")]
    pub variadic_min: usize,

    /// Maximum count when `variadic` is `true`. `None` = unlimited.
    #[serde(default)]
    pub variadic_max: Option<usize>,

    /// Default value when `required` is `false` and the argument is absent.
    #[serde(default)]
    pub default: Option<Value>,

    /// Valid values when `arg_type` is `"enum"`.
    #[serde(default)]
    pub enum_values: Vec<String>,

    /// This argument is optional if any of the listed flag IDs is present.
    #[serde(default)]
    pub required_unless_flag: Vec<String>,
}

fn default_variadic_min() -> usize {
    1
}

/// A subcommand definition (§2.4). The structure is recursive.
#[derive(Debug, Clone, Deserialize)]
pub struct CommandDef {
    /// Unique ID among siblings.
    pub id: String,

    /// The token the user types (e.g. `"add"`, `"commit"`).
    pub name: String,

    /// Alternative tokens for this command.
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Human-readable description.
    pub description: String,

    /// Whether `global_flags` from the root apply in this context. Default: `true`.
    #[serde(default = "bool_true")]
    pub inherit_global_flags: bool,

    /// Flags specific to this subcommand context.
    #[serde(default)]
    pub flags: Vec<FlagDef>,

    /// Positional arguments for this subcommand.
    #[serde(default)]
    pub arguments: Vec<ArgumentDef>,

    /// Nested subcommands (recursive).
    #[serde(default)]
    pub commands: Vec<CommandDef>,

    /// Mutually exclusive flag groups for this subcommand.
    #[serde(default)]
    pub mutually_exclusive_groups: Vec<ExclusiveGroup>,
}

/// A mutually exclusive flag group (§2.5).
#[derive(Debug, Clone, Deserialize)]
pub struct ExclusiveGroup {
    /// Unique identifier.
    pub id: String,

    /// IDs of flags in this group.
    pub flag_ids: Vec<String>,

    /// If `true`, exactly one of the flags must be present. Default: `false`.
    #[serde(default)]
    pub required: bool,
}

// ===========================================================================
// Parser output types (§7)
// ===========================================================================

/// The result of a successful argv parse (§7).
///
/// All flags in scope appear in `flags` — absent optional flags use `false`
/// for booleans, `null` for others (or `default` if set). Variadic arguments
/// produce JSON arrays.
///
/// # Example
///
/// ```
/// # use cli_builder::types::ParseResult;
/// # use std::collections::HashMap;
/// # use serde_json::json;
/// let result = ParseResult {
///     program: "git".into(),
///     command_path: vec!["git".into(), "commit".into()],
///     flags: HashMap::from([("message".into(), json!("initial commit"))]),
///     arguments: HashMap::new(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Always `argv[0]`.
    pub program: String,

    /// Full path of commands from root to resolved leaf.
    ///
    /// For root-level invocation: `["program-name"]`.
    /// For `git remote add`: `["git", "remote", "add"]`.
    pub command_path: Vec<String>,

    /// Map from flag `id` to parsed value.
    ///
    /// All flags in scope are present. Absent boolean flags → `false`.
    /// Absent non-boolean optional flags → `null` (or `default` if set).
    /// Repeatable flags → JSON array.
    pub flags: HashMap<String, Value>,

    /// Map from argument `id` to parsed value.
    ///
    /// Variadic arguments → JSON array. Absent optional arguments → `null`
    /// (or `default` if set).
    pub arguments: HashMap<String, Value>,
}

/// The result of a `--help` or `-h` invocation (§7).
///
/// The caller should print `text` and exit with code 0.
#[derive(Debug, Clone)]
pub struct HelpResult {
    /// The rendered help text for the deepest resolved command.
    pub text: String,

    /// The command path at the point where `--help` was encountered.
    pub command_path: Vec<String>,
}

/// The result of a `--version` invocation (§7).
///
/// The caller should print `version` and exit with code 0.
#[derive(Debug, Clone)]
pub struct VersionResult {
    /// The version string from the spec.
    pub version: String,
}

/// The three possible outcomes of a successful `parse()` call.
///
/// The caller pattern-matches on this to decide what to do:
///
/// ```text
/// match parser.parse(&args)? {
///     ParserOutput::Parse(r)   => { /* use r.flags, r.arguments */ }
///     ParserOutput::Help(h)    => { print!("{}", h.text); std::process::exit(0); }
///     ParserOutput::Version(v) => { println!("{}", v.version); std::process::exit(0); }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum ParserOutput {
    /// Normal parse succeeded.
    Parse(ParseResult),
    /// `--help` or `-h` was encountered.
    Help(HelpResult),
    /// `--version` was encountered.
    Version(VersionResult),
}
