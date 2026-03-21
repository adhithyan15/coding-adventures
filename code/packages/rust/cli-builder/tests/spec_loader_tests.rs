// tests/spec_loader_tests.rs -- Integration tests for spec loading and validation
// =================================================================================
//
// These tests exercise the spec_loader through the public crate API,
// covering both valid specs from the examples in §10 and invalid specs
// that should be rejected.

use cli_builder::errors::CliBuilderError;
use cli_builder::spec_loader::load_spec_from_str;

// ---------------------------------------------------------------------------
// Version validation
// ---------------------------------------------------------------------------

#[test]
fn test_version_10_accepted() {
    let json = r#"{"cli_builder_spec_version":"1.0","name":"x","description":"y"}"#;
    let spec = load_spec_from_str(json).unwrap();
    assert_eq!(spec.name, "x");
}

#[test]
fn test_version_20_rejected() {
    let json = r#"{"cli_builder_spec_version":"2.0","name":"x","description":"y"}"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

#[test]
fn test_invalid_json() {
    assert!(matches!(load_spec_from_str("{bad"), Err(CliBuilderError::JsonError(_))));
}

// ---------------------------------------------------------------------------
// Minimal valid spec
// ---------------------------------------------------------------------------

#[test]
fn test_minimal_spec_defaults() {
    let json = r#"{"cli_builder_spec_version":"1.0","name":"minimal","description":"Minimal spec"}"#;
    let spec = load_spec_from_str(json).unwrap();
    assert_eq!(spec.parsing_mode, "gnu");
    assert!(spec.builtin_flags.help);
    assert!(spec.builtin_flags.version);
    assert!(spec.global_flags.is_empty());
    assert!(spec.flags.is_empty());
    assert!(spec.arguments.is_empty());
    assert!(spec.commands.is_empty());
}

// ---------------------------------------------------------------------------
// Flag validation
// ---------------------------------------------------------------------------

#[test]
fn test_flag_with_short_only() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[{"id":"v","short":"v","description":"verbose","type":"boolean"}]
    }"#;
    assert!(load_spec_from_str(json).is_ok());
}

#[test]
fn test_flag_with_long_only() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[{"id":"verbose","long":"verbose","description":"verbose","type":"boolean"}]
    }"#;
    assert!(load_spec_from_str(json).is_ok());
}

#[test]
fn test_flag_with_sdl_only() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[{"id":"cp","single_dash_long":"cp","description":"classpath","type":"string"}]
    }"#;
    assert!(load_spec_from_str(json).is_ok());
}

#[test]
fn test_flag_no_form_rejected() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[{"id":"bad","description":"no form","type":"boolean"}]
    }"#;
    let err = load_spec_from_str(json).unwrap_err();
    assert!(matches!(err, CliBuilderError::SpecError(_)));
}

#[test]
fn test_duplicate_flag_id_rejected() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[
            {"id":"v","short":"v","description":"verbose","type":"boolean"},
            {"id":"v","short":"q","description":"quiet","type":"boolean"}
        ]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

#[test]
fn test_enum_flag_requires_values() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[{"id":"fmt","long":"format","description":"format","type":"enum","enum_values":[]}]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

#[test]
fn test_enum_flag_with_values_valid() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[{"id":"fmt","long":"format","description":"format","type":"enum","enum_values":["json","csv"]}]
    }"#;
    assert!(load_spec_from_str(json).is_ok());
}

#[test]
fn test_conflicts_with_unknown_id() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[
            {"id":"f","short":"f","description":"force","type":"boolean","conflicts_with":["nonexistent"]}
        ]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

#[test]
fn test_requires_unknown_id() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[
            {"id":"h","short":"h","description":"human","type":"boolean","requires":["nonexistent"]}
        ]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

#[test]
fn test_circular_requires_rejected() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[
            {"id":"a","short":"a","description":"a","type":"boolean","requires":["b"]},
            {"id":"b","short":"b","description":"b","type":"boolean","requires":["a"]}
        ]
    }"#;
    let err = load_spec_from_str(json).unwrap_err();
    assert!(matches!(err, CliBuilderError::SpecError(_)));
    if let CliBuilderError::SpecError(msg) = err {
        assert!(msg.contains("circular") || msg.contains("cycle"));
    }
}

#[test]
fn test_three_way_cycle_rejected() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[
            {"id":"a","short":"a","description":"a","type":"boolean","requires":["b"]},
            {"id":"b","short":"b","description":"b","type":"boolean","requires":["c"]},
            {"id":"c","short":"c","description":"c","type":"boolean","requires":["a"]}
        ]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

// ---------------------------------------------------------------------------
// Argument validation
// ---------------------------------------------------------------------------

#[test]
fn test_two_variadic_args_rejected() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "arguments":[
            {"id":"a","name":"A","description":"a","type":"string","variadic":true},
            {"id":"b","name":"B","description":"b","type":"string","variadic":true}
        ]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

#[test]
fn test_one_variadic_arg_valid() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "arguments":[
            {"id":"a","name":"A","description":"a","type":"string"},
            {"id":"b","name":"B","description":"b","type":"string","variadic":true}
        ]
    }"#;
    assert!(load_spec_from_str(json).is_ok());
}

#[test]
fn test_duplicate_argument_id_rejected() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "arguments":[
            {"id":"file","name":"FILE","description":"a","type":"path"},
            {"id":"file","name":"DIR","description":"b","type":"path"}
        ]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

// ---------------------------------------------------------------------------
// Mutually exclusive group validation
// ---------------------------------------------------------------------------

#[test]
fn test_exclusive_group_unknown_flag_rejected() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "mutually_exclusive_groups":[
            {"id":"g","flag_ids":["unknown"],"required":false}
        ]
    }"#;
    assert!(matches!(load_spec_from_str(json), Err(CliBuilderError::SpecError(_))));
}

#[test]
fn test_exclusive_group_valid() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "flags":[
            {"id":"a","short":"a","description":"a","type":"boolean"},
            {"id":"b","short":"b","description":"b","type":"boolean"}
        ],
        "mutually_exclusive_groups":[
            {"id":"g","flag_ids":["a","b"],"required":false}
        ]
    }"#;
    assert!(load_spec_from_str(json).is_ok());
}

// ---------------------------------------------------------------------------
// Full spec examples from §10
// ---------------------------------------------------------------------------

#[test]
fn test_echo_spec_valid() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"echo","description":"Display a line of text","version":"8.32",
        "flags":[
            {"id":"no-newline","short":"n","description":"No trailing newline","type":"boolean"},
            {"id":"enable-escapes","short":"e","description":"Enable backslash escapes","type":"boolean","conflicts_with":["disable-escapes"]},
            {"id":"disable-escapes","short":"E","description":"Disable backslash escapes","type":"boolean","conflicts_with":["enable-escapes"]}
        ],
        "arguments":[
            {"id":"string","name":"STRING","description":"Text to print","type":"string","required":false,"variadic":true,"variadic_min":0}
        ]
    }"#;
    let spec = load_spec_from_str(json).unwrap();
    assert_eq!(spec.name, "echo");
    assert_eq!(spec.flags.len(), 3);
    assert_eq!(spec.arguments.len(), 1);
}

#[test]
fn test_ls_spec_valid() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"ls","description":"List directory contents","version":"8.32",
        "parsing_mode":"gnu",
        "flags":[
            {"id":"long-listing","short":"l","description":"Long listing","type":"boolean","conflicts_with":["single-column"]},
            {"id":"all","short":"a","long":"all","description":"Show hidden","type":"boolean"},
            {"id":"human-readable","short":"h","long":"human-readable","description":"Human sizes","type":"boolean","requires":["long-listing"]},
            {"id":"single-column","short":"1","description":"One per line","type":"boolean","conflicts_with":["long-listing"]}
        ],
        "arguments":[
            {"id":"path","name":"PATH","description":"Path","type":"path","required":false,"variadic":true,"variadic_min":0}
        ]
    }"#;
    let spec = load_spec_from_str(json).unwrap();
    assert_eq!(spec.parsing_mode, "gnu");
    assert_eq!(spec.flags.len(), 4);
}

#[test]
fn test_git_spec_with_subcommands_valid() {
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"git","description":"The stupid content tracker",
        "version":"2.43.0","parsing_mode":"subcommand_first",
        "global_flags":[
            {"id":"no-pager","long":"no-pager","description":"No pager","type":"boolean"}
        ],
        "commands":[
            {
                "id":"cmd-add","name":"add","description":"Add files",
                "flags":[{"id":"verbose","short":"v","long":"verbose","description":"Verbose","type":"boolean"}],
                "arguments":[{"id":"pathspec","name":"PATHSPEC","description":"Files","type":"path","required":false,"variadic":true,"variadic_min":0}]
            },
            {
                "id":"cmd-commit","name":"commit","aliases":["ci"],"description":"Commit",
                "flags":[{"id":"message","short":"m","long":"message","description":"Message","type":"string","required":true}],
                "arguments":[]
            }
        ]
    }"#;
    let spec = load_spec_from_str(json).unwrap();
    assert_eq!(spec.commands.len(), 2);
    assert_eq!(spec.commands[1].aliases, vec!["ci"]);
}

#[test]
fn test_nested_command_requires_cross_reference_with_global() {
    // A nested command's flag can reference global_flags via cross-reference.
    let json = r#"{
        "cli_builder_spec_version":"1.0","name":"x","description":"y",
        "global_flags":[{"id":"verbose","short":"v","description":"verbose","type":"boolean"}],
        "commands":[
            {
                "id":"cmd-foo","name":"foo","description":"foo",
                "flags":[{"id":"quiet","short":"q","description":"quiet","type":"boolean","conflicts_with":["verbose"]}]
            }
        ]
    }"#;
    // Nested command's flags validate against global_flags cross-references.
    assert!(load_spec_from_str(json).is_ok());
}
