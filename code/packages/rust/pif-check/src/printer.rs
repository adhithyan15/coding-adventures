//! The printer `pif-pp` (VER32 3.6, 1.1; trusted-base id `tcb.pif.printer`).
//!
//! # What it is for
//!
//! A person signs a *printed* goal and reviews *printed* definitions (the prelude, the
//! model). The printer maps a term to text. If it mapped two different terms to one text, the
//! person would sign or approve the wrong one. So the printer is trusted **unless** the gate
//! checks `parse(print(x)) = x` with an independent parser ([`crate::ppparse`], which shares
//! no code with this module). The gate does check it ([`crate::printcheck`]); this module's
//! job is to be *injective on what it accepts* and to refuse the rest.
//!
//! # The printed syntax
//!
//! ASCII, one line per declaration, de Bruijn indices written `#n` (so shadowing cannot
//! change a meaning, and the text mirrors the kernel term exactly):
//!
//! ```text
//!   term   ::= 'forall' binder ',' term | 'fun' binder '=>' term
//!            | 'let' name ':' term ':=' term ';' term | arrow
//!   arrow  ::= app [ '->' term ]            (only for a binder named `_`, default info, unused)
//!   app    ::= atom { atom }
//!   atom   ::= '#' NAT | const | '(' term ')' | 'Sort' level
//!   const  ::= name [ '.{' level { ',' level } '}' ]
//!   binder ::= '(' name ':' term ')'  default      '{' name ':' term '}'  implicit
//!            | '{|' name ':' term '|}' strict       '[' name ':' term ']'  inst-implicit
//!   level  ::= base { '+' NAT }        base ::= NAT | name | 'max(' level ',' level ')' | 'imax(' ... ')'
//! ```
//!
//! A name is its components joined by `.`. A numeral `n` is `succ` applied `n` times to zero;
//! `u+2` is `succ (succ u)`. Names and levels are *syntactic*, as in the CTE: `max u v` and
//! `max v u` print differently.
//!
//! # What it refuses (and why)
//!
//! * **Shadowed binders** ([`PrintError::Shadowed`]): `forall (a : T), forall (a : U), f #1 #0`.
//!   `#`-indices keep the text unambiguous, but a reader who sees two `a` will misread it, so
//!   the printer refuses when either binder is referred to. The anonymous `_` is exempt, and
//!   so are two binders that are *both unused* (they cannot be confused: the committed
//!   prelude writes `Prop -> Prop` as two unused binders named `a`).
//! * **Names outside the ASCII class**, numeric components and the anonymous name
//!   ([`PrintError::NameNotPrintable`]): there is no unambiguous spelling for them.
//! * **Reserved words** as names (`forall`, `fun`, `let`, `Sort`).
//! * **Literals and projections** ([`PrintError::Unprintable`]): the reader accepts neither
//!   yet, and the printer fails closed rather than invent a spelling.
//! * Output over 8 MiB ([`PrintError::TooLarge`]).
//!
//! Binder names come from the [`Dterm`] given (the display CTE), never from the kernel's
//! store, where they are whichever spelling was interned first (VER32 3.6).
//!
//! # No recursion on input-controlled depth
//!
//! Everything uses explicit stacks, like the rest of the gate.

use crate::cte::{Dterm, Lv, Node};
use lean_kernel::{BinderInfo, Name, NamePart};

/// The output cap.
pub const MAX_PRINT_BYTES: usize = 8 << 20;

/// Why a term could not be printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrintError {
    /// A binder name is already in scope.
    Shadowed(String),
    /// A name with no unambiguous printed spelling.
    NameNotPrintable,
    /// A name that is a keyword of the printed syntax.
    ReservedWord(String),
    /// A term form the printer has no spelling for (literal, projection).
    Unprintable(&'static str),
    /// The output would exceed [`MAX_PRINT_BYTES`].
    TooLarge,
    /// A malformed tree (an index out of range); cannot happen for a tree the gate built.
    Malformed,
}

const RESERVED: [&str; 4] = ["forall", "fun", "let", "Sort"];

fn comp_ok(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|x| x.is_ascii_alphabetic() || x == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '\'')
}

/// A name as printed: string components in the ASCII class joined by `.`.
pub fn print_name(n: &Name) -> Result<String, PrintError> {
    let parts = n.parts();
    let mut comps = Vec::with_capacity(parts.len());
    for p in &parts {
        match p {
            NamePart::Str(s) if comp_ok(s) => comps.push(s.as_str()),
            _ => return Err(PrintError::NameNotPrintable),
        }
    }
    if comps.is_empty() {
        return Err(PrintError::NameNotPrintable);
    }
    if comps.len() == 1 && RESERVED.contains(&comps[0]) {
        return Err(PrintError::ReservedWord(comps[0].to_string()));
    }
    Ok(comps.join("."))
}

/// Where a term is printed, which decides whether it needs parentheses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Top,
    ArrowLhs,
    AppHead,
    AppArg,
}

enum Item {
    Txt(String),
    T(u32, Ctx),
    /// Enter a binder's scope: its name and what its body refers to (the loose-variable
    /// mask of the body: bit `k` set when `#k` occurs).
    Push(Name, (u64, bool)),
    Pop,
}

/// What the tables say about a node's loose variables: bit `k` set if `#k` occurs free in the
/// node (for `k < 64`), and a flag for "some index of 64 or more may occur". The flag is
/// conservative: it can only make a binder print as `forall` instead of an arrow, never
/// change a meaning.
fn loose_masks(t: &Dterm) -> Vec<(u64, bool)> {
    let mut m: Vec<(u64, bool)> = Vec::with_capacity(t.nodes.len());
    let get = |m: &Vec<(u64, bool)>, i: u32| m.get(i as usize).copied().unwrap_or((u64::MAX, true));
    for n in &t.nodes {
        let v = match n {
            Node::Var(k) if *k < 64 => (1u64 << *k, false),
            Node::Var(_) => (0, true),
            Node::App(f, a) => {
                let (x, y) = (get(&m, *f), get(&m, *a));
                (x.0 | y.0, x.1 || y.1)
            }
            Node::Lam(_, _, ty, b) | Node::Pi(_, _, ty, b) => {
                let (x, y) = (get(&m, *ty), get(&m, *b));
                (x.0 | (y.0 >> 1), x.1 || y.1)
            }
            Node::Let(_, ty, val, b) => {
                let (x, y, z) = (get(&m, *ty), get(&m, *val), get(&m, *b));
                (x.0 | y.0 | (z.0 >> 1), x.1 || y.1 || z.1)
            }
            Node::Proj(_, _, e) => get(&m, *e),
            _ => (0, false),
        };
        m.push(v);
    }
    m
}

/// Print a level (iteratively): see the module documentation for the shapes.
pub fn print_level(t: &Dterm, id: u32) -> Result<String, PrintError> {
    enum L {
        Txt(&'static str),
        Lv(u32),
        Suffix(u32),
    }
    let mut out = String::new();
    let mut st = vec![L::Lv(id)];
    while let Some(x) = st.pop() {
        if out.len() > MAX_PRINT_BYTES {
            return Err(PrintError::TooLarge);
        }
        match x {
            L::Txt(s) => out.push_str(s),
            L::Suffix(k) => {
                if k > 0 {
                    out.push_str(&format!("+{k}"));
                }
            }
            L::Lv(i) => {
                // Peel the succ chain: `succ^k base`.
                let (mut k, mut cur) = (0u32, i);
                while let Some(Lv::Succ(a)) = t.levels.get(cur as usize) {
                    k += 1;
                    cur = *a;
                }
                match t.levels.get(cur as usize).ok_or(PrintError::Malformed)? {
                    Lv::Zero => out.push_str(&k.to_string()),
                    Lv::Param(n) => {
                        out.push_str(&print_name(n)?);
                        st.push(L::Suffix(k));
                    }
                    Lv::Max(a, b) | Lv::IMax(a, b) => {
                        let word = if matches!(t.levels.get(cur as usize), Some(Lv::Max(..))) {
                            "max("
                        } else {
                            "imax("
                        };
                        st.push(L::Suffix(k));
                        st.extend([L::Txt(")"), L::Lv(*b), L::Txt(", "), L::Lv(*a)]);
                        out.push_str(word);
                    }
                    Lv::Succ(_) => return Err(PrintError::Malformed),
                }
            }
        }
    }
    Ok(out)
}

/// The text of a binder's opening and closing bracket.
fn brackets(i: BinderInfo) -> (&'static str, &'static str) {
    match i {
        BinderInfo::Default => ("(", ")"),
        BinderInfo::Implicit => ("{", "}"),
        BinderInfo::StrictImplicit => ("{|", "|}"),
        BinderInfo::InstImplicit => ("[", "]"),
    }
}

/// Print a term. Refuses shadowed binders and anything without an unambiguous spelling.
pub fn print_term(t: &Dterm) -> Result<String, PrintError> {
    let masks = loose_masks(t);
    let mut out = String::new();
    // Every binder in scope, innermost last (so a binder's de Bruijn distance is its position).
    let mut scope: Vec<Name> = Vec::new();
    let mask_of = |b: u32| masks.get(b as usize).copied().unwrap_or((u64::MAX, true));
    let mut st = vec![Item::T(t.root, Ctx::Top)];
    let anon = Name::from_dotted("_");
    while let Some(item) = st.pop() {
        if out.len() > MAX_PRINT_BYTES {
            return Err(PrintError::TooLarge);
        }
        match item {
            Item::Txt(s) => out.push_str(&s),
            Item::Pop => {
                scope.pop();
            }
            Item::Push(n, mask) => {
                // A name already in scope is shadowed. That only misleads a reader when the
                // text can then refer to either binder: the new binder is used in its body, or
                // the old one is used there (at de Bruijn distance `d`). Two binders that are
                // never referred to inside the inner scope cannot be confused: the committed
                // prelude writes `Prop -> Prop` as two unused binders named `a`.
                let i = scope.len();
                let inner_used = mask.0 & 1 != 0 || mask.1;
                let clash = n != anon
                    && scope.iter().enumerate().any(|(o, m)| {
                        let d = i - o;
                        *m == n
                            && (inner_used
                                || if d < 64 {
                                    (mask.0 >> d) & 1 != 0
                                } else {
                                    mask.1
                                })
                    });
                if clash {
                    return Err(PrintError::Shadowed(print_name(&n)?));
                }
                scope.push(n);
            }
            Item::T(i, ctx) => {
                let node = t.nodes.get(i as usize).ok_or(PrintError::Malformed)?;
                // Push `items` so the first is printed first, wrapped in parentheses when `paren`.
                let emit = |st: &mut Vec<Item>, paren: bool, mut items: Vec<Item>| {
                    if paren {
                        items.insert(0, Item::Txt("(".into()));
                        items.push(Item::Txt(")".into()));
                    }
                    st.extend(items.into_iter().rev());
                };
                match node {
                    Node::Var(k) => out.push_str(&format!("#{k}")),
                    Node::Nat(_) => return Err(PrintError::Unprintable("natural literal")),
                    Node::Str(_) => return Err(PrintError::Unprintable("string literal")),
                    Node::Proj(..) => return Err(PrintError::Unprintable("projection")),
                    Node::Const(n, ls) => {
                        out.push_str(&print_name(n)?);
                        if !ls.is_empty() {
                            out.push_str(".{");
                            for (k, l) in ls.iter().enumerate() {
                                if k > 0 {
                                    out.push_str(", ");
                                }
                                out.push_str(&print_level(t, *l)?);
                            }
                            out.push('}');
                        }
                    }
                    Node::Sort(l) => {
                        let text = format!("Sort {}", print_level(t, *l)?);
                        out.push_str(&if matches!(ctx, Ctx::AppHead | Ctx::AppArg) {
                            format!("({text})")
                        } else {
                            text
                        });
                    }
                    Node::App(..) => {
                        // Collect the spine: head and arguments, without recursion.
                        let (mut args, mut cur) = (Vec::new(), i);
                        while let Some(Node::App(f, a)) = t.nodes.get(cur as usize) {
                            args.push(*a);
                            cur = *f;
                        }
                        let mut items = vec![Item::T(cur, Ctx::AppHead)];
                        for a in args.iter().rev() {
                            items.extend([Item::Txt(" ".into()), Item::T(*a, Ctx::AppArg)]);
                        }
                        emit(&mut st, ctx == Ctx::AppArg, items);
                    }
                    Node::Lam(info, n, ty, b) | Node::Pi(info, n, ty, b) => {
                        let lam = matches!(node, Node::Lam(..));
                        let paren = ctx != Ctx::Top;
                        let arrow = !lam
                            && *n == anon
                            && *info == BinderInfo::Default
                            && masks.get(*b as usize).is_some_and(|m| m.0 & 1 == 0 && !m.1);
                        if arrow {
                            emit(
                                &mut st,
                                paren,
                                vec![
                                    Item::T(*ty, Ctx::ArrowLhs),
                                    Item::Txt(" -> ".into()),
                                    Item::Push(anon.clone(), mask_of(*b)),
                                    Item::T(*b, Ctx::Top),
                                    Item::Pop,
                                ],
                            );
                        } else {
                            let (open, close) = brackets(*info);
                            let kw = if lam { "fun " } else { "forall " };
                            let sep = if lam { " => " } else { ", " };
                            emit(
                                &mut st,
                                paren,
                                vec![
                                    Item::Txt(format!("{kw}{open}{} : ", print_name(n)?)),
                                    Item::T(*ty, Ctx::Top),
                                    Item::Txt(format!("{close}{sep}")),
                                    Item::Push(n.clone(), mask_of(*b)),
                                    Item::T(*b, Ctx::Top),
                                    Item::Pop,
                                ],
                            );
                        }
                    }
                    Node::Let(n, ty, val, b) => emit(
                        &mut st,
                        ctx != Ctx::Top,
                        vec![
                            Item::Txt(format!("let {} : ", print_name(n)?)),
                            Item::T(*ty, Ctx::Top),
                            Item::Txt(" := ".into()),
                            Item::T(*val, Ctx::Top),
                            Item::Txt("; ".into()),
                            Item::Push(n.clone(), mask_of(*b)),
                            Item::T(*b, Ctx::Top),
                            Item::Pop,
                        ],
                    ),
                }
            }
        }
    }
    Ok(out)
}
