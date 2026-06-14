//! Core data types for vm-type-suggestions.
//!
//! Two main types:
//!
//! - [`Confidence`] — three-level enum describing how certain the suggestion is.
//!   Only `Certain` suggestions are shown as actionable advice.  `Mixed` and
//!   `NoData` are reported so the developer understands why no suggestion was
//!   made.
//!
//! - [`ParamSuggestion`] — one parameter in one function.  Bundles the profiler
//!   observation (`call_count`, `observed_type`) with the confidence level and a
//!   ready-to-use suggestion string like `"declare 'n: u8'"`.
//!
//! - [`SuggestionReport`] — the top-level result of [`crate::suggest`].  Provides
//!   [`SuggestionReport::actionable`] (only `Certain` suggestions),
//!   [`SuggestionReport::by_function`] (grouped view),
//!   [`SuggestionReport::format_text`] (terminal), and
//!   [`SuggestionReport::format_json`] (tooling).

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// How certain is the type suggestion?
///
/// | Variant | Meaning |
/// |---|---|
/// | `Certain` | Exactly one concrete type observed on every call — safe to annotate |
/// | `Mixed` | Multiple types seen (`"polymorphic"` in IIR) — annotation would be wrong |
/// | `NoData` | Profiler never reached this parameter — no advice possible |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// The profiler observed exactly one concrete type on every call.
    Certain,
    /// The profiler saw multiple different types (polymorphic).
    Mixed,
    /// The profiler never reached this parameter.
    NoData,
}

impl Confidence {
    /// Return the JSON serialisation name.
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Certain => "certain",
            Confidence::Mixed   => "mixed",
            Confidence::NoData  => "no_data",
        }
    }
}

// ---------------------------------------------------------------------------
// ParamSuggestion
// ---------------------------------------------------------------------------

/// A type suggestion for one function parameter.
///
/// # Fields
///
/// - `function` — name of the `IIRFunction` containing this parameter.
/// - `param_name` — the parameter name as declared in `IIRFunction.params`.
/// - `param_index` — 0-based position of the parameter in the function signature.
/// - `observed_type` — the IIR type string the profiler observed (e.g. `"u8"`),
///   `"polymorphic"` for mixed types, or `None` for no data.
/// - `call_count` — how many times the parameter-loading instruction was profiled.
/// - `confidence` — `Certain` / `Mixed` / `NoData`.
/// - `suggestion` — human-readable advice string (e.g. `"declare 'n: u8'"`), or
///   `None` when no safe suggestion can be made.
#[derive(Debug, Clone)]
pub struct ParamSuggestion {
    /// Name of the function containing this parameter.
    pub function: String,
    /// Parameter name as declared in `IIRFunction.params`.
    pub param_name: String,
    /// 0-based position of the parameter in the function signature.
    pub param_index: usize,
    /// IIR type string the profiler observed, `"polymorphic"`, or `None`.
    pub observed_type: Option<String>,
    /// How many times the parameter-loading instruction was profiled.
    pub call_count: u64,
    /// Confidence level.
    pub confidence: Confidence,
    /// Human-readable advice string, or `None` for Mixed/NoData.
    pub suggestion: Option<String>,
}

// ---------------------------------------------------------------------------
// SuggestionReport
// ---------------------------------------------------------------------------

/// Top-level output of [`crate::suggest`].
///
/// # Fields
///
/// - `program_name` — friendly label for the output.
/// - `total_calls` — sum of `call_count` across all `Certain` suggestions.
/// - `suggestions` — all `ParamSuggestion` entries (including Mixed and NoData).
#[derive(Debug, Clone)]
pub struct SuggestionReport {
    /// Friendly label for the output.
    pub program_name: String,
    /// Sum of `call_count` across all `Certain` suggestions.
    pub total_calls: u64,
    /// All `ParamSuggestion` entries.
    pub suggestions: Vec<ParamSuggestion>,
}

impl SuggestionReport {
    /// Return only `Certain` suggestions — the ones to act on.
    pub fn actionable(&self) -> Vec<&ParamSuggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.confidence == Confidence::Certain)
            .collect()
    }

    /// Group suggestions by function name, preserving insertion order.
    ///
    /// Returns a `Vec<(function_name, suggestions)>` in the order the
    /// functions were first seen.
    pub fn by_function(&self) -> Vec<(&str, Vec<&ParamSuggestion>)> {
        let mut order: Vec<&str> = Vec::new();
        let mut map: std::collections::HashMap<&str, Vec<&ParamSuggestion>> =
            std::collections::HashMap::new();
        for s in &self.suggestions {
            let key = s.function.as_str();
            if !map.contains_key(key) {
                order.push(key);
            }
            map.entry(key).or_default().push(s);
        }
        order.into_iter().map(|k| (k, map.remove(k).unwrap())).collect()
    }

    /// Render the report as a human-readable string.
    ///
    /// # Example output
    ///
    /// ```text
    /// VM Type Suggestions — fibonacci (1,048,576 total calls)
    /// ════════════════════════════════════════════════════════
    ///
    /// Function: fibonacci — 1,048,576 calls
    ///   ✅ 'n' (arg 0): always u8
    ///      → declare 'n: u8'
    ///
    /// Summary: 1 of 1 untyped parameters can be annotated.
    /// ```
    pub fn format_text(&self) -> String {
        let mut lines = Vec::new();
        let title = format!(
            "VM Type Suggestions — {} ({} total calls)",
            self.program_name,
            format_number(self.total_calls),
        );
        lines.push(title.clone());
        lines.push("═".repeat(title.chars().count()));

        let grouped = self.by_function();

        if grouped.is_empty() {
            lines.push(String::new());
            lines.push("✅ No untyped parameters found — everything is already typed.".into());
            return lines.join("\n");
        }

        for (fn_name, params) in &grouped {
            lines.push(String::new());
            let call_count = params.iter().find(|p| p.call_count > 0).map_or(0, |p| p.call_count);
            lines.push(format!("Function: {fn_name} — {} calls", format_number(call_count)));

            for p in params {
                match p.confidence {
                    Confidence::Certain => {
                        let ty = p.observed_type.as_deref().unwrap_or("?");
                        lines.push(format!(
                            "  ✅ '{}' (arg {}): always {ty}",
                            p.param_name, p.param_index,
                        ));
                        if let Some(sug) = &p.suggestion {
                            lines.push(format!("     → {sug}"));
                        }
                    }
                    Confidence::Mixed => {
                        lines.push(format!(
                            "  ⚠️  '{}' (arg {}): mixed types observed (polymorphic)",
                            p.param_name, p.param_index,
                        ));
                        lines.push("     → cannot suggest; consider typed overloads instead".into());
                    }
                    Confidence::NoData => {
                        lines.push(format!(
                            "  ℹ️  '{}' (arg {}): no profiling data",
                            p.param_name, p.param_index,
                        ));
                    }
                }
            }
        }

        lines.push(String::new());
        let n_actionable = self.actionable().len();
        let n_total = self.suggestions.len();
        let noun = if n_total == 1 { "parameter" } else { "parameters" };
        lines.push(format!(
            "Summary: {n_actionable} of {n_total} untyped {noun} can be annotated.",
        ));

        lines.join("\n")
    }

    /// Render the report as a pretty-printed JSON string.
    pub fn format_json(&self) -> String {
        let suggestions_json: Vec<String> = self.suggestions.iter().map(|s| {
            let observed = match &s.observed_type {
                Some(t) => format!("{t:?}"),
                None    => "null".into(),
            };
            let suggestion = match &s.suggestion {
                Some(sg) => format!("{sg:?}"),
                None     => "null".into(),
            };
            format!(
                concat!(
                    "    {{\n",
                    "      \"function\": {:?},\n",
                    "      \"param_name\": {:?},\n",
                    "      \"param_index\": {},\n",
                    "      \"observed_type\": {},\n",
                    "      \"call_count\": {},\n",
                    "      \"confidence\": {:?},\n",
                    "      \"suggestion\": {}\n",
                    "    }}",
                ),
                s.function,
                s.param_name,
                s.param_index,
                observed,
                s.call_count,
                s.confidence.as_str(),
                suggestion,
            )
        }).collect();

        format!(
            "{{\n  \"program_name\": {:?},\n  \"total_calls\": {},\n  \"suggestions\": [\n{}\n  ]\n}}",
            self.program_name,
            self.total_calls,
            suggestions_json.join(",\n"),
        )
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(c);
    }
    out.chars().rev().collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_report(suggestions: Vec<ParamSuggestion>) -> SuggestionReport {
        SuggestionReport {
            program_name: "test".into(),
            total_calls: suggestions.iter().filter(|s| s.confidence == Confidence::Certain).map(|s| s.call_count).sum(),
            suggestions,
        }
    }

    fn certain(fn_: &str, param: &str, idx: usize, ty: &str, calls: u64) -> ParamSuggestion {
        ParamSuggestion {
            function: fn_.into(),
            param_name: param.into(),
            param_index: idx,
            observed_type: Some(ty.into()),
            call_count: calls,
            confidence: Confidence::Certain,
            suggestion: Some(format!("declare '{param}: {ty}'")),
        }
    }

    fn no_data(fn_: &str, param: &str, idx: usize) -> ParamSuggestion {
        ParamSuggestion {
            function: fn_.into(),
            param_name: param.into(),
            param_index: idx,
            observed_type: None,
            call_count: 0,
            confidence: Confidence::NoData,
            suggestion: None,
        }
    }

    #[test]
    fn confidence_as_str() {
        assert_eq!(Confidence::Certain.as_str(), "certain");
        assert_eq!(Confidence::Mixed.as_str(), "mixed");
        assert_eq!(Confidence::NoData.as_str(), "no_data");
    }

    #[test]
    fn actionable_filters_certain_only() {
        let report = make_report(vec![
            certain("f", "a", 0, "u8", 100),
            no_data("f", "b", 1),
        ]);
        let act = report.actionable();
        assert_eq!(act.len(), 1);
        assert_eq!(act[0].param_name, "a");
    }

    #[test]
    fn by_function_groups_correctly() {
        let report = make_report(vec![
            certain("f", "a", 0, "u8", 100),
            certain("g", "x", 0, "i32", 200),
            certain("f", "b", 1, "u8", 100),
        ]);
        let grouped = report.by_function();
        assert_eq!(grouped.len(), 2);
        // "f" first (appeared first)
        assert_eq!(grouped[0].0, "f");
        assert_eq!(grouped[0].1.len(), 2);
    }

    #[test]
    fn format_text_empty_program() {
        let report = SuggestionReport {
            program_name: "empty".into(),
            total_calls: 0,
            suggestions: vec![],
        };
        let text = report.format_text();
        assert!(text.contains("No untyped parameters"));
    }

    #[test]
    fn format_text_with_certain_suggestion() {
        let report = make_report(vec![certain("add", "n", 0, "u8", 1_000_000)]);
        let text = report.format_text();
        assert!(text.contains("add"));
        assert!(text.contains("declare 'n: u8'"));
        assert!(text.contains("✅"));
    }

    #[test]
    fn format_text_summary_line() {
        let report = make_report(vec![certain("f", "x", 0, "u8", 10)]);
        let text = report.format_text();
        assert!(text.contains("1 of 1"));
    }

    #[test]
    fn format_json_contains_fields() {
        let report = make_report(vec![certain("f", "x", 0, "u8", 5)]);
        let json = report.format_json();
        assert!(json.contains("\"program_name\""));
        assert!(json.contains("\"total_calls\""));
        assert!(json.contains("\"suggestions\""));
        assert!(json.contains("certain"));
    }

    #[test]
    fn format_number_no_comma_below_1000() {
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn format_number_with_comma() {
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(1_000_000), "1,000,000");
    }
}
