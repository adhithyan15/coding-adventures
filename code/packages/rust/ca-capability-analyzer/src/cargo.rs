//! # Minimal `Cargo.toml` Facts (VER08a §3.5, §3.6)
//!
//! Rust source is not the only thing that decides what code runs: `Cargo.toml`
//! can mark the crate as a procedural macro, point the library or a build script
//! at a file the scanner skips, and pull in third-party dependencies.  This module
//! extracts exactly those facts with a deliberately small line-oriented reader
//! (no TOML parser; zero dependencies).
//!
//! ```text
//!   [lib]                       proc-macro = true   -> CAP003
//!   [lib] / [[bin]]             path = "tests/x.rs" -> CAP003 (shipped, but never scanned)
//!   [package]                   build = "../b.rs"   -> CAP003
//!   [dependencies] / [build-dependencies] / [target.*.dependencies]
//!                               names               -> reported as unanalyzed
//! ```
//!
//! The reader understands: `[section]`, `[[array-of-tables]]`, `key = value`
//! lines, `#` comments outside strings, and `[dependencies.name]` sub-tables.
//! Anything fancier (dotted keys on one line such as `lib.proc-macro = true`
//! at the top level) is handled for the keys we care about; everything else is
//! ignored, never trusted.

use std::fs;
use std::path::Path;

/// A fail-closed finding inside `Cargo.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoFinding {
    pub line: usize,
    pub reason: String,
    pub evidence: String,
}

/// Facts extracted from `Cargo.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CargoFacts {
    pub findings: Vec<CargoFinding>,
    /// Sorted, de-duplicated names under dependency / build-dependency tables.
    pub dependencies: Vec<String>,
}

const UNSCANNED_ROOTS: &[&str] = &["tests", "benches", "examples", "target", "node_modules", "_build"];

/// Strip a trailing `#` comment that is not inside a quoted string.
fn strip_comment(line: &str) -> &str {
    let mut in_str: Option<char> = None;
    for (i, c) in line.char_indices() {
        match in_str {
            Some(q) if c == q => in_str = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => in_str = Some(c),
            None if c == '#' => return &line[..i],
            None => {}
        }
    }
    line
}

fn unquote(v: &str) -> Option<&str> {
    let v = v.trim();
    if v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')))
    {
        Some(&v[1..v.len() - 1])
    } else {
        None
    }
}

/// True if a Cargo `path`/`build` value would point outside the scanned set.
pub fn path_escapes_scan(raw: &str) -> bool {
    let p = raw.replace('\\', "/");
    if p.starts_with('/') || p.contains(':') {
        return true;
    }
    let mut comps = p.split('/').filter(|c| !c.is_empty() && *c != ".");
    let comp_vec: Vec<&str> = comps.by_ref().collect();
    if comp_vec.contains(&"..") {
        return true;
    }
    if comp_vec.iter().any(|c| c.starts_with('.')) {
        return true;
    }
    match comp_vec.first() {
        Some(first) => UNSCANNED_ROOTS.contains(first),
        None => false,
    }
}

fn is_dependency_section(sec: &str) -> bool {
    let s = sec.trim();
    s == "dependencies"
        || s == "build-dependencies"
        || s.ends_with(".dependencies") && s.starts_with("target.")
        || s.ends_with(".build-dependencies") && s.starts_with("target.")
}

/// Analyse the text of a `Cargo.toml`.
pub fn analyze_cargo_text(text: &str) -> CargoFacts {
    let mut facts = CargoFacts::default();
    let mut section = String::new();
    for (idx, raw_line) in text.lines().enumerate() {
        let lineno = idx + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            let inner = line.trim_start_matches('[').trim_end_matches(']').trim();
            section = inner.to_string();
            // [dependencies.foo] / [target.x.dependencies.foo]
            for pre in ["dependencies.", "build-dependencies."] {
                if let Some(rest) = section.strip_prefix(pre) {
                    facts.dependencies.push(rest.trim_matches('"').to_string());
                }
            }
            if section.starts_with("target.") {
                for marker in [".dependencies.", ".build-dependencies."] {
                    if let Some(pos) = section.find(marker) {
                        facts
                            .dependencies
                            .push(section[pos + marker.len()..].trim_matches('"').to_string());
                    }
                }
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        let value = value.trim();

        if is_dependency_section(&section) {
            facts.dependencies.push(key.split('.').next().unwrap_or(key).to_string());
            continue;
        }

        // Top-level dotted keys such as `lib.proc-macro = true`.
        let effective_section = if let Some((tbl, k)) = key.split_once('.') {
            if tbl == "lib" && section.is_empty() {
                format!("lib:{k}")
            } else {
                format!("{section}:{key}")
            }
        } else {
            format!("{section}:{key}")
        };

        let sec_name = section.as_str();
        // [[test]] / [[bench]] / [[example]] are not shipped (VER08a G-5), so their
        // paths into tests/ benches/ examples/ are legitimate and not flagged.
        let target_section = matches!(sec_name, "lib" | "bin");
        if (sec_name == "lib" && matches!(key, "proc-macro" | "proc_macro") && value == "true")
            || effective_section == "lib:proc-macro" && value == "true"
        {
            facts.findings.push(CargoFinding {
                line: lineno,
                reason: "proc-macro crate runs arbitrary code at compile time".into(),
                evidence: "proc-macro = true".into(),
            });
        }
        if (target_section && key == "path") || (sec_name == "package" && key == "build") {
            if let Some(p) = unquote(value) {
                if path_escapes_scan(p) {
                    facts.findings.push(CargoFinding {
                        line: lineno,
                        reason: format!("{key} points outside the scanned source set"),
                        evidence: format!("{key} = \"{p}\""),
                    });
                }
            }
        }
    }
    facts.dependencies.sort();
    facts.dependencies.dedup();
    facts
}

/// Read and analyse `<package_dir>/Cargo.toml` (absent file: no facts).
pub fn analyze_cargo_toml(package_dir: &Path) -> Result<CargoFacts, String> {
    let path = package_dir.join("Cargo.toml");
    let meta = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(_) => return Ok(CargoFacts::default()),
    };
    if meta.is_symlink() {
        return Err("Cargo.toml must not be a symlink".into());
    }
    if meta.len() > 1024 * 1024 {
        return Err("Cargo.toml exceeds 1 MiB".into());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(analyze_cargo_text(&text))
}
