//! Shared readers for the LANG84 integration tests (`release_subsets.rs`,
//! `release_sweep.rs`): the Perl release inventory and the `pl-int-print`
//! corpus, both read from `code/grammars/perl/`.
//!
//! ```text
//!   code/grammars/perl/
//!     releases.csv                 one row per release; columns read by NAME
//!     subsets.csv                  pl-int-print -> corpus/pl-int-print
//!     corpus/pl-int-print/
//!       add.pl  add.expect  add.cite                    a positive case
//!       zero_and_negation.reject@perl_nv               ... refused under one profile
//!       reject_octal.pl  reject_octal.reject  ...cite   a negative case
//!     release_classes.csv          written by the sweep
//! ```
//!
//! Every file here is a trusted repository input, but the readers still fail
//! loudly on anything malformed (a row with the wrong field count, a case with
//! two outcomes) instead of guessing, because a silently skipped row would
//! make a stage claim vacuous.

#![allow(dead_code)] // each test binary uses a different part of this module

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use perl_iir_compiler::SUBSET;

/// `code/grammars/perl`, from this crate's manifest directory.
pub fn perl_grammars() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/perl")
}

/// Rows of a CSV whose leading `#` lines are provenance comments, as maps
/// keyed by header name (lesson: read the header, never assume a column
/// position). None of these files quote a cell, so a plain split is exact;
/// a row whose field count differs from the header's fails the test.
pub fn csv_rows(path: &Path) -> Vec<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = text.lines().filter(|line| !line.starts_with('#') && !line.is_empty());
    let header: Vec<&str> = lines.next().expect("a header row").split(',').collect();
    lines
        .map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            assert_eq!(cells.len(), header.len(), "{}: {line}", path.display());
            header.iter().zip(cells).map(|(k, v)| (k.to_string(), v.to_string())).collect()
        })
        .collect()
}

/// Every row of `releases.csv`, in inventory order.
pub fn inventory() -> Vec<BTreeMap<String, String>> {
    csv_rows(&perl_grammars().join("releases.csv"))
}

/// Does a `;`-separated status cell (`iir_status`, `backend_status`) list
/// the subset?
pub fn lists_subset(cell: &str) -> bool {
    cell.split(';').any(|name| name == SUBSET)
}

/// What a case must do under one profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Compile, and print exactly this (the `.expect` text minus its final
    /// LF: Perl's `print` subset never writes a newline).
    Prints(String),
    /// Be refused with this LANG84 C6 category.
    Rejected(String),
}

/// One corpus case: `<name>.pl` plus `.expect` (positive) or `.reject`
/// (negative), and per-profile overrides.
#[derive(Debug)]
pub struct Case {
    /// The file stem, also the `case` in `pl-int-print/<name>`.
    pub name: String,
    /// The exact bytes of `<name>.pl` (CRLF included where the case has it).
    pub source: String,
    /// The outcome with no override.
    pub outcome: Outcome,
    /// `<name>.expect@<profile>` / `<name>.reject@<profile>`, by profile.
    pub overrides: BTreeMap<String, Outcome>,
    /// The `.cite` oracle record.
    pub cite: String,
}

impl Case {
    /// A positive case (it has a plain `.expect`).
    pub fn is_positive(&self) -> bool {
        matches!(self.outcome, Outcome::Prints(_))
    }

    /// What the case must do under `profile`.
    pub fn expected_under(&self, profile: &str) -> &Outcome {
        self.overrides.get(profile).unwrap_or(&self.outcome)
    }
}

/// `corpus/pl-int-print/`, located through `subsets.csv`.
pub fn corpus_dir() -> PathBuf {
    let rows = csv_rows(&perl_grammars().join("subsets.csv"));
    let row = rows.iter().find(|row| row["subset"] == SUBSET).expect("subsets.csv declares pl-int-print");
    perl_grammars().join(&row["corpus_dir"])
}

fn printed(expect: &str, name: &str) -> String {
    let stdout = expect.strip_suffix('\n').unwrap_or_else(|| panic!("{name}: .expect must end with one LF"));
    assert!(!stdout.contains('\n'), "{name}: Perl's print subset never writes a newline");
    stdout.to_string()
}

fn category(reject: &str, name: &str) -> String {
    let category = reject.trim_end_matches('\n');
    assert!(
        ["Lex", "Parse", "Unsupported"].contains(&category),
        "{name}: a .reject names Lex, Parse or Unsupported, not {category:?}"
    );
    category.to_string()
}

/// Every case, sorted by name.
pub fn corpus() -> Vec<Case> {
    let dir = corpus_dir();
    let mut files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let name = entry.unwrap().file_name().into_string().expect("UTF-8 file names");
        let (stem, suffix) = name.split_once('.').unwrap_or_else(|| panic!("{name}: no suffix"));
        files.entry(stem.to_string()).or_default().push(suffix.to_string());
    }
    files
        .into_iter()
        .map(|(name, suffixes)| {
            let read = |suffix: &str| std::fs::read_to_string(dir.join(format!("{name}.{suffix}")));
            assert!(suffixes.iter().any(|s| s == "pl"), "{name}: no .pl source");
            let outcome = match (read("expect").ok(), read("reject").ok()) {
                (Some(expect), None) => Outcome::Prints(printed(&expect, &name)),
                (None, Some(reject)) => Outcome::Rejected(category(&reject, &name)),
                _ => panic!("{name}: exactly one of .expect and .reject"),
            };
            let mut overrides = BTreeMap::new();
            for suffix in &suffixes {
                let text = || read(suffix).unwrap();
                if let Some(profile) = suffix.strip_prefix("expect@") {
                    overrides.insert(profile.to_string(), Outcome::Prints(printed(&text(), &name)));
                } else if let Some(profile) = suffix.strip_prefix("reject@") {
                    overrides.insert(profile.to_string(), Outcome::Rejected(category(&text(), &name)));
                } else {
                    assert!(["pl", "expect", "reject", "cite"].contains(&suffix.as_str()), "{name}.{suffix}");
                }
            }
            let cite = read("cite").unwrap_or_else(|_| panic!("{name}: every case has a .cite"));
            assert!(cite.starts_with("oracle: "), "{name}.cite must open with its oracle record");
            let source = read("pl").unwrap();
            Case { name, source, outcome, overrides, cite }
        })
        .collect()
}
