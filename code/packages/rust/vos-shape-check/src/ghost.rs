//! # The ghost face: SH13, SH14, SH17, SH18
//!
//! ## The problem the ghost face solves (VOS02 section 4)
//!
//! A component's `State` has **private** fields (rule 2: nobody else may touch
//! it).  The evidence generator, however, must build and read a by-value
//! record, which needs **public** fields.  The answer is a *ghost face*: a
//! `mod ghost` compiled only when a cargo feature `ghost` is on, holding
//!
//! ```text
//! pub struct Image { pub mask: u8 }        // the same fields as State, public
//! pub fn vos_ghost_insert(s: Image, p: u8) -> (Upd, Image) {
//!     let (o, Bitmap { mask }) =           // exhaustive destructure of State
//!         <BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p); // ONE call
//!     (o, Image { mask })                  // restructure
//! }
//! ```
//!
//! The wrapper computes nothing: what it returns is what the production code
//! returns, so the code the evidence runs **is** the code that ships.  Four
//! rules keep that true, and this file checks them:
//!
//! * **SH13** the ghost module is behind the feature, the feature is not a
//!   default feature, and no `alpha` or `vos_ghost_*` function lives outside it;
//! * **SH14** (field-privacy half) a component's `State` struct has no `pub`
//!   field outside the ghost module;
//! * **SH17** the wrappers have exactly the wrapper shape and the `Image`
//!   mirrors `State` field for field, so a field added to `State` and forgotten
//!   in the image cannot go unseen;
//! * **SH18** nothing ghost leaks into the shipped build: every use of an
//!   evidence feature is the one gated `mod ghost` (or its gated `pub use`),
//!   and no path into `ghost::` appears in non-ghost code.
//!
//! The wrapper shape is checked lexically: exactly one UFCS call
//! `<T as Trait>::f(`, no other call, no branch, no operator, exhaustive struct
//! patterns (no `..`) whose field names equal the production struct's.  The
//! functions of the ghost module that contain *no* UFCS call (`alpha`, the
//! configuration exports) are not wrappers; their bodies are free, and their
//! correctness is the evidence's job, not the lexer's (VOS03 section 4, SH17).

use std::collections::BTreeSet;

use crate::items::TypeDef;
use crate::lexer::{Kind, Token};
use crate::manifest::Kind as VKind;
use crate::report::Rule;
use crate::rules::Ctx;
use crate::syntax::{is_keyword, item_end, matching_lt, scan_to_body, Attr, FileCtx};

const GHOST_PREFIX: &str = "vos_ghost_";

/// The features named by `feature = "F"` pairs inside `toks`.
fn features_in(toks: &[Token]) -> Vec<String> {
    let mut out = Vec::new();
    for w in toks.windows(3) {
        if w[0].is_ident("feature") && w[1].is_punct("=") && w[2].kind == Kind::Str {
            out.push(w[2].text.clone());
        }
    }
    out
}

/// If `a` is exactly `cfg(feature = "F")`, the feature `F`.
fn exact_cfg_feature(f: &FileCtx, a: &Attr) -> Option<String> {
    let b = f.attr_body(a);
    (b.len() == 6
        && b[0].is_ident("cfg")
        && b[1].is_punct("(")
        && b[2].is_ident("feature")
        && b[3].is_punct("=")
        && b[4].kind == Kind::Str
        && b[5].is_punct(")"))
    .then(|| b[4].text.clone())
}

/// True if the item that follows attribute `a` is `mod ghost` or a
/// `pub use ghost::..` / `pub use self::ghost::..`.
fn gates_ghost_item(f: &FileCtx, a: &Attr) -> bool {
    let t = &f.toks;
    let mut j = a.close + 1;
    while t.get(j).is_some_and(|x| x.is_punct("#")) && t.get(j + 1).is_some_and(|x| x.is_punct("["))
    {
        j = f.pair[j + 1] + 1;
    }
    let mut public = false;
    if t.get(j).is_some_and(|x| x.is_ident("pub")) {
        public = true;
        j += 1;
        if t.get(j).is_some_and(|x| x.is_punct("(")) {
            j = f.pair[j] + 1;
        }
    }
    match t.get(j) {
        Some(x) if x.is_ident("mod") => t.get(j + 1).is_some_and(|n| n.is_ident("ghost")),
        Some(x) if x.is_ident("use") && public => {
            let mut k = j + 1;
            if t.get(k).is_some_and(|x| x.is_ident("self"))
                && t.get(k + 1).is_some_and(|x| x.is_punct("::"))
            {
                k += 2;
            }
            t.get(k).is_some_and(|n| n.is_ident("ghost"))
                && t.get(k + 1).is_some_and(|n| n.is_punct("::"))
        }
        _ => false,
    }
}

/// Run SH13, SH14, SH17 and SH18 over the crate.
pub fn ghost_rules(cx: &mut Ctx<'_>) {
    let files = cx.files;
    for f in files {
        cfg_gating(cx, f);
        ghost_leaks(cx, f);
        ghost_module_gating(cx, f);
        stray_ghost_fns(cx, f);
    }
    ghost_without_mod(cx);
    default_features(cx);
    state_privacy(cx);
    wrappers(cx);
}

/// SH18 (a): every `cfg` / `cfg_attr` / `cfg!` naming a feature.
fn cfg_gating(cx: &mut Ctx<'_>, f: &FileCtx) {
    let t = &f.toks;
    let declared = |name: &str| cx.cargo.is_none_or(|c| c.features.contains_key(name));
    let mut uses: Vec<(String, u32, bool, Option<&Attr>)> = Vec::new();
    for a in &f.attrs {
        let body = f.attr_body(a);
        let Some(first) = body.first() else { continue };
        let is_cfg = first.is_ident("cfg");
        if !is_cfg && !first.is_ident("cfg_attr") {
            continue;
        }
        for feat in features_in(body) {
            uses.push((feat, first.line, is_cfg, Some(a)));
        }
    }
    for i in 0..t.len() {
        if !f.in_attr[i]
            && t[i].is_ident("cfg")
            && t.get(i + 1).is_some_and(|x| x.is_punct("!"))
            && t.get(i + 2).is_some_and(|x| x.is_punct("("))
        {
            for feat in features_in(&t[i + 3..f.pair[i + 2]]) {
                uses.push((feat, t[i].line, false, None));
            }
        }
    }
    let mut msgs: Vec<(u32, String)> = Vec::new();
    // EVERY cfg predicate varies the build between the evidence host and the
    // image (`cfg(target_arch = ..)` gives two different crates), so the only
    // cfg a shape-checked crate may carry is the exact gate of `mod ghost` /
    // `pub use ghost::..` (the `cfg(test)` items are masked earlier).
    for a in &f.attrs {
        let body = f.attr_body(a);
        let Some(first) = body.first() else { continue };
        if !(first.is_ident("cfg") || first.is_ident("cfg_attr")) {
            continue;
        }
        let gate_ok = exact_cfg_feature(f, a).is_some_and(|ft| cx.evidence.contains(&ft))
            && gates_ghost_item(f, a);
        if !gate_ok {
            msgs.push((
                first.line,
                format!(
                    "`{}` varies the build; only `#[cfg(feature = \"ghost\")]` on `mod ghost` or `pub use ghost::..` is allowed",
                    first.text
                ),
            ));
        }
    }
    for i in 0..t.len() {
        if !f.in_attr[i] && t[i].is_ident("cfg") && t.get(i + 1).is_some_and(|x| x.is_punct("!")) {
            msgs.push((
                t[i].line,
                "`cfg!` varies the build between the evidence host and the image".to_string(),
            ));
        }
    }
    for (feat, line, plain_cfg, attr) in uses {
        if !declared(&feat) {
            msgs.push((
                line,
                format!("feature `{feat}` is not declared in Cargo.toml [features]"),
            ));
        }
        if cx.evidence.contains(&feat) {
            let ok = plain_cfg
                && attr.is_some_and(|a| {
                    exact_cfg_feature(f, a).as_deref() == Some(feat.as_str())
                        && gates_ghost_item(f, a)
                });
            if !ok {
                msgs.push((
                    line,
                    format!(
                        "evidence feature `{feat}` may appear only as `#[cfg(feature = \"{feat}\")]` on `mod ghost` or `pub use ghost::..` (not in cfg_attr, cfg!, not(..) or a combination, and nowhere else)"
                    ),
                ));
            }
        } else if cx.class == crate::manifest::Class::K {
            msgs.push((
                line,
                format!("feature `{feat}` gates code but is not an evidence feature; class K code does not vary by feature"),
            ));
        }
    }
    for (line, m) in msgs {
        cx.add(Rule::Sh18, &f.path, line, m);
    }
}

/// SH18 (b): `ghost::` paths, `use` of ghost items and `vos_ghost_*` names
/// outside the ghost scope, except inside a gated `pub use ghost::..`.
fn ghost_leaks(cx: &mut Ctx<'_>, f: &FileCtx) {
    let t = &f.toks;
    // Token ranges of gated `pub use ghost::..` items.
    let mut allowed = vec![false; t.len()];
    let mut covered = 0usize;
    for i in 0..t.len() {
        if i < covered || f.in_attr[i] || !t[i].is_ident("use") || f.in_ghost(i) {
            continue;
        }
        let attrs = f.attrs_before(i);
        let gated = attrs.iter().any(|a| {
            exact_cfg_feature(f, a).is_some_and(|ft| cx.evidence.contains(&ft))
                && gates_ghost_item(f, a)
        });
        if gated && i > 0 && (t[i - 1].is_ident("pub") || t[i - 1].is_punct(")")) {
            let end = item_end(t, &f.pair, i);
            covered = end + 1;
            for a in &mut allowed[i..=end.min(t.len() - 1)] {
                *a = true;
            }
        }
    }
    for i in 0..t.len() {
        if f.in_attr[i] || f.in_ghost(i) || t[i].kind != Kind::Ident {
            continue;
        }
        if allowed[i] {
            continue;
        }
        let prev_mod = i > 0 && t[i - 1].is_ident("mod");
        if t[i].text == "ghost" && !prev_mod {
            cx.add(
                Rule::Sh18,
                &f.path,
                t[i].line,
                "`ghost` named outside the ghost module; the ghost face must not be reachable from shipped code (only `mod ghost` and a gated `pub use ghost::..` may name it)",
            );
        } else if t[i].text.starts_with(GHOST_PREFIX) {
            cx.add(
                Rule::Sh18,
                &f.path,
                t[i].line,
                format!("ghost item `{}` named outside the ghost module", t[i].text),
            );
        }
    }
}

/// SH13 (a): each `mod ghost` carries exactly the gating attribute.
fn ghost_module_gating(cx: &mut Ctx<'_>, f: &FileCtx) {
    for g in &f.ghost_mods {
        let gated = f
            .attrs_before(g.mod_idx)
            .iter()
            .any(|a| exact_cfg_feature(f, a).is_some_and(|ft| cx.evidence.contains(&ft)));
        if !gated {
            cx.add(
                Rule::Sh13,
                &f.path,
                f.toks[g.mod_idx].line,
                "`mod ghost` must be preceded by `#[cfg(feature = \"ghost\")]` (an evidence feature): alpha and the state builder are evidence-only",
            );
        }
    }
}

/// SH13 (b): `alpha*` and `vos_ghost_*` functions belong inside the ghost
/// module.
fn stray_ghost_fns(cx: &mut Ctx<'_>, f: &FileCtx) {
    let t = &f.toks;
    for i in 0..t.len() {
        if f.in_attr[i] || f.in_ghost(i) || !t[i].is_ident("fn") {
            continue;
        }
        let Some(n) = t.get(i + 1).filter(|x| x.kind == Kind::Ident) else {
            continue;
        };
        if n.text == "alpha" || n.text.starts_with("alpha_") || n.text.starts_with(GHOST_PREFIX) {
            cx.add(
                Rule::Sh13,
                &f.path,
                n.line,
                format!(
                    "`fn {}` is evidence-only and must live in the `ghost` module",
                    n.text
                ),
            );
        }
    }
}

/// SH13: a `ghost` file with no gated `mod ghost` to put it behind a feature.
fn ghost_without_mod(cx: &mut Ctx<'_>) {
    let files = cx.files;
    let any_mod = files.iter().any(|f| !f.ghost_mods.is_empty());
    if !any_mod {
        for f in files.iter().filter(|f| f.ghost_file) {
            cx.add(
                Rule::Sh13,
                &f.path,
                1,
                "ghost source file but no `mod ghost` item declares it behind the evidence feature",
            );
        }
    }
}

/// SH13 (c): no evidence feature is reachable from `default`.
fn default_features(cx: &mut Ctx<'_>) {
    let Some(c) = cx.cargo else { return };
    let Some((line, _)) = c.features.get("default") else {
        return;
    };
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut stack: Vec<&str> = vec!["default"];
    while let Some(n) = stack.pop() {
        if !seen.insert(n) {
            continue;
        }
        if let Some((_, items)) = c.features.get(n) {
            for it in items {
                if c.features.contains_key(it.as_str()) {
                    stack.push(it.as_str());
                }
            }
        }
    }
    let line = *line;
    let hit: Vec<String> = seen
        .iter()
        .filter(|n| cx.evidence.contains(**n))
        .map(|n| (*n).to_string())
        .collect();
    for n in hit {
        cx.add(
            Rule::Sh13,
            "Cargo.toml",
            line,
            format!("evidence feature `{n}` is reachable from `default`; the image build must not enable it"),
        );
    }
}

/// SH14, field-privacy half.
fn state_privacy(cx: &mut Ctx<'_>) {
    // A component implementation keeps its state private.  An UNSPECIFIED kind
    // in class K is treated as `production`: a missing vos.toml must not turn
    // the rule off (fail closed).
    let component = matches!(
        cx.kind,
        Some(VKind::Production | VKind::Reference | VKind::Any)
    ) || (cx.kind.is_none() && cx.class == crate::manifest::Class::K);
    if !component {
        return;
    }
    let files = cx.files;
    let mut msgs = Vec::new();
    for td in cx
        .types
        .iter()
        .filter(|t| (t.keyword == "struct" || t.keyword == "union") && !t.in_ghost)
    {
        for fld in td.fields.iter().filter(|x| x.public) {
            msgs.push((
                files[td.file].path.clone(),
                fld.line,
                format!(
                    "field `{}` of `{}` is `pub`; a component's state fields are private (rule 2)",
                    fld.name, td.name
                ),
            ));
        }
    }
    for (p, l, m) in msgs {
        cx.add(Rule::Sh14, &p, l, m);
    }
}

/// Field names in a `{ .. }` pattern or literal, and whether `..` appears.
fn braced_names(f: &FileCtx, open: usize) -> (Vec<String>, bool) {
    let t = &f.toks;
    let close = f.pair[open];
    let mut names = Vec::new();
    let mut rest = false;
    let mut j = open + 1;
    let mut fresh = true;
    while j < close {
        if t[j].is_punct("..") {
            rest = true;
        }
        if fresh && t[j].kind == Kind::Ident && !matches!(t[j].text.as_str(), "ref" | "mut") {
            names.push(t[j].text.clone());
            fresh = false;
        }
        if t[j].kind == Kind::Punct && matches!(t[j].text.as_str(), "(" | "[" | "{") {
            j = f.pair[j];
        } else if t[j].is_punct(",") {
            fresh = true;
        }
        j += 1;
    }
    (names, rest)
}

fn find_type<'a>(types: &'a [TypeDef], name: &str, ghost: bool) -> Option<&'a TypeDef> {
    types
        .iter()
        .find(|d| d.keyword == "struct" && d.name == name && d.in_ghost == ghost)
}

/// SH17: prefixes, wrapper shape, exhaustive destructures, image mirrors.
fn wrappers(cx: &mut Ctx<'_>) {
    let files = cx.files;
    let types = cx.types.clone();
    let mut paired: BTreeSet<String> = BTreeSet::new();
    let mut msgs: Vec<(String, u32, String)> = Vec::new();
    for f in files {
        let t = &f.toks;
        reexport_prefixes(f, &mut msgs);
        for i in 0..t.len() {
            if f.in_attr[i] || !f.in_ghost(i) || !t[i].is_ident("fn") {
                continue;
            }
            let Some(name) = t.get(i + 1).filter(|x| x.kind == Kind::Ident) else {
                continue;
            };
            if !name.text.starts_with(GHOST_PREFIX) {
                msgs.push((
                    f.path.clone(),
                    name.line,
                    format!(
                        "ghost function `{}` must be named with the prefix `{GHOST_PREFIX}`",
                        name.text
                    ),
                ));
            }
            let b = scan_to_body(t, &f.pair, i + 2);
            if b < t.len() && !t[b].is_punct("{") && !t[b].is_punct(";") {
                // the header scan gave up: do not let a long signature skip the
                // wrapper checks (fail closed)
                msgs.push((
                    f.path.clone(),
                    name.line,
                    format!(
                        "ghost function `{}` has a header too long to check",
                        name.text
                    ),
                ));
            }
            if b >= t.len() || !t[b].is_punct("{") {
                continue;
            }
            wrapper_body(f, &types, name, b, &mut paired, &mut msgs);
        }
    }
    for td in types
        .iter()
        .filter(|d| d.keyword == "struct" && d.in_ghost && d.name == "Image")
    {
        if !paired.contains(&td.name) {
            msgs.push((
                files[td.file].path.clone(),
                td.line,
                "ghost `Image` is not built by any wrapper that destructures a production struct, so it cannot be checked against `State`".to_string(),
            ));
        }
    }
    for (p, l, m) in msgs {
        cx.add(Rule::Sh17, &p, l, m);
    }
}

/// SH17 (a), re-export half: every leaf of `pub use ghost::..` is prefixed.
fn reexport_prefixes(f: &FileCtx, msgs: &mut Vec<(String, u32, String)>) {
    let t = &f.toks;
    let mut covered = 0usize;
    for i in 0..t.len() {
        if i < covered || f.in_attr[i] || f.in_ghost(i) || !t[i].is_ident("use") {
            continue;
        }
        let mut k = i + 1;
        if t.get(k).is_some_and(|x| x.is_ident("self"))
            && t.get(k + 1).is_some_and(|x| x.is_punct("::"))
        {
            k += 2;
        }
        if !(t.get(k).is_some_and(|x| x.is_ident("ghost"))
            && t.get(k + 1).is_some_and(|x| x.is_punct("::")))
        {
            continue;
        }
        let end = item_end(t, &f.pair, i);
        covered = end + 1;
        let mut j = k + 2;
        while j <= end && j < t.len() {
            let x = &t[j];
            if x.is_punct("*") {
                msgs.push((f.path.clone(), x.line, "glob re-export `ghost::*` cannot be checked for the `vos_ghost_` prefix; name each item".to_string()));
            } else if x.kind == Kind::Ident && x.text != "self" && x.text != "as" {
                if t.get(j + 1).is_some_and(|n| n.is_ident("as")) {
                    if let Some(alias) = t.get(j + 2) {
                        if !alias.text.starts_with(GHOST_PREFIX) {
                            msgs.push((
                                f.path.clone(),
                                alias.line,
                                format!(
                                    "re-export `{}` must be named with the prefix `{GHOST_PREFIX}`",
                                    alias.text
                                ),
                            ));
                        }
                    }
                    j += 2;
                } else if t
                    .get(j + 1)
                    .is_some_and(|n| n.is_punct(",") || n.is_punct("}") || n.is_punct(";"))
                    && !x.text.starts_with(GHOST_PREFIX)
                {
                    msgs.push((
                        f.path.clone(),
                        x.line,
                        format!(
                            "re-export `{}` must be named with the prefix `{GHOST_PREFIX}`",
                            x.text
                        ),
                    ));
                }
            }
            j += 1;
        }
    }
}

/// SH17 (b), (c), (d) for one ghost function body starting at `{` index `b`.
fn wrapper_body(
    f: &FileCtx,
    types: &[TypeDef],
    name: &Token,
    b: usize,
    paired: &mut BTreeSet<String>,
    msgs: &mut Vec<(String, u32, String)>,
) {
    let t = &f.toks;
    let end = f.pair[b];
    // UFCS calls: `< .. as .. > :: f (`.
    let mut ufcs: Vec<(usize, usize, usize)> = Vec::new();
    for k in b + 1..end {
        if t[k].is_punct(">")
            && t.get(k + 1).is_some_and(|x| x.is_punct("::"))
            && t.get(k + 2).is_some_and(|x| x.kind == Kind::Ident)
            && t.get(k + 3).is_some_and(|x| x.is_punct("("))
        {
            if let Some(open) = matching_lt(t, k) {
                let mut depth = 1usize;
                let mut has_as = false;
                for x in &t[open + 1..k] {
                    if x.is_punct("<") {
                        depth += 1;
                    } else if x.is_punct(">") {
                        depth = depth.saturating_sub(1);
                    } else if depth == 1 && x.is_ident("as") {
                        has_as = true;
                    }
                }
                if has_as {
                    ufcs.push((open, k, k + 2));
                }
            }
        }
    }
    let line = name.line;
    let fname = &name.text;
    let path = &f.path;
    let mut bad = |l: u32, m: String| msgs.push((path.clone(), l, m));
    if ufcs.is_empty() {
        // Only a CLOSED list of exports may have a free body: the abstraction
        // `alpha` (and the interim `alpha_w`) and the configuration `cfg_*`
        // exports of VOS02 section 4.4.  Every other ghost function is a
        // wrapper and MUST contain its one UFCS call; otherwise it could
        // reimplement the operation and the evidence would cover a lookalike.
        let export = fname == "vos_ghost_alpha"
            || fname.starts_with("vos_ghost_alpha_")
            || fname.starts_with("vos_ghost_cfg_");
        if !export {
            bad(
                line,
                format!("ghost function `{fname}` has no UFCS call `<T as Trait>::f(..)`; only `vos_ghost_alpha*` and `vos_ghost_cfg_*` may have a free body, every other ghost function is a wrapper"),
            );
        }
        return;
    }
    if ufcs.len() != 1 {
        bad(line, format!("wrapper `{fname}` must contain exactly one UFCS call `<T as Trait>::f(..)`, found {}", ufcs.len()));
    }
    let in_ufcs_group = |k: usize| ufcs.iter().any(|&(o, c, _)| k >= o && k <= c);
    let callee: BTreeSet<usize> = ufcs.iter().map(|&(_, _, c)| c).collect();
    // let-pattern ranges: `let PATTERN = ..`
    let mut let_ranges: Vec<(usize, usize)> = Vec::new();
    for k in b + 1..end {
        if t[k].is_ident("let") {
            let mut e = k + 1;
            while e < end && !t[e].is_punct("=") && !t[e].is_punct(";") && !t[e].is_ident("let") {
                if t[e].kind == Kind::Punct && matches!(t[e].text.as_str(), "(" | "[" | "{") {
                    e = f.pair[e];
                }
                e += 1;
            }
            let_ranges.push((k + 1, e));
        }
    }
    let in_let = |k: usize| let_ranges.iter().any(|&(a, e)| k >= a && k < e);
    for k in b + 1..end {
        let x = &t[k];
        let nxt = t.get(k + 1);
        if x.kind == Kind::Ident {
            let call = nxt.is_some_and(|n| n.is_punct("(") || n.is_punct("!"));
            if call && !callee.contains(&k) && !is_keyword(&x.text) {
                bad(
                    x.line,
                    format!(
                        "wrapper `{fname}` makes a call other than its one UFCS call (`{}`)",
                        x.text
                    ),
                );
            }
            if matches!(
                x.text.as_str(),
                "if" | "else"
                    | "match"
                    | "while"
                    | "loop"
                    | "for"
                    | "return"
                    | "break"
                    | "continue"
            ) || (x.text == "as" && !in_ufcs_group(k))
            {
                bad(x.line, format!("wrapper `{fname}` computes (`{}`); it may only destructure, call once and restructure", x.text));
            }
        } else if x.kind == Kind::Punct {
            let op = matches!(
                x.text.as_str(),
                "+" | "-" | "*" | "/" | "%" | "^" | "|" | "&" | "!" | "?" | ".."
            ) || ((x.text == "<" || x.text == ">") && !in_ufcs_group(k));
            // `.method(` is a call
            if x.is_punct(".")
                && t.get(k + 1).is_some_and(|n| n.kind == Kind::Ident)
                && t.get(k + 2).is_some_and(|n| n.is_punct("("))
            {
                bad(
                    x.line,
                    format!(
                        "wrapper `{fname}` makes a method call `.{}(`",
                        t[k + 1].text
                    ),
                );
            }
            if op && !(x.text == ".." && in_let(k)) {
                bad(
                    x.line,
                    format!("wrapper `{fname}` computes (operator `{}`)", x.text),
                );
            }
        }
    }
    // Exhaustive destructure: struct patterns in `let` patterns.
    let mut production: Option<&TypeDef> = None;
    for &(a, e) in &let_ranges {
        for k in a..e {
            if t[k].kind == Kind::Ident && t.get(k + 1).is_some_and(|n| n.is_punct("{")) {
                let (names, rest) = braced_names(f, k + 1);
                if rest {
                    bad(t[k].line, format!("destructure of `{}` in `{fname}` uses `..`; it must be exhaustive so a new field cannot be forgotten", t[k].text));
                }
                if let Some(s) = find_type(types, &t[k].text, false) {
                    production.get_or_insert(s);
                    let want: Vec<&str> = s.fields.iter().map(|x| x.name.as_str()).collect();
                    let missing: Vec<&str> = want
                        .iter()
                        .copied()
                        .filter(|w| !names.iter().any(|n| n == w))
                        .collect();
                    let extra: Vec<&str> = names
                        .iter()
                        .map(String::as_str)
                        .filter(|n| !want.contains(n))
                        .collect();
                    if !missing.is_empty() || !extra.is_empty() {
                        bad(t[k].line, format!("destructure of `{}` in `{fname}` does not list exactly its fields (missing: {}; unknown: {})", s.name, missing.join(", "), extra.join(", ")));
                    }
                }
            }
        }
    }
    // Image mirrors: ghost structs built in the body.
    if let Some(s) = production {
        for k in b + 1..end {
            if in_let(k)
                || t[k].kind != Kind::Ident
                || !t.get(k + 1).is_some_and(|n| n.is_punct("{"))
            {
                continue;
            }
            if let Some(img) = find_type(types, &t[k].text, true) {
                paired.insert(img.name.clone());
                let same = img.fields.len() == s.fields.len()
                    && img
                        .fields
                        .iter()
                        .zip(&s.fields)
                        .all(|(a, c)| a.name == c.name && a.ty == c.ty);
                if !same {
                    let show = |d: &TypeDef| {
                        d.fields
                            .iter()
                            .map(|x| format!("{}: {}", x.name, x.ty))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    bad(img.line, format!("ghost `{}` does not mirror `{}` (same field names, types and order required): `{}` has [{}], `{}` has [{}]", img.name, s.name, img.name, show(img), s.name, show(s)));
                }
            }
        }
    }
}
