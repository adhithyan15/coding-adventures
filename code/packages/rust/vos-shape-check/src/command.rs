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
//! ## What this check does not see
//!
//! It is lexical.  It reads what is written (VOS03 section 6, item 18).  The HAL
//! allowlist, the other half of the safety argument, is VOS-025a.

use std::collections::BTreeMap;

use crate::items::TypeDef;
use crate::report::{Finding, Rule};
use crate::rules::Ctx;

/// The accepted payload types, as VOS03's SH19 row spells them.  A test reads the
/// spec and asserts that it states each of these, so the list and the text cannot
/// drift apart.
pub const ACCEPTED_TYPES: [&str; 4] = ["u8", "bool", "[u8; N]", "same-crate struct or enum"];

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
                    place: format!("variant `{}` field `{}`", v.name, f.name),
                    line: f.line,
                    ty: f.ty.clone(),
                });
            }
        }
    } else {
        for f in &t.fields {
            crate::steps::tick();
            out.push(Slot {
                place: format!("field `{}`", f.name),
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
    types: &'a [TypeDef],
    /// Name to the indexes of every item of that name (a struct, an alias, ...).
    by_name: BTreeMap<&'a str, Vec<usize>>,
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
}

impl Local {
    fn why(&self) -> &'static str {
        match self {
            Local::Spelling => "a command payload is `u8`, `bool`, `[u8; N]` with a decimal literal `N`, or a same-crate struct or enum that passes",
            Local::Unknown => "it is not `u8` or `bool`, and no struct or enum of this crate has that name (a wider integer, a path or a foreign type is not accepted)",
            Local::Alias("type") => "a `type` alias is not resolved, so it is rejected (write the underlying `u8`, `bool` or `[u8; N]`)",
            Local::Alias(_) => "a `union` is not a command payload",
        }
    }
}

impl<'a> Names<'a> {
    fn new(types: &'a [TypeDef]) -> Names<'a> {
        let mut by_name: BTreeMap<&'a str, Vec<usize>> = BTreeMap::new();
        for (i, t) in types.iter().enumerate() {
            crate::steps::tick();
            by_name.entry(t.name.as_str()).or_default().push(i);
        }
        Names { types, by_name }
    }

    /// The struct and enum items named `name`, and whether some other item
    /// (a `type` alias, a `union`) also carries that name.
    fn data_named(&self, name: &str) -> (Vec<usize>, Option<&'static str>) {
        let mut data = Vec::new();
        let mut other = None;
        for &j in self.by_name.get(name).map_or(&[][..], Vec::as_slice) {
            crate::steps::tick();
            if is_data(&self.types[j]) {
                data.push(j);
            } else {
                other = Some(self.types[j].keyword);
            }
        }
        (data, other)
    }

    /// The verdict on one field type that does not depend on other items:
    /// `Err(reason)` when it fails here, `Ok(indexes)` with the same-crate
    /// struct and enum items it names (empty when it is accepted outright).
    fn judge(&self, ty: &str) -> Result<Vec<usize>, Local> {
        match spell(ty) {
            Spelling::Accepted => Ok(Vec::new()),
            Spelling::Rejected => Err(Local::Spelling),
            Spelling::Name(n) => {
                let (data, other) = self.data_named(n);
                match (data.is_empty(), other) {
                    (_, Some(kw)) => Err(Local::Alias(kw)),
                    (true, None) => Err(Local::Unknown),
                    (false, None) => Ok(data),
                }
            }
        }
    }
}

/// The message of a field that fails on its own.
fn local_message(ty: &str, why: &Local) -> String {
    format!("type `{ty}` is not a command payload: {}", why.why())
}

/// The first defect an item has before any field is read: the flags, in the
/// order they are explained above.
fn flag_defect(t: &TypeDef) -> Option<String> {
    let what = format!("{} `{}`", t.keyword, t.name);
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

/// SH19 over a crate that declares `kind = "contract"`.
pub fn command_rules(cx: &mut Ctx<'_>) {
    let names = Names::new(&cx.types);
    let n = cx.types.len();

    // 1. Local judgement, and the reverse-reference graph.
    let mut failing = vec![false; n];
    let mut cause: Vec<String> = vec![String::new(); n];
    let mut refs_by: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut queue: Vec<usize> = Vec::new();
    for (i, t) in cx.types.iter().enumerate() {
        crate::steps::tick();
        if !is_data(t) {
            continue;
        }
        let mut defect = flag_defect(t);
        for s in slots(t) {
            match names.judge(&s.ty) {
                Ok(named) => {
                    for j in named {
                        refs_by[j].push(i);
                    }
                }
                Err(why) => {
                    defect.get_or_insert_with(|| {
                        format!(
                            "{} `{}`, {}: {}",
                            t.keyword,
                            t.name,
                            s.place,
                            local_message(&s.ty, &why)
                        )
                    });
                }
            }
        }
        if let Some(d) = defect {
            failing[i] = true;
            cause[i] = d;
            queue.push(i);
        }
    }

    // 2. A type that mentions a failing type fails, with the same root cause.
    let mut head = 0;
    while head < queue.len() {
        crate::steps::tick();
        let j = queue[head];
        head += 1;
        for &p in &refs_by[j] {
            if !failing[p] {
                failing[p] = true;
                cause[p] = cause[j].clone();
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
        let what = format!("enum `{}`", t.name);
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
                    v.name
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
                Ok(named) => {
                    if let Some(&j) = named.iter().find(|&&j| failing[j]) {
                        add(
                            s.line,
                            format!(
                                "{what} {}: type `{}` is not a command payload ({})",
                                s.place, s.ty, cause[j]
                            ),
                        );
                    }
                }
            }
        }
    }
    cx.out.extend(found);
}
