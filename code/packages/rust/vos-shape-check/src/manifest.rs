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

/// The parsed `vos.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VosToml {
    /// The class tag, if written.
    pub class: Option<Class>,
    /// The kind, if written.
    pub kind: Option<Kind>,
    /// Declared evidence features, in file order.
    pub evidence_features: Vec<String>,
}

/// A `vos.toml` error: 1-based line and reason.
pub type VosTomlError = (u32, String);

fn bad<T>(line: usize, m: &str) -> Result<T, VosTomlError> {
    Err((u32::try_from(line).unwrap_or(u32::MAX), m.to_string()))
}

/// Parse a double-quoted string at the start of `s`; return it and the rest.
fn quoted(s: &str, line: usize) -> Result<(String, &str), VosTomlError> {
    let Some(rest) = s.strip_prefix('"') else {
        return bad(line, "expected a double-quoted string");
    };
    let Some(end) = rest.find('"') else {
        return bad(line, "unterminated string");
    };
    let body = &rest[..end];
    if body.contains('\\') {
        return bad(line, "escapes are not allowed in vos.toml strings");
    }
    Ok((body.to_string(), &rest[end + 1..]))
}

/// Parse `vos.toml`.  Everything outside the grammar is an error.
pub fn parse_vos_toml(text: &str) -> Result<VosToml, VosTomlError> {
    if text.len() > VOS_TOML_MAX_BYTES {
        return bad(1, "vos.toml is over 4 KiB");
    }
    if !text.is_ascii() {
        return bad(1, "vos.toml must be ASCII");
    }
    let mut out = VosToml {
        class: None,
        kind: None,
        evidence_features: Vec::new(),
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
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
    for (n, raw) in text.lines().enumerate() {
        let line = n + 1;
        if raw.len() > VOS_TOML_MAX_LINE {
            return bad(line, "line over 1,024 bytes");
        }
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some((key, value)) = t.split_once('=') else {
            return bad(line, "expected `key = value`");
        };
        let key = key.trim();
        if !seen.insert(key.to_string()) {
            return bad(line, "duplicate key");
        }
        let value = value.trim();
        let tail_ok = |rest: &str| {
            let r = rest.trim();
            r.is_empty() || r.starts_with('#')
        };
        match key {
            "class" => {
                let (v, rest) = quoted(value, line)?;
                if !tail_ok(rest) {
                    return bad(line, "text after the value");
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
            "kind" => {
                let (v, rest) = quoted(value, line)?;
                if !tail_ok(rest) {
                    return bad(line, "text after the value");
                }
                let Some(k) = Kind::parse(&v) else {
                    return bad(
                        line,
                        "kind must be contract, production, reference, any, spine, hal or profile",
                    );
                };
                out.kind = Some(k);
            }
            "evidence-features" => {
                let Some(mut rest) = value.strip_prefix('[') else {
                    return bad(line, "evidence-features must be a one-line array");
                };
                loop {
                    rest = rest.trim_start();
                    if let Some(r) = rest.strip_prefix(']') {
                        if !tail_ok(r) {
                            return bad(line, "text after the array");
                        }
                        break;
                    }
                    let (v, r) = quoted(rest, line)?;
                    if v.is_empty() || v.len() > 32 {
                        return bad(line, "a feature name is 1 to 32 bytes");
                    }
                    if out.evidence_features.len() >= MAX_EVIDENCE_NAMES {
                        return bad(line, "more than 64 evidence features");
                    }
                    out.evidence_features.push(v);
                    rest = r.trim_start();
                    if let Some(r) = rest.strip_prefix(',') {
                        rest = r;
                    } else if !rest.starts_with(']') {
                        return bad(line, "expected `,` or `]`");
                    }
                }
            }
            _ => {
                return bad(
                    line,
                    "unknown key (allowed: class, kind, evidence-features)",
                )
            }
        }
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
    /// `(line, why)` for a `[lib]`/`[[bin]]` path outside `src/`.
    pub bad_paths: Vec<(u32, String)>,
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
        let p: Vec<&str> = path.iter().map(String::as_str).collect();
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
        let is_target_path =
            p.last() == Some(&"path") && (p.first() == Some(&"lib") || p.first() == Some(&"bin"));
        if is_target_path {
            match value {
                Value::Str(s) => {
                    let n = s.replace('\\', "/");
                    let escapes = n.starts_with('/')
                        || n.contains(':')
                        || n.split('/').any(|c| c == "..")
                        || !n.starts_with("src/");
                    if escapes {
                        f.bad_paths
                            .push((line, format!("target path `{s}` is not under `src/`")));
                    }
                }
                _ => f
                    .bad_paths
                    .push((line, "target path is not a plain string".to_string())),
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
