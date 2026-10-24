//! Shared readers for the LANG84 integration tests (`release_subsets.rs`,
//! `release_sweep.rs`): the Python release inventory and the `py-int-print`
//! corpus, both read from `code/grammars/python/`.
//!
//! ```text
//!   code/grammars/python/
//!     releases.csv                 one row per release; columns read by NAME
//!     subsets.csv                  py-int-print -> corpus/py-int-print
//!     corpus/py-int-print/
//!       mod_zero.py  mod_zero.expect  mod_zero.cite        a positive case
//!       int_underscores.reject@3.0a1                       ... refused by one release
//!       reject_hex.py  reject_hex.reject  reject_hex.cite  a negative case
//!     release_classes.csv          written by the sweep (phase 7b slice 2)
//! ```
//!
//! These readers were the top of `release_subsets.rs` until the release
//! sweep needed the same corpus; they moved here unchanged except for one
//! added guard (an override other than `.reject@<release>` fails).

#![allow(dead_code)] // each test binary uses a different part of this module

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use python_iir_compiler::{supported_releases, SUBSET};

/// `code/grammars/python`, from this crate's manifest directory.
pub fn python_grammars() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/python")
}

/// Rows of a CSV with `#` provenance lines, as maps keyed by header name
/// (lesson: read the header, never assume a column position).
pub fn csv_rows(path: &Path) -> Vec<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut lines = text.lines().filter(|line| !line.starts_with('#') && !line.is_empty());
    let header: Vec<String> = lines.next().unwrap().split(',').map(str::to_string).collect();
    lines
        .map(|line| header.iter().cloned().zip(line.split(',').map(str::to_string)).collect())
        .collect()
}

/// Every row of `releases.csv`, in inventory order.
pub fn inventory() -> Vec<BTreeMap<String, String>> {
    csv_rows(&python_grammars().join("releases.csv"))
}

/// Releases whose `iir_status` lists the subset, in inventory order.
pub fn claiming_releases() -> Vec<String> {
    inventory()
        .into_iter()
        .filter(|row| row["iir_status"].split(';').any(|s| s == SUBSET))
        .map(|row| row["release"].clone())
        .collect()
}

/// One corpus case: `<name>.py` plus `.expect` (accepted) or `.reject`, and
/// any `.reject@<release>` overrides.
pub struct Case {
    pub name: String,
    pub source: String,
    pub outcome: Outcome,
    pub overrides: BTreeMap<String, String>,
    pub cite: String,
}

/// What a case must do.
#[derive(Clone)]
pub enum Outcome {
    /// Exact stdout, then the LANG83 trap kind if the program stops on one.
    Runs { stdout: String, trap: Option<String> },
    /// The C6 category the compile must fail with.
    Rejected(String),
}

impl Case {
    /// A positive case (it has a plain `.expect`).
    pub fn is_positive(&self) -> bool {
        matches!(self.outcome, Outcome::Runs { .. })
    }

    /// The outcome this case must have on `release`.
    pub fn outcome_on(&self, release: &str) -> Outcome {
        match self.overrides.get(release) {
            Some(category) => Outcome::Rejected(category.clone()),
            None => self.outcome.clone(),
        }
    }
}

/// `corpus/py-int-print/`, located through `subsets.csv`.
pub fn corpus_dir() -> PathBuf {
    let rows = csv_rows(&python_grammars().join("subsets.csv"));
    let row = rows.iter().find(|row| row["subset"] == SUBSET).expect("subsets.csv declares py-int-print");
    python_grammars().join(&row["corpus_dir"])
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
            // An `.expect@<x>` override would otherwise be silently ignored
            // and the case run under its plain outcome (added in 7b slice 2).
            panic!("{file}: only .reject@<release> overrides are supported");
        } else if let Some(stem) = file.strip_suffix(".py") {
            names.push(stem.to_string());
        }
    }
    names.sort();
    for (stem, by_release) in &overrides {
        assert!(names.contains(stem), "{stem}: an override without a case");
        for release in by_release.keys() {
            assert!(supported_releases().contains(&release.as_str()), "{stem}: override for unpaired {release}");
        }
    }
    names
        .into_iter()
        .map(|name| {
            let read = |ext: &str| std::fs::read_to_string(dir.join(format!("{name}.{ext}"))).ok();
            let source = read("py").unwrap();
            let cite = read("cite").unwrap_or_default();
            assert!(cite.contains("oracle:"), "{name}.cite must record its oracle");
            let outcome = match (read("expect"), read("reject")) {
                (Some(expect), None) => {
                    let (stdout, trap) = match expect.rfind("trap:") {
                        Some(at) => (expect[..at].to_string(), Some(expect[at + 5..].trim().to_string())),
                        None => (expect, None),
                    };
                    Outcome::Runs { stdout, trap }
                }
                (None, Some(reject)) => Outcome::Rejected(reject.trim().to_string()),
                _ => panic!("{name}: exactly one of .expect and .reject"),
            };
            Case {
                overrides: overrides.remove(&name).unwrap_or_default(),
                name,
                source,
                outcome,
                cite,
            }
        })
        .collect()
}
