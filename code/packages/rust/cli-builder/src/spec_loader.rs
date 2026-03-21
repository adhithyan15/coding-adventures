// spec_loader.rs -- Load and validate a CLI spec from JSON
// ==========================================================
//
// The spec loader is the first thing that runs. It:
//
//   1. Deserializes JSON → `CliSpec` via serde.
//   2. Validates the spec semantics (§6.4.3):
//      a. Checks the `cli_builder_spec_version` field.
//      b. Checks for duplicate IDs within each scope.
//      c. Verifies every flag has at least one form (short/long/single_dash_long).
//      d. Verifies all `conflicts_with` and `requires` IDs exist.
//      e. Verifies `enum_values` is non-empty for `type: "enum"` flags/args.
//      f. Verifies at most one variadic argument per scope.
//      g. Builds G_flag for each scope and checks for cycles using
//         `directed_graph::Graph::has_cycle()`.
//
// If validation passes, the caller gets back a `CliSpec` that is safe to use.
// Any validation failure surfaces as `CliBuilderError::SpecError`.

use std::collections::HashSet;

use directed_graph::Graph;

use crate::errors::CliBuilderError;
use crate::types::{ArgumentDef, CliSpec, CommandDef, FlagDef};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse and validate a CLI spec from a JSON string.
///
/// # Errors
///
/// Returns `CliBuilderError::JsonError` if JSON parsing fails, or
/// `CliBuilderError::SpecError` if semantic validation fails.
///
/// # Example
///
/// ```
/// # use cli_builder::spec_loader::load_spec_from_str;
/// let json = r#"{"cli_builder_spec_version":"1.0","name":"echo","description":"Print text"}"#;
/// let spec = load_spec_from_str(json).unwrap();
/// assert_eq!(spec.name, "echo");
/// ```
pub fn load_spec_from_str(json: &str) -> Result<CliSpec, CliBuilderError> {
    let spec: CliSpec = serde_json::from_str(json)
        .map_err(|e| CliBuilderError::JsonError(e.to_string()))?;
    validate_spec(&spec)?;
    Ok(spec)
}

/// Parse and validate a CLI spec from a file on disk.
///
/// # Errors
///
/// Returns `CliBuilderError::IoError` if the file cannot be read, or
/// `CliBuilderError::JsonError` / `CliBuilderError::SpecError` on bad content.
pub fn load_spec_from_file(path: &str) -> Result<CliSpec, CliBuilderError> {
    let content = std::fs::read_to_string(path)?;
    load_spec_from_str(&content)
}

// ---------------------------------------------------------------------------
// Validation entry point
// ---------------------------------------------------------------------------

/// Validate all semantic constraints on a parsed `CliSpec`.
///
/// This is a recursive descent through the command tree. For each node
/// (root + every command at any depth), we validate:
/// - No duplicate IDs.
/// - All `conflicts_with` / `requires` IDs exist in scope + global flags.
/// - `enum_values` present for enum types.
/// - At most one variadic argument.
/// - No cycle in the flag dependency graph (G_flag).
fn validate_spec(spec: &CliSpec) -> Result<(), CliBuilderError> {
    // Version check: only "1.0" is supported.
    if spec.cli_builder_spec_version != "1.0" {
        return Err(CliBuilderError::SpecError(format!(
            "unsupported cli_builder_spec_version {:?}; only \"1.0\" is supported",
            spec.cli_builder_spec_version
        )));
    }

    // Validate global_flags in isolation first (they have no parent scope).
    validate_flags_self_consistency(&spec.global_flags, "global_flags")?;

    // Collect global flag IDs so child scopes can reference them.
    let global_ids: HashSet<String> =
        spec.global_flags.iter().map(|f| f.id.clone()).collect();

    // Validate root scope: global_flags + root flags + root arguments + root commands.
    let root_scope_name = format!("root ({})", spec.name);
    validate_scope(
        &spec.flags,
        &spec.arguments,
        &spec.commands,
        &spec.mutually_exclusive_groups,
        &global_ids,
        &root_scope_name,
    )?;

    // Recursively validate each command subtree.
    for cmd in &spec.commands {
        validate_command(cmd, &global_ids)?;
    }

    Ok(())
}

/// Validate a single command node and recurse into its children.
fn validate_command(cmd: &CommandDef, global_ids: &HashSet<String>) -> Result<(), CliBuilderError> {
    let scope_name = format!("command '{}'", cmd.name);
    validate_scope(
        &cmd.flags,
        &cmd.arguments,
        &cmd.commands,
        &cmd.mutually_exclusive_groups,
        global_ids,
        &scope_name,
    )?;
    for child in &cmd.commands {
        validate_command(child, global_ids)?;
    }
    Ok(())
}

/// Validate one scope (root or a command node).
///
/// A scope's "available flag IDs" = its own flags + global_flags (when
/// `inherit_global_flags` is true by default). `conflicts_with` and `requires`
/// must reference IDs within this set.
fn validate_scope(
    flags: &[FlagDef],
    arguments: &[ArgumentDef],
    _commands: &[CommandDef],
    exclusive_groups: &[crate::types::ExclusiveGroup],
    global_ids: &HashSet<String>,
    scope_name: &str,
) -> Result<(), CliBuilderError> {
    // -----------------------------------------------------------------------
    // 1. No duplicate flag IDs
    // -----------------------------------------------------------------------
    let mut seen_flag_ids: HashSet<&str> = HashSet::new();
    for f in flags {
        if !seen_flag_ids.insert(f.id.as_str()) {
            return Err(CliBuilderError::SpecError(format!(
                "{}: duplicate flag id {:?}",
                scope_name, f.id
            )));
        }
    }

    // -----------------------------------------------------------------------
    // 2. No duplicate argument IDs
    // -----------------------------------------------------------------------
    let mut seen_arg_ids: HashSet<&str> = HashSet::new();
    for a in arguments {
        if !seen_arg_ids.insert(a.id.as_str()) {
            return Err(CliBuilderError::SpecError(format!(
                "{}: duplicate argument id {:?}",
                scope_name, a.id
            )));
        }
    }

    // -----------------------------------------------------------------------
    // 3. Every flag must have at least one form
    // -----------------------------------------------------------------------
    for f in flags {
        validate_flag_form(f, scope_name)?;
    }

    // -----------------------------------------------------------------------
    // 4. Build set of valid flag IDs for cross-reference checks
    //    (local flags + global flags)
    // -----------------------------------------------------------------------
    let local_flag_ids: HashSet<String> = flags.iter().map(|f| f.id.clone()).collect();
    let all_flag_ids: HashSet<String> = local_flag_ids.iter().cloned()
        .chain(global_ids.iter().cloned())
        .collect();

    // -----------------------------------------------------------------------
    // 5. Validate conflicts_with and requires cross-references
    // -----------------------------------------------------------------------
    for f in flags {
        for cid in &f.conflicts_with {
            if !all_flag_ids.contains(cid.as_str()) {
                return Err(CliBuilderError::SpecError(format!(
                    "{}: flag {:?} conflicts_with unknown id {:?}",
                    scope_name, f.id, cid
                )));
            }
        }
        for rid in &f.requires {
            if !all_flag_ids.contains(rid.as_str()) {
                return Err(CliBuilderError::SpecError(format!(
                    "{}: flag {:?} requires unknown id {:?}",
                    scope_name, f.id, rid
                )));
            }
        }
        for uid in &f.required_unless {
            if !all_flag_ids.contains(uid.as_str()) {
                return Err(CliBuilderError::SpecError(format!(
                    "{}: flag {:?} required_unless unknown id {:?}",
                    scope_name, f.id, uid
                )));
            }
        }
        // enum_values must be non-empty for enum flags
        if f.flag_type == "enum" && f.enum_values.is_empty() {
            return Err(CliBuilderError::SpecError(format!(
                "{}: flag {:?} has type \"enum\" but enum_values is empty",
                scope_name, f.id
            )));
        }
    }

    // -----------------------------------------------------------------------
    // 6. Validate arguments
    // -----------------------------------------------------------------------
    let mut variadic_count = 0usize;
    for a in arguments {
        if a.variadic {
            variadic_count += 1;
        }
        if a.arg_type == "enum" && a.enum_values.is_empty() {
            return Err(CliBuilderError::SpecError(format!(
                "{}: argument {:?} has type \"enum\" but enum_values is empty",
                scope_name, a.id
            )));
        }
        // required_unless_flag cross-references
        for fid in &a.required_unless_flag {
            if !all_flag_ids.contains(fid.as_str()) {
                return Err(CliBuilderError::SpecError(format!(
                    "{}: argument {:?} required_unless_flag references unknown flag id {:?}",
                    scope_name, a.id, fid
                )));
            }
        }
    }
    if variadic_count > 1 {
        return Err(CliBuilderError::SpecError(format!(
            "{}: at most one argument may be variadic, found {}",
            scope_name, variadic_count
        )));
    }

    // -----------------------------------------------------------------------
    // 7. Validate mutually exclusive groups
    // -----------------------------------------------------------------------
    for group in exclusive_groups {
        for fid in &group.flag_ids {
            if !all_flag_ids.contains(fid.as_str()) {
                return Err(CliBuilderError::SpecError(format!(
                    "{}: exclusive group {:?} references unknown flag id {:?}",
                    scope_name, group.id, fid
                )));
            }
        }
    }

    // -----------------------------------------------------------------------
    // 8. Cycle detection in G_flag (flag dependency graph)
    //
    //    Build a directed graph where an edge A → B means "flag A requires B".
    //    A cycle like A→B→A means the spec is self-contradictory.
    // -----------------------------------------------------------------------
    let mut g_flag = Graph::new_allow_self_loops(); // self-loops are caught as a cycle
    for f in flags {
        g_flag.add_node(&f.id);
        for req in &f.requires {
            // add_edge implicitly creates nodes, so we don't need to pre-add.
            // We ignore the error from add_edge since we've already validated
            // that `req` exists, and Graph allows duplicate edges silently.
            let _ = g_flag.add_edge(&f.id, req);
        }
    }
    if g_flag.has_cycle() {
        // Find the cycle for a descriptive error message.
        return Err(CliBuilderError::SpecError(format!(
            "{}: circular 'requires' dependency detected in flag dependency graph",
            scope_name
        )));
    }

    Ok(())
}

/// Check that a flag in the global_flags array (which has no cross-references
/// to validate) still satisfies the basic structural rules.
fn validate_flags_self_consistency(flags: &[FlagDef], scope_name: &str) -> Result<(), CliBuilderError> {
    let mut seen_ids: HashSet<&str> = HashSet::new();
    for f in flags {
        if !seen_ids.insert(f.id.as_str()) {
            return Err(CliBuilderError::SpecError(format!(
                "{}: duplicate flag id {:?}",
                scope_name, f.id
            )));
        }
        validate_flag_form(f, scope_name)?;
        if f.flag_type == "enum" && f.enum_values.is_empty() {
            return Err(CliBuilderError::SpecError(format!(
                "{}: flag {:?} has type \"enum\" but enum_values is empty",
                scope_name, f.id
            )));
        }
    }
    Ok(())
}

/// Verify that a flag has at least one usable form.
fn validate_flag_form(f: &FlagDef, scope_name: &str) -> Result<(), CliBuilderError> {
    if f.short.is_none() && f.long.is_none() && f.single_dash_long.is_none() {
        return Err(CliBuilderError::SpecError(format!(
            "{}: flag {:?} must have at least one of 'short', 'long', or 'single_dash_long'",
            scope_name, f.id
        )));
    }
    Ok(())
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal valid spec used as a baseline in multiple tests.
    const MINIMAL_SPEC: &str = r#"{
        "cli_builder_spec_version": "1.0",
        "name": "echo",
        "description": "Print text"
    }"#;

    #[test]
    fn test_load_minimal_spec() {
        let spec = load_spec_from_str(MINIMAL_SPEC).unwrap();
        assert_eq!(spec.name, "echo");
        assert_eq!(spec.parsing_mode, "gnu");
        assert!(spec.global_flags.is_empty());
        assert!(spec.commands.is_empty());
    }

    #[test]
    fn test_wrong_version_rejected() {
        let json = r#"{"cli_builder_spec_version":"2.0","name":"x","description":"y"}"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("2.0"));
        }
    }

    #[test]
    fn test_invalid_json_rejected() {
        let err = load_spec_from_str("{not valid json").unwrap_err();
        assert!(matches!(err, CliBuilderError::JsonError(_)));
    }

    #[test]
    fn test_duplicate_flag_id_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"verbose","short":"v","description":"verbose","type":"boolean"},
                {"id":"verbose","short":"q","description":"quiet","type":"boolean"}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("duplicate"));
        }
    }

    #[test]
    fn test_flag_with_no_form_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"bad","description":"has no form","type":"boolean"}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("short") || msg.contains("long"));
        }
    }

    #[test]
    fn test_enum_without_values_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"fmt","long":"format","description":"output format","type":"enum","enum_values":[]}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("enum_values"));
        }
    }

    #[test]
    fn test_conflicts_with_unknown_id_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"verbose","short":"v","description":"verbose","type":"boolean","conflicts_with":["nonexistent"]}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
    }

    #[test]
    fn test_requires_unknown_id_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"human","short":"h","description":"human","type":"boolean","requires":["nonexistent"]}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
    }

    #[test]
    fn test_cycle_in_requires_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
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
    fn test_two_variadic_args_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "arguments": [
                {"id":"a","name":"A","description":"a","type":"string","variadic":true},
                {"id":"b","name":"B","description":"b","type":"string","variadic":true}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("variadic"));
        }
    }

    #[test]
    fn test_valid_conflicts_with_global_flag() {
        // A local flag that conflicts_with a global flag should pass.
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "global_flags": [
                {"id":"verbose","short":"v","description":"verbose","type":"boolean"}
            ],
            "flags": [
                {"id":"quiet","short":"q","description":"quiet","type":"boolean","conflicts_with":["verbose"]}
            ]
        }"#;
        let spec = load_spec_from_str(json).unwrap();
        assert_eq!(spec.name, "x");
    }

    #[test]
    fn test_exclusive_group_unknown_flag_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "mutually_exclusive_groups": [
                {"id":"grp","flag_ids":["nonexistent"],"required":false}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
    }

    #[test]
    fn test_ls_spec_valid() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "ls",
            "description": "List directory contents",
            "version": "8.32",
            "parsing_mode": "gnu",
            "flags": [
                {"id":"long-listing","short":"l","description":"Long listing","type":"boolean","conflicts_with":["single-column"]},
                {"id":"all","short":"a","long":"all","description":"Show hidden","type":"boolean"},
                {"id":"human-readable","short":"h","long":"human-readable","description":"Human sizes","type":"boolean","requires":["long-listing"]},
                {"id":"single-column","short":"1","description":"One per line","type":"boolean","conflicts_with":["long-listing"]}
            ],
            "arguments": [
                {"id":"path","name":"PATH","description":"Path","type":"path","required":false,"variadic":true,"variadic_min":0}
            ]
        }"#;
        let spec = load_spec_from_str(json).unwrap();
        assert_eq!(spec.name, "ls");
        assert_eq!(spec.flags.len(), 4);
    }

    // -----------------------------------------------------------------------
    // required_unless references unknown flag id
    // -----------------------------------------------------------------------

    #[test]
    fn test_required_unless_unknown_id_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "flags": [
                {"id":"msg","short":"m","description":"msg","type":"string","required":true,"required_unless":["nonexistent"]}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("required_unless") || msg.contains("nonexistent"));
        }
    }

    // -----------------------------------------------------------------------
    // argument enum type with empty enum_values rejected
    // -----------------------------------------------------------------------

    #[test]
    fn test_argument_enum_without_values_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "arguments": [
                {"id":"mode","name":"MODE","description":"mode","type":"enum","enum_values":[]}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("enum_values"));
        }
    }

    // -----------------------------------------------------------------------
    // argument required_unless_flag references unknown flag id
    // -----------------------------------------------------------------------

    #[test]
    fn test_argument_required_unless_flag_unknown_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "arguments": [
                {"id":"pattern","name":"PATTERN","description":"pattern","type":"string","required_unless_flag":["nonexistent"]}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("required_unless_flag") || msg.contains("nonexistent"));
        }
    }

    // -----------------------------------------------------------------------
    // load_spec_from_file — missing file
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_spec_from_file_missing() {
        let err = load_spec_from_file("/tmp/cli_builder_no_such_spec_xyz_12345.json").unwrap_err();
        assert!(matches!(err, CliBuilderError::IoError(_)));
    }

    // -----------------------------------------------------------------------
    // global_flags: duplicate ID rejected
    // -----------------------------------------------------------------------

    #[test]
    fn test_global_flags_duplicate_id_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "global_flags": [
                {"id":"verbose","short":"v","description":"verbose","type":"boolean"},
                {"id":"verbose","short":"q","description":"quiet","type":"boolean"}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("duplicate"));
        }
    }

    // -----------------------------------------------------------------------
    // global_flags: flag with no form rejected
    // -----------------------------------------------------------------------

    #[test]
    fn test_global_flags_no_form_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "global_flags": [
                {"id":"bad","description":"no form","type":"boolean"}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
    }

    // -----------------------------------------------------------------------
    // global_flags: enum without values rejected
    // -----------------------------------------------------------------------

    #[test]
    fn test_global_flags_enum_without_values_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "global_flags": [
                {"id":"fmt","long":"format","description":"format","type":"enum","enum_values":[]}
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
    }

    // -----------------------------------------------------------------------
    // Nested subcommand validation — errors propagate
    // -----------------------------------------------------------------------

    #[test]
    fn test_nested_command_duplicate_flag_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "commands": [
                {
                    "id": "cmd-foo",
                    "name": "foo",
                    "description": "foo",
                    "flags": [
                        {"id":"verbose","short":"v","description":"verbose","type":"boolean"},
                        {"id":"verbose","short":"q","description":"quiet","type":"boolean"}
                    ]
                }
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
    }

    #[test]
    fn test_nested_command_cycle_rejected() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "commands": [
                {
                    "id": "cmd-foo",
                    "name": "foo",
                    "description": "foo",
                    "flags": [
                        {"id":"a","short":"a","description":"a","type":"boolean","requires":["b"]},
                        {"id":"b","short":"b","description":"b","type":"boolean","requires":["a"]}
                    ]
                }
            ]
        }"#;
        let err = load_spec_from_str(json).unwrap_err();
        assert!(matches!(err, CliBuilderError::SpecError(_)));
        if let CliBuilderError::SpecError(msg) = err {
            assert!(msg.contains("circular") || msg.contains("cycle"));
        }
    }

    #[test]
    fn test_deeply_nested_command_validated() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "commands": [
                {
                    "id": "cmd-remote",
                    "name": "remote",
                    "description": "remote",
                    "commands": [
                        {
                            "id": "cmd-remote-add",
                            "name": "add",
                            "description": "add remote",
                            "flags": [
                                {"id":"verbose","short":"v","description":"verbose","type":"boolean"}
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let spec = load_spec_from_str(json).unwrap();
        assert_eq!(spec.commands[0].commands[0].name, "add");
    }

    // -----------------------------------------------------------------------
    // Spec with display_name and version fields
    // -----------------------------------------------------------------------

    #[test]
    fn test_spec_with_display_name_and_version() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "myapp",
            "display_name": "My Application",
            "description": "A cool app",
            "version": "2.5.0"
        }"#;
        let spec = load_spec_from_str(json).unwrap();
        assert_eq!(spec.display_name, Some("My Application".to_string()));
        assert_eq!(spec.version, Some("2.5.0".to_string()));
    }

    // -----------------------------------------------------------------------
    // Spec with builtin_flags explicitly disabled
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_flags_disabled() {
        let json = r#"{
            "cli_builder_spec_version": "1.0",
            "name": "x",
            "description": "y",
            "builtin_flags": {"help": false, "version": false}
        }"#;
        let spec = load_spec_from_str(json).unwrap();
        assert!(!spec.builtin_flags.help);
        assert!(!spec.builtin_flags.version);
    }
}
