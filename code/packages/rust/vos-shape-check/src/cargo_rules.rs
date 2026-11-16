//! # Manifest and crate-root rules
//!
//! Source code is not the only thing that decides what runs.  `Cargo.toml` can
//! name a build script (code that runs *before* the crate and can write any
//! source file), mark the crate a procedural macro (code that runs inside the
//! compiler), relocate the library root outside the directory we scan, or pull
//! in a dependency the footprint analysis never sees.  And a source file can
//! simply *claim* to be `no_std` without saying so.  This module reads those
//! facts and enforces:
//!
//! ```text
//!   SH1   src/lib.rs has #![no_std]
//!   SH2   src/lib.rs has #![forbid(unsafe_code)]; no allow/warn/deny/expect waiver
//!         (the attribute-level SH2 / SH15 bans are in `attr_rules`)
//!   SH4   edition is 2021 or 2024 (older editions accept a bare trait object)
//!   SH12  dependencies are `core` or a path dependency on a contract crate
//!   SH15  no build.rs file, no `build` key (ANY value), no `links` key,
//!         no proc-macro, no relocated target path
//!   VT02  a `-any` crate declares kind `any` and the converse
//! ```
//!
//! ## Why "any value" for `build`
//!
//! `build = false` looks harmless, and Cargo does honour it, but a rule that
//! allows one spelling of a forbidden key has to be right about every other
//! spelling forever.  The rule is "no `build` key at all, and no `build.rs`
//! file" (Cargo auto-detects a root `build.rs`), which a reader can verify at
//! a glance.

use crate::manifest::{CargoFacts, Kind as VKind};
use crate::report::Rule;
use crate::rules::Ctx;
use crate::syntax::FileCtx;

/// Names a SH15 attribute may not contain, and the rule that owns each.
const ATTR_BANS: &[(&str, Rule)] = &[
    ("path", Rule::Sh15),
    ("link_section", Rule::Sh15),
    ("no_mangle", Rule::Sh15),
    ("export_name", Rule::Sh15),
    ("link", Rule::Sh2),
    ("link_name", Rule::Sh2),
];

/// Attribute-level rules: SH2 waivers and linkage, SH15 relocation and symbol
/// attributes.  The contents of `cfg` attributes are SH18's business.
pub fn attr_rules(cx: &mut Ctx<'_>, f: &FileCtx) {
    for a in &f.attrs {
        let body = f.attr_body(a);
        let Some(first) = body.first() else { continue };
        let line = first.line;
        for x in body {
            if x.kind != crate::lexer::Kind::Ident {
                continue;
            }
            if let Some((name, rule)) = ATTR_BANS.iter().find(|(n, _)| *n == x.text) {
                cx.add(*rule, &f.path, x.line, format!("attribute names `{name}`; it relocates code or symbols out of the lexer's sight"));
            }
        }
        let waiver = matches!(
            first.text.as_str(),
            "allow" | "warn" | "deny" | "expect" | "cfg_attr"
        ) && body.iter().any(|x| x.is_ident("unsafe_code"));
        if waiver {
            cx.add(
                Rule::Sh2,
                &f.path,
                line,
                "`unsafe_code` is lowered or waived; only `forbid(unsafe_code)` is allowed",
            );
        }
    }
}

/// The crate-root checks: `#![no_std]` and `#![forbid(unsafe_code)]`.
pub fn root_rules(cx: &mut Ctx<'_>, root: &FileCtx) {
    let mut no_std = false;
    let mut forbid = false;
    for a in root.attrs.iter().filter(|a| a.inner) {
        let body = root.attr_body(a);
        if body.len() == 1 && body[0].is_ident("no_std") {
            no_std = true;
        }
        if body.first().is_some_and(|x| x.is_ident("forbid"))
            && body.iter().any(|x| x.is_ident("unsafe_code"))
        {
            forbid = true;
        }
    }
    if !no_std {
        cx.add(
            Rule::Sh1,
            &root.path,
            1,
            "the crate root lacks `#![no_std]`",
        );
    }
    if !forbid {
        cx.add(
            Rule::Sh2,
            &root.path,
            1,
            "the crate root lacks `#![forbid(unsafe_code)]`",
        );
    }
}

/// `vos-<iface>-contract` (hyphens and underscores are interchangeable).
fn is_contract_name(name: &str) -> bool {
    let n = name.replace('_', "-");
    n.len() > "vos--contract".len()
        && n.starts_with("vos-")
        && n.ends_with("-contract")
        && n.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Rules read from `Cargo.toml` and the crate-root entries.
pub fn manifest_rules(cx: &mut Ctx<'_>, c: &CargoFacts, root_entries: &[String]) {
    // SH4: the edition.
    match c.edition.as_deref() {
        Some("2021" | "2024") => {}
        Some(e) => cx.add(Rule::Sh4, "Cargo.toml", c.edition_line, format!("edition `{e}`; editions before 2021 accept a bare trait object with no `dyn`")),
        None => cx.add(Rule::Sh4, "Cargo.toml", c.edition_line, "edition is not the literal \"2021\" or \"2024\" (an inherited or missing edition cannot be verified)"),
    }
    // SH15: build scripts, links, proc macros, relocated targets.
    if root_entries.iter().any(|e| e == "build.rs") {
        cx.add(
            Rule::Sh15,
            "build.rs",
            1,
            "`build.rs` exists; Cargo runs it before the crate",
        );
    }
    for &l in &c.build_keys {
        cx.add(Rule::Sh15, "Cargo.toml", l, "`build` key (any value, `false` included); a build script runs code the lexer does not see");
    }
    for &l in &c.links_keys {
        cx.add(
            Rule::Sh15,
            "Cargo.toml",
            l,
            "`links` key; it names a native library",
        );
    }
    for (l, why) in &c.proc_macro {
        cx.add(
            Rule::Sh15,
            "Cargo.toml",
            *l,
            format!("proc-macro crate: {why}"),
        );
    }
    for (l, why) in &c.bad_paths {
        cx.add(Rule::Sh15, "Cargo.toml", *l, why.clone());
    }
    // SH12: dependencies.
    for d in &c.deps {
        if d.table == "dev-dependencies" {
            continue;
        }
        let source_keys: Vec<&str> = d.keys.iter().map(String::as_str).collect();
        let msg = if d.name == "core" {
            (!source_keys.is_empty()).then(|| "`core` takes no source key".to_string())
        } else if !is_contract_name(&d.name) {
            Some(format!(
                "dependency `{}` is neither `core` nor a `vos-<iface>-contract` crate",
                d.name
            ))
        } else if source_keys != ["path"] {
            Some(format!(
                "contract dependency `{}` must be exactly `{{ path = \"..\" }}` (found keys: {})",
                d.name,
                source_keys.join(", ")
            ))
        } else {
            None
        };
        if let Some(m) = msg {
            cx.add(
                Rule::Sh12,
                "Cargo.toml",
                d.line,
                format!("[{}] {m}", d.table),
            );
        }
    }
    // VT02: the `-any` name lint.
    if let Some(name) = &c.name {
        let named_any = name.ends_with("-any") || name.ends_with("_any");
        match cx.kind {
            Some(k) if named_any && k != VKind::Any => cx.add(
                Rule::Vt02,
                "vos.toml",
                1,
                format!(
                    "package `{name}` is named `-any` but vos.toml declares kind `{}`",
                    k.name()
                ),
            ),
            Some(VKind::Any) if !named_any => cx.add(
                Rule::Vt02,
                "vos.toml",
                1,
                format!("vos.toml declares kind `any` but package `{name}` is not named `...-any`"),
            ),
            None if named_any => cx.add(
                Rule::Vt02,
                "vos.toml",
                1,
                format!("package `{name}` is named `-any` but declares no kind `any`"),
            ),
            _ => {}
        }
    }
}
