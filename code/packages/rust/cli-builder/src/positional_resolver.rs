// positional_resolver.rs -- Assign positional tokens to argument slots
// =======================================================================
//
// After scanning produces a flat list of positional token strings, this module
// maps them to the named argument definitions for the resolved command node.
//
// # The core algorithm (§6.4.1)
//
// There are two cases:
//
// Case A — No variadic argument:
//   One-to-one assignment in order. Extra tokens = too_many_arguments error.
//   Missing required tokens = missing_required_argument error.
//
// Case B — One variadic argument:
//   The positional list is partitioned into three regions:
//
//     [leading_defs | variadic_def (greedy) | trailing_defs]
//
//   "Leading" arguments are those before the variadic in the spec array.
//   "Trailing" arguments are those after the variadic. They are consumed
//   from the END of the token list, so the variadic gets everything in between.
//
//   This handles the classic `cp SOURCE... DEST` pattern naturally:
//
//     cp a.txt b.txt c.txt /dest/
//       leading_defs  = []            (variadic is first)
//       variadic_def  = "source"
//       trailing_defs = ["dest"]      (consumed from the end)
//       dest     = positional_tokens[3]       = "/dest/"
//       variadic = positional_tokens[0..=2]   = ["a.txt","b.txt","c.txt"]

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::errors::ParseError;
use crate::types::ArgumentDef;

/// Result from positional resolution.
pub struct PositionalResolution {
    /// Successfully assigned argument values (id → value).
    pub assignments: HashMap<String, Value>,
    /// Any errors detected during resolution.
    pub errors: Vec<ParseError>,
}

/// Resolve positional tokens against argument definitions.
///
/// # Arguments
///
/// * `tokens` — flat list of positional token strings (in the order they appeared in argv).
/// * `arg_defs` — argument definitions for the resolved command node.
/// * `parsed_flags` — the flags already parsed, used for `required_unless_flag` checks.
/// * `command_path` — the command path for error context.
pub fn resolve_positionals(
    tokens: &[String],
    arg_defs: &[ArgumentDef],
    parsed_flags: &HashMap<String, Value>,
    command_path: &[String],
) -> PositionalResolution {
    let mut assignments: HashMap<String, Value> = HashMap::new();
    let mut errors: Vec<ParseError> = Vec::new();

    // Split arg_defs into "effective" (participate in token assignment) and
    // "exempt" (optional due to required_unless_flag being satisfied).
    //
    // Exempt args do not consume positional tokens. They receive their default
    // value (or null). This mirrors the TypeScript implementation's filtering.
    let is_exempt = |def: &ArgumentDef| -> bool {
        !def.required_unless_flag.is_empty()
            && def.required_unless_flag.iter().any(|fid| {
                if let Some(v) = parsed_flags.get(fid) {
                    // Present and truthy (non-false, non-empty-array, non-null)
                    match v {
                        Value::Bool(false) | Value::Null => false,
                        Value::Array(arr) => !arr.is_empty(),
                        _ => true,
                    }
                } else {
                    false
                }
            })
    };

    let effective_defs: Vec<&ArgumentDef> = arg_defs.iter().filter(|d| !is_exempt(d)).collect();
    let exempt_defs: Vec<&ArgumentDef> = arg_defs.iter().filter(|d| is_exempt(d)).collect();

    // Assign default (or null) for all exempt args upfront.
    for def in &exempt_defs {
        let default_val = def.default.clone().unwrap_or(Value::Null);
        assignments.insert(def.id.clone(), default_val);
    }

    // Find the index of the first variadic argument in the effective defs.
    let variadic_idx = effective_defs.iter().position(|a| a.variadic);

    match variadic_idx {
        None => {
            // Case A: no variadic — strict one-to-one assignment.
            resolve_no_variadic(
                tokens,
                &effective_defs,
                parsed_flags,
                command_path,
                &mut assignments,
                &mut errors,
            );
        }
        Some(vi) => {
            // Case B: one variadic argument at index `vi`.
            resolve_with_variadic_refs(
                tokens,
                &effective_defs,
                vi,
                parsed_flags,
                command_path,
                &mut assignments,
                &mut errors,
            );
        }
    }

    // Populate defaults for any argument that wasn't assigned and is optional.
    for def in arg_defs {
        if assignments.contains_key(&def.id) {
            continue;
        }
        if !def.required {
            // Use the default value if provided, otherwise null.
            let default_val = def.default.clone().unwrap_or(Value::Null);
            // Variadic defaults wrap in an array if needed.
            if def.variadic {
                match default_val {
                    Value::Array(_) => assignments.insert(def.id.clone(), default_val),
                    Value::Null => assignments.insert(def.id.clone(), json!([])),
                    other => assignments.insert(def.id.clone(), json!([other])),
                };
            } else {
                assignments.insert(def.id.clone(), default_val);
            }
        }
    }

    PositionalResolution { assignments, errors }
}

// ---------------------------------------------------------------------------
// Case A: no variadic
// ---------------------------------------------------------------------------

fn resolve_no_variadic(
    tokens: &[String],
    arg_defs: &[&ArgumentDef],
    parsed_flags: &HashMap<String, Value>,
    command_path: &[String],
    assignments: &mut HashMap<String, Value>,
    errors: &mut Vec<ParseError>,
) {
    for (i, def) in arg_defs.iter().enumerate() {
        if i < tokens.len() {
            let val = coerce_value(&tokens[i], &def.arg_type, &def.enum_values);
            match val {
                Ok(v) => { assignments.insert(def.id.clone(), v); }
                Err(e) => errors.push(e),
            }
        } else {
            // Token not present for this argument slot.
            if def.required {
                errors.push(ParseError::new(
                    "missing_required_argument",
                    format!("Missing required argument: <{}>", def.name),
                    command_path.to_vec(),
                ));
            }
            // Missing optional arguments are populated with defaults in the caller.
        }
    }

    // Extra tokens after all argument slots are filled.
    if tokens.len() > arg_defs.len() {
        let extra = &tokens[arg_defs.len()..];
        errors.push(ParseError::new(
            "too_many_arguments",
            format!(
                "Expected at most {} argument(s), got {} (extra: {:?})",
                arg_defs.len(),
                tokens.len(),
                extra
            ),
            command_path.to_vec(),
        ));
    }
}

// ---------------------------------------------------------------------------
// Case B: one variadic argument
// ---------------------------------------------------------------------------

/// Version of `resolve_with_variadic` that accepts a slice of references.
/// Used by the public `resolve_positionals` function after filtering exempt args.
fn resolve_with_variadic_refs(
    tokens: &[String],
    arg_defs: &[&ArgumentDef],
    variadic_idx: usize,
    parsed_flags: &HashMap<String, Value>,
    command_path: &[String],
    assignments: &mut HashMap<String, Value>,
    errors: &mut Vec<ParseError>,
) {
    let variadic_def = arg_defs[variadic_idx];
    let leading_defs = &arg_defs[..variadic_idx];
    let trailing_defs = &arg_defs[variadic_idx + 1..];

    let n = tokens.len();
    let n_leading = leading_defs.len();
    let n_trailing = trailing_defs.len();

    // Check if there's a shortage: not enough tokens for variadic min + trailing.
    let has_shortage = n_trailing > 0
        && n < n_trailing + variadic_def.variadic_min;

    // Assign leading arguments (from the start of tokens).
    for (i, def) in leading_defs.iter().enumerate() {
        if i < n {
            match coerce_value(&tokens[i], &def.arg_type, &def.enum_values) {
                Ok(v) => { assignments.insert(def.id.clone(), v); }
                Err(e) => errors.push(e),
            }
        } else {
            if def.required {
                errors.push(ParseError::new(
                    "missing_required_argument",
                    format!("Missing required argument: <{}>", def.name),
                    command_path.to_vec(),
                ));
            }
        }
    }

    // Assign trailing arguments (from the end of tokens).
    // When there's a shortage, push trailing start to end so trailing gets nothing.
    let trailing_start = if has_shortage {
        n
    } else if n >= n_trailing {
        n - n_trailing
    } else {
        0
    };

    for (i, def) in trailing_defs.iter().enumerate() {
        let token_idx = trailing_start + i;
        if token_idx < n && token_idx >= n_leading {
            match coerce_value(&tokens[token_idx], &def.arg_type, &def.enum_values) {
                Ok(v) => { assignments.insert(def.id.clone(), v); }
                Err(e) => errors.push(e),
            }
        } else {
            if def.required {
                errors.push(ParseError::new(
                    "missing_required_argument",
                    format!("Missing required argument: <{}>", def.name),
                    command_path.to_vec(),
                ));
            }
        }
    }

    // The variadic gets everything in between leading and trailing.
    let variadic_end = if has_shortage { n } else { trailing_start.max(n_leading) };
    // Guard against n_leading or variadic_end exceeding token slice length.
    let safe_start = n_leading.min(n);
    let safe_end = variadic_end.min(n);
    let variadic_tokens = if safe_start <= safe_end {
        &tokens[safe_start..safe_end]
    } else {
        &tokens[0..0]
    };

    let count = variadic_tokens.len();
    let v_min = variadic_def.variadic_min;

    if count < v_min {
        let exempt = variadic_def.required_unless_flag.iter().any(|fid| parsed_flags.contains_key(fid));
        if !exempt {
            errors.push(ParseError::new(
                "too_few_arguments",
                format!(
                    "Expected at least {} <{}>, got {}",
                    v_min, variadic_def.name, count
                ),
                command_path.to_vec(),
            ));
        }
    }

    if let Some(v_max) = variadic_def.variadic_max {
        if count > v_max {
            errors.push(ParseError::new(
                "too_many_arguments",
                format!(
                    "Expected at most {} <{}>, got {}",
                    v_max, variadic_def.name, count
                ),
                command_path.to_vec(),
            ));
        }
    }

    // Coerce each variadic token and collect into an array.
    let mut arr: Vec<Value> = Vec::new();
    for t in variadic_tokens {
        match coerce_value(t, &variadic_def.arg_type, &variadic_def.enum_values) {
            Ok(v) => arr.push(v),
            Err(e) => errors.push(e),
        }
    }

    // When no tokens were consumed by the variadic and there's a default, use it.
    if arr.is_empty() {
        if let Some(ref default_val) = variadic_def.default {
            assignments.insert(variadic_def.id.clone(), default_val.clone());
        } else {
            assignments.insert(variadic_def.id.clone(), Value::Array(arr));
        }
    } else {
        assignments.insert(variadic_def.id.clone(), Value::Array(arr));
    }
}

/// Wrapper that accepts `&[ArgumentDef]` (used by unit tests).
fn resolve_with_variadic(
    tokens: &[String],
    arg_defs: &[ArgumentDef],
    variadic_idx: usize,
    parsed_flags: &HashMap<String, Value>,
    command_path: &[String],
    assignments: &mut HashMap<String, Value>,
    errors: &mut Vec<ParseError>,
) {
    let refs: Vec<&ArgumentDef> = arg_defs.iter().collect();
    resolve_with_variadic_refs(tokens, &refs, variadic_idx, parsed_flags, command_path, assignments, errors);
}

// ---------------------------------------------------------------------------
// Type coercion
// ---------------------------------------------------------------------------

/// Coerce a raw string token to the correct `serde_json::Value` type.
///
/// The spec requires eager coercion (§3): integers and floats must be
/// represented as native numeric values in the output, not strings.
///
/// `file` and `directory` types are validated for existence on the filesystem
/// at parse time. Permission errors are treated as `invalid_value` errors,
/// not crashes.
pub fn coerce_value(raw: &str, type_name: &str, enum_values: &[String]) -> Result<Value, ParseError> {
    match type_name {
        "boolean" => {
            // Boolean positional arguments are unusual but allowed in the spec.
            match raw {
                "true" | "1" | "yes" => Ok(Value::Bool(true)),
                "false" | "0" | "no" => Ok(Value::Bool(false)),
                _ => Err(ParseError::new(
                    "invalid_value",
                    format!("Invalid boolean value: {:?}", raw),
                    vec![],
                )),
            }
        }
        "string" => {
            if raw.is_empty() {
                Err(ParseError::new(
                    "invalid_value",
                    "String value must not be empty".to_string(),
                    vec![],
                ))
            } else {
                Ok(Value::String(raw.to_string()))
            }
        }
        "integer" => raw.parse::<i64>().map(|n| json!(n)).map_err(|_| {
            ParseError::new(
                "invalid_value",
                format!("Invalid integer: {:?}", raw),
                vec![],
            )
        }),
        "float" => raw.parse::<f64>().map(|n| json!(n)).map_err(|_| {
            ParseError::new(
                "invalid_value",
                format!("Invalid float: {:?}", raw),
                vec![],
            )
        }),
        "path" => {
            // Path is syntactically valid if non-empty. Existence is NOT checked.
            if raw.is_empty() {
                Err(ParseError::new(
                    "invalid_value",
                    "Path must not be empty".to_string(),
                    vec![],
                ))
            } else {
                Ok(Value::String(raw.to_string()))
            }
        }
        "file" => {
            // File existence is checked at parse time (§3).
            let meta = std::fs::metadata(raw);
            match meta {
                Ok(m) if m.is_file() => Ok(Value::String(raw.to_string())),
                Ok(_) => Err(ParseError::new(
                    "invalid_value",
                    format!("Expected a file, but {:?} is not a file", raw),
                    vec![],
                )),
                Err(_) => Err(ParseError::new(
                    "invalid_value",
                    format!("File does not exist or is not readable: {:?}", raw),
                    vec![],
                )),
            }
        }
        "directory" => {
            // Directory existence is checked at parse time (§3).
            let meta = std::fs::metadata(raw);
            match meta {
                Ok(m) if m.is_dir() => Ok(Value::String(raw.to_string())),
                Ok(_) => Err(ParseError::new(
                    "invalid_value",
                    format!("Expected a directory, but {:?} is not a directory", raw),
                    vec![],
                )),
                Err(_) => Err(ParseError::new(
                    "invalid_value",
                    format!("Directory does not exist: {:?}", raw),
                    vec![],
                )),
            }
        }
        "enum" => {
            if enum_values.iter().any(|v| v == raw) {
                Ok(Value::String(raw.to_string()))
            } else {
                Err(ParseError::new(
                    "invalid_enum_value",
                    format!(
                        "Invalid value {:?}. Must be one of: {}",
                        raw,
                        enum_values.join(", ")
                    ),
                    vec![],
                ))
            }
        }
        _ => {
            // Unknown types treated as string (forward-compatible).
            Ok(Value::String(raw.to_string()))
        }
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn no_flags() -> HashMap<String, Value> {
        HashMap::new()
    }

    fn mk_arg(id: &str, name: &str, required: bool, variadic: bool, variadic_min: usize) -> ArgumentDef {
        ArgumentDef {
            id: id.to_string(),
            name: name.to_string(),
            description: "".to_string(),
            arg_type: "string".to_string(),
            required,
            variadic,
            variadic_min,
            variadic_max: None,
            default: None,
            enum_values: vec![],
            required_unless_flag: vec![],
        }
    }

    fn mk_path_arg(id: &str, name: &str, required: bool, variadic: bool, variadic_min: usize) -> ArgumentDef {
        let mut a = mk_arg(id, name, required, variadic, variadic_min);
        a.arg_type = "path".to_string();
        a
    }

    fn tokens(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn ctx() -> Vec<String> {
        vec!["prog".to_string()]
    }

    // -----------------------------------------------------------------------
    // Coerce tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_coerce_integer() {
        assert_eq!(coerce_value("42", "integer", &[]).unwrap(), json!(42));
    }

    #[test]
    fn test_coerce_negative_integer() {
        assert_eq!(coerce_value("-7", "integer", &[]).unwrap(), json!(-7));
    }

    #[test]
    fn test_coerce_invalid_integer() {
        assert!(coerce_value("abc", "integer", &[]).is_err());
    }

    #[test]
    fn test_coerce_float() {
        assert_eq!(coerce_value("3.14", "float", &[]).unwrap(), json!(3.14));
    }

    #[test]
    fn test_coerce_string() {
        assert_eq!(coerce_value("hello", "string", &[]).unwrap(), json!("hello"));
    }

    #[test]
    fn test_coerce_empty_string_rejected() {
        assert!(coerce_value("", "string", &[]).is_err());
    }

    #[test]
    fn test_coerce_enum_valid() {
        let enums = vec!["json".to_string(), "csv".to_string()];
        assert_eq!(coerce_value("json", "enum", &enums).unwrap(), json!("json"));
    }

    #[test]
    fn test_coerce_enum_invalid() {
        let enums = vec!["json".to_string(), "csv".to_string()];
        let err = coerce_value("xml", "enum", &enums).unwrap_err();
        assert_eq!(err.error_type, "invalid_enum_value");
    }

    #[test]
    fn test_coerce_path() {
        assert_eq!(coerce_value("/tmp/foo", "path", &[]).unwrap(), json!("/tmp/foo"));
    }

    // -----------------------------------------------------------------------
    // No-variadic resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_required_arg() {
        let defs = vec![mk_arg("file", "FILE", true, false, 1)];
        let r = resolve_positionals(&tokens(&["myfile.txt"]), &defs, &no_flags(), &ctx());
        assert!(r.errors.is_empty());
        assert_eq!(r.assignments["file"], json!("myfile.txt"));
    }

    #[test]
    fn test_missing_required_arg() {
        let defs = vec![mk_arg("file", "FILE", true, false, 1)];
        let r = resolve_positionals(&tokens(&[]), &defs, &no_flags(), &ctx());
        assert_eq!(r.errors.len(), 1);
        assert_eq!(r.errors[0].error_type, "missing_required_argument");
    }

    #[test]
    fn test_too_many_args() {
        let defs = vec![mk_arg("file", "FILE", true, false, 1)];
        let r = resolve_positionals(&tokens(&["a", "b"]), &defs, &no_flags(), &ctx());
        assert_eq!(r.errors.len(), 1);
        assert_eq!(r.errors[0].error_type, "too_many_arguments");
    }

    #[test]
    fn test_optional_arg_absent_uses_null() {
        let mut def = mk_arg("file", "FILE", false, false, 0);
        def.required = false;
        let defs = vec![def];
        let r = resolve_positionals(&tokens(&[]), &defs, &no_flags(), &ctx());
        assert!(r.errors.is_empty());
        assert_eq!(r.assignments["file"], Value::Null);
    }

    #[test]
    fn test_optional_arg_uses_default() {
        let mut def = mk_arg("path", "PATH", false, false, 0);
        def.required = false;
        def.default = Some(json!("."));
        let defs = vec![def];
        let r = resolve_positionals(&tokens(&[]), &defs, &no_flags(), &ctx());
        assert!(r.errors.is_empty());
        assert_eq!(r.assignments["path"], json!("."));
    }

    // -----------------------------------------------------------------------
    // Variadic resolution tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cp_pattern_source_variadic_dest_trailing() {
        // cp SOURCE... DEST
        let defs = vec![
            mk_path_arg("source", "SOURCE", true, true, 1),
            mk_path_arg("dest",   "DEST",   true, false, 1),
        ];
        let r = resolve_positionals(
            &tokens(&["a.txt", "b.txt", "c.txt", "/dest/"]),
            &defs,
            &no_flags(),
            &ctx(),
        );
        assert!(r.errors.is_empty(), "errors: {:?}", r.errors);
        assert_eq!(r.assignments["source"], json!(["a.txt", "b.txt", "c.txt"]));
        assert_eq!(r.assignments["dest"], json!("/dest/"));
    }

    #[test]
    fn test_cp_single_source_single_dest() {
        let defs = vec![
            mk_path_arg("source", "SOURCE", true, true, 1),
            mk_path_arg("dest",   "DEST",   true, false, 1),
        ];
        let r = resolve_positionals(&tokens(&["a.txt", "/tmp/"]), &defs, &no_flags(), &ctx());
        assert!(r.errors.is_empty(), "errors: {:?}", r.errors);
        assert_eq!(r.assignments["source"], json!(["a.txt"]));
        assert_eq!(r.assignments["dest"], json!("/tmp/"));
    }

    #[test]
    fn test_cp_missing_dest_error() {
        let defs = vec![
            mk_path_arg("source", "SOURCE", true, true, 1),
            mk_path_arg("dest",   "DEST",   true, false, 1),
        ];
        let r = resolve_positionals(&tokens(&["a.txt"]), &defs, &no_flags(), &ctx());
        // source gets "a.txt", dest is missing → error
        assert!(!r.errors.is_empty());
    }

    #[test]
    fn test_variadic_min_too_few() {
        let mut def = mk_arg("files", "FILE", true, true, 2);
        def.variadic_min = 2;
        let defs = vec![def];
        let r = resolve_positionals(&tokens(&["one.txt"]), &defs, &no_flags(), &ctx());
        assert!(!r.errors.is_empty());
        assert_eq!(r.errors[0].error_type, "too_few_arguments");
    }

    #[test]
    fn test_variadic_max_exceeded() {
        let mut def = mk_arg("files", "FILE", true, true, 1);
        def.variadic_max = Some(2);
        let defs = vec![def];
        let r = resolve_positionals(&tokens(&["a", "b", "c"]), &defs, &no_flags(), &ctx());
        assert!(!r.errors.is_empty());
        assert_eq!(r.errors[0].error_type, "too_many_arguments");
    }

    #[test]
    fn test_required_unless_flag_exempts_arg() {
        let mut def = mk_arg("pattern", "PATTERN", true, false, 1);
        def.required_unless_flag = vec!["regexp".to_string()];
        let defs = vec![def];
        // No tokens, but the "regexp" flag is present → no error.
        let flags = HashMap::from([("regexp".to_string(), json!(["foo"]))]);
        let r = resolve_positionals(&tokens(&[]), &defs, &flags, &ctx());
        assert!(r.errors.is_empty(), "errors: {:?}", r.errors);
    }

    #[test]
    fn test_optional_variadic_empty() {
        let mut def = mk_arg("files", "FILE", false, true, 0);
        def.required = false;
        def.variadic_min = 0;
        let defs = vec![def];
        let r = resolve_positionals(&tokens(&[]), &defs, &no_flags(), &ctx());
        assert!(r.errors.is_empty());
        assert_eq!(r.assignments["files"], json!([]));
    }
}
