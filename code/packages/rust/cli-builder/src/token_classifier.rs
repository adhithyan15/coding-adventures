// token_classifier.rs -- Token Classification DFA
// ==================================================
//
// Before the modal state machine can drive parsing, each raw argv string token
// must be classified into a typed event. This module implements the token
// classification DFA described in §5 of the spec.
//
// # Token classification rules (§5.1)
//
// Tokens are classified by reading characters left-to-right, with a priority
// ordering that implements "longest-match-first" (§5.2):
//
//   1. Exactly `"--"` → END_OF_FLAGS
//   2. Starts with `"--"`:
//      a. `"--name=value"` → LONG_FLAG_WITH_VALUE
//      b. `"--name"` → LONG_FLAG
//   3. Exactly `"-"` → POSITIONAL("-")
//   4. Starts with `-` followed by 2+ chars:
//      a. Rule 1: matches a single_dash_long flag exactly → SINGLE_DASH_LONG
//      b. Rule 2: first char matches a short flag:
//         - boolean: SHORT_FLAG, rest is a new potential stack
//         - non-boolean with remainder: SHORT_FLAG_WITH_VALUE
//         - non-boolean, no remainder: SHORT_FLAG
//      c. Rule 3: stacked boolean short flags, with optional non-boolean last
//   5. Starts with `-x` (single char):
//      a. Matches a short flag: SHORT_FLAG
//      b. Doesn't match: UNKNOWN_FLAG
//   6. Otherwise: POSITIONAL
//
// # Stacking (Rule 3)
//
// `-lah` where l, a, h are all boolean short flags produces STACKED_FLAGS.
// `-lf` where l is boolean and f is a path flag produces [SHORT_FLAG('l'), SHORT_FLAG('f')]
// (value comes from the next token). This is handled by decompose_token returning
// multiple TokenEvent items.

use crate::types::FlagDef;

/// A classified token event emitted by the token classifier.
///
/// Each variant corresponds to a row in §5.1 of the spec.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenEvent {
    /// The `--` sentinel. All remaining tokens are positional.
    EndOfFlags,

    /// A `--name` flag (no inline value). May be boolean or value-taking.
    LongFlag(String),

    /// A `--name=value` flag (value inline in same token).
    LongFlagWithValue(String, String),

    /// A `-name` single-dash-long flag.
    SingleDashLong(String),

    /// A `-x` short flag where x is a single character.
    ShortFlag(char),

    /// A `-xVALUE` short flag with an inline value.
    ShortFlagWithValue(char, String),

    /// A `-xyz` stack where all characters are valid boolean short flags,
    /// and optionally the last one is non-boolean (with its value in the
    /// next token).
    StackedFlags(Vec<char>),

    /// A positional argument value (not a flag).
    Positional(String),

    /// An unrecognized flag token.
    UnknownFlag(String),
}

// ---------------------------------------------------------------------------
// Flat view of flags in scope
// ---------------------------------------------------------------------------

/// A compact descriptor of a single flag's disambiguation data, extracted from
/// `FlagDef` at classifier construction time.
///
/// We pre-process the full `FlagDef` list into this flat form to keep the
/// classification hot-path O(F) without touching `FlagDef` indirection.
#[derive(Debug, Clone)]
pub struct FlagInfo {
    /// The flag's `id` (used in error messages and results).
    pub id: String,
    /// Single character without `-`, if present.
    pub short: Option<char>,
    /// Long name without `--`, if present.
    pub long: Option<String>,
    /// Single-dash-long name, if present.
    pub single_dash_long: Option<String>,
    /// Whether this is a boolean (no value) flag.
    pub is_boolean: bool,
    /// Whether this is a count flag (no value, each occurrence increments).
    ///
    /// Count flags behave like booleans for token classification purposes:
    /// they consume no value token. The difference is in the parser, which
    /// increments a counter instead of setting `true`.
    pub is_count: bool,
    /// Whether this flag has `default_when_present` set (enum with optional value).
    ///
    /// For token classification, flags with `default_when_present` behave
    /// like boolean flags: they don't necessarily consume a value token.
    /// The parser performs disambiguation at a higher level.
    pub has_default_when_present: bool,
}

impl FlagInfo {
    /// Build a `FlagInfo` from a `FlagDef`.
    pub fn from_flag_def(def: &FlagDef) -> Self {
        let has_dwp = def.default_when_present.is_some();
        FlagInfo {
            id: def.id.clone(),
            short: def.short.as_ref().and_then(|s| s.chars().next()),
            long: def.long.clone(),
            single_dash_long: def.single_dash_long.clone(),
            // Count flags and boolean flags consume no value token.
            // Enum flags with default_when_present are also treated as
            // "boolean-like" for token classification — the parser handles
            // the disambiguation of whether to consume the next token.
            is_boolean: def.flag_type == "boolean"
                || def.flag_type == "count"
                || has_dwp,
            is_count: def.flag_type == "count",
            has_default_when_present: has_dwp,
        }
    }
}

// ---------------------------------------------------------------------------
// Token classifier
// ---------------------------------------------------------------------------

/// The token classification DFA (§5).
///
/// Constructed once per parse pass from the active flag set, then used to
/// classify each argv token character-by-character.
///
/// # Example
///
/// ```
/// # use cli_builder::token_classifier::{TokenClassifier, FlagInfo, TokenEvent};
/// let flags = vec![
///     FlagInfo { id: "verbose".into(), short: Some('v'), long: Some("verbose".into()),
///                single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
/// ];
/// let tc = TokenClassifier::new(flags);
/// let events = tc.classify("-v");
/// assert_eq!(events, vec![TokenEvent::ShortFlag('v')]);
/// ```
pub struct TokenClassifier {
    /// All flags in scope.
    flags: Vec<FlagInfo>,
}

impl TokenClassifier {
    /// Create a classifier from the active flag set.
    pub fn new(flags: Vec<FlagInfo>) -> Self {
        TokenClassifier { flags }
    }

    // -----------------------------------------------------------------------
    // Lookup helpers (O(F) linear scans — flag lists are short in practice)
    // -----------------------------------------------------------------------

    /// Look up a flag by its long form (without `--`).
    pub fn find_by_long(&self, name: &str) -> Option<&FlagInfo> {
        self.flags.iter().find(|f| f.long.as_deref() == Some(name))
    }

    /// Look up a flag by its single-dash-long form (without `-`).
    pub fn find_by_sdl(&self, name: &str) -> Option<&FlagInfo> {
        self.flags.iter().find(|f| f.single_dash_long.as_deref() == Some(name))
    }

    /// Look up a flag by its short character.
    pub fn find_by_short(&self, ch: char) -> Option<&FlagInfo> {
        self.flags.iter().find(|f| f.short == Some(ch))
    }

    // -----------------------------------------------------------------------
    // Main classification method
    // -----------------------------------------------------------------------

    /// Classify a single argv token into one or more `TokenEvent` values.
    ///
    /// Most tokens produce exactly one event. Stacked short flags like `-lah`
    /// produce a single `StackedFlags` event rather than three separate events,
    /// keeping the modal machine's scanning loop simple.
    ///
    /// Returns a `Vec` because decomposition of some edge cases (like `-lf`
    /// where l is boolean and f is non-boolean) can conceptually produce
    /// multiple logical events — but we represent this as a single
    /// `StackedFlags` containing mixed types, and let `parser.rs` handle it.
    pub fn classify(&self, token: &str) -> Vec<TokenEvent> {
        // Case 1: exactly "--"
        if token == "--" {
            return vec![TokenEvent::EndOfFlags];
        }

        // Case 2: starts with "--" (long flags)
        if let Some(rest) = token.strip_prefix("--") {
            return self.classify_long(rest);
        }

        // Case 3: exactly "-" (single dash = positional stdin/stdout convention)
        if token == "-" {
            return vec![TokenEvent::Positional("-".to_string())];
        }

        // Case 4+5: starts with "-" followed by at least one character
        if let Some(rest) = token.strip_prefix('-') {
            if !rest.is_empty() {
                return self.classify_short_or_sdl(rest);
            }
        }

        // Case 6: positional
        vec![TokenEvent::Positional(token.to_string())]
    }

    /// Classify a token that starts with `--` (the `--` prefix already stripped).
    fn classify_long(&self, rest: &str) -> Vec<TokenEvent> {
        // `--name=value`
        if let Some(eq_pos) = rest.find('=') {
            let name = rest[..eq_pos].to_string();
            let value = rest[eq_pos + 1..].to_string();
            // Validate name exists (unknown long flags become UnknownFlag)
            if self.find_by_long(&name).is_some() {
                return vec![TokenEvent::LongFlagWithValue(name, value)];
            } else {
                return vec![TokenEvent::UnknownFlag(format!("--{}", name))];
            }
        }

        // `--name`
        // Builtin flags --help and --version are always valid at this point
        // (they are injected by the parser before classification).
        if self.find_by_long(rest).is_some() || rest == "help" || rest == "version" {
            return vec![TokenEvent::LongFlag(rest.to_string())];
        }

        vec![TokenEvent::UnknownFlag(format!("--{}", rest))]
    }

    /// Classify a token that starts with `-` (the `-` stripped, `rest` is what follows).
    ///
    /// Implements the longest-match-first rules from §5.2.
    fn classify_short_or_sdl(&self, rest: &str) -> Vec<TokenEvent> {
        // Rule 1: try single-dash-long (longest-match-first).
        // `-classpath` must not be decomposed as stacked chars.
        if let Some(_f) = self.find_by_sdl(rest) {
            return vec![TokenEvent::SingleDashLong(rest.to_string())];
        }

        // Rule 2: first character matches a known short flag.
        let mut chars = rest.chars();
        if let Some(first) = chars.next() {
            if let Some(flag) = self.find_by_short(first) {
                let remainder: String = chars.collect();
                if flag.is_boolean {
                    if remainder.is_empty() {
                        // `-v` — single boolean short flag
                        return vec![TokenEvent::ShortFlag(first)];
                    } else {
                        // `-vX...` — boolean short flag followed by more characters.
                        // Attempt to parse the remainder as a stack.
                        return self.classify_as_stack(first, &remainder);
                    }
                } else {
                    // Non-boolean short flag
                    if remainder.is_empty() {
                        // `-f` — value comes from the next token
                        return vec![TokenEvent::ShortFlag(first)];
                    } else {
                        // `-fVALUE` — inline value
                        return vec![TokenEvent::ShortFlagWithValue(first, remainder)];
                    }
                }
            }
        }

        // No match at all.
        vec![TokenEvent::UnknownFlag(format!("-{}", rest))]
    }

    /// Try to interpret `first` (already identified as a boolean short flag)
    /// plus `remainder` as a stack of short flags.
    ///
    /// All characters except possibly the last must be boolean short flags.
    /// The last may be a non-boolean flag (its value comes from the next token).
    ///
    /// Returns `STACKED_FLAGS([first, ...rest_chars])` on success, or a mix of
    /// known flags followed by an `UNKNOWN_FLAG` error event.
    fn classify_as_stack(&self, first: char, remainder: &str) -> Vec<TokenEvent> {
        let mut stack: Vec<char> = vec![first];

        let chars: Vec<char> = remainder.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            let is_last = i == chars.len() - 1;
            if let Some(flag) = self.find_by_short(ch) {
                stack.push(ch);
                if !flag.is_boolean && !is_last {
                    // Non-boolean flag in the middle of the stack — that means
                    // the remaining chars are its inline value. This is technically
                    // SHORT_FLAG_WITH_VALUE, so we terminate the stack here.
                    // Per spec §5.2 Rule 3: all chars except last must be boolean.
                    // This is an invalid stack — emit what we have plus the
                    // non-boolean flag with remaining as its inline value.
                    //
                    // In practice most tools just reject this, but we handle it
                    // gracefully: emit StackedFlags for the booleans, then
                    // ShortFlagWithValue for the non-boolean flag.
                    let bool_stack: Vec<char> = stack[..stack.len()-1].to_vec();
                    let value_chars: String = chars[i+1..].iter().collect();
                    if bool_stack.is_empty() {
                        return vec![TokenEvent::ShortFlagWithValue(ch, value_chars)];
                    } else {
                        return vec![
                            TokenEvent::StackedFlags(bool_stack),
                            TokenEvent::ShortFlagWithValue(ch, value_chars),
                        ];
                    }
                }
            } else {
                // Unknown character in stack → UNKNOWN_FLAG
                return vec![
                    TokenEvent::StackedFlags(stack),
                    TokenEvent::UnknownFlag(format!("-{}", ch)),
                ];
            }
        }

        vec![TokenEvent::StackedFlags(stack)]
    }

    // -----------------------------------------------------------------------
    // Traditional mode support (§5.3)
    // -----------------------------------------------------------------------

    /// Classify the very first user token in `"traditional"` parsing mode.
    ///
    /// If the token doesn't start with `-` and doesn't match any known
    /// subcommand, it is treated as a stack of short flag characters without
    /// a leading dash.
    ///
    /// # Arguments
    ///
    /// * `token` — the raw token string (no `-` prefix expected).
    /// * `known_commands` — the set of valid subcommand names at root level.
    pub fn classify_traditional(&self, token: &str, known_commands: &[String]) -> Vec<TokenEvent> {
        // If it starts with `-`, handle normally.
        if token.starts_with('-') {
            return self.classify(token);
        }

        // If it matches a known subcommand, let routing handle it as positional.
        if known_commands.iter().any(|c| c == token) {
            return vec![TokenEvent::Positional(token.to_string())];
        }

        // Try to interpret as a stack of short flags without a leading dash.
        let chars: Vec<char> = token.chars().collect();
        let mut stack: Vec<char> = Vec::new();

        for (i, &ch) in chars.iter().enumerate() {
            let is_last = i == chars.len() - 1;
            if let Some(flag) = self.find_by_short(ch) {
                stack.push(ch);
                if !flag.is_boolean {
                    if is_last {
                        // Non-boolean last: value from next token.
                        // Return as StackedFlags; the parser will interpret the
                        // last non-boolean entry and expect the next token as value.
                    } else {
                        // Non-boolean in middle: remaining chars are inline value.
                        let bool_stack: Vec<char> = stack[..stack.len()-1].to_vec();
                        let value_chars: String = chars[i+1..].iter().collect();
                        if bool_stack.is_empty() {
                            return vec![TokenEvent::ShortFlagWithValue(ch, value_chars)];
                        } else {
                            return vec![
                                TokenEvent::StackedFlags(bool_stack),
                                TokenEvent::ShortFlagWithValue(ch, value_chars),
                            ];
                        }
                    }
                }
            } else {
                // Unknown char → fall back to positional.
                return vec![TokenEvent::Positional(token.to_string())];
            }
        }

        if stack.is_empty() {
            vec![TokenEvent::Positional(token.to_string())]
        } else {
            vec![TokenEvent::StackedFlags(stack)]
        }
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_flags() -> Vec<FlagInfo> {
        vec![
            FlagInfo { id: "long-listing".into(), short: Some('l'), long: Some("long-listing".into()), single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "all".into(), short: Some('a'), long: Some("all".into()), single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "human-readable".into(), short: Some('h'), long: Some("human-readable".into()), single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "output".into(), short: Some('o'), long: Some("output".into()), single_dash_long: None, is_boolean: false, is_count: false, has_default_when_present: false },
            FlagInfo { id: "classpath".into(), short: None, long: None, single_dash_long: Some("classpath".into()), is_boolean: false, is_count: false, has_default_when_present: false },
            FlagInfo { id: "verbose-sdl".into(), short: None, long: None, single_dash_long: Some("verbose".into()), is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "help".into(), short: Some('h'), long: Some("help".into()), single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
        ]
    }

    fn tc() -> TokenClassifier {
        TokenClassifier::new(make_flags())
    }

    #[test]
    fn test_end_of_flags() {
        assert_eq!(tc().classify("--"), vec![TokenEvent::EndOfFlags]);
    }

    #[test]
    fn test_long_flag() {
        assert_eq!(tc().classify("--all"), vec![TokenEvent::LongFlag("all".into())]);
    }

    #[test]
    fn test_long_flag_with_value() {
        assert_eq!(
            tc().classify("--output=foo.txt"),
            vec![TokenEvent::LongFlagWithValue("output".into(), "foo.txt".into())]
        );
    }

    #[test]
    fn test_unknown_long_flag() {
        let result = tc().classify("--nonexistent");
        assert_eq!(result, vec![TokenEvent::UnknownFlag("--nonexistent".into())]);
    }

    #[test]
    fn test_unknown_long_flag_with_value() {
        let result = tc().classify("--nonexistent=foo");
        assert_eq!(result, vec![TokenEvent::UnknownFlag("--nonexistent".into())]);
    }

    #[test]
    fn test_single_dash_positional() {
        assert_eq!(tc().classify("-"), vec![TokenEvent::Positional("-".into())]);
    }

    #[test]
    fn test_short_boolean_flag() {
        assert_eq!(tc().classify("-l"), vec![TokenEvent::ShortFlag('l')]);
    }

    #[test]
    fn test_short_nonboolean_flag_no_value() {
        assert_eq!(tc().classify("-o"), vec![TokenEvent::ShortFlag('o')]);
    }

    #[test]
    fn test_short_nonboolean_flag_inline_value() {
        assert_eq!(
            tc().classify("-ofoo.txt"),
            vec![TokenEvent::ShortFlagWithValue('o', "foo.txt".into())]
        );
    }

    #[test]
    fn test_stacked_boolean_flags() {
        let result = tc().classify("-lah");
        // h matches human-readable (boolean), but note h also matches help.
        // The first match wins. -lah → StackedFlags(['l','a','h']).
        assert_eq!(result, vec![TokenEvent::StackedFlags(vec!['l', 'a', 'h'])]);
    }

    #[test]
    fn test_single_dash_long_match() {
        // -classpath must match single_dash_long "classpath" before trying char stacks.
        assert_eq!(
            tc().classify("-classpath"),
            vec![TokenEvent::SingleDashLong("classpath".into())]
        );
    }

    #[test]
    fn test_single_dash_long_boolean() {
        assert_eq!(
            tc().classify("-verbose"),
            vec![TokenEvent::SingleDashLong("verbose".into())]
        );
    }

    #[test]
    fn test_positional() {
        assert_eq!(
            tc().classify("hello"),
            vec![TokenEvent::Positional("hello".into())]
        );
    }

    #[test]
    fn test_positional_path() {
        assert_eq!(
            tc().classify("/tmp/foo.txt"),
            vec![TokenEvent::Positional("/tmp/foo.txt".into())]
        );
    }

    #[test]
    fn test_unknown_short_flag() {
        // -Z matches nothing
        let result = tc().classify("-Z");
        assert_eq!(result, vec![TokenEvent::UnknownFlag("-Z".into())]);
    }

    #[test]
    fn test_builtin_help_flag() {
        // --help is always valid even without a help FlagInfo (parser injects it)
        assert_eq!(tc().classify("--help"), vec![TokenEvent::LongFlag("help".into())]);
    }

    #[test]
    fn test_traditional_mode_stack() {
        let flags = vec![
            FlagInfo { id: "extract".into(), short: Some('x'), long: None, single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "verbose".into(), short: Some('v'), long: None, single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "file".into(), short: Some('f'), long: None, single_dash_long: None, is_boolean: false, is_count: false, has_default_when_present: false },
        ];
        let tc = TokenClassifier::new(flags);
        // "xvf" (no leading dash, not a subcommand) → STACKED_FLAGS(['x','v','f'])
        // 'f' is non-boolean and last → value comes from next token.
        let result = tc.classify_traditional("xvf", &[]);
        assert_eq!(result, vec![TokenEvent::StackedFlags(vec!['x', 'v', 'f'])]);
    }

    #[test]
    fn test_traditional_mode_known_command() {
        let tc = TokenClassifier::new(vec![]);
        // A known subcommand token → Positional (routing handles it)
        let result = tc.classify_traditional("add", &["add".to_string(), "remove".to_string()]);
        assert_eq!(result, vec![TokenEvent::Positional("add".into())]);
    }

    #[test]
    fn test_traditional_mode_unknown_chars_fallback() {
        // If characters don't match any short flag, fall back to Positional.
        let tc = TokenClassifier::new(vec![]);
        let result = tc.classify_traditional("foo", &[]);
        assert_eq!(result, vec![TokenEvent::Positional("foo".into())]);
    }

    // -----------------------------------------------------------------------
    // FlagInfo::from_flag_def coverage
    // -----------------------------------------------------------------------

    #[test]
    fn test_flag_info_from_flag_def_boolean() {
        use crate::types::FlagDef;
        let def = FlagDef {
            id: "verbose".into(),
            short: Some("v".into()),
            long: Some("verbose".into()),
            single_dash_long: None,
            description: "Be verbose".into(),
            flag_type: "boolean".into(),
            required: false,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
            default_when_present: None,
        };
        let info = FlagInfo::from_flag_def(&def);
        assert_eq!(info.id, "verbose");
        assert_eq!(info.short, Some('v'));
        assert_eq!(info.long, Some("verbose".to_string()));
        assert!(info.is_boolean);
    }

    #[test]
    fn test_flag_info_from_flag_def_non_boolean() {
        use crate::types::FlagDef;
        let def = FlagDef {
            id: "output".into(),
            short: Some("o".into()),
            long: Some("output".into()),
            single_dash_long: None,
            description: "Output file".into(),
            flag_type: "string".into(),
            required: false,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
            default_when_present: None,
        };
        let info = FlagInfo::from_flag_def(&def);
        assert!(!info.is_boolean);
    }

    #[test]
    fn test_flag_info_from_flag_def_sdl() {
        use crate::types::FlagDef;
        let def = FlagDef {
            id: "classpath".into(),
            short: None,
            long: None,
            single_dash_long: Some("classpath".into()),
            description: "classpath".into(),
            flag_type: "string".into(),
            required: false,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
            default_when_present: None,
        };
        let info = FlagInfo::from_flag_def(&def);
        assert_eq!(info.single_dash_long, Some("classpath".to_string()));
        assert!(info.short.is_none());
    }

    #[test]
    fn test_flag_info_from_flag_def_empty_short_string() {
        // An empty "short" string should produce short = None (no first char).
        use crate::types::FlagDef;
        let def = FlagDef {
            id: "test".into(),
            short: Some("".into()), // empty string
            long: Some("test".into()),
            single_dash_long: None,
            description: "test".into(),
            flag_type: "boolean".into(),
            required: false,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
            default_when_present: None,
        };
        let info = FlagInfo::from_flag_def(&def);
        assert!(info.short.is_none());
    }

    // -----------------------------------------------------------------------
    // classify_as_stack: non-boolean flag in the MIDDLE of a stack
    // -----------------------------------------------------------------------

    #[test]
    fn test_stack_nonboolean_in_middle_emits_stacked_then_with_value() {
        // Flags: l=boolean, o=non-boolean, a=boolean
        // "-lox" → l is boolean, o is non-boolean in middle → StackedFlags(['l']) + ShortFlagWithValue('o', "x")
        let flags = vec![
            FlagInfo { id: "long".into(), short: Some('l'), long: None, single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "output".into(), short: Some('o'), long: None, single_dash_long: None, is_boolean: false, is_count: false, has_default_when_present: false },
            FlagInfo { id: "all".into(), short: Some('a'), long: None, single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
        ];
        let tc = TokenClassifier::new(flags);
        let result = tc.classify("-lox");
        // l is boolean, then o is non-boolean with remainder "x" → split
        assert_eq!(result.len(), 2);
        assert!(matches!(&result[0], TokenEvent::StackedFlags(v) if v == &['l']));
        assert!(matches!(&result[1], TokenEvent::ShortFlagWithValue('o', ref v) if v == "x"));
    }

    #[test]
    fn test_stack_nonboolean_first_in_stack_no_bool_stack() {
        // "-ox" where o is non-boolean and first: results in ShortFlagWithValue directly
        // (bool_stack is empty, so just ShortFlagWithValue)
        let flags = vec![
            FlagInfo { id: "output".into(), short: Some('o'), long: None, single_dash_long: None, is_boolean: false, is_count: false, has_default_when_present: false },
            FlagInfo { id: "all".into(), short: Some('a'), long: None, single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
        ];
        let tc = TokenClassifier::new(flags);
        // -oax: o is non-boolean, remainder "ax" — ShortFlagWithValue('o', "ax")
        let result = tc.classify("-oax");
        assert_eq!(result, vec![TokenEvent::ShortFlagWithValue('o', "ax".into())]);
    }

    // -----------------------------------------------------------------------
    // classify_traditional: non-boolean in middle (without leading dash)
    // -----------------------------------------------------------------------

    #[test]
    fn test_traditional_nonboolean_in_middle_produces_stacked_then_with_value() {
        let flags = vec![
            FlagInfo { id: "extract".into(), short: Some('x'), long: None, single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
            FlagInfo { id: "file".into(), short: Some('f'), long: None, single_dash_long: None, is_boolean: false, is_count: false, has_default_when_present: false },
            FlagInfo { id: "verbose".into(), short: Some('v'), long: None, single_dash_long: None, is_boolean: true, is_count: false, has_default_when_present: false },
        ];
        let tc = TokenClassifier::new(flags);
        // "xfv": x=boolean, f=non-boolean in middle, remaining "v" is inline value
        // Result: StackedFlags(['x']) + ShortFlagWithValue('f', "v")
        let result = tc.classify_traditional("xfv", &[]);
        assert_eq!(result.len(), 2);
        assert!(matches!(&result[0], TokenEvent::StackedFlags(v) if v == &['x']));
        assert!(matches!(&result[1], TokenEvent::ShortFlagWithValue('f', ref val) if val == "v"));
    }

    #[test]
    fn test_traditional_nonboolean_first_no_bool_stack() {
        // "farchive.tar" where f is non-boolean and first → ShortFlagWithValue directly
        let flags = vec![
            FlagInfo { id: "file".into(), short: Some('f'), long: None, single_dash_long: None, is_boolean: false, is_count: false, has_default_when_present: false },
        ];
        let tc = TokenClassifier::new(flags);
        let result = tc.classify_traditional("farchive.tar", &[]);
        assert_eq!(result, vec![TokenEvent::ShortFlagWithValue('f', "archive.tar".into())]);
    }

    // -----------------------------------------------------------------------
    // classify_traditional: empty token
    // -----------------------------------------------------------------------

    #[test]
    fn test_traditional_empty_token_is_positional() {
        let tc = TokenClassifier::new(vec![]);
        let result = tc.classify_traditional("", &[]);
        assert_eq!(result, vec![TokenEvent::Positional("".into())]);
    }

    // -----------------------------------------------------------------------
    // find_by_long / find_by_sdl / find_by_short lookup helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_by_long_miss() {
        assert!(tc().find_by_long("nonexistent").is_none());
    }

    #[test]
    fn test_find_by_long_hit() {
        let binding = tc();
        let result = binding.find_by_long("all");
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "all");
    }

    #[test]
    fn test_find_by_sdl_miss() {
        assert!(tc().find_by_sdl("nonexistent").is_none());
    }

    #[test]
    fn test_find_by_sdl_hit() {
        let binding = tc();
        let result = binding.find_by_sdl("classpath");
        assert!(result.is_some());
    }

    #[test]
    fn test_find_by_short_miss() {
        assert!(tc().find_by_short('Z').is_none());
    }

    #[test]
    fn test_find_by_short_hit() {
        let binding = tc();
        let result = binding.find_by_short('l');
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "long-listing");
    }
}
