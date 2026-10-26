//! Shared readers for the LANG84 integration tests (`release_subsets.rs`,
//! `release_sweep.rs`): the ECMAScript release inventory and the
//! `es-number-log` corpus, both read from `code/grammars/ecmascript/`.
//!
//! ```text
//!   code/grammars/ecmascript/
//!     releases.csv                        one row per release; columns read by NAME
//!     subsets.csv                         es-number-log -> corpus/es-number-log
//!     corpus/es-number-log/
//!       arithmetic.js  .expect  .cite     a positive case (node is the oracle)
//!       reject_var.js  .reject            a negative case: the C6 category
//!       numeric_separators.reject@es2016  one edition refuses a positive case
//!     release_classes.csv                 written by the sweep
//! ```

#![allow(dead_code)] // each test binary uses a different part of this module

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use javascript_iir_compiler::SUBSET;

/// `code/grammars/ecmascript`, from this crate's manifest directory.
pub fn ecmascript_grammars() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/ecmascript")
}

/// Rows of a CSV with `#` provenance lines, as maps keyed by header name
/// (lesson: read the header, never assume a column position). None of these
/// files quotes a cell, so a plain split is exact.
pub fn csv_rows(path: &Path) -> Vec<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = text.lines().filter(|line| !line.starts_with('#') && !line.is_empty());
    let header: Vec<String> = lines.next().unwrap().split(',').map(str::to_string).collect();
    lines
        .map(|line| {
            let cells: Vec<&str> = line.split(',').collect();
            assert_eq!(cells.len(), header.len(), "{}: {line}", path.display());
            header.iter().cloned().zip(cells.into_iter().map(str::to_string)).collect()
        })
        .collect()
}

/// Every row of `releases.csv`, in inventory order.
pub fn inventory() -> Vec<BTreeMap<String, String>> {
    csv_rows(&ecmascript_grammars().join("releases.csv"))
}

/// Releases whose `iir_status` lists the subset, in inventory order.
pub fn claiming_releases() -> Vec<String> {
    inventory()
        .into_iter()
        .filter(|row| row["iir_status"].split(';').any(|s| s == SUBSET))
        .map(|row| row["release"].clone())
        .collect()
}

/// What a case must do under one release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Exact stdout (the oracle's bytes).
    Runs(String),
    /// The C6 category the compile must fail with.
    Rejected(String),
}

/// One corpus case: `<name>.js` plus `.expect` (accepted) or `.reject`,
/// and any `.reject@<release>` overrides.
pub struct Case {
    /// The file stem, also the `case` in `es-number-log/<name>`.
    pub name: String,
    /// The exact bytes of `<name>.js`.
    pub source: String,
    /// The plain outcome, for every release without an override.
    pub outcome: Outcome,
    /// `release -> category` from `<name>.reject@<release>`.
    pub overrides: BTreeMap<String, String>,
}

impl Case {
    /// A positive case (it has an `.expect`).
    pub fn is_positive(&self) -> bool {
        matches!(self.outcome, Outcome::Runs(_))
    }

    /// What `release` must do with this case.
    pub fn outcome_on(&self, release: &str) -> Outcome {
        match self.overrides.get(release) {
            Some(category) => Outcome::Rejected(category.clone()),
            None => self.outcome.clone(),
        }
    }
}

/// `corpus/es-number-log/`, located through `subsets.csv`.
pub fn corpus_dir() -> PathBuf {
    let rows = csv_rows(&ecmascript_grammars().join("subsets.csv"));
    let row = rows.iter().find(|row| row["subset"] == SUBSET).expect("subsets.csv declares es-number-log");
    ecmascript_grammars().join(&row["corpus_dir"])
}

/// Every case, sorted by name, with its `.reject@<release>` overrides.
pub fn corpus() -> Vec<Case> {
    let dir = corpus_dir();
    let mut names: Vec<String> = Vec::new();
    let mut overrides: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        if let Some((stem, release)) = file.split_once(".reject@") {
            let category = std::fs::read_to_string(&path).unwrap().trim().to_string();
            overrides.entry(stem.to_string()).or_default().insert(release.to_string(), category);
        } else if file.contains('@') {
            // An `.expect@<x>` override would otherwise be silently ignored.
            panic!("{file}: only .reject@<release> overrides are supported");
        } else if let Some(stem) = file.strip_suffix(".js") {
            names.push(stem.to_string());
        }
    }
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let read = |ext: &str| std::fs::read_to_string(dir.join(format!("{name}.{ext}"))).ok();
            let source = read("js").unwrap();
            let outcome = match (read("expect"), read("reject")) {
                (Some(expect), None) => {
                    assert!(
                        read("cite").is_some_and(|c| c.contains("oracle:")),
                        "{name}.cite must record its oracle"
                    );
                    Outcome::Runs(expect)
                }
                (None, Some(reject)) => Outcome::Rejected(reject.trim().to_string()),
                _ => panic!("{name}: exactly one of .expect and .reject"),
            };
            let overrides = overrides.remove(&name).unwrap_or_default();
            Case { name, source, outcome, overrides }
        })
        .collect()
}
