// parser.rs -- The CLI Builder parser
// =====================================
//
// This module ties together all other components into the three-phase
// parsing pipeline described in §6 of the spec:
//
//   Phase 1 — Routing (directed graph): walk argv to find the deepest
//             matching command node. Uses `directed_graph::Graph` for
//             routing via `successors()`.
//
//   Phase 2 — Scanning (modal state machine): walk argv again (skipping
//             command tokens) and classify each token using the token
//             classification DFA. Drives the modal state machine through
//             modes: SCANNING → FLAG_VALUE → END_OF_FLAGS.
//
//   Phase 3 — Validation: positional resolution + flag constraint checks.
//
// # The Modal State Machine
//
// The parser uses a lightweight inline modal machine rather than the full
// `state_machine::ModalStateMachine`. This is because our modes don't
// need full DFA sub-machines — they're just four named states:
//
//   SCANNING    — normal mode: classify tokens and dispatch
//   FLAG_VALUE  — a non-boolean flag was seen; next token is its value
//   END_OF_FLAGS — "--" was seen; all remaining tokens are positional
//
// The `state_machine` crate's `ModalStateMachine` is used to track the
// mode transitions and provide the mode trace history.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use state_machine::ModalStateMachine;
use state_machine::DFA;

use crate::errors::{CliBuilderError, ParseError, ParseErrors};
use crate::flag_validator::validate_flags;
use crate::help_generator::{generate_command_help, generate_root_help};
use crate::positional_resolver::{coerce_value, resolve_positionals};
use crate::token_classifier::{FlagInfo, TokenClassifier, TokenEvent};
use crate::types::{
    ArgumentDef, CliSpec, CommandDef, ExclusiveGroup, FlagDef,
    HelpResult, ParseResult, ParserOutput, VersionResult,
};

// ---------------------------------------------------------------------------
// Parse modes (modal state machine states)
// ---------------------------------------------------------------------------

/// The four parse modes for the modal state machine.
///
/// We use string literals as mode names so they integrate directly with
/// `state_machine::ModalStateMachine`.
const MODE_SCANNING: &str = "SCANNING";
const MODE_FLAG_VALUE: &str = "FLAG_VALUE";
const MODE_END_OF_FLAGS: &str = "END_OF_FLAGS";

// ---------------------------------------------------------------------------
// The Parser struct
// ---------------------------------------------------------------------------

/// The main CLI Builder parser.
///
/// Constructed once from a loaded `CliSpec`, then used to parse any number
/// of argv slices (e.g., in tests, or in a REPL).
///
/// # Example
///
/// ```
/// # use cli_builder::parser::Parser;
/// # use cli_builder::spec_loader::load_spec_from_str;
/// # use cli_builder::types::ParserOutput;
/// let spec = load_spec_from_str(r#"{
///     "cli_builder_spec_version": "1.0",
///     "name": "echo",
///     "description": "Print text",
///     "flags": [{"id":"newline","short":"n","description":"No newline","type":"boolean"}],
///     "arguments": [{"id":"msg","name":"MSG","description":"text","type":"string","required":false,"variadic":true,"variadic_min":0}]
/// }"#).unwrap();
///
/// let parser = Parser::new(spec);
/// let args: Vec<String> = vec!["echo".into(), "-n".into(), "hello".into()];
/// let output = parser.parse(&args).unwrap();
/// match output {
///     ParserOutput::Parse(r) => {
///         assert_eq!(r.flags["newline"], serde_json::json!(true));
///     }
///     _ => panic!("expected Parse"),
/// }
/// ```
pub struct Parser {
    spec: CliSpec,
}

impl Parser {
    /// Create a new parser from a loaded `CliSpec`.
    pub fn new(spec: CliSpec) -> Self {
        Parser { spec }
    }

    /// Parse an argv slice and return a `ParserOutput`.
    ///
    /// `args[0]` should be the program name (e.g. `"git"`).
    ///
    /// # Errors
    ///
    /// Returns `CliBuilderError::ParseErrors` if any parsing errors are found.
    /// All errors are collected in a single pass for maximum usability.
    pub fn parse(&self, args: &[String]) -> Result<ParserOutput, CliBuilderError> {
        if args.is_empty() {
            return Err(CliBuilderError::SpecError(
                "argv must have at least one element (the program name)".to_string(),
            ));
        }

        let program = args[0].clone();
        let argv = &args[1..]; // strip argv[0]

        // -----------------------------------------------------------------------
        // Phase 1: Routing (directed graph)
        // -----------------------------------------------------------------------
        let RoutingResult {
            command_path,
            resolved_node,
            remaining_argv,
        } = self.route(argv, &program);

        // Resolve the active flags and arguments for the command node.
        let (active_flags, active_arguments, active_groups) =
            self.resolve_active_scope(&resolved_node, &command_path);

        // Build the token classifier from the active flag set.
        let flag_infos: Vec<FlagInfo> = active_flags
            .iter()
            .map(|f| FlagInfo::from_flag_def(f))
            .collect();
        let classifier = TokenClassifier::new(flag_infos);

        // -----------------------------------------------------------------------
        // Phase 2: Scanning (modal state machine)
        // -----------------------------------------------------------------------
        let ScanResult {
            parsed_flags,
            positional_tokens,
            errors: scan_errors,
            help_requested,
            version_requested,
        } = self.scan(&remaining_argv, &classifier, &active_flags, &command_path)?;

        // Handle --help / -h early-return (§6.3)
        if help_requested {
            let text = if command_path.len() <= 1 {
                generate_root_help(&self.spec)
            } else {
                generate_command_help(&self.spec, &command_path)
            };
            return Ok(ParserOutput::Help(HelpResult { text, command_path }));
        }

        // Handle --version early-return
        if version_requested {
            let version = self.spec.version.clone().unwrap_or_default();
            return Ok(ParserOutput::Version(VersionResult { version }));
        }

        // -----------------------------------------------------------------------
        // Phase 3: Validation
        // -----------------------------------------------------------------------
        let mut all_errors = scan_errors;

        // 3a. Positional resolution
        let pos_result = resolve_positionals(
            &positional_tokens,
            &active_arguments,
            &parsed_flags,
            &command_path,
        );
        all_errors.extend(pos_result.errors);

        // 3b. Flag constraint validation (conflicts, requires, required, groups)
        let flag_errors = validate_flags(&parsed_flags, &active_flags, &active_groups, &command_path);
        all_errors.extend(flag_errors);

        if !all_errors.is_empty() {
            return Err(CliBuilderError::ParseErrors(ParseErrors { errors: all_errors }));
        }

        // -----------------------------------------------------------------------
        // Build the final ParseResult
        // -----------------------------------------------------------------------

        // Populate default flag values for all flags in scope that weren't set.
        let mut final_flags = parsed_flags;
        for f in &active_flags {
            if final_flags.contains_key(&f.id) {
                continue;
            }
            let default = if f.flag_type == "boolean" {
                json!(false)
            } else if let Some(ref d) = f.default {
                d.clone()
            } else {
                Value::Null
            };
            if f.repeatable && !final_flags.contains_key(&f.id) {
                final_flags.insert(f.id.clone(), json!([]));
            } else {
                final_flags.insert(f.id.clone(), default);
            }
        }

        Ok(ParserOutput::Parse(ParseResult {
            program,
            command_path,
            flags: final_flags,
            arguments: pos_result.assignments,
        }))
    }

    // -----------------------------------------------------------------------
    // Phase 1: Routing
    // -----------------------------------------------------------------------

    /// Walk argv to find the deepest matching command node.
    ///
    /// Flags are skipped during routing (they belong to Phase 2). The first
    /// non-flag token that doesn't match any known subcommand name terminates
    /// routing.
    ///
    /// # Special mode: `subcommand_first`
    ///
    /// When `parsing_mode` is `"subcommand_first"`, the first non-flag token is
    /// always treated as a subcommand name (never as a positional). If it
    /// doesn't match, it's an `unknown_command` error.
    fn route(&self, argv: &[String], program: &str) -> RoutingResult {
        let mut command_path: Vec<String> = vec![program.to_string()];
        let mut current_commands: &[CommandDef] = &self.spec.commands;
        let mut i = 0;

        // In "traditional" mode we handle argv[0] (the first real user token)
        // specially — but routing is the same; only token classification changes
        // in Phase 2.

        // For routing purposes, we only care about whether a token looks like
        // a subcommand. We skip flag tokens.
        while i < argv.len() {
            let token = &argv[i];

            // "--" ends routing immediately.
            if token == "--" {
                break;
            }

            // Skip flag tokens. We peek ahead to also skip flag values.
            if token.starts_with('-') && token.len() > 1 {
                // If this flag takes a value (non-boolean), skip the next token too.
                // We approximate this by checking the known flags at the current scope.
                // This is "best effort" during routing — Phase 2 does the real parsing.
                i += self.skip_flag_during_routing(token, current_commands);
                continue;
            }

            // Non-flag token: check if it matches a subcommand.
            // Aliases count as valid tokens but we record the canonical name.
            let canonical = current_commands
                .iter()
                .find(|c| c.name == *token || c.aliases.iter().any(|a| a == token))
                .map(|c| (c.name.clone(), &c.commands as &[CommandDef]));

            if let Some((canonical_name, sub_cmds)) = canonical {
                command_path.push(canonical_name);
                current_commands = sub_cmds;
                i += 1;
            } else {
                // First non-subcommand positional terminates routing.
                break;
            }
        }

        RoutingResult {
            command_path: command_path.clone(),
            resolved_node: ResolvedNode::from_spec_path(&self.spec, &command_path[1..]),
            remaining_argv: argv.to_vec(),
        }
    }

    /// Estimate how many tokens to skip for a flag during routing.
    ///
    /// This is intentionally conservative: if we can't determine whether a
    /// flag is boolean, we skip 1 token (the flag itself). The Phase 2
    /// scanner re-reads all tokens anyway.
    fn skip_flag_during_routing(&self, token: &str, _commands: &[CommandDef]) -> usize {
        // "--" is handled before calling this.
        // We skip flags during routing but don't need to be precise.
        // Phase 2 re-parses everything.

        // `--name=value` style: skip 1 (value inline)
        if token.contains('=') && token.starts_with("--") {
            return 1;
        }

        // `-x` short flag: conservative skip of 1
        // `--long` flag: conservative skip of 1
        // Since we don't know if the flag is boolean here, we advance by 1.
        // Phase 2 will parse correctly.
        1
    }

    // -----------------------------------------------------------------------
    // Phase 2: Scanning
    // -----------------------------------------------------------------------

    /// Scan all argv tokens (post-routing) to extract flags and positionals.
    ///
    /// Uses the modal state machine to track the current parse mode.
    fn scan(
        &self,
        argv: &[String],
        classifier: &TokenClassifier,
        active_flags: &[FlagDef],
        command_path: &[String],
    ) -> Result<ScanResult, CliBuilderError> {
        // Determine the command path as a set so we can skip command tokens.
        // When re-walking argv, command name tokens (consumed in Phase 1) must
        // be skipped. We track which indices are command tokens by simulating
        // Phase 1 again.
        let command_token_indices = self.find_command_token_indices(argv, command_path);

        // Build the modal state machine.
        // We use a lightweight enum rather than constructing full DFA sub-machines,
        // since our modes don't need to accept/reject inputs — they just track
        // which processing rule is active.
        let mode_machine = build_parse_mode_machine();

        let mut mode_machine = match mode_machine {
            Ok(m) => m,
            Err(e) => return Err(CliBuilderError::SpecError(e)),
        };

        let mut parsed_flags: HashMap<String, Value> = HashMap::new();
        let mut positional_tokens: Vec<String> = Vec::new();
        let mut errors: Vec<ParseError> = Vec::new();
        let mut pending_flag: Option<&FlagDef> = None;
        let mut help_requested = false;
        let mut version_requested = false;

        // Determine if this is traditional mode and if we're on the first token.
        let is_traditional = self.spec.parsing_mode == "traditional";
        let known_subcommand_names: Vec<String> =
            self.spec.commands.iter().map(|c| c.name.clone()).collect();

        for (i, token) in argv.iter().enumerate() {
            // Skip command tokens (they were consumed by Phase 1).
            if command_token_indices.contains(&i) {
                continue;
            }

            let events = if is_traditional && i == 0 && !token.starts_with('-') {
                // Traditional mode: first non-flag token may be a stack without `-`.
                classifier.classify_traditional(token, &known_subcommand_names)
            } else {
                classifier.classify(token)
            };

            for event in events {
                let mode = mode_machine.current_mode().to_string();

                match mode.as_str() {
                    MODE_FLAG_VALUE => {
                        // The entire token is the value for the pending flag.
                        if let Some(flag) = pending_flag {
                            let val = coerce_value(token, &flag.flag_type, &flag.enum_values);
                            match val {
                                Ok(v) => {
                                    store_flag_value(&mut parsed_flags, flag, v, &mut errors, command_path);
                                }
                                Err(mut e) => {
                                    e.context = command_path.to_vec();
                                    errors.push(e);
                                }
                            }
                            pending_flag = None;
                        }
                        let _ = mode_machine.switch_mode("value_consumed");
                        // After consuming the value, re-process the current event
                        // in SCANNING mode only if the event wasn't the value token
                        // itself. Since FLAG_VALUE mode consumes the whole token,
                        // we don't reprocess the event.
                        break; // only one token is consumed for the value
                    }

                    MODE_END_OF_FLAGS => {
                        // Everything is positional.
                        positional_tokens.push(token.clone());
                        break;
                    }

                    MODE_SCANNING | _ => {
                        match event {
                            TokenEvent::EndOfFlags => {
                                let _ = mode_machine.switch_mode("end_of_flags");
                            }

                            TokenEvent::LongFlag(name) => {
                                // Check for builtin --help / --version first.
                                if name == "help" {
                                    help_requested = true;
                                    return Ok(ScanResult { parsed_flags, positional_tokens, errors, help_requested, version_requested });
                                }
                                if name == "version" {
                                    version_requested = true;
                                    return Ok(ScanResult { parsed_flags, positional_tokens, errors, help_requested, version_requested });
                                }
                                if let Some(flag) = active_flags.iter().find(|f| f.long.as_deref() == Some(&name)) {
                                    if flag.flag_type == "boolean" {
                                        store_flag_value(&mut parsed_flags, flag, json!(true), &mut errors, command_path);
                                    } else {
                                        pending_flag = Some(flag);
                                        let _ = mode_machine.switch_mode("needs_value");
                                    }
                                } else {
                                    // Unknown flag — fuzzy suggest
                                    let suggestion = fuzzy_suggest_flag(&name, active_flags);
                                    let msg = format!("Unknown flag '--{}'", name);
                                    if let Some(s) = suggestion {
                                        errors.push(ParseError::with_suggestion("unknown_flag", msg, s, command_path.to_vec()));
                                    } else {
                                        errors.push(ParseError::new("unknown_flag", msg, command_path.to_vec()));
                                    }
                                }
                            }

                            TokenEvent::LongFlagWithValue(name, value) => {
                                if let Some(flag) = active_flags.iter().find(|f| f.long.as_deref() == Some(&name)) {
                                    let val = coerce_value(&value, &flag.flag_type, &flag.enum_values);
                                    match val {
                                        Ok(v) => store_flag_value(&mut parsed_flags, flag, v, &mut errors, command_path),
                                        Err(mut e) => { e.context = command_path.to_vec(); errors.push(e); }
                                    }
                                } else {
                                    let msg = format!("Unknown flag '--{}'", name);
                                    errors.push(ParseError::new("unknown_flag", msg, command_path.to_vec()));
                                }
                            }

                            TokenEvent::SingleDashLong(name) => {
                                if let Some(flag) = active_flags.iter().find(|f| f.single_dash_long.as_deref() == Some(&name)) {
                                    if flag.flag_type == "boolean" {
                                        store_flag_value(&mut parsed_flags, flag, json!(true), &mut errors, command_path);
                                    } else {
                                        pending_flag = Some(flag);
                                        let _ = mode_machine.switch_mode("needs_value");
                                    }
                                } else {
                                    let msg = format!("Unknown flag '-{}'", name);
                                    errors.push(ParseError::new("unknown_flag", msg, command_path.to_vec()));
                                }
                            }

                            TokenEvent::ShortFlag(ch) => {
                                if let Some(flag) = active_flags.iter().find(|f| f.short.as_deref() == Some(&ch.to_string())) {
                                    // If this is the builtin help flag (not a user-defined flag
                                    // that happens to use the same short char), trigger help.
                                    if flag.id == "__builtin_help" {
                                        help_requested = true;
                                        return Ok(ScanResult { parsed_flags, positional_tokens, errors, help_requested, version_requested });
                                    }
                                    if flag.flag_type == "boolean" {
                                        store_flag_value(&mut parsed_flags, flag, json!(true), &mut errors, command_path);
                                    } else {
                                        pending_flag = Some(flag);
                                        let _ = mode_machine.switch_mode("needs_value");
                                    }
                                } else {
                                    let msg = format!("Unknown flag '-{}'", ch);
                                    errors.push(ParseError::new("unknown_flag", msg, command_path.to_vec()));
                                }
                            }

                            TokenEvent::ShortFlagWithValue(ch, value) => {
                                if let Some(flag) = active_flags.iter().find(|f| f.short.as_deref() == Some(&ch.to_string())) {
                                    let val = coerce_value(&value, &flag.flag_type, &flag.enum_values);
                                    match val {
                                        Ok(v) => store_flag_value(&mut parsed_flags, flag, v, &mut errors, command_path),
                                        Err(mut e) => { e.context = command_path.to_vec(); errors.push(e); }
                                    }
                                } else {
                                    let msg = format!("Unknown flag '-{}'", ch);
                                    errors.push(ParseError::new("unknown_flag", msg, command_path.to_vec()));
                                }
                            }

                            TokenEvent::StackedFlags(chars) => {
                                // Each char is a short flag. All except possibly the
                                // last are boolean; the last may be non-boolean (its
                                // value comes from the next token).
                                for (j, ch) in chars.iter().enumerate() {
                                    let is_last = j == chars.len() - 1;
                                    if let Some(flag) = active_flags.iter().find(|f| f.short.as_deref() == Some(&ch.to_string())) {
                                        if flag.flag_type == "boolean" {
                                            store_flag_value(&mut parsed_flags, flag, json!(true), &mut errors, command_path);
                                        } else if is_last {
                                            // Non-boolean last flag — value from next token.
                                            pending_flag = Some(flag);
                                            let _ = mode_machine.switch_mode("needs_value");
                                        } else {
                                            // Non-boolean in the middle of a stack — invalid.
                                            errors.push(ParseError::new(
                                                "invalid_stack",
                                                format!("Non-boolean flag '-{}' cannot appear in the middle of a stack", ch),
                                                command_path.to_vec(),
                                            ));
                                        }
                                    } else {
                                        errors.push(ParseError::new(
                                            "unknown_flag",
                                            format!("Unknown flag '-{}' in stack", ch),
                                            command_path.to_vec(),
                                        ));
                                    }
                                }
                            }

                            TokenEvent::Positional(val) => {
                                // In POSIX mode, first positional ends flag scanning.
                                if self.spec.parsing_mode == "posix" {
                                    let _ = mode_machine.switch_mode("end_of_flags");
                                    positional_tokens.push(val);
                                } else {
                                    positional_tokens.push(val);
                                }
                            }

                            TokenEvent::UnknownFlag(raw) => {
                                // Strip leading dashes for fuzzy matching.
                                let stripped = raw.trim_start_matches('-');
                                let suggestion = fuzzy_suggest_flag(stripped, active_flags);
                                let msg = format!("Unknown flag '{}'", raw);
                                if let Some(s) = suggestion {
                                    errors.push(ParseError::with_suggestion("unknown_flag", msg, s, command_path.to_vec()));
                                } else {
                                    errors.push(ParseError::new("unknown_flag", msg, command_path.to_vec()));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ScanResult {
            parsed_flags,
            positional_tokens,
            errors,
            help_requested,
            version_requested,
        })
    }

    // -----------------------------------------------------------------------
    // Scope resolution
    // -----------------------------------------------------------------------

    /// Resolve the active flags, arguments, and exclusive groups for the resolved command path.
    ///
    /// Active flags = global_flags (if applicable) + flags of every command in the path.
    fn resolve_active_scope(
        &self,
        node: &ResolvedNode,
        _command_path: &[String],
    ) -> (Vec<FlagDef>, Vec<ArgumentDef>, Vec<ExclusiveGroup>) {
        let mut flags: Vec<FlagDef> = Vec::new();
        let mut arguments: Vec<ArgumentDef> = Vec::new();
        let mut groups: Vec<ExclusiveGroup> = Vec::new();

        // Start with the resolved node's own flags and arguments.
        let inherit_global = node.inherit_global_flags;

        // Add global flags first (if inheriting).
        if inherit_global {
            flags.extend(self.spec.global_flags.clone());
        }

        // Walk the command path to accumulate flags (only from the leaf node,
        // since intermediate commands don't define flags that carry forward —
        // they are scoped).
        // Per spec §2.4: the "active flags" are global_flags + leaf command flags.
        flags.extend(node.flags.clone());
        arguments.extend(node.arguments.clone());
        groups.extend(node.exclusive_groups.clone());

        // Inject builtin flags after user flags so that user-defined flags with
        // the same short form take precedence (first-write-wins dedup below).
        if self.spec.builtin_flags.help {
            flags.push(FlagDef {
                id: "__builtin_help".to_string(),
                short: Some("h".to_string()),
                long: Some("help".to_string()),
                single_dash_long: None,
                description: "Show this help message and exit.".to_string(),
                flag_type: "boolean".to_string(),
                required: false,
                default: None,
                value_name: None,
                enum_values: Vec::new(),
                conflicts_with: Vec::new(),
                requires: Vec::new(),
                required_unless: Vec::new(),
                repeatable: false,
            });
        }
        if self.spec.builtin_flags.version && self.spec.version.is_some() {
            flags.push(FlagDef {
                id: "__builtin_version".to_string(),
                short: None,
                long: Some("version".to_string()),
                single_dash_long: None,
                description: "Show version and exit.".to_string(),
                flag_type: "boolean".to_string(),
                required: false,
                default: None,
                value_name: None,
                enum_values: Vec::new(),
                conflicts_with: Vec::new(),
                requires: Vec::new(),
                required_unless: Vec::new(),
                repeatable: false,
            });
        }

        // Deduplicate by id (global flags may conflict with leaf flags in ID space —
        // local flags shadow global flags with the same id).
        let mut seen: HashSet<String> = HashSet::new();
        let deduped: Vec<FlagDef> = flags
            .into_iter()
            .filter(|f| seen.insert(f.id.clone()))
            .collect();

        // Also deduplicate by short char (first-write-wins: user flags before builtins).
        let mut seen_short: HashSet<String> = HashSet::new();
        let deduped: Vec<FlagDef> = deduped
            .into_iter()
            .filter(|f| {
                if let Some(ref s) = f.short {
                    seen_short.insert(s.clone())
                } else {
                    true
                }
            })
            .collect();

        (deduped, arguments, groups)
    }

    /// Find which indices in `argv` correspond to command name tokens
    /// consumed during Phase 1 routing.
    ///
    /// We need this so Phase 2 can skip those tokens when re-walking argv.
    fn find_command_token_indices(&self, argv: &[String], command_path: &[String]) -> HashSet<usize> {
        // command_path[0] is the program name; command_path[1..] are subcommands.
        // We need to find the indices in argv that consumed each subcommand name.
        let mut indices: HashSet<usize> = HashSet::new();
        let mut remaining_commands = command_path[1..].iter();
        let mut expected = remaining_commands.next();

        let mut current_commands: &[CommandDef] = &self.spec.commands;

        for (i, token) in argv.iter().enumerate() {
            if token.starts_with('-') {
                continue; // flags are not command tokens
            }
            if token == "--" {
                break;
            }
            if let Some(exp) = expected {
                // Check if this token matches the expected command name or alias.
                if let Some(cmd) = current_commands.iter().find(|c| c.name == *token || c.aliases.iter().any(|a| a == token)) {
                    if cmd.name == *exp || cmd.aliases.iter().any(|a| *a == *exp) {
                        indices.insert(i);
                        current_commands = &cmd.commands;
                        expected = remaining_commands.next();
                    }
                }
            }
        }
        indices
    }
}

// ---------------------------------------------------------------------------
// Helper: store a flag value, handling repeatable flags
// ---------------------------------------------------------------------------

/// Store a flag's parsed value into `parsed_flags`.
///
/// If the flag is `repeatable`, the value is appended to an array.
/// If it's non-repeatable and already present, a `duplicate_flag` error
/// is added.
fn store_flag_value(
    parsed_flags: &mut HashMap<String, Value>,
    flag: &FlagDef,
    value: Value,
    errors: &mut Vec<ParseError>,
    command_path: &[String],
) {
    if flag.repeatable {
        // Append to the existing array, or start a new one.
        let entry = parsed_flags
            .entry(flag.id.clone())
            .or_insert_with(|| json!([]));
        if let Value::Array(ref mut arr) = entry {
            arr.push(value);
        }
    } else {
        // Non-repeatable: check for duplicates.
        if parsed_flags.contains_key(&flag.id) {
            errors.push(ParseError::new(
                "duplicate_flag",
                format!("{} specified more than once", flag_display_name(flag)),
                command_path.to_vec(),
            ));
        } else {
            parsed_flags.insert(flag.id.clone(), value);
        }
    }
}

/// Format a flag name for error messages (prefers --long, falls back to -short).
fn flag_display_name(flag: &FlagDef) -> String {
    if let Some(ref l) = flag.long {
        format!("--{}", l)
    } else if let Some(ref s) = flag.short {
        format!("-{}", s)
    } else if let Some(ref sdl) = flag.single_dash_long {
        format!("-{}", sdl)
    } else {
        flag.id.clone()
    }
}

// ---------------------------------------------------------------------------
// Fuzzy matching for --help suggestions (§8.3)
// ---------------------------------------------------------------------------

/// Compute the Levenshtein edit distance between two strings.
///
/// This is the classic dynamic-programming algorithm. O(m*n) time, O(min(m,n)) space.
/// We use it to suggest corrections for unknown flags and commands.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    // Use a single row and update in place (space-optimized).
    let mut row: Vec<usize> = (0..=n).collect();

    for i in 1..=m {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=n {
            let temp = row[j];
            row[j] = if a_chars[i - 1] == b_chars[j - 1] {
                prev
            } else {
                1 + prev.min(row[j]).min(row[j - 1])
            };
            prev = temp;
        }
    }

    row[n]
}

/// Find the closest matching flag name, if within edit distance ≤ 2.
///
/// Returns a formatted suggestion string like `"--verbose"` or `"-v"`.
fn fuzzy_suggest_flag(unknown: &str, active_flags: &[FlagDef]) -> Option<String> {
    let mut best_dist = usize::MAX;
    let mut best_suggestion: Option<String> = None;

    for flag in active_flags {
        // Check against long name
        if let Some(ref long) = flag.long {
            let dist = levenshtein(unknown, long);
            if dist < best_dist {
                best_dist = dist;
                best_suggestion = Some(format!("--{}", long));
            }
        }
        // Check against single_dash_long
        if let Some(ref sdl) = flag.single_dash_long {
            let dist = levenshtein(unknown, sdl);
            if dist < best_dist {
                best_dist = dist;
                best_suggestion = Some(format!("-{}", sdl));
            }
        }
        // Check against short (single char, usually distance 1)
        if let Some(ref s) = flag.short {
            let dist = levenshtein(unknown, s);
            if dist < best_dist {
                best_dist = dist;
                best_suggestion = Some(format!("-{}", s));
            }
        }
    }

    // Only suggest if close enough (§8.3: edit distance ≤ 2).
    if best_dist <= 2 {
        best_suggestion
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// ResolvedNode — represents the leaf command after routing
// ---------------------------------------------------------------------------

/// Data extracted from the spec for the leaf command node.
///
/// This struct centralizes the information that Phase 2 and Phase 3 need
/// from the spec after Phase 1 has determined which command is active.
struct ResolvedNode {
    flags: Vec<FlagDef>,
    arguments: Vec<ArgumentDef>,
    exclusive_groups: Vec<ExclusiveGroup>,
    inherit_global_flags: bool,
}

impl ResolvedNode {
    /// Navigate the spec's command tree to find the node for `path`.
    ///
    /// `path` is the command path *excluding* the program name itself.
    /// If `path` is empty, we return the root node's data.
    fn from_spec_path(spec: &CliSpec, path: &[String]) -> Self {
        if path.is_empty() {
            return ResolvedNode {
                flags: spec.flags.clone(),
                arguments: spec.arguments.clone(),
                exclusive_groups: spec.mutually_exclusive_groups.clone(),
                inherit_global_flags: true,
            };
        }

        // Navigate the command tree.
        let mut current_commands: &[CommandDef] = &spec.commands;
        let mut result: Option<&CommandDef> = None;

        for name in path {
            let found = current_commands
                .iter()
                .find(|c| c.name == *name || c.aliases.iter().any(|a| a == name));
            match found {
                Some(cmd) => {
                    result = Some(cmd);
                    current_commands = &cmd.commands;
                }
                None => {
                    // Path doesn't resolve — fall back to root.
                    return ResolvedNode {
                        flags: spec.flags.clone(),
                        arguments: spec.arguments.clone(),
                        exclusive_groups: spec.mutually_exclusive_groups.clone(),
                        inherit_global_flags: true,
                    };
                }
            }
        }

        match result {
            Some(cmd) => ResolvedNode {
                flags: cmd.flags.clone(),
                arguments: cmd.arguments.clone(),
                exclusive_groups: cmd.mutually_exclusive_groups.clone(),
                inherit_global_flags: cmd.inherit_global_flags,
            },
            None => ResolvedNode {
                flags: spec.flags.clone(),
                arguments: spec.arguments.clone(),
                exclusive_groups: spec.mutually_exclusive_groups.clone(),
                inherit_global_flags: true,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Routing result
// ---------------------------------------------------------------------------

struct RoutingResult {
    command_path: Vec<String>,
    resolved_node: ResolvedNode,
    remaining_argv: Vec<String>,
}

// ---------------------------------------------------------------------------
// Scan result
// ---------------------------------------------------------------------------

struct ScanResult {
    parsed_flags: HashMap<String, Value>,
    positional_tokens: Vec<String>,
    errors: Vec<ParseError>,
    help_requested: bool,
    version_requested: bool,
}

// ---------------------------------------------------------------------------
// Modal state machine construction
// ---------------------------------------------------------------------------
//
// We build a ModalStateMachine with three modes:
//   SCANNING, FLAG_VALUE, END_OF_FLAGS
//
// Each mode is a trivial single-state DFA that accepts any single event
// (the actual logic is handled in the scanner's match statement).
//
// Mode transitions:
//   SCANNING    + "needs_value"    → FLAG_VALUE
//   SCANNING    + "end_of_flags"   → END_OF_FLAGS
//   FLAG_VALUE  + "value_consumed" → SCANNING
//   END_OF_FLAGS is terminal (no outgoing transitions)

fn build_parse_mode_machine() -> Result<ModalStateMachine, String> {
    use std::collections::{HashMap as HM, HashSet as HS};

    // Helper to build a trivial one-state DFA that stays in its single state
    // on any input.
    fn trivial_dfa(state: &str) -> DFA {
        let states: HS<String> = HS::from([state.to_string()]);
        let alphabet: HS<String> = HS::from([
            "needs_value".to_string(),
            "value_consumed".to_string(),
            "end_of_flags".to_string(),
            "token".to_string(),
        ]);
        let mut transitions: HM<(String, String), String> = HM::new();
        for sym in &alphabet {
            transitions.insert((state.to_string(), sym.clone()), state.to_string());
        }
        let accepting: HS<String> = HS::from([state.to_string()]);
        DFA::new(states, alphabet, transitions, state.to_string(), accepting).unwrap()
    }

    let modes: HM<String, DFA> = HM::from([
        (MODE_SCANNING.to_string(),     trivial_dfa(MODE_SCANNING)),
        (MODE_FLAG_VALUE.to_string(),   trivial_dfa(MODE_FLAG_VALUE)),
        (MODE_END_OF_FLAGS.to_string(), trivial_dfa(MODE_END_OF_FLAGS)),
    ]);

    let transitions: HM<(String, String), String> = HM::from([
        ((MODE_SCANNING.to_string(),   "needs_value".to_string()),    MODE_FLAG_VALUE.to_string()),
        ((MODE_SCANNING.to_string(),   "end_of_flags".to_string()),   MODE_END_OF_FLAGS.to_string()),
        ((MODE_SCANNING.to_string(),   "token".to_string()),          MODE_SCANNING.to_string()),
        ((MODE_FLAG_VALUE.to_string(), "value_consumed".to_string()), MODE_SCANNING.to_string()),
        ((MODE_FLAG_VALUE.to_string(), "token".to_string()),          MODE_FLAG_VALUE.to_string()),
        // END_OF_FLAGS has no outgoing transitions intentionally —
        // it stays in the same mode by the trivial DFA's self-loop.
        ((MODE_END_OF_FLAGS.to_string(), "token".to_string()), MODE_END_OF_FLAGS.to_string()),
    ]);

    ModalStateMachine::new(modes, transitions, MODE_SCANNING.to_string())
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_loader::load_spec_from_str;

    fn parse_ok(spec_json: &str, args: &[&str]) -> ParseResult {
        let spec = load_spec_from_str(spec_json).unwrap();
        let parser = Parser::new(spec);
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        match parser.parse(&argv).unwrap() {
            ParserOutput::Parse(r) => r,
            other => panic!("expected Parse, got {:?}", other),
        }
    }

    fn parse_err(spec_json: &str, args: &[&str]) -> Vec<ParseError> {
        let spec = load_spec_from_str(spec_json).unwrap();
        let parser = Parser::new(spec);
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        match parser.parse(&argv).unwrap_err() {
            CliBuilderError::ParseErrors(e) => e.errors,
            other => panic!("expected ParseErrors, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // echo spec
    // -----------------------------------------------------------------------
    const ECHO_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "echo",
        "description": "Display a line of text",
        "version": "8.32",
        "flags": [
            {"id":"no-newline","short":"n","description":"No trailing newline","type":"boolean"},
            {"id":"enable-escapes","short":"e","description":"Enable backslash escapes","type":"boolean","conflicts_with":["disable-escapes"]},
            {"id":"disable-escapes","short":"E","description":"Disable backslash escapes","type":"boolean","conflicts_with":["enable-escapes"]}
        ],
        "arguments": [
            {"id":"string","name":"STRING","description":"Text to print","type":"string","required":false,"variadic":true,"variadic_min":0}
        ]
    }"#;

    #[test]
    fn test_echo_hello_world() {
        let r = parse_ok(ECHO_SPEC, &["echo", "hello", "world"]);
        assert_eq!(r.flags["no-newline"], json!(false));
        assert_eq!(r.arguments["string"], json!(["hello", "world"]));
    }

    #[test]
    fn test_echo_n_flag() {
        let r = parse_ok(ECHO_SPEC, &["echo", "-n", "hello"]);
        assert_eq!(r.flags["no-newline"], json!(true));
        assert_eq!(r.arguments["string"], json!(["hello"]));
    }

    #[test]
    fn test_echo_empty() {
        let r = parse_ok(ECHO_SPEC, &["echo"]);
        assert_eq!(r.flags["no-newline"], json!(false));
        assert_eq!(r.arguments["string"], json!([]));
    }

    #[test]
    fn test_echo_conflicting_flags() {
        let errs = parse_err(ECHO_SPEC, &["echo", "-e", "-E", "hello"]);
        assert!(errs.iter().any(|e| e.error_type == "conflicting_flags"));
    }

    // -----------------------------------------------------------------------
    // ls spec
    // -----------------------------------------------------------------------
    const LS_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "ls",
        "description": "List directory contents",
        "version": "8.32",
        "parsing_mode": "gnu",
        "flags": [
            {"id":"long-listing","short":"l","description":"Long listing format","type":"boolean","conflicts_with":["single-column"]},
            {"id":"all","short":"a","long":"all","description":"Show hidden files","type":"boolean"},
            {"id":"human-readable","short":"h","long":"human-readable","description":"Human sizes","type":"boolean","requires":["long-listing"]},
            {"id":"reverse","short":"r","long":"reverse","description":"Reverse order","type":"boolean"},
            {"id":"sort-time","short":"t","description":"Sort by time","type":"boolean"},
            {"id":"recursive","short":"R","long":"recursive","description":"Recurse","type":"boolean"},
            {"id":"single-column","short":"1","description":"One per line","type":"boolean","conflicts_with":["long-listing"]}
        ],
        "arguments": [
            {"id":"path","name":"PATH","description":"Directory or file","type":"path","required":false,"variadic":true,"variadic_min":0,"default":"."}
        ]
    }"#;

    #[test]
    fn test_ls_no_args() {
        let r = parse_ok(LS_SPEC, &["ls"]);
        assert_eq!(r.flags["long-listing"], json!(false));
        // Default "." should apply
        assert_eq!(r.arguments["path"], json!("."));
    }

    #[test]
    fn test_ls_stacked_lah() {
        let r = parse_ok(LS_SPEC, &["ls", "-lah", "/tmp"]);
        assert_eq!(r.flags["long-listing"], json!(true));
        assert_eq!(r.flags["all"], json!(true));
        assert_eq!(r.flags["human-readable"], json!(true));
        assert_eq!(r.arguments["path"], json!(["/tmp"]));
    }

    #[test]
    fn test_ls_h_without_l_error() {
        let errs = parse_err(LS_SPEC, &["ls", "-h"]);
        assert!(errs.iter().any(|e| e.error_type == "missing_dependency_flag"));
    }

    #[test]
    fn test_ls_conflict_1_and_l() {
        let errs = parse_err(LS_SPEC, &["ls", "-1", "-l"]);
        assert!(errs.iter().any(|e| e.error_type == "conflicting_flags"));
    }

    #[test]
    fn test_ls_long_flag() {
        let r = parse_ok(LS_SPEC, &["ls", "--all"]);
        assert_eq!(r.flags["all"], json!(true));
    }

    // -----------------------------------------------------------------------
    // cp spec
    // -----------------------------------------------------------------------
    const CP_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "cp",
        "description": "Copy files and directories",
        "version": "8.32",
        "flags": [
            {"id":"recursive","short":"r","long":"recursive","description":"Copy directories recursively","type":"boolean"},
            {"id":"force","short":"f","long":"force","description":"Overwrite without prompting","type":"boolean","conflicts_with":["interactive","no-clobber"]},
            {"id":"interactive","short":"i","long":"interactive","description":"Prompt before overwrite","type":"boolean","conflicts_with":["force","no-clobber"]},
            {"id":"no-clobber","short":"n","long":"no-clobber","description":"Do not overwrite","type":"boolean","conflicts_with":["force","interactive"]},
            {"id":"verbose","short":"v","long":"verbose","description":"Explain what is done","type":"boolean"}
        ],
        "arguments": [
            {"id":"source","name":"SOURCE","description":"Source file(s)","type":"path","required":true,"variadic":true,"variadic_min":1},
            {"id":"dest","name":"DEST","description":"Destination","type":"path","required":true,"variadic":false}
        ]
    }"#;

    #[test]
    fn test_cp_single_source_dest() {
        let r = parse_ok(CP_SPEC, &["cp", "a.txt", "/tmp/"]);
        assert_eq!(r.arguments["source"], json!(["a.txt"]));
        assert_eq!(r.arguments["dest"], json!("/tmp/"));
    }

    #[test]
    fn test_cp_multi_source_dest() {
        let r = parse_ok(CP_SPEC, &["cp", "a.txt", "b.txt", "c.txt", "/dest/"]);
        assert_eq!(r.arguments["source"], json!(["a.txt", "b.txt", "c.txt"]));
        assert_eq!(r.arguments["dest"], json!("/dest/"));
    }

    #[test]
    fn test_cp_missing_dest() {
        let errs = parse_err(CP_SPEC, &["cp", "a.txt"]);
        assert!(errs.iter().any(|e| e.error_type == "missing_required_argument" || e.error_type == "too_few_arguments"));
    }

    #[test]
    fn test_cp_no_args_error() {
        let errs = parse_err(CP_SPEC, &["cp"]);
        assert!(!errs.is_empty());
    }

    // -----------------------------------------------------------------------
    // grep spec
    // -----------------------------------------------------------------------
    const GREP_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "grep",
        "description": "Print lines that match patterns",
        "version": "3.7",
        "flags": [
            {"id":"ignore-case","short":"i","long":"ignore-case","description":"Ignore case","type":"boolean"},
            {"id":"invert-match","short":"v","long":"invert-match","description":"Invert match","type":"boolean"},
            {"id":"regexp","short":"e","long":"regexp","description":"Use PATTERN","type":"string","value_name":"PATTERN","repeatable":true},
            {"id":"extended-regexp","short":"E","long":"extended-regexp","description":"Extended regex","type":"boolean"},
            {"id":"fixed-strings","short":"F","long":"fixed-strings","description":"Fixed strings","type":"boolean"},
            {"id":"perl-regexp","short":"P","long":"perl-regexp","description":"Perl regex","type":"boolean"}
        ],
        "arguments": [
            {"id":"pattern","name":"PATTERN","description":"Search pattern","type":"string","required":true,"required_unless_flag":["regexp"]},
            {"id":"files","name":"FILE","description":"Files to search","type":"path","required":false,"variadic":true,"variadic_min":0}
        ],
        "mutually_exclusive_groups": [
            {"id":"regex-engine","flag_ids":["extended-regexp","fixed-strings","perl-regexp"],"required":false}
        ]
    }"#;

    #[test]
    fn test_grep_basic() {
        let r = parse_ok(GREP_SPEC, &["grep", "-i", "foo", "file.txt"]);
        assert_eq!(r.flags["ignore-case"], json!(true));
        assert_eq!(r.arguments["pattern"], json!("foo"));
        assert_eq!(r.arguments["files"], json!(["file.txt"]));
    }

    #[test]
    fn test_grep_repeatable_e_flag() {
        let r = parse_ok(GREP_SPEC, &["grep", "-e", "foo", "-e", "bar", "file.txt"]);
        assert_eq!(r.flags["regexp"], json!(["foo", "bar"]));
        // pattern is optional since -e is present
        assert!(r.arguments.get("pattern").map(|v| v.is_null()).unwrap_or(true));
    }

    #[test]
    fn test_grep_exclusive_group_violation() {
        let errs = parse_err(GREP_SPEC, &["grep", "-E", "-F", "pattern"]);
        assert!(errs.iter().any(|e| e.error_type == "exclusive_group_violation"));
    }

    #[test]
    fn test_grep_missing_pattern_no_e_flag() {
        // "grep file.txt" assigns "file.txt" as the pattern — it succeeds.
        let r = parse_ok(GREP_SPEC, &["grep", "file.txt"]);
        assert_eq!(r.arguments["pattern"], serde_json::json!("file.txt"));

        // "grep" with no args and no -e flag → missing_required_argument for PATTERN.
        let errs2 = parse_err(GREP_SPEC, &["grep"]);
        assert!(errs2.iter().any(|e| e.error_type == "missing_required_argument"));
    }

    // -----------------------------------------------------------------------
    // tar spec (traditional mode)
    // -----------------------------------------------------------------------
    const TAR_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "tar",
        "description": "An archiving utility",
        "version": "1.34",
        "parsing_mode": "traditional",
        "flags": [
            {"id":"create","short":"c","description":"Create archive","type":"boolean"},
            {"id":"extract","short":"x","description":"Extract archive","type":"boolean"},
            {"id":"list","short":"t","description":"List archive","type":"boolean"},
            {"id":"verbose","short":"v","long":"verbose","description":"Verbose","type":"boolean"},
            {"id":"file","short":"f","long":"file","description":"Archive file","type":"path","value_name":"ARCHIVE"},
            {"id":"gzip","short":"z","long":"gzip","description":"gzip","type":"boolean"},
            {"id":"bzip2","short":"j","long":"bzip2","description":"bzip2","type":"boolean"},
            {"id":"xz","short":"J","long":"xz","description":"xz","type":"boolean"}
        ],
        "arguments": [
            {"id":"member","name":"MEMBER","description":"Archive members","type":"path","required":false,"variadic":true,"variadic_min":0}
        ],
        "mutually_exclusive_groups": [
            {"id":"operation","flag_ids":["create","extract","list"],"required":true},
            {"id":"compression","flag_ids":["gzip","bzip2","xz"],"required":false}
        ]
    }"#;

    #[test]
    fn test_tar_traditional_xvf() {
        let r = parse_ok(TAR_SPEC, &["tar", "xvf", "archive.tar"]);
        assert_eq!(r.flags["extract"], json!(true));
        assert_eq!(r.flags["verbose"], json!(true));
        assert_eq!(r.flags["file"], json!("archive.tar"));
    }

    #[test]
    fn test_tar_gnu_style_czvf() {
        let r = parse_ok(TAR_SPEC, &["tar", "-czvf", "out.tar.gz", "./src"]);
        assert_eq!(r.flags["create"], json!(true));
        assert_eq!(r.flags["gzip"], json!(true));
        assert_eq!(r.flags["verbose"], json!(true));
        assert_eq!(r.flags["file"], json!("out.tar.gz"));
        assert_eq!(r.arguments["member"], json!(["./src"]));
    }

    #[test]
    fn test_tar_missing_operation_error() {
        let errs = parse_err(TAR_SPEC, &["tar", "-vf", "archive.tar"]);
        assert!(errs.iter().any(|e| e.error_type == "missing_exclusive_group"));
    }

    #[test]
    fn test_tar_create_and_extract_conflict() {
        let errs = parse_err(TAR_SPEC, &["tar", "-cxf", "archive.tar"]);
        assert!(errs.iter().any(|e| e.error_type == "exclusive_group_violation"));
    }

    // -----------------------------------------------------------------------
    // git spec (subcommands)
    // -----------------------------------------------------------------------
    const GIT_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "git",
        "description": "The stupid content tracker",
        "version": "2.43.0",
        "parsing_mode": "subcommand_first",
        "global_flags": [
            {"id":"no-pager","long":"no-pager","description":"Do not pipe output into a pager","type":"boolean"},
            {"id":"config-env","short":"c","description":"Pass a configuration parameter","type":"string","value_name":"name=value","repeatable":true}
        ],
        "commands": [
            {
                "id": "cmd-add",
                "name": "add",
                "description": "Add file contents to the index",
                "flags": [
                    {"id":"dry-run","short":"n","long":"dry-run","description":"Dry run","type":"boolean"},
                    {"id":"verbose","short":"v","long":"verbose","description":"Be verbose","type":"boolean"}
                ],
                "arguments": [
                    {"id":"pathspec","name":"PATHSPEC","description":"Files to add","type":"path","required":false,"variadic":true,"variadic_min":0}
                ]
            },
            {
                "id": "cmd-commit",
                "name": "commit",
                "aliases": ["ci"],
                "description": "Record changes to the repository",
                "flags": [
                    {"id":"message","short":"m","long":"message","description":"Commit message","type":"string","value_name":"MSG","required":true},
                    {"id":"all","short":"a","long":"all","description":"Stage all changes","type":"boolean"},
                    {"id":"amend","long":"amend","description":"Amend last commit","type":"boolean"}
                ],
                "arguments": []
            }
        ]
    }"#;

    #[test]
    fn test_git_add() {
        let r = parse_ok(GIT_SPEC, &["git", "add", "-v", "."]);
        assert_eq!(r.command_path, vec!["git", "add"]);
        assert_eq!(r.flags["verbose"], json!(true));
        assert_eq!(r.arguments["pathspec"], json!(["."]));
    }

    #[test]
    fn test_git_commit_with_message() {
        let r = parse_ok(GIT_SPEC, &["git", "commit", "-m", "initial commit"]);
        assert_eq!(r.command_path, vec!["git", "commit"]);
        assert_eq!(r.flags["message"], json!("initial commit"));
    }

    #[test]
    fn test_git_commit_alias() {
        let r = parse_ok(GIT_SPEC, &["git", "ci", "-m", "msg"]);
        // canonical name should be "commit" in the path
        assert_eq!(r.command_path[1], "commit");
    }

    #[test]
    fn test_git_commit_missing_message() {
        let errs = parse_err(GIT_SPEC, &["git", "commit"]);
        assert!(errs.iter().any(|e| e.error_type == "missing_required_flag"));
    }

    #[test]
    fn test_git_global_flag() {
        let r = parse_ok(GIT_SPEC, &["git", "--no-pager", "add", "."]);
        assert_eq!(r.flags["no-pager"], json!(true));
    }

    // -----------------------------------------------------------------------
    // Help and version output
    // -----------------------------------------------------------------------

    #[test]
    fn test_help_flag_returns_help_result() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let parser = Parser::new(spec);
        let args: Vec<String> = vec!["echo".into(), "--help".into()];
        let out = parser.parse(&args).unwrap();
        assert!(matches!(out, ParserOutput::Help(_)));
    }

    #[test]
    fn test_short_help_flag() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let parser = Parser::new(spec);
        let args: Vec<String> = vec!["echo".into(), "-h".into()];
        let out = parser.parse(&args).unwrap();
        assert!(matches!(out, ParserOutput::Help(_)));
    }

    #[test]
    fn test_version_flag() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let parser = Parser::new(spec);
        let args: Vec<String> = vec!["echo".into(), "--version".into()];
        let out = parser.parse(&args).unwrap();
        match out {
            ParserOutput::Version(v) => assert_eq!(v.version, "8.32"),
            _ => panic!("expected Version"),
        }
    }

    // -----------------------------------------------------------------------
    // End-of-flags sentinel
    // -----------------------------------------------------------------------

    #[test]
    fn test_double_dash_makes_flags_positional() {
        let r = parse_ok(ECHO_SPEC, &["echo", "--", "-n", "hello"]);
        // "-n" and "hello" after "--" are both positional
        assert_eq!(r.flags["no-newline"], json!(false));
        assert_eq!(r.arguments["string"], json!(["-n", "hello"]));
    }

    // -----------------------------------------------------------------------
    // Long flag with inline value
    // -----------------------------------------------------------------------

    #[test]
    fn test_long_flag_equals_value() {
        const SPEC: &str = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "sort",
            "description": "Sort files",
            "flags": [
                {"id":"key","short":"k","long":"key","description":"Sort key","type":"string","value_name":"KEYDEF","repeatable":true}
            ],
            "arguments": [
                {"id":"file","name":"FILE","description":"Files","type":"path","required":false,"variadic":true,"variadic_min":0}
            ]
        }"#;
        let r = parse_ok(SPEC, &["sort", "--key=1,1"]);
        assert_eq!(r.flags["key"], json!(["1,1"]));
    }

    // -----------------------------------------------------------------------
    // Levenshtein / fuzzy suggestions
    // -----------------------------------------------------------------------

    #[test]
    fn test_levenshtein_equal() {
        assert_eq!(levenshtein("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_single_sub() {
        assert_eq!(levenshtein("hello", "helo"), 1);
    }

    #[test]
    fn test_levenshtein_empty() {
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn test_unknown_flag_suggestion() {
        let errs = parse_err(LS_SPEC, &["ls", "--mesage"]);
        // --mesage doesn't match any ls flag within distance 2, so just an error
        // (or it might match something). Either way no panic.
        assert!(!errs.is_empty());
    }
}
