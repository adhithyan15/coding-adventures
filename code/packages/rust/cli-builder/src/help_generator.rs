// help_generator.rs -- Generate human-readable help text from a spec
// ====================================================================
//
// This module produces the help text described in §9 of the spec. The
// format is:
//
//   USAGE
//     <name> [OPTIONS] [COMMAND] [ARGS...]
//
//   DESCRIPTION
//     <description>
//
//   COMMANDS
//     subcommand    Description of the subcommand.
//
//   OPTIONS
//     -s, --long-name <VALUE>    Description [default: val]
//     -b, --boolean              Description.
//
//   GLOBAL OPTIONS
//     -h, --help     Show this help message and exit.
//     --version      Show version and exit.
//
//   ARGUMENTS (shown for leaf commands with positional args)
//     <ARG>      Description. Required.
//     [ARG...]   Description. Optional, repeatable.
//
// # Why generate at runtime?
//
// Help text generated directly from the spec is guaranteed to stay in sync
// with the parsed behaviour. There's no separate documentation to maintain.
// A drift between help and reality is impossible.

use crate::types::{ArgumentDef, CliSpec, CommandDef, FlagDef};

/// Generate help text for the root command.
///
/// # Arguments
///
/// * `spec` — the loaded CLI specification.
///
/// # Returns
///
/// A formatted help string ready to be printed to stdout.
pub fn generate_root_help(spec: &CliSpec) -> String {
    let display_name = spec.display_name.as_deref().unwrap_or(&spec.name);
    let mut out = String::new();

    // Build usage line
    append_usage_root(&mut out, &spec.name, spec);

    // Description
    out.push_str("\nDESCRIPTION\n");
    out.push_str(&format!("  {}\n", spec.description));

    // Commands section (if any)
    if !spec.commands.is_empty() {
        append_commands_section(&mut out, &spec.commands);
    }

    // Options (root flags)
    let all_root_flags: Vec<&FlagDef> = spec.flags.iter().collect();
    if !all_root_flags.is_empty() {
        append_options_section(&mut out, "OPTIONS", &all_root_flags);
    }

    // Arguments (root arguments)
    if !spec.arguments.is_empty() {
        append_arguments_section(&mut out, &spec.arguments);
    }

    // Global options section
    let global_flags: Vec<&FlagDef> = spec.global_flags.iter().collect();
    let builtins = builtin_flag_stubs(spec);
    let all_global: Vec<&FlagDef> = global_flags
        .iter()
        .copied()
        .chain(builtins.iter())
        .collect();
    if !all_global.is_empty() {
        append_options_section(&mut out, "GLOBAL OPTIONS", &all_global);
    }

    let _ = display_name; // used in header building
    out
}

/// Generate help text for a specific subcommand.
///
/// # Arguments
///
/// * `spec` — the root CLI specification (needed for program name and global flags).
/// * `command_path` — the resolved command path, e.g. `["git", "remote", "add"]`.
///   The last entry is the subcommand whose help we're generating.
pub fn generate_command_help(spec: &CliSpec, command_path: &[String]) -> String {
    // Navigate to the command node.
    let cmd = match find_command(spec, command_path) {
        Some(c) => c,
        None => return generate_root_help(spec),
    };

    let mut out = String::new();

    // Usage line for the subcommand
    append_usage_command(&mut out, &spec.name, command_path, cmd);

    // Description
    out.push_str("\nDESCRIPTION\n");
    out.push_str(&format!("  {}\n", cmd.description));

    // Sub-subcommands (if any)
    if !cmd.commands.is_empty() {
        append_commands_section(&mut out, &cmd.commands);
    }

    // Command-specific flags
    if !cmd.flags.is_empty() {
        let refs: Vec<&FlagDef> = cmd.flags.iter().collect();
        append_options_section(&mut out, "OPTIONS", &refs);
    }

    // Arguments section
    if !cmd.arguments.is_empty() {
        append_arguments_section(&mut out, &cmd.arguments);
    }

    // Global options (if this command inherits them)
    if cmd.inherit_global_flags {
        let global_refs: Vec<&FlagDef> = spec.global_flags.iter().collect();
        let builtins = builtin_flag_stubs(spec);
        let all_global: Vec<&FlagDef> = global_refs.iter().copied().chain(builtins.iter()).collect();
        if !all_global.is_empty() {
            append_options_section(&mut out, "GLOBAL OPTIONS", &all_global);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Section builders
// ---------------------------------------------------------------------------

/// Build the USAGE line for the root command.
fn append_usage_root(out: &mut String, program: &str, spec: &CliSpec) {
    out.push_str("USAGE\n");

    let mut parts: Vec<String> = vec![program.to_string()];

    // Show [OPTIONS] if there are any flags
    if !spec.flags.is_empty() || !spec.global_flags.is_empty() || has_builtins(spec) {
        parts.push("[OPTIONS]".to_string());
    }

    // Show [COMMAND] if there are subcommands
    if !spec.commands.is_empty() {
        parts.push("[COMMAND]".to_string());
    }

    // Append positional arguments
    for arg in &spec.arguments {
        parts.push(format_arg_usage(arg));
    }

    out.push_str(&format!("  {}\n", parts.join(" ")));
}

/// Build the USAGE line for a subcommand.
fn append_usage_command(out: &mut String, program: &str, command_path: &[String], cmd: &CommandDef) {
    out.push_str("USAGE\n");

    let mut parts: Vec<String> = vec![program.to_string()];

    // Include the subcommand names (skip first which is program name)
    for name in command_path.iter().skip(1) {
        parts.push(name.clone());
    }

    // OPTIONS
    if !cmd.flags.is_empty() {
        parts.push("[OPTIONS]".to_string());
    }

    // Sub-subcommands
    if !cmd.commands.is_empty() {
        parts.push("[COMMAND]".to_string());
    }

    // Positional arguments
    for arg in &cmd.arguments {
        parts.push(format_arg_usage(arg));
    }

    out.push_str(&format!("  {}\n", parts.join(" ")));
}

/// Append a COMMANDS section to `out`.
fn append_commands_section(out: &mut String, commands: &[CommandDef]) {
    out.push_str("\nCOMMANDS\n");

    // Compute column width from the longest command name.
    let max_name = commands.iter().map(|c| c.name.len()).max().unwrap_or(0);
    let col_width = max_name.max(8) + 2;

    for cmd in commands {
        let padding = col_width - cmd.name.len();
        out.push_str(&format!(
            "  {}{}  {}\n",
            cmd.name,
            " ".repeat(padding),
            cmd.description
        ));
    }
}

/// Append an OPTIONS or GLOBAL OPTIONS section.
fn append_options_section(out: &mut String, section_title: &str, flags: &[&FlagDef]) {
    out.push_str(&format!("\n{}\n", section_title));

    // Compute column width for the flag forms.
    let flag_strs: Vec<String> = flags.iter().map(|f| format_flag_signature(f)).collect();
    let max_len = flag_strs.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = max_len.max(16) + 2;

    for (f, sig) in flags.iter().zip(flag_strs.iter()) {
        let padding = col_width - sig.len();
        let mut desc = f.description.clone();

        // Append default value hint if set.
        if let Some(ref default) = f.default {
            desc.push_str(&format!(" [default: {}]", default));
        }

        out.push_str(&format!("  {}{}  {}\n", sig, " ".repeat(padding), desc));
    }
}

/// Append an ARGUMENTS section.
fn append_arguments_section(out: &mut String, args: &[ArgumentDef]) {
    out.push_str("\nARGUMENTS\n");

    let arg_strs: Vec<String> = args.iter().map(|a| format_arg_signature(a)).collect();
    let max_len = arg_strs.iter().map(|s| s.len()).max().unwrap_or(0);
    let col_width = max_len.max(8) + 2;

    for (a, sig) in args.iter().zip(arg_strs.iter()) {
        let padding = col_width - sig.len();
        let mut desc = a.description.clone();
        if a.required {
            desc.push_str(" Required.");
        } else {
            desc.push_str(" Optional.");
        }
        if a.variadic {
            desc.push_str(" Repeatable.");
        }
        if let Some(ref default) = a.default {
            desc.push_str(&format!(" [default: {}]", default));
        }
        out.push_str(&format!("  {}{}  {}\n", sig, " ".repeat(padding), desc));
    }
}

// ---------------------------------------------------------------------------
// Format helpers
// ---------------------------------------------------------------------------

/// Format a flag's declaration signature for the OPTIONS table.
///
/// Examples:
/// - `-l, --long-listing`
/// - `-h, --human-readable`
/// - `-o, --output <FILE>`
/// - `-classpath <classpath>`
fn format_flag_signature(f: &FlagDef) -> String {
    let value_part = if f.flag_type == "boolean" {
        String::new()
    } else {
        let upper = f.flag_type.to_uppercase();
        let vn = f.value_name.as_deref().unwrap_or(&upper);
        format!(" <{}>", vn)
    };

    let mut forms: Vec<String> = Vec::new();
    if let Some(ref s) = f.short {
        forms.push(format!("-{}", s));
    }
    if let Some(ref l) = f.long {
        forms.push(format!("--{}{}", l, value_part));
    } else if let Some(ref sdl) = f.single_dash_long {
        forms.push(format!("-{}{}", sdl, value_part));
    } else if !value_part.is_empty() {
        // Append value_part to the short form if no long form.
        if let Some(last) = forms.last_mut() {
            last.push_str(&value_part);
        }
    }

    forms.join(", ")
}

/// Format an argument's signature for the USAGE line.
///
/// Required variadic: `<NAME>...`
/// Required scalar:   `<NAME>`
/// Optional variadic: `[NAME...]`
/// Optional scalar:   `[NAME]`
fn format_arg_usage(a: &ArgumentDef) -> String {
    if a.variadic {
        if a.required {
            format!("<{}...>", a.name)
        } else {
            format!("[{}...]", a.name)
        }
    } else if a.required {
        format!("<{}>", a.name)
    } else {
        format!("[{}]", a.name)
    }
}

/// Format an argument's signature for the ARGUMENTS section.
fn format_arg_signature(a: &ArgumentDef) -> String {
    format_arg_usage(a)
}

// ---------------------------------------------------------------------------
// Builtin flag stubs (--help, --version)
// ---------------------------------------------------------------------------

/// Produce synthetic `FlagDef` objects for the builtin --help and --version
/// flags so they appear in the help output without being in the user's spec.
fn builtin_flag_stubs(spec: &CliSpec) -> Vec<FlagDef> {
    let mut builtins: Vec<FlagDef> = Vec::new();

    if spec.builtin_flags.help {
        builtins.push(FlagDef {
            id: "__help__".to_string(),
            short: Some("h".to_string()),
            long: Some("help".to_string()),
            single_dash_long: None,
            description: "Show this help message and exit.".to_string(),
            flag_type: "boolean".to_string(),
            required: false,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
        });
    }

    if spec.builtin_flags.version && spec.version.is_some() {
        builtins.push(FlagDef {
            id: "__version__".to_string(),
            short: None,
            long: Some("version".to_string()),
            single_dash_long: None,
            description: "Show version and exit.".to_string(),
            flag_type: "boolean".to_string(),
            required: false,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
        });
    }

    builtins
}

/// Check if the spec has any builtin flags enabled.
fn has_builtins(spec: &CliSpec) -> bool {
    spec.builtin_flags.help || (spec.builtin_flags.version && spec.version.is_some())
}

// ---------------------------------------------------------------------------
// Command tree navigation
// ---------------------------------------------------------------------------

/// Navigate the command tree to find the `CommandDef` for a given path.
///
/// `command_path[0]` is the program name; subsequent entries are subcommand names.
/// Returns `None` if the path doesn't resolve to a command (e.g. root-level).
fn find_command<'a>(spec: &'a CliSpec, command_path: &[String]) -> Option<&'a CommandDef> {
    if command_path.len() <= 1 {
        return None;
    }
    // The first element is the program name; start searching from the second.
    let mut current: Option<&CommandDef> = None;
    let mut commands: &[CommandDef] = &spec.commands;

    for name in command_path.iter().skip(1) {
        let found = commands.iter().find(|c| {
            c.name == *name || c.aliases.iter().any(|a| a == name)
        });
        match found {
            Some(cmd) => {
                current = Some(cmd);
                commands = &cmd.commands;
            }
            None => return None,
        }
    }

    current
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_loader::load_spec_from_str;

    const ECHO_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "echo",
        "description": "Display a line of text",
        "version": "8.32",
        "flags": [
            {"id":"no-newline","short":"n","description":"Do not output trailing newline","type":"boolean"},
            {"id":"enable-escapes","short":"e","description":"Enable backslash escapes","type":"boolean"}
        ],
        "arguments": [
            {"id":"string","name":"STRING","description":"Text to print","type":"string","required":false,"variadic":true,"variadic_min":0}
        ]
    }"#;

    #[test]
    fn test_root_help_has_usage() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let help = generate_root_help(&spec);
        assert!(help.contains("USAGE"), "expected USAGE section");
        assert!(help.contains("echo"), "expected program name");
    }

    #[test]
    fn test_root_help_has_description() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let help = generate_root_help(&spec);
        assert!(help.contains("DESCRIPTION"));
        assert!(help.contains("Display a line of text"));
    }

    #[test]
    fn test_root_help_has_options() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let help = generate_root_help(&spec);
        assert!(help.contains("OPTIONS"));
        assert!(help.contains("-n"));
        assert!(help.contains("-e"));
    }

    #[test]
    fn test_root_help_has_global_options_with_help() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let help = generate_root_help(&spec);
        assert!(help.contains("GLOBAL OPTIONS") || help.contains("--help"));
    }

    #[test]
    fn test_root_help_has_arguments() {
        let spec = load_spec_from_str(ECHO_SPEC).unwrap();
        let help = generate_root_help(&spec);
        assert!(help.contains("ARGUMENTS") || help.contains("STRING"));
    }

    const GIT_PARTIAL_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "git",
        "description": "The stupid content tracker",
        "version": "2.43.0",
        "global_flags": [
            {"id":"no-pager","long":"no-pager","description":"Do not pipe output into a pager","type":"boolean"}
        ],
        "commands": [
            {
                "id":"cmd-add",
                "name":"add",
                "description":"Add file contents to the index",
                "flags":[
                    {"id":"dry-run","short":"n","long":"dry-run","description":"Dry run","type":"boolean"}
                ],
                "arguments":[
                    {"id":"pathspec","name":"PATHSPEC","description":"Files to add","type":"path","required":true,"variadic":true,"variadic_min":1}
                ]
            }
        ]
    }"#;

    #[test]
    fn test_root_help_shows_commands() {
        let spec = load_spec_from_str(GIT_PARTIAL_SPEC).unwrap();
        let help = generate_root_help(&spec);
        assert!(help.contains("COMMANDS"));
        assert!(help.contains("add"));
        assert!(help.contains("Add file contents"));
    }

    #[test]
    fn test_command_help_usage_line() {
        let spec = load_spec_from_str(GIT_PARTIAL_SPEC).unwrap();
        let path = vec!["git".to_string(), "add".to_string()];
        let help = generate_command_help(&spec, &path);
        assert!(help.contains("USAGE"));
        assert!(help.contains("git"));
        assert!(help.contains("add"));
    }

    #[test]
    fn test_command_help_shows_flags() {
        let spec = load_spec_from_str(GIT_PARTIAL_SPEC).unwrap();
        let path = vec!["git".to_string(), "add".to_string()];
        let help = generate_command_help(&spec, &path);
        assert!(help.contains("--dry-run") || help.contains("dry-run"));
    }

    #[test]
    fn test_command_help_shows_arguments() {
        let spec = load_spec_from_str(GIT_PARTIAL_SPEC).unwrap();
        let path = vec!["git".to_string(), "add".to_string()];
        let help = generate_command_help(&spec, &path);
        assert!(help.contains("PATHSPEC") || help.contains("ARGUMENTS"));
    }

    #[test]
    fn test_format_flag_signature_boolean() {
        let f = FlagDef {
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
        };
        let sig = format_flag_signature(&f);
        assert!(sig.contains("-v"));
        assert!(sig.contains("--verbose"));
        assert!(!sig.contains("<"));
    }

    #[test]
    fn test_format_flag_signature_nonboolean() {
        let f = FlagDef {
            id: "output".into(),
            short: Some("o".into()),
            long: Some("output".into()),
            single_dash_long: None,
            description: "Output file".into(),
            flag_type: "file".into(),
            required: false,
            default: None,
            value_name: Some("FILE".into()),
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
        };
        let sig = format_flag_signature(&f);
        assert!(sig.contains("<FILE>"));
    }

    #[test]
    fn test_format_arg_usage_required_variadic() {
        let a = ArgumentDef {
            id: "src".into(),
            name: "SOURCE".into(),
            description: "".into(),
            arg_type: "path".into(),
            required: true,
            variadic: true,
            variadic_min: 1,
            variadic_max: None,
            default: None,
            enum_values: vec![],
            required_unless_flag: vec![],
        };
        let s = format_arg_usage(&a);
        assert!(s.contains("SOURCE"));
        assert!(s.contains("...") || s.starts_with('<'));
    }

    #[test]
    fn test_format_arg_usage_optional_scalar() {
        let a = ArgumentDef {
            id: "p".into(),
            name: "PATH".into(),
            description: "".into(),
            arg_type: "path".into(),
            required: false,
            variadic: false,
            variadic_min: 0,
            variadic_max: None,
            default: None,
            enum_values: vec![],
            required_unless_flag: vec![],
        };
        let s = format_arg_usage(&a);
        assert!(s.starts_with('['));
        assert!(s.ends_with(']'));
    }
}
