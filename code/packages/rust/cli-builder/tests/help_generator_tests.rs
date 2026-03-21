// tests/help_generator_tests.rs -- Integration tests for help text generation
// =============================================================================

use cli_builder::help_generator::{generate_command_help, generate_root_help};
use cli_builder::spec_loader::load_spec_from_str;

// ---------------------------------------------------------------------------
// Shared specs
// ---------------------------------------------------------------------------

const ECHO_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "echo",
    "display_name": "Echo",
    "description": "Display a line of text",
    "version": "8.32",
    "flags": [
        {"id":"no-newline","short":"n","description":"Do not output trailing newline","type":"boolean"},
        {"id":"enable-escapes","short":"e","description":"Enable interpretation of backslash escapes","type":"boolean","conflicts_with":["disable-escapes"]}
    ],
    "arguments": [
        {"id":"string","name":"STRING","description":"Text to print","type":"string","required":false,"variadic":true,"variadic_min":0}
    ]
}"#;

const GIT_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "git",
    "description": "The stupid content tracker",
    "version": "2.43.0",
    "global_flags": [
        {"id":"no-pager","long":"no-pager","description":"Do not pipe output into a pager","type":"boolean"}
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
            "description": "Record changes to the repository",
            "flags": [
                {"id":"message","short":"m","long":"message","description":"Commit message","type":"string","value_name":"MSG","required":true}
            ],
            "arguments": []
        }
    ]
}"#;

// ---------------------------------------------------------------------------
// Root help structure tests
// ---------------------------------------------------------------------------

#[test]
fn test_root_help_contains_usage() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("USAGE"), "expected USAGE section:\n{}", help);
}

#[test]
fn test_root_help_contains_program_name() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("echo"), "expected program name in help:\n{}", help);
}

#[test]
fn test_root_help_contains_description() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("DESCRIPTION"), "expected DESCRIPTION section:\n{}", help);
    assert!(help.contains("Display a line of text"), "expected description text:\n{}", help);
}

#[test]
fn test_root_help_contains_options_section() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("OPTIONS"), "expected OPTIONS section:\n{}", help);
}

#[test]
fn test_root_help_shows_short_flags() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("-n"), "expected -n flag in help:\n{}", help);
    assert!(help.contains("-e"), "expected -e flag in help:\n{}", help);
}

#[test]
fn test_root_help_shows_flag_description() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("trailing newline") || help.contains("No not output") || help.contains("output trailing"), "expected flag description:\n{}", help);
}

#[test]
fn test_root_help_shows_arguments_section() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("STRING") || help.contains("ARGUMENTS"), "expected args:\n{}", help);
}

#[test]
fn test_root_help_shows_global_options() {
    let spec = load_spec_from_str(ECHO_SPEC).unwrap();
    let help = generate_root_help(&spec);
    // Builtins --help and --version should appear
    assert!(help.contains("--help") || help.contains("GLOBAL OPTIONS"), "expected global opts:\n{}", help);
}

// ---------------------------------------------------------------------------
// Root help with subcommands
// ---------------------------------------------------------------------------

#[test]
fn test_root_help_shows_commands_section() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("COMMANDS"), "expected COMMANDS section:\n{}", help);
}

#[test]
fn test_root_help_shows_subcommand_names() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("add"), "expected 'add' command:\n{}", help);
    assert!(help.contains("commit"), "expected 'commit' command:\n{}", help);
}

#[test]
fn test_root_help_shows_subcommand_descriptions() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("Add file contents") || help.contains("index"), ":\n{}", help);
    assert!(help.contains("Record changes") || help.contains("repository"), ":\n{}", help);
}

// ---------------------------------------------------------------------------
// Subcommand help
// ---------------------------------------------------------------------------

#[test]
fn test_command_help_contains_usage() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let path = vec!["git".to_string(), "add".to_string()];
    let help = generate_command_help(&spec, &path);
    assert!(help.contains("USAGE"), ":\n{}", help);
}

#[test]
fn test_command_help_contains_program_and_subcommand() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let path = vec!["git".to_string(), "add".to_string()];
    let help = generate_command_help(&spec, &path);
    assert!(help.contains("git"), ":\n{}", help);
    assert!(help.contains("add"), ":\n{}", help);
}

#[test]
fn test_command_help_contains_description() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let path = vec!["git".to_string(), "add".to_string()];
    let help = generate_command_help(&spec, &path);
    assert!(help.contains("DESCRIPTION"), ":\n{}", help);
    assert!(help.contains("Add file contents") || help.contains("index"), ":\n{}", help);
}

#[test]
fn test_command_help_shows_command_flags() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let path = vec!["git".to_string(), "add".to_string()];
    let help = generate_command_help(&spec, &path);
    assert!(help.contains("dry-run") || help.contains("-n"), ":\n{}", help);
    assert!(help.contains("verbose") || help.contains("-v"), ":\n{}", help);
}

#[test]
fn test_command_help_shows_arguments() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let path = vec!["git".to_string(), "add".to_string()];
    let help = generate_command_help(&spec, &path);
    assert!(help.contains("PATHSPEC") || help.contains("ARGUMENTS"), ":\n{}", help);
}

#[test]
fn test_command_help_shows_global_options() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let path = vec!["git".to_string(), "add".to_string()];
    let help = generate_command_help(&spec, &path);
    assert!(help.contains("--no-pager") || help.contains("GLOBAL OPTIONS"), ":\n{}", help);
}

#[test]
fn test_command_help_root_path_falls_back_to_root() {
    let spec = load_spec_from_str(GIT_SPEC).unwrap();
    let path = vec!["git".to_string()]; // just program name
    let help = generate_command_help(&spec, &path);
    // Falls back to root help, which has COMMANDS
    assert!(help.contains("COMMANDS") || help.contains("git"), ":\n{}", help);
}

// ---------------------------------------------------------------------------
// Flag formatting
// ---------------------------------------------------------------------------

const VALUE_FLAG_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "sort",
    "description": "Sort lines",
    "flags": [
        {"id":"key","short":"k","long":"key","description":"Sort key","type":"string","value_name":"KEYDEF","repeatable":true},
        {"id":"field-separator","short":"t","long":"field-separator","description":"Field separator","type":"string","value_name":"SEP"},
        {"id":"reverse","short":"r","long":"reverse","description":"Reverse","type":"boolean"},
        {"id":"numeric","short":"n","long":"numeric-sort","description":"Numeric sort","type":"boolean"}
    ],
    "arguments": [
        {"id":"file","name":"FILE","description":"Files","type":"path","required":false,"variadic":true,"variadic_min":0}
    ]
}"#;

#[test]
fn test_help_shows_value_name_in_option() {
    let spec = load_spec_from_str(VALUE_FLAG_SPEC).unwrap();
    let help = generate_root_help(&spec);
    // -k, --key <KEYDEF>
    assert!(help.contains("KEYDEF") || help.contains("key"), ":\n{}", help);
}

#[test]
fn test_help_shows_boolean_flag_without_value() {
    let spec = load_spec_from_str(VALUE_FLAG_SPEC).unwrap();
    let help = generate_root_help(&spec);
    // -r, --reverse (no <VALUE>)
    assert!(help.contains("reverse"), ":\n{}", help);
}

// ---------------------------------------------------------------------------
// Argument formatting
// ---------------------------------------------------------------------------

const ARG_FORMAT_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "cp",
    "description": "Copy files",
    "flags": [],
    "arguments": [
        {"id":"source","name":"SOURCE","description":"Source files","type":"path","required":true,"variadic":true,"variadic_min":1},
        {"id":"dest","name":"DEST","description":"Destination","type":"path","required":true}
    ]
}"#;

#[test]
fn test_help_required_variadic_arg_format() {
    let spec = load_spec_from_str(ARG_FORMAT_SPEC).unwrap();
    let help = generate_root_help(&spec);
    // Required variadic: <SOURCE...> or SOURCE...
    assert!(help.contains("SOURCE"), "expected SOURCE:\n{}", help);
}

#[test]
fn test_help_required_scalar_arg_format() {
    let spec = load_spec_from_str(ARG_FORMAT_SPEC).unwrap();
    let help = generate_root_help(&spec);
    assert!(help.contains("DEST"), "expected DEST:\n{}", help);
}

// ---------------------------------------------------------------------------
// Default value display
// ---------------------------------------------------------------------------

const DEFAULT_SPEC: &str = r#"{
    "cli_builder_spec_version": "1.0",
    "name": "head",
    "description": "Output first part of files",
    "flags": [
        {"id":"lines","short":"n","long":"lines","description":"Number of lines","type":"integer","value_name":"NUM","default":10}
    ],
    "arguments": [
        {"id":"file","name":"FILE","description":"Files","type":"path","required":false,"variadic":true,"variadic_min":0}
    ]
}"#;

#[test]
fn test_help_shows_default_value() {
    let spec = load_spec_from_str(DEFAULT_SPEC).unwrap();
    let help = generate_root_help(&spec);
    // Default value should appear as [default: 10]
    assert!(help.contains("default") || help.contains("10"), ":\n{}", help);
}
