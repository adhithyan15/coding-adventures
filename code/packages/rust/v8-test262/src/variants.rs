//! # One file, one or two runs
//!
//! Most test262 tests must behave the same in sloppy and strict code, so a
//! runner executes them **twice**: once as written, once with `"use strict";`
//! prepended (V8C09 §2). Flags narrow that:
//!
//! | flags | runs |
//! |---|---|
//! | none | sloppy **and** strict |
//! | `onlyStrict` | strict |
//! | `noStrict` | sloppy |
//! | `raw` | as written, no harness, no prefix |
//! | `module` | as a module (always strict) |
//!
//! Each run is a separate result with its own id, so the expected-pass list
//! can say "passes strict, fails sloppy" precisely.

use crate::header::{Flag, Metadata};

/// How one run of a test is prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variant {
    /// Script goal, sloppy mode, harness prepended.
    Sloppy,
    /// Script goal, `"use strict";` first, harness prepended.
    Strict,
    /// Exactly the file's text.
    Raw,
    /// Module goal.
    Module,
}

impl Variant {
    /// The suffix on a result id: `path#strict`, `path#sloppy`. Raw and module
    /// runs are the only run of their file, so their id is the bare path.
    pub fn id_suffix(self) -> &'static str {
        match self {
            Variant::Sloppy => "#sloppy",
            Variant::Strict => "#strict",
            Variant::Raw | Variant::Module => "",
        }
    }

    /// Whether the harness includes (`assert.js`, `sta.js`, the `includes:`)
    /// are part of this run.
    pub fn uses_harness(self) -> bool {
        !matches!(self, Variant::Raw)
    }
}

/// The runs a test's flags call for, in a stable order.
pub fn variants(metadata: &Metadata) -> Vec<Variant> {
    if metadata.has_flag(&Flag::Module) {
        return vec![Variant::Module];
    }
    if metadata.has_flag(&Flag::Raw) {
        return vec![Variant::Raw];
    }
    match (
        metadata.has_flag(&Flag::OnlyStrict),
        metadata.has_flag(&Flag::NoStrict),
    ) {
        (true, false) => vec![Variant::Strict],
        (false, true) => vec![Variant::Sloppy],
        // Both flags together is a malformed header upstream would reject; run
        // neither variant rather than invent one. The runner reports it.
        (true, true) => Vec::new(),
        (false, false) => vec![Variant::Sloppy, Variant::Strict],
    }
}

/// A result's id: the path under `test/`, plus the variant suffix.
pub fn result_id(path_under_test: &str, variant: Variant) -> String {
    format!("{path_under_test}{}", variant.id_suffix())
}

/// The harness files a run prepends, in order: `assert.js` and `sta.js`
/// always (unless `raw`), then the header's `includes`, then `doneprintHandle.js`
/// for `async` tests. Duplicates keep their first position.
pub fn harness_files(metadata: &Metadata, variant: Variant) -> Vec<String> {
    if !variant.uses_harness() {
        return Vec::new();
    }
    let mut files = vec!["assert.js".to_string(), "sta.js".to_string()];
    if metadata.has_flag(&Flag::Async) {
        files.push("doneprintHandle.js".to_string());
    }
    for include in &metadata.includes {
        if !files.contains(include) {
            files.push(include.clone());
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::parse_header;

    fn meta(flags: &str) -> Metadata {
        parse_header(&format!("/*---\nflags: [{flags}]\nincludes: [compareArray.js, sta.js]\n---*/")).unwrap()
    }

    #[test]
    fn unflagged_tests_run_twice() {
        assert_eq!(variants(&meta("")), vec![Variant::Sloppy, Variant::Strict]);
        assert_eq!(result_id("language/a.js", Variant::Strict), "language/a.js#strict");
    }

    #[test]
    fn flags_narrow_the_runs() {
        assert_eq!(variants(&meta("onlyStrict")), vec![Variant::Strict]);
        assert_eq!(variants(&meta("noStrict")), vec![Variant::Sloppy]);
        assert_eq!(variants(&meta("raw")), vec![Variant::Raw]);
        assert_eq!(variants(&meta("module, onlyStrict")), vec![Variant::Module]);
        assert!(variants(&meta("onlyStrict, noStrict")).is_empty());
        assert_eq!(result_id("language/m.js", Variant::Module), "language/m.js");
    }

    #[test]
    fn harness_is_assert_sta_then_includes_without_duplicates() {
        assert_eq!(
            harness_files(&meta(""), Variant::Sloppy),
            vec!["assert.js", "sta.js", "compareArray.js"]
        );
        assert!(harness_files(&meta("raw"), Variant::Raw).is_empty());
        assert_eq!(
            harness_files(&meta("async"), Variant::Strict),
            vec!["assert.js", "sta.js", "doneprintHandle.js", "compareArray.js"]
        );
    }
}
