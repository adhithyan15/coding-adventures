//! # Findings and the report
//!
//! A finding is `RULE FILE:LINE MESSAGE`.  The report is **deterministic**:
//! sorted, de-duplicated, with no timestamps, absolute paths or hash-map order
//! in it, so two runs on two machines print the same bytes and a CI diff means
//! the source changed (VOS03 section 3).

use std::fmt::Write as _;

use crate::manifest::{Class, ClassSource, Kind};

/// Every rule id the tool can print.  Declaration order is report order:
/// the numbered shape rules first, then the tool's own `VT` rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// `#![no_std]`; no `extern crate`, `alloc` or `std`.
    Sh1,
    /// No `unsafe`, inline asm or foreign linkage, and no waiver.
    Sh2,
    /// No `static`, no `thread_local!`, no interior-mutable `const`.
    Sh3,
    /// No `dyn`, function pointer, closure or `Fn*` bound.
    Sh4,
    /// No recursion.
    Sh5,
    /// The only loop is `for i in 0..K`.
    Sh6,
    /// No division or remainder.
    Sh7,
    /// No path to a panic.
    Sh8,
    /// Shift amounts are literals or wrapped.
    Sh9,
    /// Fixed-width types only.
    Sh10,
    /// No floats.
    Sh11,
    /// Dependencies are `core` and the contract crate.
    Sh12,
    /// `alpha` and the builders live in a `ghost` module behind a feature.
    Sh13,
    /// A `State` field is private to its component.
    Sh14,
    /// No macros but pure `core` ones; no include, path, link or build tricks.
    Sh15,
    /// The ghost face is a faithful wrapper.
    Sh17,
    /// Nothing ghost leaks into the shipped build.
    Sh18,
    /// `vos.toml` is invalid.
    Vt01,
    /// The package name and the declared kind disagree.
    Vt02,
    /// Input the tool could not read or lex: fail closed.
    Vt03,
}

impl Rule {
    /// The printed id.
    pub fn id(self) -> &'static str {
        match self {
            Rule::Sh1 => "SH1",
            Rule::Sh2 => "SH2",
            Rule::Sh3 => "SH3",
            Rule::Sh4 => "SH4",
            Rule::Sh5 => "SH5",
            Rule::Sh6 => "SH6",
            Rule::Sh7 => "SH7",
            Rule::Sh8 => "SH8",
            Rule::Sh9 => "SH9",
            Rule::Sh10 => "SH10",
            Rule::Sh11 => "SH11",
            Rule::Sh12 => "SH12",
            Rule::Sh13 => "SH13",
            Rule::Sh14 => "SH14",
            Rule::Sh15 => "SH15",
            Rule::Sh17 => "SH17",
            Rule::Sh18 => "SH18",
            Rule::Vt01 => "VT01",
            Rule::Vt02 => "VT02",
            Rule::Vt03 => "VT03",
        }
    }
}

/// One violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The rule violated.
    pub rule: Rule,
    /// Crate-relative path (`src/lib.rs`, `Cargo.toml`, `vos.toml`).
    pub file: String,
    /// 1-based line (1 when the finding is about a whole file).
    pub line: u32,
    /// What and why.
    pub message: String,
}

/// The most findings printed; more is still FAIL.
pub const MAX_FINDINGS: usize = 1_000;

/// The result of checking one crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// `[package] name`, if the manifest gave one.
    pub package: Option<String>,
    /// The class the rules were selected by.
    pub class: Class,
    /// Where the class came from.
    pub class_source: ClassSource,
    /// The declared kind, if any.
    pub kind: Option<Kind>,
    /// Sorted, de-duplicated findings (at most [`MAX_FINDINGS`]).
    pub findings: Vec<Finding>,
    /// How many findings were dropped over the cap.
    pub dropped: usize,
}

impl Report {
    /// True when there is nothing to fix.
    pub fn passed(&self) -> bool {
        self.findings.is_empty() && self.dropped == 0
    }

    /// The process exit code of section 3: 0 pass, 1 findings.
    pub fn exit_code(&self) -> i32 {
        i32::from(!self.passed())
    }

    /// Render the byte-for-byte deterministic report.
    pub fn render(&self) -> String {
        let mut s = String::new();
        let name = self.package.as_deref().unwrap_or("?");
        let kind = self.kind.map_or("unspecified", Kind::name);
        let _ = writeln!(
            s,
            "vos-shape-check: crate {name}, class {} ({}), kind {kind}",
            self.class.letter(),
            self.class_source.name()
        );
        for f in &self.findings {
            let _ = writeln!(s, "{} {}:{} {}", f.rule.id(), f.file, f.line, f.message);
        }
        if self.dropped > 0 {
            let _ = writeln!(
                s,
                "VT03 -:0 {} further findings not printed (cap {MAX_FINDINGS})",
                self.dropped
            );
        }
        let n = self.findings.len() + self.dropped;
        if n == 0 {
            s.push_str("result: PASS, 0 findings\n");
        } else {
            let _ = writeln!(s, "result: FAIL, {n} findings");
        }
        s
    }
}

/// Sort, de-duplicate and cap a finding list.
pub fn finish(mut findings: Vec<Finding>) -> (Vec<Finding>, usize) {
    findings.sort_by(|a, b| {
        (&a.file, a.line, a.rule, &a.message).cmp(&(&b.file, b.line, b.rule, &b.message))
    });
    findings.dedup();
    let dropped = findings.len().saturating_sub(MAX_FINDINGS);
    findings.truncate(MAX_FINDINGS);
    (findings, dropped)
}
