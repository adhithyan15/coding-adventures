//! Shared readers for the LANG84 integration tests (`release_subsets.rs`,
//! `release_sweep.rs`): the Ruby release inventory and the `rb-int-puts`
//! corpus, both read from `code/grammars/ruby/`.
//!
//! ```text
//!   code/grammars/ruby/
//!     releases.csv                 one row per release; columns read by NAME
//!     subsets.csv                  rb-int-puts -> corpus/rb-int-puts
//!     corpus/rb-int-puts/
//!       mod_zero.rb  mod_zero.expect  mod_zero.cite       a positive case
//!       reject_hex.rb  reject_hex.reject  reject_hex.cite a negative case
//!     release_classes.csv          written by the sweep (phase 5b slice 2)
//! ```
//!
//! These readers were the top of `release_subsets.rs` until the release
//! sweep needed the same corpus; they moved here unchanged except for one
//! added guard (the corpus holds no per-release override, see [`corpus`]).

#![allow(dead_code)] // each test binary uses a different part of this module

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ruby_iir_compiler::SUBSET;

/// `code/grammars/ruby`, from this crate's manifest directory.
pub fn ruby_grammars() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../grammars/ruby")
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
    csv_rows(&ruby_grammars().join("releases.csv"))
}

/// Does a `;`-separated status cell (`iir_status`, `backend_status`) list
/// the subset?
pub fn lists_subset(cell: &str) -> bool {
    cell.split(';').any(|s| s == SUBSET)
}

/// Releases whose `iir_status` lists the subset, in inventory order.
pub fn claiming_releases() -> Vec<String> {
    inventory()
        .into_iter()
        .filter(|row| lists_subset(&row["iir_status"]))
        .map(|row| row["release"].clone())
        .collect()
}

/// One corpus case: `<name>.rb` plus `.expect` (accepted) or `.reject`.
pub struct Case {
    /// The file stem, also the `case` in `rb-int-puts/<name>`.
    pub name: String,
    /// The exact bytes of `<name>.rb`.
    pub source: String,
    /// What every claiming release must do with it.
    pub outcome: Outcome,
}

/// What a case must do.
pub enum Outcome {
    /// Exact stdout, then the LANG83 trap kind if the program stops on one.
    Runs { stdout: String, trap: Option<String> },
    /// The C6 category the compile must fail with.
    Rejected(String),
}

impl Case {
    /// A positive case (it has an `.expect`).
    pub fn is_positive(&self) -> bool {
        matches!(self.outcome, Outcome::Runs { .. })
    }
}

/// `corpus/rb-int-puts/`, located through `subsets.csv`.
pub fn corpus_dir() -> PathBuf {
    let rows = csv_rows(&ruby_grammars().join("subsets.csv"));
    let row = rows.iter().find(|row| row["subset"] == SUBSET).expect("subsets.csv declares rb-int-puts");
    ruby_grammars().join(&row["corpus_dir"])
}

/// Every case, sorted by name.
///
/// Ruby's corpus has no `.expect@<x>` / `.reject@<x>` override (every
/// claiming release shares the `ruby33` profile and accepts every case).
/// `release_inventory.py` would accept one, and this reader would then run
/// the case under its plain outcome, so an override fails here until the
/// readers learn it (Python's `common/mod.rs` shows how).
pub fn corpus() -> Vec<Case> {
    let dir = corpus_dir();
    let mut names: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(!file.contains('@'), "{file}: Ruby's corpus readers do not support per-release overrides yet");
        if let Some(stem) = file.strip_suffix(".rb") {
            names.push(stem.to_string());
        }
    }
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let read = |ext: &str| std::fs::read_to_string(dir.join(format!("{name}.{ext}"))).ok();
            let source = read("rb").unwrap();
            assert!(read("cite").is_some_and(|c| c.contains("oracle:")), "{name}.cite must record its oracle");
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
            Case { name, source, outcome }
        })
        .collect()
}
