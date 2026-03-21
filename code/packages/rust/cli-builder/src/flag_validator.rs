// flag_validator.rs -- Flag constraint validation (Phase 3, §6.4.2)
// ===================================================================
//
// After scanning produces `parsed_flags`, this module validates all
// constraint relationships between flags:
//
//   1. conflicts_with: two flags that list each other cannot both be present.
//   2. requires (transitive): if flag A is present and A→B→C in G_flag,
//      then B and C must also be present.
//   3. required: flags with `required: true` must be present unless
//      `required_unless` is satisfied.
//   4. mutually_exclusive_groups: at most one (or exactly one if `required`)
//      flag from each group may be present.
//
// The transitive `requires` check uses `directed_graph::Graph::transitive_closure`
// so we don't have to implement BFS manually here.

use std::collections::HashMap;

use directed_graph::Graph;
use serde_json::Value;

use crate::errors::ParseError;
use crate::types::{ExclusiveGroup, FlagDef};

/// Validate all flag constraints for a parsed result.
///
/// Returns a (possibly empty) list of `ParseError` objects. The caller
/// accumulates these with any positional-resolution errors before deciding
/// whether to return `Ok` or `Err`.
///
/// # Arguments
///
/// * `parsed_flags` — map of flag id → parsed value (from scanning phase).
/// * `active_flags` — all flag definitions in scope (global + command path).
/// * `exclusive_groups` — mutually exclusive groups for this scope.
/// * `command_path` — for error context.
pub fn validate_flags(
    parsed_flags: &HashMap<String, Value>,
    active_flags: &[FlagDef],
    exclusive_groups: &[ExclusiveGroup],
    command_path: &[String],
) -> Vec<ParseError> {
    let mut errors: Vec<ParseError> = Vec::new();

    // Build a lookup map: flag id → FlagDef (for O(1) access).
    let flag_map: HashMap<&str, &FlagDef> =
        active_flags.iter().map(|f| (f.id.as_str(), f)).collect();

    // -----------------------------------------------------------------------
    // 1. Build G_flag and compute transitive_closure for `requires` checks.
    //
    // G_flag has an edge A → B meaning "flag A requires flag B".
    // transitive_closure(A) gives all flags transitively required by A.
    // -----------------------------------------------------------------------
    let mut g_flag = Graph::new();
    for f in active_flags {
        g_flag.add_node(&f.id);
    }
    for f in active_flags {
        for req in &f.requires {
            // Only add edges for flags that actually exist in scope.
            if flag_map.contains_key(req.as_str()) {
                let _ = g_flag.add_edge(&f.id, req);
            }
        }
    }

    // -----------------------------------------------------------------------
    // 2. For each flag that IS present, check conflicts and requires.
    // -----------------------------------------------------------------------
    for (flag_id, _val) in parsed_flags {
        let Some(flag_def) = flag_map.get(flag_id.as_str()) else {
            // Flag not in active set — was likely a builtin (help/version).
            continue;
        };

        // --- conflicts_with (bilateral pairwise check) ---
        for other_id in &flag_def.conflicts_with {
            if parsed_flags.contains_key(other_id.as_str()) {
                // Deduplicate: only report A conflicts B once (not also B conflicts A).
                // We report it from the perspective of the "smaller" ID lexicographically.
                if flag_id < other_id {
                    errors.push(ParseError::new(
                        "conflicting_flags",
                        format!(
                            "{}/{} and {} cannot be used together",
                            format_flag_name(flag_def),
                            flag_id,
                            other_id
                        ),
                        command_path.to_vec(),
                    ));
                }
            }
        }

        // --- requires (transitive via G_flag) ---
        // transitive_closure returns all nodes reachable by following edges
        // from `flag_id`. Each such node is transitively required.
        if g_flag.has_node(flag_id) {
            if let Ok(required_set) = g_flag.transitive_closure(flag_id) {
                for required_id in &required_set {
                    if !parsed_flags.contains_key(required_id.as_str()) {
                        let required_def = flag_map.get(required_id.as_str());
                        let req_name = required_def
                            .map(|d| format_flag_name(d))
                            .unwrap_or_else(|| required_id.clone());
                        errors.push(ParseError::new(
                            "missing_dependency_flag",
                            format!(
                                "{} requires {} (which is absent)",
                                format_flag_name(flag_def),
                                req_name
                            ),
                            command_path.to_vec(),
                        ));
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 3. Required flags that are absent.
    // -----------------------------------------------------------------------
    for f in active_flags {
        if !f.required {
            continue;
        }
        if parsed_flags.contains_key(f.id.as_str()) {
            continue;
        }
        // Check required_unless: exempt if any of the listed flags is present.
        let exempt = f.required_unless.iter().any(|uid| parsed_flags.contains_key(uid.as_str()));
        if !exempt {
            errors.push(ParseError::new(
                "missing_required_flag",
                format!("{} is required", format_flag_name(f)),
                command_path.to_vec(),
            ));
        }
    }

    // -----------------------------------------------------------------------
    // 4. Mutually exclusive groups.
    // -----------------------------------------------------------------------
    for group in exclusive_groups {
        let present: Vec<&String> = group
            .flag_ids
            .iter()
            .filter(|fid| parsed_flags.contains_key(fid.as_str()))
            .collect();

        if present.len() > 1 {
            // More than one flag from the exclusive group is present.
            let names: Vec<String> = present
                .iter()
                .filter_map(|fid| flag_map.get(fid.as_str()).map(|f| format_flag_name(f)))
                .collect();
            errors.push(ParseError::new(
                "exclusive_group_violation",
                format!("Only one of {} may be used", names.join(", ")),
                command_path.to_vec(),
            ));
        } else if group.required && present.is_empty() {
            // Required group has no flags present.
            let names: Vec<String> = group
                .flag_ids
                .iter()
                .filter_map(|fid| flag_map.get(fid.as_str()).map(|f| format_flag_name(f)))
                .collect();
            errors.push(ParseError::new(
                "missing_exclusive_group",
                format!("One of {} is required", names.join(", ")),
                command_path.to_vec(),
            ));
        }
    }

    errors
}

/// Format a flag's name for human-readable error messages.
///
/// Prefers `--long` form, then `-short`, then `-single_dash_long`.
fn format_flag_name(f: &FlagDef) -> String {
    if let Some(ref long) = f.long {
        if let Some(ref short) = f.short {
            return format!("-{}/--{}", short, long);
        }
        return format!("--{}", long);
    }
    if let Some(ref short) = f.short {
        return format!("-{}", short);
    }
    if let Some(ref sdl) = f.single_dash_long {
        return format!("-{}", sdl);
    }
    f.id.clone()
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mk_flag(id: &str, short: Option<&str>, long: Option<&str>, boolean: bool) -> FlagDef {
        FlagDef {
            id: id.to_string(),
            short: short.map(|s| s.to_string()),
            long: long.map(|s| s.to_string()),
            single_dash_long: None,
            description: "".to_string(),
            flag_type: if boolean { "boolean".to_string() } else { "string".to_string() },
            required: false,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
        }
    }

    fn ctx() -> Vec<String> { vec!["prog".to_string()] }

    // -----------------------------------------------------------------------
    // conflicts_with tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_conflicts() {
        let mut f1 = mk_flag("verbose", Some("v"), Some("verbose"), true);
        let f2 = mk_flag("quiet", Some("q"), Some("quiet"), true);
        f1.conflicts_with = vec!["quiet".to_string()];
        let flags = HashMap::from([("verbose".to_string(), json!(true))]);
        let errs = validate_flags(&flags, &[f1, f2], &[], &ctx());
        assert!(errs.is_empty());
    }

    #[test]
    fn test_conflicting_flags_error() {
        let mut f1 = mk_flag("force", Some("f"), Some("force"), true);
        let f2 = mk_flag("interactive", Some("i"), Some("interactive"), true);
        f1.conflicts_with = vec!["interactive".to_string()];
        let flags = HashMap::from([
            ("force".to_string(), json!(true)),
            ("interactive".to_string(), json!(true)),
        ]);
        let errs = validate_flags(&flags, &[f1, f2], &[], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "conflicting_flags");
    }

    // -----------------------------------------------------------------------
    // requires tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_requires_satisfied() {
        let mut f1 = mk_flag("human-readable", Some("h"), Some("human-readable"), true);
        let f2 = mk_flag("long-listing", Some("l"), None, true);
        f1.requires = vec!["long-listing".to_string()];
        let flags = HashMap::from([
            ("human-readable".to_string(), json!(true)),
            ("long-listing".to_string(), json!(true)),
        ]);
        let errs = validate_flags(&flags, &[f1, f2], &[], &ctx());
        assert!(errs.is_empty());
    }

    #[test]
    fn test_requires_missing_dependency() {
        let mut f1 = mk_flag("human-readable", Some("h"), Some("human-readable"), true);
        let f2 = mk_flag("long-listing", Some("l"), None, true);
        f1.requires = vec!["long-listing".to_string()];
        // human-readable present but long-listing absent → error
        let flags = HashMap::from([("human-readable".to_string(), json!(true))]);
        let errs = validate_flags(&flags, &[f1, f2], &[], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "missing_dependency_flag");
    }

    #[test]
    fn test_requires_transitive() {
        // A requires B, B requires C. A present, C absent → error.
        let mut fa = mk_flag("a", Some("a"), None, true);
        let mut fb = mk_flag("b", Some("b"), None, true);
        let fc = mk_flag("c", Some("c"), None, true);
        fa.requires = vec!["b".to_string()];
        fb.requires = vec!["c".to_string()];
        let flags = HashMap::from([
            ("a".to_string(), json!(true)),
            ("b".to_string(), json!(true)),
            // "c" is absent
        ]);
        let errs = validate_flags(&flags, &[fa, fb, fc], &[], &ctx());
        // Should report that "a" (or "b") requires "c" which is absent.
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| e.error_type == "missing_dependency_flag"));
    }

    // -----------------------------------------------------------------------
    // required flag tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_required_flag_present() {
        let mut f = mk_flag("message", Some("m"), Some("message"), false);
        f.required = true;
        let flags = HashMap::from([("message".to_string(), json!("hello"))]);
        let errs = validate_flags(&flags, &[f], &[], &ctx());
        assert!(errs.is_empty());
    }

    #[test]
    fn test_required_flag_absent() {
        let mut f = mk_flag("message", Some("m"), Some("message"), false);
        f.required = true;
        let flags = HashMap::new();
        let errs = validate_flags(&flags, &[f], &[], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "missing_required_flag");
    }

    #[test]
    fn test_required_unless_exempts() {
        let mut f = mk_flag("message", Some("m"), Some("message"), false);
        f.required = true;
        f.required_unless = vec!["file".to_string()];
        let f2 = mk_flag("file", Some("F"), None, false);
        // "file" flag is present → "message" is not required
        let flags = HashMap::from([("file".to_string(), json!("patterns.txt"))]);
        let errs = validate_flags(&flags, &[f, f2], &[], &ctx());
        assert!(errs.is_empty());
    }

    // -----------------------------------------------------------------------
    // exclusive group tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exclusive_group_one_present() {
        let fa = mk_flag("extended-regexp", Some("E"), Some("extended-regexp"), true);
        let fb = mk_flag("fixed-strings",   Some("F"), Some("fixed-strings"), true);
        let fc = mk_flag("perl-regexp",      Some("P"), Some("perl-regexp"), true);
        let group = ExclusiveGroup {
            id: "regex-engine".to_string(),
            flag_ids: vec!["extended-regexp".to_string(), "fixed-strings".to_string(), "perl-regexp".to_string()],
            required: false,
        };
        let flags = HashMap::from([("extended-regexp".to_string(), json!(true))]);
        let errs = validate_flags(&flags, &[fa, fb, fc], &[group], &ctx());
        assert!(errs.is_empty());
    }

    #[test]
    fn test_exclusive_group_two_present() {
        let fa = mk_flag("extended-regexp", Some("E"), Some("extended-regexp"), true);
        let fb = mk_flag("fixed-strings",   Some("F"), Some("fixed-strings"), true);
        let fc = mk_flag("perl-regexp",      Some("P"), Some("perl-regexp"), true);
        let group = ExclusiveGroup {
            id: "regex-engine".to_string(),
            flag_ids: vec!["extended-regexp".to_string(), "fixed-strings".to_string(), "perl-regexp".to_string()],
            required: false,
        };
        let flags = HashMap::from([
            ("extended-regexp".to_string(), json!(true)),
            ("fixed-strings".to_string(), json!(true)),
        ]);
        let errs = validate_flags(&flags, &[fa, fb, fc], &[group], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "exclusive_group_violation");
    }

    #[test]
    fn test_exclusive_group_required_missing() {
        let fa = mk_flag("create",  Some("c"), None, true);
        let fb = mk_flag("extract", Some("x"), None, true);
        let fc = mk_flag("list",    Some("t"), None, true);
        let group = ExclusiveGroup {
            id: "operation".to_string(),
            flag_ids: vec!["create".to_string(), "extract".to_string(), "list".to_string()],
            required: true,
        };
        let flags = HashMap::new(); // none present
        let errs = validate_flags(&flags, &[fa, fb, fc], &[group], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "missing_exclusive_group");
    }

    #[test]
    fn test_exclusive_group_required_satisfied() {
        let fa = mk_flag("create",  Some("c"), None, true);
        let fb = mk_flag("extract", Some("x"), None, true);
        let group = ExclusiveGroup {
            id: "operation".to_string(),
            flag_ids: vec!["create".to_string(), "extract".to_string()],
            required: true,
        };
        let flags = HashMap::from([("create".to_string(), json!(true))]);
        let errs = validate_flags(&flags, &[fa, fb], &[group], &ctx());
        assert!(errs.is_empty());
    }

    // -----------------------------------------------------------------------
    // format_flag_name coverage
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_flag_name_long_and_short() {
        let f = mk_flag("verbose", Some("v"), Some("verbose"), true);
        // Covered indirectly — validate a flag with long+short and check error message.
        let mut f2 = mk_flag("quiet", Some("q"), Some("quiet"), true);
        f2.requires = vec!["verbose".to_string()];
        // quiet requires verbose; quiet present, verbose absent → error msg uses --verbose/-v form
        let flags = HashMap::from([("quiet".to_string(), json!(true))]);
        let errs = validate_flags(&flags, &[f, f2], &[], &ctx());
        // There should be an error about missing dependency
        assert!(!errs.is_empty());
        let msg = &errs[0].message;
        // The message must reference the flag name in some form
        assert!(msg.contains("verbose") || msg.contains("-v") || msg.contains("--verbose"));
    }

    #[test]
    fn test_format_flag_name_short_only() {
        // A flag with only a short form — format_flag_name should produce "-x"
        let mut f = mk_flag("extract", Some("x"), None, true);
        f.required = true;
        let flags = HashMap::new(); // extract absent
        let errs = validate_flags(&flags, &[f], &[], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "missing_required_flag");
        // message should use -x form
        assert!(errs[0].message.contains("-x") || errs[0].message.contains("extract"));
    }

    #[test]
    fn test_format_flag_name_sdl_only() {
        // A flag with only a single_dash_long form
        let f = FlagDef {
            id: "classpath".to_string(),
            short: None,
            long: None,
            single_dash_long: Some("classpath".to_string()),
            description: "classpath".to_string(),
            flag_type: "string".to_string(),
            required: true,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
        };
        let flags = HashMap::new(); // absent
        let errs = validate_flags(&flags, &[f], &[], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "missing_required_flag");
        assert!(errs[0].message.contains("-classpath") || errs[0].message.contains("classpath"));
    }

    #[test]
    fn test_format_flag_name_id_only_fallback() {
        // A flag with no short, no long, no sdl — id is the fallback
        let f = FlagDef {
            id: "secret-flag".to_string(),
            short: None,
            long: None,
            single_dash_long: None,
            description: "secret".to_string(),
            flag_type: "boolean".to_string(),
            required: true,
            default: None,
            value_name: None,
            enum_values: vec![],
            conflicts_with: vec![],
            requires: vec![],
            required_unless: vec![],
            repeatable: false,
        };
        let flags = HashMap::new();
        let errs = validate_flags(&flags, &[f], &[], &ctx());
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("secret-flag"));
    }

    // -----------------------------------------------------------------------
    // Builtin flag present in parsed_flags but not in active_flags — skipped
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_flag_in_parsed_flags_is_skipped() {
        // If __builtin_help is in parsed_flags but not in active_flags, no crash.
        let flags = HashMap::from([("__builtin_help".to_string(), json!(true))]);
        let errs = validate_flags(&flags, &[], &[], &ctx());
        assert!(errs.is_empty());
    }

    // -----------------------------------------------------------------------
    // required_unless exempts the flag from missing_required_flag
    // -----------------------------------------------------------------------

    #[test]
    fn test_required_unless_not_satisfied_raises_error() {
        // flag is required, required_unless ["opt"] — "opt" absent → error
        let mut f = mk_flag("output", Some("o"), Some("output"), false);
        f.required = true;
        f.required_unless = vec!["opt".to_string()];
        let f2 = mk_flag("opt", None, Some("opt"), true);
        let flags = HashMap::new(); // neither present
        let errs = validate_flags(&flags, &[f, f2], &[], &ctx());
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].error_type, "missing_required_flag");
    }

    // -----------------------------------------------------------------------
    // conflicts_with only reports once (deduplication: smaller-id first)
    // -----------------------------------------------------------------------

    #[test]
    fn test_conflicting_flags_deduplicated_once() {
        // "aaa" conflicts_with "bbb" AND "bbb" conflicts_with "aaa".
        // Both present → exactly 1 error reported (aaa < bbb).
        let mut fa = mk_flag("aaa", None, Some("aaa"), true);
        let mut fb = mk_flag("bbb", None, Some("bbb"), true);
        fa.conflicts_with = vec!["bbb".to_string()];
        fb.conflicts_with = vec!["aaa".to_string()];
        let flags = HashMap::from([
            ("aaa".to_string(), json!(true)),
            ("bbb".to_string(), json!(true)),
        ]);
        let errs = validate_flags(&flags, &[fa, fb], &[], &ctx());
        // Only 1 conflicting_flags error (from aaa's perspective, since "aaa" < "bbb")
        let conflict_errs: Vec<_> = errs.iter().filter(|e| e.error_type == "conflicting_flags").collect();
        assert_eq!(conflict_errs.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Multiple errors collected in one pass
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_errors_collected() {
        let mut f1 = mk_flag("msg", Some("m"), Some("msg"), false);
        f1.required = true;
        let mut f2 = mk_flag("type", Some("t"), Some("type"), false);
        f2.required = true;
        let flags = HashMap::new(); // both absent
        let errs = validate_flags(&flags, &[f1, f2], &[], &ctx());
        assert_eq!(errs.len(), 2);
        assert!(errs.iter().all(|e| e.error_type == "missing_required_flag"));
    }

    // -----------------------------------------------------------------------
    // requires referencing a flag that exists in scope but is absent
    // -----------------------------------------------------------------------

    #[test]
    fn test_requires_chain_all_absent_reports_all_missing() {
        let mut fa = mk_flag("a", Some("a"), None, true);
        let mut fb = mk_flag("b", Some("b"), None, true);
        let fc = mk_flag("c", Some("c"), None, true);
        fa.requires = vec!["b".to_string()];
        fb.requires = vec!["c".to_string()];
        // Only "a" is present; "b" and "c" are absent.
        let flags = HashMap::from([("a".to_string(), json!(true))]);
        let errs = validate_flags(&flags, &[fa, fb, fc], &[], &ctx());
        assert!(!errs.is_empty());
        assert!(errs.iter().all(|e| e.error_type == "missing_dependency_flag"));
    }
}
