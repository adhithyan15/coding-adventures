//! # The token rules: SH1-SH4, SH6-SH11, SH15 (source half)
//!
//! Every rule below protects one promise of the verified OS: a *component* is
//! a total, bounded, by-value step function that compiles, unchanged, to the
//! RV32 image and to the host evidence build, and means the same thing in
//! both.  VOS01 section 1.4 lists the rules; this file is how a lexer sees
//! them.  Each function names the rule it enforces and says, in one line, what
//! disaster the rule prevents.
//!
//! ```text
//!   SH1   no_std, no alloc, no std      -> no heap, no OS authority
//!   SH2   no unsafe / asm / FFI         -> memory safety is a type fact
//!   SH3   no static, no Cell in a const -> no global state (rule 2)
//!   SH4   no dyn, fn pointers, closures -> every call target is nameable
//!   SH6   only `for i in 0..K`          -> every loop bound is a constant
//!   SH7   no division                   -> RV32I has none; no trap path
//!   SH8   no panic path, no raw index   -> a panic is an uncontracted exit
//!   SH9   shifts are literal or wrapped -> debug and release agree
//!   SH10  fixed-width types only        -> 64-bit host == 32-bit image
//!   SH11  no floats                     -> no FPU, no host rounding
//!   SH15  no macros / includes / build  -> no code the lexer cannot see
//! ```
//!
//! The recursion rule SH5 is in [`crate::graph`], the ghost-face rules SH13,
//! SH14, SH17 and SH18 in [`crate::ghost`], and the manifest rules SH12 and the
//! `Cargo.toml` half of SH15 in [`crate::cargo_rules`].
//!
//! ## False positives are cheaper than false negatives
//!
//! Where a token is ambiguous (`>>` closing generics or shifting, `|` as
//! closure or "or") we pick the reading that *reports*, unless the
//! unambiguous context says otherwise.  A contributor rewrites a line; nobody
//! reviews a violation the check silently chose to ignore.

use std::collections::BTreeSet;

use crate::items::TypeDef;
use crate::lexer::{Kind, Token};
use crate::manifest::{CargoFacts, Class, Kind as VKind};
use crate::report::{Finding, Rule};
use crate::syntax::{angle_close, is_const_name, is_keyword, item_end, FileCtx};

/// Everything a rule needs, plus the findings it appends to.
pub struct Ctx<'a> {
    /// The crate class that selects the rules.
    pub class: Class,
    /// The declared kind, if any.
    pub kind: Option<VKind>,
    /// All lexed source files, sorted by path.
    pub files: &'a [FileCtx],
    /// Every type definition of the crate.
    pub types: Vec<TypeDef>,
    /// Facts read from `Cargo.toml` (absent if it did not parse).
    pub cargo: Option<&'a CargoFacts>,
    /// Evidence features: the central list plus any declared in `vos.toml`.
    pub evidence: &'a BTreeSet<String>,
    /// Findings so far (unsorted).
    pub out: Vec<Finding>,
}

/// Which rules bind which class: VOS01 section 1.6 as one function.  A change
/// to that table is a one-line edit here.
pub fn binds(class: Class, rule: Rule) -> bool {
    match rule {
        Rule::Vt01 | Rule::Vt02 | Rule::Vt03 => true,
        _ => match class {
            Class::K => true,
            Class::C | Class::S => !matches!(rule, Rule::Sh1 | Rule::Sh12),
            Class::M => !matches!(rule, Rule::Sh1 | Rule::Sh10 | Rule::Sh12),
            Class::P | Class::H => false,
        },
    }
}

impl Ctx<'_> {
    /// Record a finding if the class is bound by `rule`.
    pub fn add(&mut self, rule: Rule, file: &str, line: u32, message: impl Into<String>) {
        if binds(self.class, rule) {
            self.out.push(Finding {
                rule,
                file: file.to_string(),
                line,
                message: message.into(),
            });
        }
    }
}

const PANIC_MACROS: &[&str] = &[
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
];
const PANIC_METHODS: &[&str] = &[
    "unwrap",
    "expect",
    "unwrap_err",
    "expect_err",
    "unwrap_unchecked",
];
const ASM_MACROS: &[&str] = &["asm", "global_asm", "naked_asm", "llvm_asm"];
const CORE_MACROS: &[&str] = &[
    "matches",
    "cfg",
    "compile_error",
    "concat",
    "stringify",
    "line",
    "column",
    "file",
    "module_path",
];
const HEAP_TYPES: &[&str] = &[
    "Vec",
    "String",
    "Box",
    "Rc",
    "Arc",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "VecDeque",
    "BinaryHeap",
];
const ITER_METHODS: &[&str] = &[
    "iter",
    "iter_mut",
    "into_iter",
    "map",
    "filter",
    "filter_map",
    "flat_map",
    "flatten",
    "zip",
    "enumerate",
    "rev",
    "fold",
    "try_fold",
    "collect",
    "step_by",
    "skip",
    "skip_while",
    "take_while",
    "chain",
    "sum",
    "product",
    "any",
    "all",
    "for_each",
    "windows",
    "chunks",
    "chunks_exact",
    "position",
    "find_map",
    "peekable",
    "scan",
    "cycle",
    "nth",
];
const IM_BASE: &[&str] = &[
    "Cell",
    "RefCell",
    "UnsafeCell",
    "OnceCell",
    "LazyCell",
    "OnceLock",
    "LazyLock",
    "Mutex",
    "RwLock",
    "SyncUnsafeCell",
];
const INT_SUFFIXES: &[&str] = &[
    "", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

/// True for the standard-library interior-mutability names of SH3.
fn is_interior_name(s: &str) -> bool {
    IM_BASE.contains(&s) || s.starts_with("Atomic")
}

/// The interior-mutability names of SH3: the base list, `Atomic*`, and every
/// type of the crate whose definition mentions one (to a fixed point, so a
/// wrapper struct around a `Cell` is caught).
pub fn interior_mutable(types: &[TypeDef]) -> BTreeSet<String> {
    let is_base = is_interior_name;
    let mut set: BTreeSet<String> = BTreeSet::new();
    loop {
        let mut changed = false;
        for td in types {
            if !set.contains(&td.name)
                && td
                    .idents
                    .iter()
                    .any(|x| is_base(x) || set.contains(x.as_str()))
            {
                set.insert(td.name.clone());
                changed = true;
            }
        }
        if !changed {
            return set;
        }
    }
}

/// True if `text` is the integer zero (any spelling of decimal 0 with an
/// optional integer suffix; `0x10` is not zero).
fn is_zero_literal(text: &str) -> bool {
    let digits: String = text
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '_')
        .collect();
    let rest = &text[digits.len()..];
    !digits.is_empty()
        && digits.chars().all(|c| c == '0' || c == '_')
        && INT_SUFFIXES.contains(&rest)
}

fn is_call_next(t: &[Token], i: usize) -> bool {
    t.get(i + 1).is_some_and(|x| x.is_punct("("))
        || (t.get(i + 1).is_some_and(|x| x.is_punct("::"))
            && t.get(i + 2).is_some_and(|x| x.is_punct("<")))
}

/// `name!` followed by an opening delimiter, not `name != x`.
fn is_macro_call(t: &[Token], i: usize) -> bool {
    t.get(i + 1).is_some_and(|x| x.is_punct("!"))
        && t.get(i + 2)
            .is_some_and(|x| x.is_punct("(") || x.is_punct("[") || x.is_punct("{"))
}

/// One loop `for ... in ...`, classified.
struct LoopInfo {
    /// The loop variable if the pattern is a single identifier.
    var: Option<String>,
    /// Token range inside the body braces.
    body: (usize, usize),
}

/// SH6: find every `for` loop, report the ones that are not `for i in 0..K`
/// and return the well-formed ones (used by the SH8 index rule).
fn for_loops(cx: &mut Ctx<'_>, f: &FileCtx) -> Vec<LoopInfo> {
    let t = &f.toks;
    let mut good = Vec::new();
    for i in 0..t.len() {
        if f.in_attr[i] || !t[i].is_ident("for") || t.get(i + 1).is_some_and(|x| x.is_punct("<")) {
            continue;
        }
        // Find `in` before any `{` or `;`; otherwise this is `impl X for Y`.
        let mut k = i + 1;
        let mut in_at = None;
        while k < t.len() && k < i + 64 {
            if t[k].kind == Kind::Punct {
                match t[k].text.as_str() {
                    "(" | "[" => k = f.pair[k],
                    "{" | ";" => break,
                    _ => {}
                }
            } else if t[k].is_ident("in") {
                in_at = Some(k);
                break;
            }
            k += 1;
        }
        let Some(p) = in_at else { continue };
        let pattern = &t[i + 1..p];
        let var =
            (pattern.len() == 1 && pattern[0].kind == Kind::Ident).then(|| pattern[0].text.clone());
        // The range expression runs to the body brace.
        let mut j = p + 1;
        while j < t.len() && !t[j].is_punct("{") && !t[j].is_punct(";") {
            if matches!(t[j].text.as_str(), "(" | "[") && t[j].kind == Kind::Punct {
                j = f.pair[j];
            }
            j += 1;
        }
        let ok_shape = j < t.len() && t[j].is_punct("{") && {
            let r = &t[p + 1..j];
            r.len() >= 3
                && r[0].kind == Kind::Int
                && is_zero_literal(&r[0].text)
                && r[1].is_punct("..")
                && {
                    let bound = &r[2..];
                    let literal = bound.len() == 1 && bound[0].kind == Kind::Int;
                    let path = bound.len() % 2 == 1
                        && bound.iter().enumerate().all(|(n, x)| {
                            if n % 2 == 0 {
                                x.kind == Kind::Ident
                            } else {
                                x.is_punct("::")
                            }
                        })
                        && is_const_name(&bound[bound.len() - 1].text);
                    literal || path
                }
        };
        if ok_shape {
            good.push(LoopInfo {
                var,
                body: (j + 1, f.pair[j]),
            });
        } else {
            cx.add(
                Rule::Sh6,
                &f.path,
                t[i].line,
                "`for` loop is not of the form `for i in 0..K` with K a literal or a const path",
            );
        }
    }
    good
}

/// `const _: () = ...;` item ranges, where `assert!` is a compile-time check.
fn const_underscore_ranges(f: &FileCtx) -> Vec<(usize, usize)> {
    let t = &f.toks;
    let mut out = Vec::new();
    for i in 0..t.len() {
        if !f.in_attr[i]
            && t[i].is_ident("const")
            && t.get(i + 1).is_some_and(|x| x.is_ident("_"))
            && t.get(i + 2).is_some_and(|x| x.is_punct(":"))
        {
            out.push((i, item_end(t, &f.pair, i)));
        }
    }
    out
}

/// Is the token before a `|` one after which a closure can start?
fn closure_can_follow(prev: Option<&Token>) -> bool {
    match prev {
        None => true,
        Some(p) if p.kind == Kind::Punct => matches!(
            p.text.as_str(),
            "(" | "," | "=" | "{" | ";" | "[" | ":" | "=>" | "->"
        ),
        Some(p) => matches!(p.text.as_str(), "return" | "move" | "async") && p.kind == Kind::Ident,
    }
}

/// Run every token rule over one file.
pub fn file_rules(cx: &mut Ctx<'_>, f: &FileCtx, im: &BTreeSet<String>) {
    let t = &f.toks;
    let loops = for_loops(cx, f);
    let const_us = const_underscore_ranges(f);
    let mut skip_to = 0usize;
    for i in 0..t.len() {
        if f.in_attr[i] || i < skip_to {
            continue;
        }
        let tok = &t[i];
        let line = tok.line;
        let prev = i.checked_sub(1).map(|p| &t[p]);
        let prev_dot = prev.is_some_and(|p| p.is_punct("."));
        match tok.kind {
            Kind::Float => {
                cx.add(
                    Rule::Sh11,
                    &f.path,
                    line,
                    format!("float literal `{}`", tok.text),
                );
            }
            Kind::Punct => {
                punct_rules(cx, f, i, &loops, &mut skip_to);
            }
            Kind::Ident => {
                ident_rules(cx, f, i, prev_dot, &const_us, im);
            }
            _ => {}
        }
    }
}

fn punct_rules(cx: &mut Ctx<'_>, f: &FileCtx, i: usize, loops: &[LoopInfo], skip_to: &mut usize) {
    let t = &f.toks;
    let tok = &t[i];
    let line = tok.line;
    let next = t.get(i + 1);
    match tok.text.as_str() {
        // SH7: RV32I has no divide instruction.
        "/" | "%" => {
            let op = if tok.text == "/" { "/" } else { "%" };
            cx.add(
                Rule::Sh7,
                &f.path,
                line,
                format!("division operator `{op}`; RV32I has no divide (use shifts and masks)"),
            );
        }
        // SH9: shifts.  `<<`/`>>` are two joint single-character tokens.
        "<" | ">" => {
            let shift =
                tok.joint && next.is_some_and(|n| n.text == tok.text && n.kind == Kind::Punct);
            if !shift {
                return;
            }
            // Whether this pair is a shift or closes generics, do not
            // re-read its second character as the start of another pair
            // (`Into<Vec<u8>>>` must not look like `>> ident`).
            *skip_to = i + 2;
            let prev_colons = i > 0 && t[i - 1].is_punct("::");
            let compound =
                t.get(i + 2).is_some_and(|x| x.is_punct("=")) && next.is_some_and(|n| n.joint);
            let after = if compound { t.get(i + 3) } else { t.get(i + 2) };
            let is_shift = if tok.text == "<" {
                let generic_qself = t.get(i + 2).is_some_and(|x| x.kind == Kind::Ident)
                    && t.get(i + 3).is_some_and(|x| x.is_ident("as"));
                !prev_colons && !generic_qself
            } else {
                compound
                    || after.is_some_and(|a| {
                        matches!(a.kind, Kind::Int | Kind::Float)
                            || a.is_punct("(")
                            || (a.kind == Kind::Ident
                                && !matches!(a.text.as_str(), "where" | "as" | "for" | "in"))
                    })
            };
            if is_shift && !after.is_some_and(|a| a.kind == Kind::Int) {
                cx.add(
                    Rule::Sh9,
                    &f.path,
                    line,
                    format!(
                        "shift `{}{}` by a non-literal amount; use wrapping_shl/shr or checked_shl/shr",
                        tok.text, tok.text
                    ),
                );
            }
        }
        // SH4: closures.  `|x|` and `||`.
        "|" => {
            let prev = i.checked_sub(1).map(|p| &t[p]);
            if closure_can_follow(prev) {
                if tok.joint && next.is_some_and(|n| n.is_punct("|")) {
                    *skip_to = i + 2;
                }
                cx.add(
                    Rule::Sh4,
                    &f.path,
                    line,
                    "closure; an indirect call the image proof cannot follow",
                );
            }
        }
        // SH8: indexing.
        "[" => {
            let Some(p) = i.checked_sub(1).map(|p| &t[p]) else {
                return;
            };
            let indexes = (p.kind == Kind::Ident && !is_keyword(&p.text))
                || p.is_punct(")")
                || p.is_punct("]");
            if !indexes {
                return;
            }
            let inner = &t[i + 1..f.pair[i]];
            let allowed = inner.len() == 1
                && inner[0].kind == Kind::Ident
                && loops.iter().any(|l| {
                    l.var.as_deref() == Some(inner[0].text.as_str())
                        && i >= l.body.0
                        && i < l.body.1
                });
            if !allowed {
                cx.add(
                    Rule::Sh8,
                    &f.path,
                    line,
                    "indexing can panic; index only by the variable of an enclosing `for v in 0..K`, otherwise use `get`",
                );
            }
        }
        // SH10: a reference with an explicit lifetime is always a type.
        "&" if next.is_some_and(|n| n.kind == Kind::Lifetime) => {
            cx.add(
                Rule::Sh10,
                &f.path,
                line,
                "reference type; State, In and Out are by-value fixed-width data",
            );
        }
        _ => {}
    }
}

fn ident_rules(
    cx: &mut Ctx<'_>,
    f: &FileCtx,
    i: usize,
    prev_dot: bool,
    const_us: &[(usize, usize)],
    im: &BTreeSet<String>,
) {
    let t = &f.toks;
    let tok = &t[i];
    let line = tok.line;
    let name = tok.text.as_str();
    let next = t.get(i + 1);
    let prev = i.checked_sub(1).map(|p| &t[p]);
    let path = f.path.as_str();

    match name {
        // SH2 / SH1: `unsafe`, foreign linkage, `extern crate`.
        "unsafe" => cx.add(
            Rule::Sh2,
            path,
            line,
            "`unsafe`; memory safety is a type fact and there is no waiver",
        ),
        "extern" => {
            if next.is_some_and(|n| n.is_ident("crate")) {
                cx.add(
                    Rule::Sh1,
                    path,
                    line,
                    "`extern crate`; components depend on `core` only",
                );
            } else {
                cx.add(Rule::Sh2, path, line, "`extern` (foreign linkage)");
            }
        }
        // SH3: global state.
        "static" => cx.add(
            Rule::Sh3,
            path,
            line,
            "`static` item; a component crate holds no global state",
        ),
        // SH4: indirect calls.
        "dyn" => cx.add(
            Rule::Sh4,
            path,
            line,
            "`dyn`; an indirect call the image proof cannot follow",
        ),
        "Fn" | "FnMut" | "FnOnce" => cx.add(
            Rule::Sh4,
            path,
            line,
            format!("`{name}` bound; closures and fn objects are indirect calls"),
        ),
        "move" => {
            if next.is_some_and(|n| n.is_punct("|")) {
                cx.add(
                    Rule::Sh4,
                    path,
                    line,
                    "closure; an indirect call the image proof cannot follow",
                );
            }
        }
        "fn" => {
            if next.is_some_and(|n| n.is_punct("(")) {
                cx.add(
                    Rule::Sh4,
                    path,
                    line,
                    "function-pointer type; an indirect call the image proof cannot follow",
                );
            } else if next.is_some_and(|n| n.kind == Kind::Ident) {
                signature_refs(cx, f, i);
            }
        }
        // SH6: loops.
        "while" | "loop" => cx.add(
            Rule::Sh6,
            path,
            line,
            format!("`{name}` loop; the only loop is `for i in 0..K`"),
        ),
        // SH10: widths and references.
        "usize" | "isize" => {
            let as_cast = prev.is_some_and(|p| p.is_ident("as"));
            let const_item =
                name == "usize" && i >= 3 && t[i - 1].is_punct(":") && t[i - 3].is_ident("const");
            if !(as_cast && name == "usize") && !const_item {
                cx.add(
                    Rule::Sh10,
                    path,
                    line,
                    format!("`{name}` is host-width; allowed only as `as usize` or a `const NAME: usize`"),
                );
            }
        }
        "struct" | "enum" | "union" => type_body_refs(cx, f, i),
        // SH11: floats.
        "f32" | "f64" => cx.add(
            Rule::Sh11,
            path,
            line,
            format!("`{name}`; there is no float unit on RV32I"),
        ),
        // SH1: std and alloc paths.
        "std" | "alloc" => {
            let second_segment = i >= 2 && t[i - 1].is_punct("::") && t[i - 2].kind == Kind::Ident;
            let after_use = prev.is_some_and(|p| p.is_ident("use"));
            if !second_segment && (next.is_some_and(|n| n.is_punct("::")) || after_use) {
                cx.add(
                    Rule::Sh1,
                    path,
                    line,
                    format!("`{name}` path; components use `core` only (no heap, no OS authority)"),
                );
            }
        }
        // SH3: const items of interior-mutable type.
        "const" => const_item(cx, f, i, im),
        _ => {}
    }

    if HEAP_TYPES.contains(&name) {
        cx.add(
            Rule::Sh10,
            path,
            line,
            format!("`{name}`; State, In and Out are fixed-width, constant-length data"),
        );
    }
    if prev_dot && PANIC_METHODS.contains(&name) && is_call_next(t, i) {
        cx.add(
            Rule::Sh8,
            path,
            line,
            format!("`{name}` is a path to a panic"),
        );
    }
    if prev_dot && ITER_METHODS.contains(&name) && is_call_next(t, i) {
        cx.add(
            Rule::Sh6,
            path,
            line,
            format!("iterator method `{name}`; the only loop is `for i in 0..K`"),
        );
    }
    if is_call_next(t, i) && name.split('_').any(|s| s == "div" || s == "rem") {
        cx.add(
            Rule::Sh7,
            path,
            line,
            format!("`{name}` is a division; RV32I has no divide (use shifts and masks)"),
        );
    }
    if name == "macro_rules" && next.is_some_and(|n| n.is_punct("!")) {
        // `macro_rules! name { .. }` has the macro's *name* after the `!`.
        cx.add(
            Rule::Sh15,
            path,
            line,
            "`macro_rules!`; a macro puts code out of a lexical analyzer's sight",
        );
    } else if is_macro_call(t, i) && !is_keyword(name) && tok.kind == Kind::Ident {
        macro_call(cx, f, i, const_us);
    }
}

fn macro_call(cx: &mut Ctx<'_>, f: &FileCtx, i: usize, const_us: &[(usize, usize)]) {
    let tok = &f.toks[i];
    let name = tok.text.as_str();
    let (path, line) = (f.path.as_str(), tok.line);
    if PANIC_MACROS.contains(&name) {
        let compile_time = name == "assert" && const_us.iter().any(|&(a, b)| i >= a && i <= b);
        if !compile_time {
            cx.add(Rule::Sh8, path, line, format!("`{name}!` is a path to a panic (only `assert!` in a `const _` item is allowed)"));
        }
    } else if name == "thread_local" {
        cx.add(
            Rule::Sh3,
            path,
            line,
            "`thread_local!`; a component crate holds no global state",
        );
    } else if ASM_MACROS.contains(&name) {
        cx.add(
            Rule::Sh2,
            path,
            line,
            format!("`{name}!` (inline assembly)"),
        );
    } else if matches!(name, "include" | "include_str" | "include_bytes") {
        cx.add(
            Rule::Sh15,
            path,
            line,
            format!("`{name}!` pulls in a file the lexer does not see"),
        );
    } else if !CORE_MACROS.contains(&name) {
        cx.add(
            Rule::Sh15,
            path,
            line,
            format!("macro `{name}!` is not a pure `core` macro"),
        );
    }
}

/// SH10: every `&` between `fn` and the body (or `;`) is part of a type.
fn signature_refs(cx: &mut Ctx<'_>, f: &FileCtx, fn_idx: usize) {
    let t = &f.toks;
    let mut k = fn_idx + 1;
    while k < t.len() && !t[k].is_punct("{") && !t[k].is_punct(";") {
        if t[k].kind == Kind::Punct {
            match t[k].text.as_str() {
                "(" | "[" => {
                    // scan the group's own tokens for `&` before skipping it
                    for x in &t[k + 1..f.pair[k]] {
                        if x.is_punct("&") {
                            cx.add(
                                Rule::Sh10,
                                &f.path,
                                x.line,
                                "reference type in a signature; State, In and Out are by-value",
                            );
                        }
                    }
                    k = f.pair[k];
                }
                "&" => cx.add(
                    Rule::Sh10,
                    &f.path,
                    t[k].line,
                    "reference type in a signature; State, In and Out are by-value",
                ),
                _ => {}
            }
        }
        k += 1;
    }
}

/// SH10: a `&` inside a `struct`, `enum` or `union` body is a field type.
fn type_body_refs(cx: &mut Ctx<'_>, f: &FileCtx, i: usize) {
    let t = &f.toks;
    if i > 0 && t[i - 1].is_punct(".") {
        return;
    }
    if !t.get(i + 1).is_some_and(|n| n.kind == Kind::Ident) {
        return;
    }
    let mut k = i + 2;
    if t.get(k).is_some_and(|x| x.is_punct("<")) {
        if let Some(c) = angle_close(t, &f.pair, k) {
            k = c + 1;
        }
    }
    while k < t.len() && !t[k].is_punct("{") && !t[k].is_punct("(") && !t[k].is_punct(";") {
        k += 1;
    }
    if k < t.len() && (t[k].is_punct("{") || t[k].is_punct("(")) {
        for x in &t[k + 1..f.pair[k]] {
            if x.is_punct("&") {
                cx.add(
                    Rule::Sh10,
                    &f.path,
                    x.line,
                    "reference field type; State, In and Out are by-value",
                );
            }
        }
    }
}

/// SH3: `const NAME: T = ...;` where `T` names an interior-mutable type.
fn const_item(cx: &mut Ctx<'_>, f: &FileCtx, i: usize, im: &BTreeSet<String>) {
    let t = &f.toks;
    let Some(name) = t.get(i + 1) else { return };
    if name.kind != Kind::Ident
        || matches!(
            name.text.as_str(),
            "fn" | "unsafe" | "async" | "extern" | "mut"
        )
        || !t.get(i + 2).is_some_and(|x| x.is_punct(":"))
    {
        return;
    }
    let mut k = i + 3;
    let mut depth = 0usize;
    while k < t.len() {
        let x = &t[k];
        if x.kind == Kind::Punct {
            match x.text.as_str() {
                "<" => depth += 1,
                ">" => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                "(" | "[" => k = f.pair[k],
                "=" | ";" | "," if depth == 0 => break,
                _ => {}
            }
        } else if x.kind == Kind::Ident
            && (im.contains(x.text.as_str()) || is_interior_name(&x.text))
        {
            cx.add(
                Rule::Sh3,
                &f.path,
                x.line,
                format!(
                    "`const` of interior-mutable type `{}`; a const must be plain data",
                    x.text
                ),
            );
        }
        k += 1;
    }
}
