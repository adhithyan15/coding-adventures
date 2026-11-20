//! # SH19: the command-type check (VOS-039b-2a, VOS-D077)
//!
//! ## The problem
//!
//! A leaf component never touches hardware.  It *returns commands* ("write this
//! many bytes to that device", "arm the timer") and the HAL, which owns a static
//! allowlist built from the board map, validates and executes them (VOS04
//! section 3, requirement 1).  That arrangement is only safe if a leaf cannot
//! even **construct** a request for an arbitrary location.  So the vocabulary of
//! commands, the `enum`s of the interface's contract package, must be *closed*,
//! and no variant may carry a raw address, a CSR number or an MMIO offset.
//!
//! Think of a vending machine whose keypad has only the buttons A1 to C3.  The
//! customer (the leaf) can ask for any of the nine slots and for nothing else; a
//! keypad with a free-form numeric field would let them ask for slot 40,000.  A
//! payload of type `usize` is that free-form numeric field.
//!
//! ## The rule, and why it is DERIVED
//!
//! The check binds crates that declare `kind = "contract"` (and only them; see
//! [`crate::rules::binds`] for why it does not go through the class table).
//! The set of commands is not a list an author writes (an author-written list can
//! be made to omit the dangerous enum).  It is **every `enum` in the crate**,
//! public or not: a superset of "every `pub` enum" that needs no visibility
//! parsing, so it can only err towards a finding.
//!
//! An enum fails if it:
//!
//! * carries `#[non_exhaustive]` (on the enum, or on a variant): "closed" means a
//!   downstream `match` may rely on the list;
//! * has a generic list: a payload type chosen by the user of the enum is a
//!   payload this check never sees;
//! * has a variant field whose type is not accepted.
//!
//! The accepted types are EXACTLY these four spellings ([`ACCEPTED_TYPES`]):
//!
//! ```text
//!   u8                      one byte: an id, a count below 256, a data byte
//!   bool                    one bit
//!   [u8; 4]                 data bytes, the length a decimal literal
//!   Name                    the bare name of a struct or enum of this crate that
//!                           itself passes (judged the same way)
//! ```
//!
//! Everything else is a finding that names the type: `usize`, `u16` and wider,
//! `i*`, `&T`, `*const T`, tuples, `Option<T>`, any `a::b` path, an array whose
//! length is a name (`[u8; N]` with `N` a constant), and a `type` alias.  An
//! alias is **not resolved, it is rejected**: `type Addr = usize; V(Addr)` must
//! not pass, and following a name across an alias is exactly the kind of work a
//! lexical check does not do (VOS03 section 6, item 18).
//!
//! ## How a nested type is judged, without recursion
//!
//! `V(Outer)` where `struct Outer { inner: Inner }` and `struct Inner { a: usize }`
//! fails because of `Inner`, two hops away.  A recursive judge would overflow the
//! stack on a hostile crate that nests ten thousand structs, so the judgement is a
//! work-list over a reverse-reference graph:
//!
//! ```text
//!   1. each struct/enum is judged LOCALLY: its own flags (generic, non_exhaustive,
//!      unreadable) and every field whose type is not a same-crate name
//!   2. a struct/enum that fails goes on a queue
//!   3. when X is popped, every type that mentions X by name fails too, with X's
//!      root cause copied (a short string, not a growing chain)
//! ```
//!
//! Every type enters the queue at most once, so the work is linear in the number
//! of fields; a cycle (`A` holds `B`, `B` holds `A`) simply terminates.
//!
//! ## The closed vocabulary of a contract crate
//!
//! "Accepted types are exactly four spellings" is only true if the four names
//! mean what they look like.  A lexical check cannot resolve names, so instead of
//! chasing every trick that changes what a name means (a `type u8 = u32;`, a
//! `use core::primitive::u32 as Addr;`, a `macro_rules!` that writes the enum, a
//! nested `mod` that defines a name twice) the check **refuses the whole
//! construct** in a contract crate.  A contract crate holds declarations and
//! nothing else, so this costs it nothing.  It is a SH19 finding to write:
//!
//! * any `use` (including `pub use` and `use .. as ..`) or `extern crate`: the
//!   crate imports nothing, every name is defined in it or is a primitive;
//! * any `mod`: one flat namespace, so a name cannot mean two things;
//! * any macro definition (`macro_rules!`, `macro`) or invocation (`name!(..)`):
//!   a macro can write an enum the check never reads;
//! * an item (`struct`, `enum`, `union`, `type`, `trait`, `fn`, `const`, `static`)
//!   named like a Rust primitive or like a Warrant built-in type
//!   ([`RESERVED_TYPE_NAMES`]): `struct u8(..)` would change what `u8` means, and
//!   `struct U8(u8)` would render, in CK10, exactly like the primitive.
//!
//! Attributes (`#[derive(..)]`, `#[repr(u8)]`, `#![no_std]`) and doc comments
//! are not touched.  Tests live in the crate's `tests/` directory, not in a
//! `#[cfg(test)] mod`.  [`spell`] also asks the crate's item table before it
//! trusts `u8` or `bool`, so a shadowing item that somehow escaped the rules
//! above still fails closed.
//!
//! ## Attributes are a closed list too (VOS-D112)
//!
//! A path dependency could supply an attribute macro (`#[evil::rewrite]`) or a foreign
//! derive (`#[derive(evil::Gen)]`) that rewrites an enum AFTER this check judged it.
//! `vos-link check-component` closes the dependency side (CC10: a contract crate has
//! none); this check closes the attribute side, as a whitelist over the spelling, not a
//! blacklist of known-bad names:
//!
//! ```text
//!   #[derive(A, B)]     only the BARE core traits Copy Clone Debug PartialEq Eq
//!                       PartialOrd Ord Hash Default (no path: `core::clone::Clone` is out)
//!   #[repr(..)]         only u8, C, transparent
//!   #[no_std] #[must_use] #[inline] #[default]            as written, no arguments
//!   #[forbid(..)] #[deny(..)] #[warn(..)] #[allow(..)]    a list of lint names (paths ok:
//!                                                         `clippy::x` names a lint, not code)
//!   #[doc = "text"]  #[doc(hidden)]
//!   #[cfg(test)]        exactly that
//!   #[non_exhaustive]   passes THIS pass; the closed-set rule above reports it
//! ```
//!
//! Everything else is a finding: any `a::b` attribute, `cfg_attr`, `path`, `macro_use`,
//! `proc_macro*`, `test`, `cfg(not(test))`, an unknown name.  `#[default]` is on the list
//! because `derive(Default)` on an enum cannot be written without it.
//!
//! ## What this check does not see
//!
//! It is lexical.  It reads what is written (VOS03 section 6, item 18).  The HAL
//! allowlist, the other half of the safety argument, is VOS-025a.

use std::collections::BTreeMap;

use std::collections::BTreeSet;

use crate::items::TypeDef;
use crate::lexer::Kind;
use crate::report::{Finding, Rule};
use crate::rules::Ctx;

/// The accepted payload types, as VOS03's SH19 row spells them.  A test reads the
/// spec and asserts that it states each of these, so the list and the text cannot
/// drift apart.
pub const ACCEPTED_TYPES: [&str; 4] = ["u8", "bool", "[u8; N]", "same-crate struct or enum"];

/// Names a contract crate may not give to an item: the Rust primitive types, and
/// the built-in type names of a Warrant document (`checker.rs` of
/// `warrant-grammar`: `V0_BUILTINS` and `V1_TYPES`; a test reads that file and
/// asserts every type name there is listed here, so the two cannot drift).
pub const RESERVED_TYPE_NAMES: [&str; 33] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "bool",
    "char", "str", "f32", "f64", "Bool", "Int", "Nat", "U8", "U16", "U32", "U64", "I8", "I16",
    "I32", "I64", "Bytes", "List", "Option", "Result", "Self",
];

/// Longest piece of source text a message quotes.
const SHOW_CAP: usize = 48;

/// `s` for a message: every character that is not printable ASCII is escaped
/// (a field type may contain a string literal with a newline, and a finding is
/// one line), and a long text is cut.
fn show(s: &str) -> String {
    let mut out = String::new();
    for (n, c) in s.chars().enumerate() {
        if n >= SHOW_CAP {
            out.push_str("...");
            break;
        }
        if c == '\\' || !(c.is_ascii_graphic() || c == ' ') {
            out.extend(c.escape_default());
        } else {
            out.push(c);
        }
    }
    out
}

/// How a field type reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spelling<'a> {
    /// `u8`, `bool` or `[u8; <decimal digits>]`.
    Accepted,
    /// A single identifier: it may name a type of the crate.
    Name(&'a str),
    /// Anything else.
    Rejected,
}

/// Classify the space-joined token text of a field type.
fn spell(ty: &str) -> Spelling<'_> {
    crate::steps::tick();
    if ty == "u8" || ty == "bool" {
        return Spelling::Accepted;
    }
    if let Some(len) = ty
        .strip_prefix("[ u8 ; ")
        .and_then(|r| r.strip_suffix(" ]"))
    {
        // `4` and nothing else: a suffix (`4usize`), a base prefix (`0x10`), an
        // underscore or a name (`N`) is a different spelling and is not accepted.
        return if !len.is_empty() && len.bytes().all(|b| b.is_ascii_digit()) {
            Spelling::Accepted
        } else {
            Spelling::Rejected
        };
    }
    let one_token = !ty.is_empty() && !ty.contains(' ');
    let identifier = ty.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !ty.starts_with(|c: char| c.is_ascii_digit());
    if one_token && identifier {
        Spelling::Name(ty)
    } else {
        Spelling::Rejected
    }
}

/// One field of a struct, or one field of one variant of an enum.
struct Slot {
    /// Where it is, for the message: ``variant `Run` field `0` ``.
    place: String,
    line: u32,
    ty: String,
}

/// The fields of a struct or an enum, in source order.
fn slots(t: &TypeDef) -> Vec<Slot> {
    let mut out = Vec::new();
    if t.keyword == "enum" {
        for v in &t.variants {
            for f in &v.fields {
                crate::steps::tick();
                out.push(Slot {
                    place: format!("variant `{}` field `{}`", show(&v.name), show(&f.name)),
                    line: f.line,
                    ty: f.ty.clone(),
                });
            }
        }
    } else {
        for f in &t.fields {
            crate::steps::tick();
            out.push(Slot {
                place: format!("field `{}`", show(&f.name)),
                line: f.line,
                ty: f.ty.clone(),
            });
        }
    }
    out
}

/// A struct or an enum (the two items a payload may name).
fn is_data(t: &TypeDef) -> bool {
    t.keyword == "struct" || t.keyword == "enum"
}

/// What the whole crate says about the names its types use.
struct Names<'a> {
    /// Per name: does a struct or enum carry it, and which other item (a `type`
    /// alias, a `union`) does.  Built once, so a lookup costs one map probe
    /// however many items share the name.
    kinds: BTreeMap<&'a str, (bool, Option<&'static str>)>,
}

/// Why a field type is not a command payload, when that is decided by the type
/// alone (not by what a same-crate struct it names contains).
enum Local {
    /// Not one of the accepted spellings, and not a name.
    Spelling,
    /// A name that no struct or enum of the crate carries, and no other item does.
    Unknown,
    /// A name that a `type` alias or a `union` carries.
    Alias(&'static str),
    /// `u8` or `bool` while the crate defines an item of that name.
    Shadowed,
}

impl Local {
    fn why(&self) -> &'static str {
        match self {
            Local::Spelling => "a command payload is `u8`, `bool`, `[u8; N]` with a decimal literal `N`, or a same-crate struct or enum that passes",
            Local::Unknown => "it is not `u8` or `bool`, and no struct or enum of this crate has that name (a wider integer, a path or a foreign type is not accepted)",
            Local::Alias("type") => "a `type` alias is not resolved, so it is rejected (write the underlying `u8`, `bool` or `[u8; N]`)",
            Local::Alias(_) => "a `union` is not a command payload",
            Local::Shadowed => "this crate defines an item named like that primitive, so the name is not known to mean the primitive",
        }
    }
}

impl<'a> Names<'a> {
    fn new(types: &'a [TypeDef]) -> Names<'a> {
        let mut kinds: BTreeMap<&'a str, (bool, Option<&'static str>)> = BTreeMap::new();
        for t in types {
            crate::steps::tick();
            let e = kinds.entry(t.name.as_str()).or_default();
            if is_data(t) {
                e.0 = true;
            } else {
                e.1 = Some(t.keyword);
            }
        }
        Names { kinds }
    }

    /// The verdict on one field type that does not depend on other items:
    /// `Err(reason)` when it fails here, `Ok(Some(name))` with the same-crate
    /// type name it refers to, `Ok(None)` when it is accepted outright.
    fn judge(&self, ty: &str) -> Result<Option<&'a str>, Local> {
        crate::steps::tick();
        match spell(ty) {
            Spelling::Accepted => {
                let prim = if ty == "bool" { "bool" } else { "u8" };
                if self.kinds.contains_key(prim) {
                    Err(Local::Shadowed)
                } else {
                    Ok(None)
                }
            }
            Spelling::Rejected => Err(Local::Spelling),
            Spelling::Name(n) => match self.kinds.get_key_value(n) {
                Some((_, (_, Some(kw)))) => Err(Local::Alias(kw)),
                Some((&key, (true, None))) => Ok(Some(key)),
                _ => Err(Local::Unknown),
            },
        }
    }
}

/// The message of a field that fails on its own.
fn local_message(ty: &str, why: &Local) -> String {
    format!(
        "type `{}` is not a command payload: {}",
        show(ty),
        why.why()
    )
}

/// The first defect an item has before any field is read: the flags, in the
/// order they are explained above.
fn flag_defect(t: &TypeDef) -> Option<String> {
    let what = format!("{} `{}`", t.keyword, show(&t.name));
    if t.non_exhaustive {
        Some(format!(
            "{what} is `#[non_exhaustive]`: a command set must be closed"
        ))
    } else if t.variants.iter().any(|v| v.non_exhaustive) {
        Some(format!(
            "{what} has a `#[non_exhaustive]` variant: a command set must be closed"
        ))
    } else if t.generic {
        Some(format!(
            "{what} is generic: a payload type chosen by its user is never seen by this check"
        ))
    } else if t.malformed {
        Some(format!(
            "{what} has a field list this check could not read (fail closed)"
        ))
    } else {
        None
    }
}

/// Is `name` one a contract crate may not define?  It is read through a
/// raw-identifier prefix (`r#u8` is `u8`).
fn reserved(name: &str) -> bool {
    let bare = name.strip_prefix("r#").unwrap_or(name);
    RESERVED_TYPE_NAMES.contains(&bare)
}

/// The closed vocabulary (see the module notes): `use`, `extern crate`, `mod`,
/// macros, and items named like a primitive or a Warrant built-in.
fn vocabulary(cx: &mut Ctx<'_>) {
    let mut found: Vec<Finding> = Vec::new();
    for f in cx.files {
        let t = &f.toks;
        let mut add = |line: u32, message: String| {
            found.push(Finding {
                rule: Rule::Sh19,
                file: f.path.clone(),
                line,
                message,
            });
        };
        for i in 0..t.len() {
            crate::steps::tick();
            if f.in_attr[i] || t[i].kind != Kind::Ident {
                continue;
            }
            if i > 0 && t[i - 1].is_punct(".") {
                continue;
            }
            let word = t[i].text.as_str();
            let next = t.get(i + 1);
            let line = t[i].line;
            match word {
                "use" => add(
                    line,
                    "`use`: a contract crate imports nothing (every name it uses is defined in it or is a primitive), so a name cannot be renamed or re-exported".to_string(),
                ),
                "extern" if next.is_some_and(|x| x.is_ident("crate")) => add(
                    line,
                    "`extern crate`: a contract crate imports nothing".to_string(),
                ),
                "mod" => add(
                    line,
                    "`mod`: a contract crate is one flat namespace, so a name cannot mean two things".to_string(),
                ),
                "macro_rules" | "macro" => add(
                    line,
                    format!("`{word}`: a contract crate defines no macro (a macro can write an enum this check never reads)"),
                ),
                _ => {}
            }
            if next.is_some_and(|x| x.is_punct("!"))
                && t.get(i + 2).is_some_and(|x| {
                    x.kind == Kind::Punct && matches!(x.text.as_str(), "(" | "[" | "{")
                })
                && word != "macro_rules"
            {
                add(
                    line,
                    format!(
                        "macro invocation `{}!`: a contract crate uses no macro (a macro can write an enum this check never reads)",
                        show(word)
                    ),
                );
            }
            if matches!(
                word,
                "struct" | "enum" | "union" | "type" | "trait" | "fn" | "const" | "static"
            ) {
                if let Some(n) = next.filter(|x| x.kind == Kind::Ident && reserved(&x.text)) {
                    add(
                        n.line,
                        format!(
                            "{word} `{}`: a contract crate may not define an item named like a Rust primitive or a Warrant built-in type ({})",
                            show(&n.text),
                            "it would change what that name means to this check, or render like the built-in in CK10"
                        ),
                    );
                }
            }
        }
    }
    cx.out.extend(found);
}

/// The traits a contract crate may derive, as bare names.
pub const DERIVE_ALLOWED: [&str; 9] = [
    "Copy",
    "Clone",
    "Debug",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Default",
];

/// The `repr` arguments a contract crate may use.
pub const REPR_ALLOWED: [&str; 3] = ["u8", "C", "transparent"];

/// Attributes written bare (no arguments).
const BARE_ALLOWED: [&str; 5] = ["no_std", "must_use", "inline", "default", "non_exhaustive"];

/// Attributes whose argument list is a list of lint names.
const LINT_ATTRS: [&str; 4] = ["forbid", "deny", "warn", "allow"];

/// The arguments of `name ( a , b , )`: the identifiers, when the tokens are exactly
/// identifiers separated by commas (one trailing comma allowed) and nothing else.
/// `lint` lets a name be a `::` path (`clippy::arithmetic_side_effects` names a lint).
fn arg_list(body: &[crate::lexer::Token], lint: bool) -> Result<Vec<String>, String> {
    let n = body.len();
    if n < 3 || !body[1].is_punct("(") || !body[n - 1].is_punct(")") {
        return Err("it takes a parenthesised list".to_string());
    }
    let mut names: Vec<String> = Vec::new();
    let mut want_name = true;
    for x in &body[2..n - 1] {
        crate::steps::tick();
        if want_name && x.kind == Kind::Ident {
            names.push(x.text.clone());
            want_name = false;
        } else if !want_name && x.is_punct(",") {
            want_name = true;
        } else if lint && !want_name && (x.is_punct("::") || x.is_punct(":")) {
            // `a :: b`: the next token continues the name.
            want_name = true;
            if let Some(last) = names.last_mut() {
                last.push_str("::");
            }
        } else if lint && want_name && x.is_punct(":") {
            // the second half of a `::` written as two `:` tokens.
        } else {
            return Err(format!("`{}` is not allowed in the list", show(&x.text)));
        }
    }
    if names.is_empty() {
        return Err("the list is empty".to_string());
    }
    Ok(names)
}

/// Why the attribute body `body` (the tokens between `[` and `]`) is not on the closed list, or
/// `None` when it is.
fn attribute_defect(body: &[crate::lexer::Token]) -> Option<String> {
    crate::steps::tick();
    let Some(first) = body.first().filter(|t| t.kind == Kind::Ident) else {
        return Some("an attribute that does not start with a plain name".to_string());
    };
    let name = first.text.as_str();
    if body
        .get(1)
        .is_some_and(|t| t.is_punct("::") || t.is_punct(":"))
    {
        return Some(format!(
            "`{}::..` is a path attribute: it may be a macro of another crate that rewrites the item after this check judged it",
            show(name)
        ));
    }
    let listed = |allowed: &[&str], list: Vec<String>, what: &str| {
        list.iter()
            .find(|a| !allowed.contains(&a.as_str()))
            .map(|a| {
                format!(
                    "`{what}` argument `{}` is not on the closed list ({})",
                    show(a),
                    allowed.join(", ")
                )
            })
    };
    match name {
        "derive" => match arg_list(body, false) {
            Ok(l) => listed(&DERIVE_ALLOWED, l, "derive"),
            Err(e) => Some(format!("`derive`: {e}")),
        },
        "repr" => match arg_list(body, false) {
            Ok(l) => listed(&REPR_ALLOWED, l, "repr"),
            Err(e) => Some(format!("`repr`: {e}")),
        },
        n if LINT_ATTRS.contains(&n) => arg_list(body, true)
            .err()
            .map(|e| format!("`{}`: {e}", show(n))),
        n if BARE_ALLOWED.contains(&n) => {
            // Written bare, except the two argument forms `must_use = "why"` and `inline(always)`.
            let must_use_reason = n == "must_use"
                && body.len() == 3
                && body[1].is_punct("=")
                && body[2].kind == Kind::Str;
            let inline_hint = n == "inline"
                && body.len() == 4
                && body[1].is_punct("(")
                && (body[2].is_ident("always") || body[2].is_ident("never"))
                && body[3].is_punct(")");
            let ok = body.len() == 1 || must_use_reason || inline_hint;
            (!ok).then(|| format!("`{}` takes no arguments here", show(n)))
        }
        "doc" => {
            let text = body.len() == 3 && body[1].is_punct("=") && body[2].kind == Kind::Str;
            let hidden = body.len() == 4
                && body[1].is_punct("(")
                && body[2].is_ident("hidden")
                && body[3].is_punct(")");
            (!(text || hidden)).then(|| "`doc` is `doc = \"text\"` or `doc(hidden)`".to_string())
        }
        "cfg" => {
            let test = body.len() == 4
                && body[1].is_punct("(")
                && body[2].is_ident("test")
                && body[3].is_punct(")");
            (!test).then(|| "`cfg` is exactly `cfg(test)`".to_string())
        }
        other => Some(format!(
            "`{}` is not on the closed list of attributes (derive of core traits, repr, no_std, forbid, deny, warn, allow, doc, must_use, inline, default, cfg(test))",
            show(other)
        )),
    }
}

/// SH19 (h): every attribute of every file is on the closed list (see the module notes).
fn attributes(cx: &mut Ctx<'_>) {
    let mut found: Vec<Finding> = Vec::new();
    for f in cx.files {
        for a in &f.attrs {
            crate::steps::tick();
            if let Some(why) = attribute_defect(f.attr_body(a)) {
                found.push(Finding {
                    rule: Rule::Sh19,
                    file: f.path.clone(),
                    line: f.toks[a.start].line,
                    message: format!("attribute: {why}"),
                });
            }
        }
    }
    cx.out.extend(found);
}

/// SH19 over a crate that declares `kind = "contract"`.
pub fn command_rules(cx: &mut Ctx<'_>) {
    vocabulary(cx);
    attributes(cx);
    let names = Names::new(&cx.types);

    // 1. Local judgement, and the reverse-reference graph, over DISTINCT NAMES:
    //    one node per name, one edge per (referencing name, referenced name) pair,
    //    so n items called `X` and n fields naming `X` cost n, not n times n.
    //    A name fails if any item of that name fails.
    let mut cause: BTreeMap<&str, String> = BTreeMap::new();
    let mut refs_by: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut queue: Vec<&str> = Vec::new();
    for t in cx.types.iter() {
        crate::steps::tick();
        if !is_data(t) {
            continue;
        }
        let mut defect = flag_defect(t);
        for s in slots(t) {
            match names.judge(&s.ty) {
                Ok(Some(named)) => {
                    refs_by.entry(named).or_default().insert(t.name.as_str());
                }
                Ok(None) => {}
                Err(why) => {
                    defect.get_or_insert_with(|| {
                        format!(
                            "{} `{}`, {}: {}",
                            t.keyword,
                            show(&t.name),
                            s.place,
                            local_message(&s.ty, &why)
                        )
                    });
                }
            }
        }
        if let Some(d) = defect {
            if !cause.contains_key(t.name.as_str()) {
                cause.insert(t.name.as_str(), d);
                queue.push(t.name.as_str());
            }
        }
    }

    // 2. A name that is mentioned by a failing name fails, with the same root cause.
    let mut head = 0;
    while head < queue.len() {
        crate::steps::tick();
        let j = queue[head];
        head += 1;
        let root = cause[j].clone();
        for &p in refs_by.get(j).into_iter().flatten() {
            crate::steps::tick();
            if !cause.contains_key(p) {
                cause.insert(p, root.clone());
                queue.push(p);
            }
        }
    }

    // 3. The findings: one per defect of each enum, in source order.
    let mut found: Vec<Finding> = Vec::new();
    for t in cx.types.iter().filter(|t| t.keyword == "enum") {
        crate::steps::tick();
        let file = cx.files[t.file].path.clone();
        let mut add = |line: u32, message: String| {
            found.push(Finding {
                rule: Rule::Sh19,
                file: file.clone(),
                line,
                message,
            });
        };
        let what = format!("enum `{}`", show(&t.name));
        if t.non_exhaustive {
            add(
                t.line,
                format!("{what} is `#[non_exhaustive]`: a command set must be closed"),
            );
        }
        for v in t.variants.iter().filter(|v| v.non_exhaustive) {
            add(
                v.line,
                format!(
                    "{what} variant `{}` is `#[non_exhaustive]`: a command set must be closed",
                    show(&v.name)
                ),
            );
        }
        if t.generic {
            add(
                t.line,
                format!("{what} is generic: a payload type chosen by its user is never seen by this check"),
            );
        }
        if t.malformed {
            add(
                t.line,
                format!("{what} has a variant list this check could not read (fail closed)"),
            );
        }
        for s in slots(t) {
            match names.judge(&s.ty) {
                Err(why) => add(
                    s.line,
                    format!("{what} {}: {}", s.place, local_message(&s.ty, &why)),
                ),
                Ok(Some(named)) => {
                    if let Some(c) = cause.get(named) {
                        add(
                            s.line,
                            format!(
                                "{what} {}: type `{}` is not a command payload ({})",
                                s.place,
                                show(&s.ty),
                                c
                            ),
                        );
                    }
                }
                Ok(None) => {}
            }
        }
    }
    cx.out.extend(found);
}
