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

use std::collections::{BTreeMap, BTreeSet};

use crate::items::TypeDef;
use crate::lexer::{Kind, Token};
use crate::manifest::{CargoFacts, Class, Kind as VKind};
use crate::report::{Finding, Rule};
use crate::syntax::{
    angle_close, is_const_name, is_keyword, item_end, scan_header, FileCtx, Header,
};

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
    "count",
    "reduce",
    "find",
];
/// Methods whose overflow or shift-amount behaviour differs between the debug
/// host build and the release image (SH9): `x.pow(n)` panics on overflow in
/// debug, `Shl::shl` shifts unchecked.
const OVERFLOW_METHODS: &[&str] = &["pow", "shl", "shr"];
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
    // Reverse map: identifier -> the definitions that mention it, so each
    // newly interior-mutable name wakes exactly its users (a worklist, linear
    // in the number of mentions, not a quadratic fixed-point sweep).
    let mut users: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (n, td) in types.iter().enumerate() {
        for id in &td.idents {
            users.entry(id.as_str()).or_default().push(n);
        }
    }
    let mut set: BTreeSet<String> = BTreeSet::new();
    let mut work: Vec<String> = Vec::new();
    for td in types {
        if td.idents.iter().any(|x| is_interior_name(x)) && set.insert(td.name.clone()) {
            work.push(td.name.clone());
        }
    }
    while let Some(name) = work.pop() {
        if let Some(defs) = users.get(name.as_str()) {
            for &n in defs {
                if set.insert(types[n].name.clone()) {
                    work.push(types[n].name.clone());
                }
            }
        }
    }
    set
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
    /// First token index at which a `let` re-binds the loop variable
    /// (`usize::MAX` if none): an index after it is not by the loop variable.
    shadow: usize,
}

/// Mark the tokens of every `impl` header (from `impl` to its `{` or `;`):
/// a `for` there is `impl Trait for Type`, not a loop.
fn impl_header_mask(f: &FileCtx) -> Vec<bool> {
    let t = &f.toks;
    let mut mask = vec![false; t.len()];
    for i in 0..t.len() {
        if f.in_attr[i] || !t[i].is_ident("impl") {
            continue;
        }
        let item_start = i == 0
            || (t[i - 1].kind == Kind::Punct
                && matches!(t[i - 1].text.as_str(), ";" | "}" | "{" | "]"))
            || t[i - 1].is_ident("unsafe");
        if !item_start {
            continue;
        }
        let mut k = i;
        while k < t.len() {
            let x = &t[k];
            if x.kind == Kind::Punct {
                match x.text.as_str() {
                    "(" | "[" => {
                        mask[k] = true;
                        k = f.pair[k];
                    }
                    "{" | ";" => break,
                    _ => {}
                }
            } else if k > i
                && x.kind == Kind::Ident
                && matches!(
                    x.text.as_str(),
                    "impl" | "fn" | "trait" | "struct" | "enum" | "mod"
                )
            {
                break;
            }
            mask[k] = true;
            k += 1;
        }
    }
    mask
}

/// The first `let` in `body` (token range) whose pattern binds `var`.
fn shadow_position(f: &FileCtx, body: (usize, usize), var: &str) -> usize {
    let t = &f.toks;
    // A loop body this large is scanned no further: report "shadowed from the
    // start" (no index in it is exempt), which is the safe direction and keeps
    // deeply nested loops from costing a full pass each.
    if body.1.saturating_sub(body.0) > 8192 {
        return body.0;
    }
    let mut k = body.0;
    while k < body.1 {
        if t[k].is_punct("=>") {
            // A match arm's pattern may bind the variable: `i => a[i]`.  Read
            // back to the arm's start (`,` or the opening `{`, skipping groups).
            let mut j = k;
            let mut start = None;
            let floor = k.saturating_sub(512).max(body.0);
            while j > floor {
                j -= 1;
                let x = &t[j];
                if x.kind == Kind::Punct {
                    match x.text.as_str() {
                        ")" | "]" | "}" => j = f.pair[j],
                        "," | "{" => {
                            start = Some(j);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            match start {
                // an arm that cannot be delimited within 512 tokens: unsafe direction
                None => return body.0,
                Some(st) => {
                    if t[st + 1..k].iter().any(|x| x.is_ident(var)) {
                        return st;
                    }
                }
            }
        }
        if t[k].is_ident("let") {
            let mut e = k + 1;
            while e < body.1 && !t[e].is_punct("=") && !t[e].is_punct(";") && !t[e].is_ident("let")
            {
                if t[e].is_ident(var) {
                    return k;
                }
                if t[e].kind == Kind::Punct && matches!(t[e].text.as_str(), "(" | "[" | "{") {
                    // look inside the group too
                    if t[e + 1..f.pair[e]].iter().any(|x| x.is_ident(var)) {
                        return k;
                    }
                    e = f.pair[e];
                }
                e += 1;
            }
        }
        k += 1;
    }
    usize::MAX
}

/// SH6: find every `for` loop, report the ones that are not `for i in 0..K`
/// and return the well-formed ones (used by the SH8 index rule).
fn for_loops(cx: &mut Ctx<'_>, f: &FileCtx) -> Vec<LoopInfo> {
    let t = &f.toks;
    let mut good = Vec::new();
    let in_impl_header = impl_header_mask(f);
    for i in 0..t.len() {
        if f.in_attr[i]
            || !t[i].is_ident("for")
            || in_impl_header[i]
            || t.get(i + 1).is_some_and(|x| x.is_punct("<"))
        {
            continue;
        }
        // A `for` that is not part of an `impl` header (or `for<'a>`) is a
        // loop.  Find its `in`, skipping bracket groups so a pattern such as
        // `S { a }` is read whole; stop at anything a pattern cannot contain.
        let mut k = i + 1;
        let mut in_at = None;
        while k < t.len() {
            let x = &t[k];
            if x.kind == Kind::Punct {
                match x.text.as_str() {
                    "(" | "[" | "{" => k = f.pair[k],
                    ";" => break,
                    _ => {}
                }
            } else if x.kind == Kind::Ident {
                if x.text == "in" {
                    in_at = Some(k);
                    break;
                }
                if matches!(
                    x.text.as_str(),
                    "for" | "while" | "loop" | "fn" | "let" | "impl" | "trait"
                ) {
                    break;
                }
            }
            k += 1;
        }
        let Some(p) = in_at else {
            cx.add(
                Rule::Sh6,
                &f.path,
                t[i].line,
                "`for` that is neither a loop `for i in 0..K` nor part of an `impl` header",
            );
            continue;
        };
        let pattern = &t[i + 1..p];
        let var =
            (pattern.len() == 1 && pattern[0].kind == Kind::Ident).then(|| pattern[0].text.clone());
        // The range expression runs to the body brace.
        let mut j = p + 1;
        while j < t.len() && !t[j].is_punct("{") && !t[j].is_punct(";") && !t[j].is_ident("for") {
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
            let body = (j + 1, f.pair[j]);
            let shadow = var
                .as_deref()
                .map_or(usize::MAX, |v| shadow_position(f, body, v));
            good.push(LoopInfo { var, body, shadow });
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
    // Items already inside a previous `const _` item are not rescanned, so a
    // file of unterminated `const _:` costs one pass, not one per item.
    let mut covered = 0usize;
    for i in 0..t.len() {
        if i >= covered
            && !f.in_attr[i]
            && t[i].is_ident("const")
            && t.get(i + 1).is_some_and(|x| x.is_ident("_"))
            && t.get(i + 2).is_some_and(|x| x.is_punct(":"))
        {
            let end = item_end(t, &f.pair, i);
            covered = end + 1;
            // `assert!` is compile time only if the initializer defines no
            // function or impl that could run at runtime (VOS03 SH8).
            let defines_code = t[i..=end]
                .iter()
                .any(|x| x.is_ident("fn") || x.is_ident("impl"));
            if !defines_code {
                out.push((i, end));
            }
        }
    }
    out
}

/// Can the token after a literal end an operand?  (`;`, `)`, a comparison...)
fn ends_operand(n: &Token) -> bool {
    match n.kind {
        Kind::Punct => matches!(
            n.text.as_str(),
            ";" | ")"
                | "]"
                | "}"
                | "{"
                | ","
                | "|"
                | "&"
                | "^"
                | "="
                | "<"
                | ">"
                | "?"
                | ":"
                | "!"
                | "=>"
        ),
        Kind::Ident => n.text == "as",
        _ => false,
    }
}

/// Can the token after `>>` follow a type?  Only these: `> , ; ) ] } = + : ?`
/// and `where`, `as`, `for`.  An operand (`n`, `1`, `*n`, `(x)`, `{ n }`) cannot,
/// and neither can `::` (`x >> y > ::core::u32::MAX`): those are shifts.  `{`
/// and `(` follow a type only in declarations and are decided by the caller.
fn generic_follow(after: Option<&Token>) -> bool {
    match after {
        None => true,
        Some(a) if a.kind == Kind::Punct => {
            matches!(
                a.text.as_str(),
                ">" | "," | ";" | ")" | "]" | "}" | "=" | "+" | ":" | "?"
            )
        }
        Some(a) => a.kind == Kind::Ident && matches!(a.text.as_str(), "where" | "as" | "for"),
    }
}

/// The nearest `<` before `k` (within 256 tokens, in the same statement) that
/// no `>` has closed: the generic list `k` sits inside, if any.
fn enclosing_lt(t: &[Token], pair: &[usize], k: usize) -> Option<usize> {
    let lo = k.saturating_sub(256);
    let mut j = k;
    let mut closes = 0usize;
    while j > lo {
        j -= 1;
        let x = &t[j];
        if x.kind != Kind::Punct {
            continue;
        }
        match x.text.as_str() {
            ";" | "{" | "}" => return None,
            ")" | "]" => j = pair[j],
            ">" => closes += 1,
            "<" => {
                if closes == 0 {
                    return Some(j);
                }
                closes -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Can the `<` at `i` OPEN a generic list in type position?  Conservatively:
/// only after `::` (turbofish), after `impl`, or after a NAME that follows
/// `dyn`, `for`, `as`, a declaration keyword (`fn struct enum trait type union
/// mod`), `->`, or a type-introducing `:`; or, inside a list that is itself
/// such an open, after `<`, `,`, `&`, a lifetime or `mut`.  Everything else
/// (`(`, `=`, `,` outside generics, literals, operators) is a COMPARISON, so
/// a real generic written in expression position (`g(Option<Option<u8>>::None)`)
/// is reported (an intended false positive, VOS03 section 6).
fn generic_open(t: &[Token], pair: &[usize], i: usize, depth: usize) -> bool {
    let Some(prev) = i.checked_sub(1).map(|p| &t[p]) else {
        return false;
    };
    if prev.is_punct("::") || prev.is_ident("impl") {
        return true;
    }
    if prev.kind != Kind::Ident || (is_keyword(&prev.text) && prev.text != "Self") {
        return false;
    }
    let Some(pi) = i.checked_sub(2) else {
        return false;
    };
    let p = &t[pi];
    match p.kind {
        Kind::Punct => match p.text.as_str() {
            ":" | "->" => true,
            // nested inside another generic list that must itself be a real open
            "<" if depth < 8 => generic_open(t, pair, pi, depth + 1),
            "," | "&" if depth < 8 => {
                enclosing_lt(t, pair, pi).is_some_and(|o| generic_open(t, pair, o, depth + 1))
            }
            _ => false,
        },
        Kind::Lifetime if depth < 8 => {
            enclosing_lt(t, pair, pi).is_some_and(|o| generic_open(t, pair, o, depth + 1))
        }
        Kind::Ident => matches!(
            p.text.as_str(),
            "impl"
                | "dyn"
                | "as"
                | "for"
                | "fn"
                | "struct"
                | "enum"
                | "trait"
                | "type"
                | "union"
                | "mod"
        ),
        _ => false,
    }
}

/// Is every token in `t[from..to]` plausible INSIDE a generic argument list:
/// names, `::`, `,`, `&`, `+`, lifetimes, nested `< >`, bracket groups, and
/// `dyn`/`mut`/`impl`/`as`?  `b && c`, `b || c`, and two names in a row (an
/// expression) are not.
fn type_ish(t: &[Token], pair: &[usize], from: usize, to: usize) -> bool {
    let mut k = from;
    let mut prev_ident = false;
    while k < to {
        let x = &t[k];
        match x.kind {
            Kind::Ident => {
                let joiner = matches!(
                    x.text.as_str(),
                    "dyn" | "mut" | "impl" | "as" | "const" | "for"
                );
                if prev_ident && !joiner {
                    return false;
                }
                prev_ident = !matches!(x.text.as_str(), "dyn" | "mut" | "impl" | "as" | "for");
            }
            Kind::Lifetime => prev_ident = false,
            Kind::Punct => {
                prev_ident = false;
                match x.text.as_str() {
                    // `:` is a bound (`T: Into<..>`), `;` an array length
                    "::" | "," | "+" | "<" | ">" | ";" | ":" => {}
                    "&" => {
                        if x.joint && t.get(k + 1).is_some_and(|n| n.is_punct("&")) {
                            return false; // `&&` is a logical and
                        }
                    }
                    "(" | "[" => k = pair[k],
                    _ => return false,
                }
            }
            _ => return false,
        }
        k += 1;
    }
    true
}

/// Is the `>>` at `i` provably the end of two open generic lists?  Scan back
/// within the statement (stopping at `;`, `{`, `}`) for two `<` not closed by a
/// `>`; both must be in type position ([`generic_open`]) and only type-ish
/// tokens may lie between the outer `<` and the `>>` ([`type_ish`]).  On any
/// doubt: not generic, so the `>>` is reported as a shift.
fn closes_two_generics(t: &[Token], pair: &[usize], i: usize, after: Option<&Token>) -> bool {
    let lo = i.saturating_sub(256);
    let mut k = i;
    let mut closes = 0usize;
    let mut opens: Vec<usize> = Vec::new();
    while k > lo {
        k -= 1;
        let x = &t[k];
        if x.kind != Kind::Punct {
            continue;
        }
        match x.text.as_str() {
            ";" | "{" | "}" => break,
            ")" | "]" => k = pair[k],
            ">" => closes += 1,
            "<" => {
                if closes > 0 {
                    closes -= 1;
                } else {
                    opens.push(k);
                    if opens.len() == 2 {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    if opens.len() != 2 || !type_ish(t, pair, opens[1] + 1, i) {
        return false;
    }
    // The OUTER open must be a real generic open (strict); the inner one must
    // directly follow a type name or `::` (it lies inside the outer list, and
    // the window between them is type-ish).
    let (inner, outer) = (opens[0], opens[1]);
    let inner_ok = inner > 0
        && (t[inner - 1].is_punct("::")
            || (t[inner - 1].kind == Kind::Ident && !is_keyword(&t[inner - 1].text)));
    if !inner_ok || !generic_open(t, pair, outer, 0) {
        return false;
    }
    // What follows must be able to follow a type.  `(` follows the generics of
    // a DECLARATION only (`fn f<T: Into<Vec<u8>>>(x: T)`); `{` follows a
    // return type, an `impl .. for` header or `impl<..>` only.  In an
    // expression both are operands: `x >> (n)`, `x >> { n }` are shifts.
    let before = outer.checked_sub(2).map(|p| &t[p]);
    let declares = before.is_some_and(|p| {
        matches!(
            p.text.as_str(),
            "fn" | "struct" | "enum" | "trait" | "type" | "union"
        )
    }) || outer.checked_sub(1).is_some_and(|p| t[p].is_ident("impl"));
    let brace_ok = before.is_some_and(|p| p.is_punct("->") || p.is_ident("for"))
        || outer.checked_sub(1).is_some_and(|p| t[p].is_ident("impl"));
    // `Vec::<Vec<u8>>::new()`: `::` follows the generics of a TURBOFISH only
    let turbofish_path = outer.checked_sub(1).is_some_and(|p| t[p].is_punct("::"))
        && after.is_some_and(|a| a.is_punct("::"));
    generic_follow(after)
        || turbofish_path
        || (declares && after.is_some_and(|a| a.is_punct("(")))
        || (brace_ok && after.is_some_and(|a| a.is_punct("{")))
}

/// `Vec<<T as X>::Y>`: the `<<` at `i` is a generic open + qualified-self open,
/// not a shift, only if it follows `::` or a type-position name, its second `<`
/// closes right before `::`, and the group holds an `as` between type-ish tokens.
/// `1u32 << s > ::core::u32::MAX` has none of that and is a shift.
fn opens_qself_generic(t: &[Token], pair: &[usize], i: usize) -> bool {
    let first_ok = generic_open(t, pair, i, 0);
    if !first_ok {
        return false;
    }
    let Some(c) = angle_close(t, pair, i + 1) else {
        return false;
    };
    t.get(c + 1).is_some_and(|x| x.is_punct("::"))
        && t[i + 2..c].iter().any(|x| x.is_ident("as"))
        && type_ish(t, pair, i + 2, c)
}

/// Is the token before a `|` one after which a closure can start?
fn closure_can_follow(prev: Option<&Token>) -> bool {
    match prev {
        None => true,
        Some(p) if p.kind == Kind::Punct => matches!(
            p.text.as_str(),
            "(" | "," | "=" | "{" | ";" | "[" | ":" | "=>" | "->" | "&" | "*" | "!"
        ),
        Some(p) => {
            matches!(
                p.text.as_str(),
                "return" | "move" | "async" | "else" | "in" | "break" | "yield" | "await"
            ) && p.kind == Kind::Ident
        }
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
            let compound =
                t.get(i + 2).is_some_and(|x| x.is_punct("=")) && next.is_some_and(|n| n.joint);
            let op_end = i + 2 + usize::from(compound);
            let after = t.get(op_end);
            let is_shift = if tok.text == "<" {
                !(!compound && opens_qself_generic(t, &f.pair, i))
            } else {
                // `>>` closes generics only when what FOLLOWS can follow a type
                // AND two type-position `<` are provably open (VOS03 SH9).
                compound || !closes_two_generics(t, &f.pair, i, after)
            };
            // A literal amount is allowed only as the WHOLE right operand:
            // `x << 1 + n` and `x << 1.max(n)` are not literal shifts.
            let whole_literal = after.is_some_and(|a| a.kind == Kind::Int)
                && t.get(op_end + 1).is_none_or(ends_operand);
            if is_shift && !whole_literal {
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
            // An allowlist of what may PRECEDE a `[` that starts an array
            // literal, an array type or an attribute; every other `[` is an
            // index (`w.0[i]`, `f()?[i]`, `{..}[i]`, `self[i]`, `b"ab"[n]`).
            let array_start = match p.kind {
                Kind::Lifetime => true,
                // only these keywords can precede an array literal or type
                // (`.await[i]`, `self[i]`, `crate[..]` are indexes)
                Kind::Ident => matches!(
                    p.text.as_str(),
                    "in" | "return"
                        | "break"
                        | "else"
                        | "if"
                        | "match"
                        | "let"
                        | "as"
                        | "mut"
                        | "ref"
                        | "dyn"
                        | "impl"
                        | "where"
                        | "for"
                        | "move"
                        | "const"
                        | "static"
                        | "type"
                        | "use"
                        | "unsafe"
                        | "yield"
                ),
                Kind::Punct => matches!(
                    p.text.as_str(),
                    "=" | "("
                        | ","
                        | ";"
                        | "{"
                        | ":"
                        | "#"
                        | "!"
                        | "&"
                        | "<"
                        | "->"
                        | "=>"
                        | "["
                        | "|"
                        | ".."
                        | "+"
                        | "-"
                        | "*"
                        | "/"
                        | "%"
                        | "^"
                ),
                _ => false,
            };
            if array_start {
                return;
            }
            let inner = &t[i + 1..f.pair[i]];
            let allowed = inner.len() == 1
                && inner[0].kind == Kind::Ident
                && loops.iter().any(|l| {
                    l.var.as_deref() == Some(inner[0].text.as_str())
                        && i >= l.body.0
                        && i < l.body.1
                        && i < l.shadow
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
            // `core::alloc::Layout`: `alloc` is a later segment.  `return ::std::x`
            // and `use ::std::x` have a KEYWORD before the `::`, so are paths from the root.
            let second_segment = i >= 2
                && t[i - 1].is_punct("::")
                && t[i - 2].kind == Kind::Ident
                && !is_keyword(&t[i - 2].text);
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
    // After `.` (a method call) and after `::` (UFCS: `Option::unwrap(o)`).
    let prev_colons = prev.is_some_and(|p| p.is_punct("::"));
    let named_call = (prev_dot || prev_colons) && is_call_next(t, i);
    if named_call && OVERFLOW_METHODS.contains(&name) {
        cx.add(
            Rule::Sh9,
            path,
            line,
            format!(
                "`{name}` overflows or shifts unchecked; use wrapping_*, checked_* or a literal"
            ),
        );
    }
    if named_call && PANIC_METHODS.contains(&name) {
        cx.add(
            Rule::Sh8,
            path,
            line,
            format!("`{name}` is a path to a panic"),
        );
    }
    if named_call && ITER_METHODS.contains(&name) {
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
    let end = match scan_header(t, &f.pair, fn_idx + 1) {
        Header::End(e) => e,
        Header::GaveUp(g) => {
            cx.add(
                Rule::Vt03,
                &f.path,
                t[fn_idx].line,
                "a function signature could not be read to its end (fail closed)",
            );
            g
        }
    };
    while k < end {
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
        match angle_close(t, &f.pair, k) {
            Some(c) => k = c + 1,
            None => {
                cx.add(
                    Rule::Vt03,
                    &f.path,
                    t[i].line,
                    "the generic list of a type definition could not be read (fail closed)",
                );
                return;
            }
        }
    }
    let body = if t.get(k).is_some_and(|x| x.is_punct("(")) && t[i].text != "enum" {
        Some(k)
    } else {
        match scan_header(t, &f.pair, k) {
            Header::End(b) if t[b].is_punct("{") => Some(b),
            Header::End(_) => None,
            Header::GaveUp(_) => {
                cx.add(
                    Rule::Vt03,
                    &f.path,
                    t[i].line,
                    "the header of a type definition could not be read (fail closed)",
                );
                None
            }
        }
    };
    if let Some(k) = body {
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
    // a const's type is short; do not scan further than 256 tokens for its end
    let limit = (k + 256).min(t.len());
    let mut terminated = false;
    while k < limit {
        let x = &t[k];
        if x.kind == Kind::Punct {
            match x.text.as_str() {
                "<" => depth += 1,
                ">" => {
                    if depth == 0 {
                        terminated = true;
                        break;
                    }
                    depth -= 1;
                }
                "(" | "[" => k = f.pair[k],
                "=" | ";" | "," if depth == 0 => {
                    terminated = true;
                    break;
                }
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
    if !terminated {
        cx.add(
            Rule::Vt03,
            &f.path,
            t[i].line,
            "the type of a `const` could not be read to its end (fail closed)",
        );
    }
}
