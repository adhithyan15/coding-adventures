//! # The expected-pass list
//!
//! `expected/<level>.txt` names every result that passes at that level, one id
//! per line, sorted (V8C09 §4). It is the gate:
//!
//! - a listed id that no longer passes is a **regression**, and the run fails;
//! - an id that passes but is not listed is a **new pass**, reported, and added
//!   by `--update`, so the growth is visible in review;
//! - the list is never edited to hide a failure. It may only grow.
//!
//! Sorting and one id per line keep concurrent PRs from conflicting on it.

use std::collections::BTreeSet;

/// Read a list: one id per line; blank lines and `#` comments ignored.
pub fn parse_expected(text: &str) -> BTreeSet<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Write a list back, sorted, with a header saying what it is.
pub fn render_expected(level: &str, revision: &str, passes: &BTreeSet<String>) -> String {
    let mut out = format!(
        "# test262 results that pass at the `{level}` level (V8C09 §4).\n\
         # Measured at test262 {revision}. This list may only grow: a listed id\n\
         # that stops passing fails the gate. Regenerate with --update.\n"
    );
    for id in passes {
        out.push_str(id);
        out.push('\n');
    }
    out
}

/// What changed between the list and a run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    /// Listed, but did not pass this run.
    pub regressions: Vec<String>,
    /// Passed this run, but not listed.
    pub new_passes: Vec<String>,
}

impl Comparison {
    pub fn is_regression_free(&self) -> bool {
        self.regressions.is_empty()
    }
}

/// Compare the list with this run's passes. `considered` is every id the run
/// actually judged: a listed id outside it (filtered out by a path argument)
/// is neither a regression nor a pass.
pub fn compare(
    expected: &BTreeSet<String>,
    passes: &BTreeSet<String>,
    considered: &BTreeSet<String>,
) -> Comparison {
    Comparison {
        regressions: expected
            .iter()
            .filter(|id| considered.contains(*id) && !passes.contains(*id))
            .cloned()
            .collect(),
        new_passes: passes.difference(expected).cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn reads_ids_and_ignores_comments() {
        assert_eq!(parse_expected("# header\n\na.js#strict\n b.js \n"), set(&["a.js#strict", "b.js"]));
    }

    #[test]
    fn a_listed_failure_is_a_regression_and_a_new_pass_is_reported() {
        let expected = set(&["a", "b"]);
        let passes = set(&["a", "c"]);
        let considered = set(&["a", "b", "c"]);
        let comparison = compare(&expected, &passes, &considered);
        assert_eq!(comparison.regressions, vec!["b"]);
        assert_eq!(comparison.new_passes, vec!["c"]);
        assert!(!comparison.is_regression_free());
    }

    #[test]
    fn a_filtered_out_id_is_not_a_regression() {
        let comparison = compare(&set(&["a", "b"]), &set(&["a"]), &set(&["a"]));
        assert!(comparison.is_regression_free());
    }

    #[test]
    fn rendering_is_sorted_and_round_trips() {
        let passes = set(&["b", "a"]);
        let text = render_expected("parse", "abc123", &passes);
        assert!(text.contains("abc123"));
        assert_eq!(parse_expected(&text), passes);
        assert!(text.find("\na\n").unwrap() < text.find("\nb\n").unwrap());
    }
}
