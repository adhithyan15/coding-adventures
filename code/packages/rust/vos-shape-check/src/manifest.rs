//! # `vos.toml` and the facts we need from `Cargo.toml`
//!
//! Two small files, two small readers.
//!
//! ## `vos.toml` (VOS02 section 7.3, VOS03 section 1.1)
//!
//! ```text
//! class = "K"                      # K, C, S, M, P or H; default K
//! kind  = "production"             # contract, production, reference, any, spine, hal, profile
//! evidence-features = ["ghost"]    # must be in the central list
//! contract = "vos-ready-contract"  # an implementation: the contract package it realises (VOS02 10.1, D-GD2)
//! obligations = ["safety"]         # an implementation: the tiers `vos-link gen-doc` embeds
//! ```
//!
//! The grammar is deliberately tiny so that it can be bounded and so that it
//! can never be a place to hide something: at most 4 KiB, 16 keys, 1,024-byte
//! lines, ASCII, double-quoted strings with no escapes, one-line arrays.  Any
//! deviation is an error (rule VT01).
//!
//! ## `Cargo.toml`
//!
//! We read only what a shape rule needs and let [`crate::toml`] do the hard
//! part (flattening every spelling of a key to one path).

use std::collections::{BTreeMap, BTreeSet};

use crate::toml::{self, Entry, Value};

/// The crate class of VOS01 section 1.6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    /// Kernel: every component, `-any`, spine and HAL-facing crate (default).
    K,
    /// Emulator contract component.
    C,
    /// Emulator spine.
    S,
    /// Model: wrapped simulators and CPU adapters.
    M,
    /// Parser.
    P,
    /// Host.
    H,
}

impl Class {
    /// The one-letter tag.
    pub fn letter(self) -> &'static str {
        match self {
            Class::K => "K",
            Class::C => "C",
            Class::S => "S",
            Class::M => "M",
            Class::P => "P",
            Class::H => "H",
        }
    }
}

/// Where the class in force came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassSource {
    /// A `class = ".."` line in `vos.toml`.
    VosToml,
    /// No tag: class K by the rule of VOS01 section 1.6.
    Default,
}

impl ClassSource {
    /// Printed in the report header.
    pub fn name(self) -> &'static str {
        match self {
            ClassSource::VosToml => "vos.toml",
            ClassSource::Default => "default",
        }
    }
}

/// The implementation kind of VOS02 sections 3.1 and 7.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A contract package.
    Contract,
    /// A production implementation.
    Production,
    /// A reference model.
    Reference,
    /// The most-general `-any` implementation.
    Any,
    /// The spine.
    Spine,
    /// A HAL crate.
    Hal,
    /// A profile crate.
    Profile,
}

impl Kind {
    /// The `vos.toml` spelling.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Contract => "contract",
            Kind::Production => "production",
            Kind::Reference => "reference",
            Kind::Any => "any",
            Kind::Spine => "spine",
            Kind::Hal => "hal",
            Kind::Profile => "profile",
        }
    }

    fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "contract" => Kind::Contract,
            "production" => Kind::Production,
            "reference" => Kind::Reference,
            "any" => Kind::Any,
            "spine" => Kind::Spine,
            "hal" => Kind::Hal,
            "profile" => Kind::Profile,
            _ => return None,
        })
    }
}

/// Caps of VOS02 section 6.8.
pub const VOS_TOML_MAX_BYTES: usize = 4 * 1024;
/// At most this many keys in `vos.toml`.
pub const VOS_TOML_MAX_KEYS: usize = 16;
/// At most this many bytes in a `vos.toml` line.
pub const VOS_TOML_MAX_LINE: usize = 1_024;
/// At most this many names in `evidence-features`.
pub const MAX_EVIDENCE_NAMES: usize = 64;
/// At most this many tiers in `obligations` (the five tiers of VOS02 R1 and 2.7).
pub const MAX_OBLIGATION_NAMES: usize = 5;

/// The parsed `vos.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VosToml {
    /// The class tag, if written.
    pub class: Option<Class>,
    /// The kind, if written.
    pub kind: Option<Kind>,
    /// Declared evidence features, in file order.
    pub evidence_features: Vec<String>,
    /// The contract package an implementation realises (VOS02 section 10.1, D-GD2),
    /// as written.  `vos-link` validates the name; this reader only bounds it.
    pub contract: Option<String>,
    /// The tiers `vos-link gen-doc` embeds, in file order, as written.
    pub obligations: Vec<String>,
    /// The declared floor of line coverage, an integer percent 0 to 100 (VOS04 section 5.6,
    /// VOS-D070).  This reader only bounds it; `vos-link check-component` requires it of an
    /// implementation, holds it to the repository minimum and ties the BUILD gate to it.
    pub coverage_floor: Option<u8>,
    /// The manifest ids of the negative controls this crate is allowed to fail (VOS-D065,
    /// VOS-D098), as written.  Never empty when present, each id 1 to 128 bytes of
    /// `[A-Za-z0-9_/.-]`.  Matched as WHOLE ids by `vos-link check-component`.
    pub excused_controls: Vec<Excuse>,
    /// The model-SAT lane opt-in (VOS31 section 10.3): `sat = "model"` is the only value, and
    /// an absent key is `None`, which means the lane is off for this contract.
    pub sat: Option<SatLane>,
}

/// The value of the `vos.toml` key `sat`.  A closed enum with one member today: the host
/// model-SAT lane (`vos-model-sat`).  Any other spelling is an error, never a silent "off".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SatLane {
    /// `sat = "model"`.
    Model,
}

impl SatLane {
    /// The `vos.toml` spelling.
    pub fn name(self) -> &'static str {
        match self {
            SatLane::Model => "model",
        }
    }
}

/// One excused negative control: the manifest id it may fail on, and the backlog row that
/// says why (VOS-039a review M4).  An excuse without a row cannot be written, and
/// `vos-link` refuses one whose row is missing or done, so an excuse expires with its row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Excuse {
    /// The whole manifest id.
    pub id: String,
    /// `VOS-D` and three digits.
    pub reason: String,
}

/// Is `s` a backlog discovered-work reference, `VOS-D` and exactly three digits?
pub fn is_backlog_ref(s: &str) -> bool {
    s.len() == 8 && s.starts_with("VOS-D") && s[5..].bytes().all(|b| b.is_ascii_digit())
}

/// At most this many excused controls (each `{ id = "..", reason = "VOS-Dnnn" }`).
pub const MAX_EXCUSED_CONTROLS: usize = 16;

/// A `vos.toml` error: 1-based line and reason.
pub type VosTomlError = (u32, String);

fn bad<T>(line: usize, m: &str) -> Result<T, VosTomlError> {
    Err((u32::try_from(line).unwrap_or(u32::MAX), m.to_string()))
}

/// Parse `vos.toml`.  The file is TOML, read by the repository's TOML crates
/// (see [`crate::toml`]); this function adds the bounds of VOS02 section 6.8
/// BEFORE parsing and the closed vocabulary AFTER it.  Everything outside the
/// grammar is an error.
pub fn parse_vos_toml(text: &str) -> Result<VosToml, VosTomlError> {
    if text.len() > VOS_TOML_MAX_BYTES {
        return bad(1, "vos.toml is over 4 KiB");
    }
    if !text.is_ascii() {
        return bad(1, "vos.toml must be ASCII");
    }
    if text.contains('\\') {
        return bad(1, "escapes (a backslash) are not allowed in vos.toml");
    }
    for (n, raw) in text.lines().enumerate() {
        crate::steps::tick();
        if raw.len() > VOS_TOML_MAX_LINE {
            return bad(n + 1, "line over 1,024 bytes");
        }
    }
    // The cap is on lines that carry a key, counted before anything else is
    // judged, so it holds however the keys are spelled.
    let key_lines = text
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .count();
    if key_lines > VOS_TOML_MAX_KEYS {
        return bad(1, "more than 16 keys");
    }
    let entries = toml::parse(text).map_err(|e| (e.line, e.message))?;
    let mut out = VosToml {
        class: None,
        kind: None,
        evidence_features: Vec::new(),
        contract: None,
        obligations: Vec::new(),
        coverage_floor: None,
        excused_controls: Vec::new(),
        sat: None,
    };
    let mut excuse_ids: Vec<String> = Vec::new();
    let mut excuse_reasons: Vec<String> = Vec::new();
    let mut excuse_tables = 0usize;
    for Entry { path, value, line } in entries {
        let line = line as usize;
        // `excused-controls = [{ id = "..", reason = ".." }, ..]` flattens to one entry per
        // key of each inline table; they are collected here and paired after the loop.
        if let [k, b, f] = path.as_slice() {
            if k == "excused-controls" && b == "[]" {
                let Value::Str(v) = value else {
                    return bad(line, "an excused control's id and reason are strings");
                };
                match f.as_str() {
                    "id" => excuse_ids.push(v),
                    "reason" => excuse_reasons.push(v),
                    _ => {
                        return bad(
                            line,
                            "an excused control has the keys `id` and `reason` only",
                        )
                    }
                }
                continue;
            }
        }
        let [key] = path.as_slice() else {
            return bad(
                line,
                "unknown key or table (allowed: class, kind, evidence-features, contract, obligations, coverage-floor, excused-controls, sat)",
            );
        };
        match (key.as_str(), value) {
            ("class", Value::Str(v)) => {
                if out.class.is_some() {
                    return bad(line, "duplicate key");
                }
                out.class = Some(match v.as_str() {
                    "K" => Class::K,
                    "C" => Class::C,
                    "S" => Class::S,
                    "M" => Class::M,
                    "P" => Class::P,
                    "H" => Class::H,
                    _ => return bad(line, "class must be one of K, C, S, M, P, H"),
                });
            }
            ("kind", Value::Str(v)) => {
                if out.kind.is_some() {
                    return bad(line, "duplicate key");
                }
                let Some(k) = Kind::parse(&v) else {
                    return bad(
                        line,
                        "kind must be contract, production, reference, any, spine, hal or profile",
                    );
                };
                out.kind = Some(k);
            }
            ("evidence-features", Value::Array(items)) => {
                for item in items {
                    crate::steps::tick();
                    let Value::Str(name) = item else {
                        return bad(line, "evidence-features holds strings");
                    };
                    if name.is_empty() || name.len() > 32 {
                        return bad(line, "a feature name is 1 to 32 bytes");
                    }
                    if out.evidence_features.len() >= MAX_EVIDENCE_NAMES {
                        return bad(line, "more than 64 evidence features");
                    }
                    out.evidence_features.push(name);
                }
            }
            ("contract", Value::Str(v)) => {
                if out.contract.is_some() {
                    return bad(line, "duplicate key");
                }
                if v.is_empty() || v.len() > 32 {
                    return bad(line, "a contract package name is 1 to 32 bytes");
                }
                out.contract = Some(v);
            }
            ("obligations", Value::Array(items)) => {
                if !out.obligations.is_empty() {
                    return bad(line, "duplicate key");
                }
                for item in items {
                    crate::steps::tick();
                    let Value::Str(name) = item else {
                        return bad(line, "obligations holds strings");
                    };
                    if name.is_empty() || name.len() > 16 {
                        return bad(line, "a tier name is 1 to 16 bytes");
                    }
                    if out.obligations.len() >= MAX_OBLIGATION_NAMES {
                        return bad(line, "more than 5 obligation tiers");
                    }
                    out.obligations.push(name);
                }
            }
            ("coverage-floor", Value::Other(v)) => {
                if out.coverage_floor.is_some() {
                    return bad(line, "duplicate key");
                }
                // An integer written as plain decimal digits: no sign, no underscore, no
                // radix prefix, no fraction (those arrive as other text and are refused).
                let floor = if !v.is_empty() && v.len() <= 3 && v.bytes().all(|b| b.is_ascii_digit()) {
                    v.parse::<u8>().ok().filter(|n| *n <= 100)
                } else {
                    None
                };
                let Some(floor) = floor else {
                    return bad(line, "coverage-floor is an integer from 0 to 100");
                };
                out.coverage_floor = Some(floor);
            }
            ("sat", Value::Str(v)) => {
                if out.sat.is_some() {
                    return bad(line, "duplicate key");
                }
                out.sat = Some(match v.as_str() {
                    "model" => SatLane::Model,
                    _ => return bad(line, "sat must be \"model\" (omit the key to turn the lane off)"),
                });
            }
            ("excused-controls", Value::Array(items)) => {
                if excuse_tables != 0 {
                    return bad(line, "duplicate key");
                }
                if items.is_empty() {
                    return bad(line, "excused-controls is never empty (omit the key)");
                }
                if items.iter().any(|i| *i != Value::Table) {
                    return bad(
                        line,
                        "excused-controls holds inline tables `{ id = \"..\", reason = \"VOS-Dnnn\" }`",
                    );
                }
                excuse_tables = items.len();
            }
            (
                "class" | "kind" | "evidence-features" | "contract" | "obligations"
                | "coverage-floor" | "excused-controls" | "sat",
                _,
            ) => {
                return bad(line, "wrong type of value for this key")
            }
            _ => {
                return bad(
                    line,
                    "unknown key (allowed: class, kind, evidence-features, contract, obligations, coverage-floor, excused-controls, sat)",
                )
            }
        }
    }
    if excuse_tables != excuse_ids.len() || excuse_tables != excuse_reasons.len() {
        return bad(
            1,
            "every excused control has exactly one `id` and one `reason` (a backlog row, VOS-Dnnn)",
        );
    }
    if excuse_tables > MAX_EXCUSED_CONTROLS {
        return bad(1, "more than 16 excused controls");
    }
    let plain = |b: u8| b.is_ascii_alphanumeric() || b"_/.-".contains(&b);
    for (id, reason) in excuse_ids.into_iter().zip(excuse_reasons) {
        crate::steps::tick();
        if id.is_empty() || id.len() > 128 || !id.bytes().all(plain) {
            return bad(
                1,
                "an excused control id is 1 to 128 bytes of [A-Za-z0-9_/.-]",
            );
        }
        if !is_backlog_ref(&reason) {
            return bad(
                1,
                "an excused control's reason is a backlog row, `VOS-D` and three digits",
            );
        }
        if out.excused_controls.iter().any(|e| e.id == id) {
            return bad(1, "an excused control is listed twice");
        }
        out.excused_controls.push(Excuse { id, reason });
    }
    Ok(out)
}

/// A dependency entry of `Cargo.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dep {
    /// `dependencies`, `build-dependencies` or `dev-dependencies`.
    pub table: String,
    /// The dependency name as written.
    pub name: String,
    /// First line mentioning it.
    pub line: u32,
    /// Sub-keys seen (`path`, `version`, `git`, `registry`, `workspace`,
    /// `package`, ...); a bare `name = "1.0"` records `version`.
    pub keys: BTreeSet<String>,
}

/// Facts read from `Cargo.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CargoFacts {
    /// `[package] name`.
    pub name: Option<String>,
    /// `[package] edition`, when a plain string.
    pub edition: Option<String>,
    /// Line of the edition key, or 1.
    pub edition_line: u32,
    /// Lines of every `build` key under `[package]`, whatever the value.
    pub build_keys: Vec<u32>,
    /// Lines of every `links` key.
    pub links_keys: Vec<u32>,
    /// `(line, why)` for a proc-macro marker.
    pub proc_macro: Vec<(u32, String)>,
    /// `(line, why)` for a target `path` that is not where its kind lives
    /// (`[lib]` exactly `src/lib.rs`; tests, examples and benches under their
    /// own directories).
    pub bad_paths: Vec<(u32, String)>,
    /// Lines of every `[[bin]]` (or `bin = [..]`): binaries are not allowed.
    pub bin_targets: Vec<u32>,
    /// Every dependency, in file order.
    pub deps: Vec<Dep>,
    /// Declared features: name to (line, entries).
    pub features: BTreeMap<String, (u32, Vec<String>)>,
}

/// Cap of `Cargo.toml`.
pub const CARGO_TOML_MAX_BYTES: usize = 64 * 1024;

fn dep_table(name: &str) -> bool {
    matches!(
        name,
        "dependencies" | "build-dependencies" | "dev-dependencies"
    )
}

fn is_proc_macro_key(k: &str) -> bool {
    k == "proc-macro" || k == "proc_macro"
}

/// Read the facts from a `Cargo.toml` text.
pub fn parse_cargo(text: &str) -> Result<CargoFacts, toml::TomlError> {
    let entries = toml::parse(text)?;
    let mut f = CargoFacts {
        edition_line: 1,
        ..CargoFacts::default()
    };
    for Entry { path, value, line } in &entries {
        let line = *line;
        let mut p: Vec<&str> = path.iter().map(String::as_str).collect();
        // `[project]` is the old spelling of `[package]`: Cargo still reads it.
        if p.first() == Some(&"project") {
            p[0] = "package";
        }
        match p.as_slice() {
            ["package", "name"] => {
                if let Value::Str(s) = value {
                    f.name = Some(s.clone());
                }
            }
            ["package", "edition", ..] => {
                f.edition_line = line;
                if let (["package", "edition"], Value::Str(s)) = (p.as_slice(), value) {
                    f.edition = Some(s.clone());
                } else if p.len() > 2 {
                    // `edition.workspace = true`: the value is elsewhere.
                    f.edition = None;
                }
            }
            ["package", "build", ..] => f.build_keys.push(line),
            ["package", "links", ..] => f.links_keys.push(line),
            _ => {}
        }
        if p.first() == Some(&"lib") && p.len() >= 2 {
            let key = p[1];
            if is_proc_macro_key(key) && !matches!(value, Value::Bool(false)) {
                f.proc_macro
                    .push((line, format!("`{key}` is not the literal `false`")));
            }
            if (key == "crate-type" || key == "crate_type")
                && match value {
                    Value::Array(items) => items
                        .iter()
                        .any(|v| matches!(v, Value::Str(s) if s == "proc-macro")),
                    Value::Str(s) => s == "proc-macro",
                    _ => false,
                }
            {
                f.proc_macro
                    .push((line, "`crate-type` names `proc-macro`".to_string()));
            }
        }
        if p == ["bin"] {
            f.bin_targets.push(line);
        }
        if p.len() >= 2 && p.last() == Some(&"path") {
            let required: Option<(&str, bool)> = match p[0] {
                "lib" => Some(("src/lib.rs", true)),
                "test" => Some(("tests/", false)),
                "example" => Some(("examples/", false)),
                "bench" => Some(("benches/", false)),
                _ => None,
            };
            if let Some((want, exact)) = required {
                match value {
                    Value::Str(s) => {
                        let n = s.replace('\\', "/");
                        let ok = !n.split('/').any(|c| c == "..")
                            && !n.starts_with('/')
                            && !n.contains(':')
                            && if exact {
                                n == want
                            } else {
                                n.starts_with(want)
                            };
                        if !ok {
                            f.bad_paths.push((
                                line,
                                format!(
                                    "`[{}] path = \"{s}\"` must be {}`{want}`",
                                    p[0],
                                    if exact { "exactly " } else { "under " }
                                ),
                            ));
                        }
                    }
                    _ => f
                        .bad_paths
                        .push((line, "target path is not a plain string".to_string())),
                }
            }
        }
        // Dependency tables: top level or under `target.<cfg>.`.
        let at = if p.first() == Some(&"target") && p.get(2).is_some_and(|t| dep_table(t)) {
            Some(2)
        } else if p.first().is_some_and(|t| dep_table(t)) {
            Some(0)
        } else {
            None
        };
        if let Some(i) = at {
            if let Some(name) = p.get(i + 1) {
                let table = p[i].to_string();
                let sub: Option<String> = p.get(i + 2).map(|s| (*s).to_string());
                let pos = f
                    .deps
                    .iter()
                    .position(|d| d.table == table && d.name == *name);
                let d = if let Some(pos) = pos {
                    &mut f.deps[pos]
                } else {
                    f.deps.push(Dep {
                        table,
                        name: (*name).to_string(),
                        line,
                        keys: BTreeSet::new(),
                    });
                    f.deps.last_mut().expect("just pushed")
                };
                match (&sub, value) {
                    (Some(k), _) => {
                        d.keys.insert(k.clone());
                    }
                    (None, Value::Str(_)) => {
                        d.keys.insert("version".to_string());
                    }
                    (None, Value::Table) => {}
                    (None, _) => {
                        d.keys.insert("other".to_string());
                    }
                }
            }
        }
        if p.first() == Some(&"features") && p.len() == 2 {
            let list = match value {
                Value::Array(items) => items
                    .iter()
                    .filter_map(|v| match v {
                        Value::Str(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            f.features.insert(p[1].to_string(), (line, list));
        }
    }
    Ok(f)
}
