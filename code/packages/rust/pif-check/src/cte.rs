//! The canonical term encoding (CTE) of VER32 3.6, and the goal digest built from it.
//!
//! # Why the signature binds a term encoding and not a printed goal
//!
//! A person signs a *goal*: the statement a theorem must have. A printed statement is not
//! injective (two different terms can print alike), so what the signature binds is the term
//! itself, serialised by a fixed prefix encoding that depends on no table index and no
//! sharing:
//!
//! ```text
//!   name  ::= '<' [comp {',' comp}] '>'        comp ::= STRING | NAT
//!   level ::= 'z' | 's(' level ')' | 'm(' level ',' level ')' | 'i(' level ',' level ')'
//!           | 'p' name
//!   expr  ::= 'v' NAT | 'S' level | 'C' name '[' [level {',' level}] ']'
//!           | 'A(' expr ',' expr ')' | 'N' NAT | 'T' STRING | 'J(' name ',' NAT ',' expr ')'
//!   binding form:  | 'L(' expr ',' expr ')' | 'P(' expr ',' expr ')' | 'Z(' expr ',' expr ',' expr ')'
//!   display form:  | 'L' b '(' name ',' expr ',' expr ')'  | 'P' b '(' name ',' expr ',' expr ')'
//!                  | 'Z(' name ',' expr ',' expr ',' expr ')'          b ::= 'd' | 'i' | 's' | 'c'
//! ```
//!
//! `A(f,a)` is an application, `L` a lambda, `P` a pi, `Z` a let, `J` a projection,
//! `v` a bound variable (its de Bruijn index), `C` a constant with its universe levels.
//!
//! * The **binding CTE** erases binder names and binder info. It is a function of the
//!   *kernel term*: the kernel's store ignores binder metadata for identity and keeps
//!   whichever spelling was interned first, so a form with names would give one goal several
//!   digests depending on interning order. The gate computes it **from the store**
//!   ([`binding_cte`]), never from the records of the file.
//! * The **display CTE** keeps binder names and info and is what the claim elaborator writes
//!   into the goals list. The gate checks `erase(display) = binding` ([`Dterm::binding`]).
//! * **Levels are compared syntactically**: `max u v` and `max v u` are different goals.
//! * Output is capped at 1 MiB ([`MAX_CTE_BYTES`]); a bigger goal is refused. A DAG of a
//!   thousand records can denote 2^1000 tree nodes, so nothing here ever builds the tree
//!   before checking the cap.
//!
//! # Byte layout, pinned
//!
//! VER32 gives the grammar above and says STRING and NAT are "written exactly as in PIF
//! (4.1)". This module therefore writes: a NAT in decimal with no sign and no leading zero;
//! a STRING in double quotes, printable ASCII 0x21 to 0x7E as itself except `"` and `\`
//! (written `\"` and `\\`), and **every other character** (space, controls, all non-ASCII)
//! as `\u{h}` with lowercase hex and no leading zeros. No byte of the output is a space or a
//! line feed. These are the same rules as the PIF reader's tokens, so a name has exactly one
//! spelling. The worked example's goal begins
//! `P(C<"Bool">[],P(C<"Bool">[],A(A(A(C<"Eq">[s(z)],C<"Bool">[]),` as the spec states.
//!
//! # No recursion on input-controlled depth
//!
//! A term's depth is attacker-controlled. Every traversal here (building a tree from the
//! store, emitting text, parsing text) uses an explicit stack whose size is bounded by the
//! output cap, never the call stack.

use coding_adventures_sha256::sha256;
use lean_kernel::{BinderInfo, ExprKind, ExprStore, Level, LevelKind, Literal, Name, NamePart};
use std::collections::HashSet;

use crate::digest::Digest;

/// The cap on an encoding: 1 MiB (VER32 3.6).
pub const MAX_CTE_BYTES: usize = 1 << 20;

/// Name limits, the reader's own (VER32 section 7): 32 components, 4096 decoded bytes.
const MAX_NAME_COMPONENTS: usize = 32;
const MAX_NAME_BYTES: usize = 4096;
/// A `nat` literal has at most 10,000 digits (VER32 4.1).
const MAX_NAT_DIGITS: usize = 10_000;

/// Why an encoding could not be produced or parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CteError {
    /// The encoding would exceed [`MAX_CTE_BYTES`].
    TooLarge,
    /// The term has a free variable; a declaration's type never does.
    FreeVariable,
    /// A term form the encoding has no spelling for yet.
    Unsupported(&'static str),
    /// The text is not a display CTE (why, byte offset).
    Syntax(&'static str, usize),
}

/// A binder's style letter in the display form.
fn info_letter(i: BinderInfo) -> u8 {
    match i {
        BinderInfo::Default => b'd',
        BinderInfo::Implicit => b'i',
        BinderInfo::StrictImplicit => b's',
        BinderInfo::InstImplicit => b'c',
    }
}

fn info_of(b: u8) -> Option<BinderInfo> {
    Some(match b {
        b'd' => BinderInfo::Default,
        b'i' => BinderInfo::Implicit,
        b's' => BinderInfo::StrictImplicit,
        b'c' => BinderInfo::InstImplicit,
        _ => return None,
    })
}

/// A node of an expression tree. Children are indices into [`Dterm::nodes`], always smaller
/// than the node's own index (a tree built in post-order), so the arena is acyclic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Var(u32),
    Sort(u32),
    Const(Name, Vec<u32>),
    App(u32, u32),
    Lam(BinderInfo, Name, u32, u32),
    Pi(BinderInfo, Name, u32, u32),
    Let(Name, u32, u32, u32),
    Nat(String),
    Str(String),
    Proj(Name, u32, u32),
}

/// A node of a level tree (indices into [`Dterm::levels`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lv {
    Zero,
    Succ(u32),
    Max(u32, u32),
    IMax(u32, u32),
    Param(Name),
}

/// A term as a **tree with names**: the display form of VER32 3.6, held as data so the same
/// value can be emitted as display CTE, erased to binding CTE, printed for a person, or
/// checked for the G7 name rules. It is built in post-order, so two equal trees are built
/// identically and derived equality is tree equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dterm {
    pub nodes: Vec<Node>,
    pub levels: Vec<Lv>,
    pub root: u32,
}

/// The size budget while building: nodes and level nodes each emit at least one byte, so a
/// tree with more than the byte cap in nodes cannot fit the encoding anyway.
fn too_big(nodes: usize, levels: usize) -> bool {
    nodes + levels > MAX_CTE_BYTES
}

// -----------------------------------------------------------------------------------------
// Building a tree from the kernel store.
// -----------------------------------------------------------------------------------------

/// A work item of the store walk. `Build` runs after the children's results are on the
/// result stack.
enum Walk {
    Visit(lean_kernel::Expr),
    Build(lean_kernel::Expr),
}

/// Unfold a level into the arena, with an explicit stack.
fn push_level(l: &Level, levels: &mut Vec<Lv>, nodes: usize) -> Result<u32, CteError> {
    enum W<'a> {
        Visit(&'a Level),
        Build(&'a Level),
    }
    let (mut work, mut out): (Vec<W>, Vec<u32>) = (vec![W::Visit(l)], vec![]);
    while let Some(w) = work.pop() {
        if too_big(nodes, levels.len()) {
            return Err(CteError::TooLarge);
        }
        match w {
            W::Visit(l) => match l.kind() {
                LevelKind::Zero => {
                    levels.push(Lv::Zero);
                    out.push(levels.len() as u32 - 1);
                }
                LevelKind::Param(n) => {
                    levels.push(Lv::Param(n.clone()));
                    out.push(levels.len() as u32 - 1);
                }
                LevelKind::Succ(a) => work.extend([W::Build(l), W::Visit(a)]),
                LevelKind::Max(a, b) | LevelKind::IMax(a, b) => {
                    work.extend([W::Build(l), W::Visit(b), W::Visit(a)]);
                }
            },
            W::Build(l) => {
                let node = match l.kind() {
                    LevelKind::Succ(_) => {
                        Lv::Succ(out.pop().ok_or(CteError::Unsupported("level"))?)
                    }
                    kind => {
                        let (Some(b), Some(a)) = (out.pop(), out.pop()) else {
                            return Err(CteError::Unsupported("level"));
                        };
                        if matches!(kind, LevelKind::Max(..)) {
                            Lv::Max(a, b)
                        } else {
                            Lv::IMax(a, b)
                        }
                    }
                };
                levels.push(node);
                out.push(levels.len() as u32 - 1);
            }
        }
    }
    out.pop().ok_or(CteError::Unsupported("level"))
}

impl Dterm {
    /// Unfold the kernel term `e` (a DAG in `store`) into a tree with the binder names the
    /// store currently holds. **For the binding CTE the names do not matter**: they are
    /// erased, and the store's choice among equal-up-to-names terms never reaches a digest.
    pub fn from_store(store: &ExprStore, e: lean_kernel::Expr) -> Result<Dterm, CteError> {
        let (mut nodes, mut levels): (Vec<Node>, Vec<Lv>) = (vec![], vec![]);
        let (mut work, mut out): (Vec<Walk>, Vec<u32>) = (vec![Walk::Visit(e)], vec![]);
        while let Some(w) = work.pop() {
            if too_big(nodes.len(), levels.len()) {
                return Err(CteError::TooLarge);
            }
            let leaf = |n: Node, nodes: &mut Vec<Node>, out: &mut Vec<u32>| {
                nodes.push(n);
                out.push(nodes.len() as u32 - 1);
            };
            match w {
                Walk::Visit(e) => match store.kind(e) {
                    ExprKind::BVar(i) => leaf(Node::Var(*i), &mut nodes, &mut out),
                    ExprKind::FVar(_) => return Err(CteError::FreeVariable),
                    ExprKind::Sort(l) => {
                        let id = push_level(l, &mut levels, nodes.len())?;
                        leaf(Node::Sort(id), &mut nodes, &mut out);
                    }
                    ExprKind::Const(n, ls) => {
                        let mut ids = Vec::with_capacity(ls.len());
                        for l in ls.iter() {
                            ids.push(push_level(l, &mut levels, nodes.len())?);
                        }
                        leaf(Node::Const(n.clone(), ids), &mut nodes, &mut out);
                    }
                    ExprKind::Lit(Literal::Nat(n)) => {
                        leaf(Node::Nat(n.to_string()), &mut nodes, &mut out)
                    }
                    ExprKind::Lit(Literal::Str(s)) => {
                        leaf(Node::Str(s.clone()), &mut nodes, &mut out)
                    }
                    ExprKind::App(f, a) => {
                        work.extend([Walk::Build(e), Walk::Visit(*a), Walk::Visit(*f)])
                    }
                    ExprKind::Lam(_, t, b) | ExprKind::Pi(_, t, b) => {
                        work.extend([Walk::Build(e), Walk::Visit(*b), Walk::Visit(*t)]);
                    }
                    ExprKind::Let(_, t, v, b) => {
                        work.extend([
                            Walk::Build(e),
                            Walk::Visit(*b),
                            Walk::Visit(*v),
                            Walk::Visit(*t),
                        ]);
                    }
                    ExprKind::Proj(_, _, x) => work.extend([Walk::Build(e), Walk::Visit(*x)]),
                },
                Walk::Build(e) => {
                    let mut pop = || out.pop().ok_or(CteError::Unsupported("term"));
                    let node = match store.kind(e) {
                        ExprKind::App(..) => {
                            let (a, f) = (pop()?, pop()?);
                            Node::App(f, a)
                        }
                        ExprKind::Lam(m, ..) => {
                            let (b, t) = (pop()?, pop()?);
                            Node::Lam(m.0.info, m.0.name.clone(), t, b)
                        }
                        ExprKind::Pi(m, ..) => {
                            let (b, t) = (pop()?, pop()?);
                            Node::Pi(m.0.info, m.0.name.clone(), t, b)
                        }
                        ExprKind::Let(m, ..) => {
                            let (b, v, t) = (pop()?, pop()?, pop()?);
                            Node::Let(m.0.name.clone(), t, v, b)
                        }
                        ExprKind::Proj(n, i, _) => Node::Proj(n.clone(), *i, pop()?),
                        _ => return Err(CteError::Unsupported("term")),
                    };
                    leaf(node, &mut nodes, &mut out);
                }
            }
        }
        let root = out.pop().ok_or(CteError::Unsupported("term"))?;
        Ok(Dterm {
            nodes,
            levels,
            root,
        })
    }

    /// The display CTE: binder names and info kept.
    pub fn display(&self) -> Result<String, CteError> {
        self.emit(true)
    }

    /// The binding CTE: this term with binder names and info **erased**.
    pub fn binding(&self) -> Result<String, CteError> {
        self.emit(false)
    }

    /// Emit with an explicit work stack. Items are pushed in reverse so they pop in order.
    fn emit(&self, display: bool) -> Result<String, CteError> {
        let mut out = String::new();
        let mut st = vec![It::E(self.root)];
        // Push `items` so that the first one is emitted first.
        fn put(st: &mut Vec<It>, items: Vec<It>) {
            st.extend(items.into_iter().rev());
        }
        while let Some(it) = st.pop() {
            if out.len() > MAX_CTE_BYTES {
                return Err(CteError::TooLarge);
            }
            match it {
                It::Lit(s) => out.push_str(s),
                It::Byte(b) => out.push(char::from(b)),
                It::Text(s) => out.push_str(&s),
                It::L(i) => match self
                    .levels
                    .get(i as usize)
                    .ok_or(CteError::Unsupported("level index"))?
                {
                    Lv::Zero => out.push('z'),
                    Lv::Succ(a) => put(&mut st, vec![It::Lit("s("), It::L(*a), It::Lit(")")]),
                    Lv::Max(a, b) => put(
                        &mut st,
                        vec![
                            It::Lit("m("),
                            It::L(*a),
                            It::Lit(","),
                            It::L(*b),
                            It::Lit(")"),
                        ],
                    ),
                    Lv::IMax(a, b) => put(
                        &mut st,
                        vec![
                            It::Lit("i("),
                            It::L(*a),
                            It::Lit(","),
                            It::L(*b),
                            It::Lit(")"),
                        ],
                    ),
                    Lv::Param(n) => {
                        out.push('p');
                        out.push_str(&name_cte(n));
                    }
                },
                It::E(i) => match self
                    .nodes
                    .get(i as usize)
                    .ok_or(CteError::Unsupported("node index"))?
                {
                    Node::Var(k) => out.push_str(&format!("v{k}")),
                    Node::Sort(l) => put(&mut st, vec![It::Lit("S"), It::L(*l)]),
                    Node::Nat(d) => out.push_str(&format!("N{d}")),
                    Node::Str(s) => out.push_str(&format!("T{}", quote(s))),
                    Node::Const(n, ls) => {
                        let mut v = vec![It::Text(format!("C{}[", name_cte(n)))];
                        for (k, l) in ls.iter().enumerate() {
                            if k > 0 {
                                v.push(It::Lit(","));
                            }
                            v.push(It::L(*l));
                        }
                        v.push(It::Lit("]"));
                        put(&mut st, v);
                    }
                    Node::App(f, a) => put(
                        &mut st,
                        vec![
                            It::Lit("A("),
                            It::E(*f),
                            It::Lit(","),
                            It::E(*a),
                            It::Lit(")"),
                        ],
                    ),
                    Node::Lam(inf, n, t, b) => put(&mut st, binder(b'L', display, *inf, n, *t, *b)),
                    Node::Pi(inf, n, t, b) => put(&mut st, binder(b'P', display, *inf, n, *t, *b)),
                    Node::Let(n, t, val, b) => {
                        let mut v = vec![It::Lit("Z(")];
                        if display {
                            v.extend([It::Text(name_cte(n)), It::Lit(",")]);
                        }
                        v.extend([
                            It::E(*t),
                            It::Lit(","),
                            It::E(*val),
                            It::Lit(","),
                            It::E(*b),
                            It::Lit(")"),
                        ]);
                        put(&mut st, v);
                    }
                    Node::Proj(n, k, x) => {
                        put(
                            &mut st,
                            vec![
                                It::Text(format!("J({},{k},", name_cte(n))),
                                It::E(*x),
                                It::Lit(")"),
                            ],
                        );
                    }
                },
            }
        }
        if out.len() > MAX_CTE_BYTES {
            return Err(CteError::TooLarge);
        }
        Ok(out)
    }
}

/// An item of the emitter's work stack.
enum It {
    Lit(&'static str),
    Byte(u8),
    Text(String),
    E(u32),
    L(u32),
}

/// The items of a binder node (`L` or `P`), in emission order.
fn binder(tag: u8, display: bool, info: BinderInfo, n: &Name, t: u32, b: u32) -> Vec<It> {
    let mut v = vec![It::Byte(tag)];
    if display {
        v.extend([
            It::Byte(info_letter(info)),
            It::Lit("("),
            It::Text(name_cte(n)),
            It::Lit(","),
        ]);
    } else {
        v.push(It::Lit("("));
    }
    v.extend([It::E(t), It::Lit(","), It::E(b), It::Lit(")")]);
    v
}

/// The binding CTE of the kernel term `e`, computed from the store (VER32 3.6).
pub fn binding_cte(store: &ExprStore, e: lean_kernel::Expr) -> Result<String, CteError> {
    Dterm::from_store(store, e)?.binding()
}

/// The display CTE of the kernel term `e` using the binder names the store holds. Used by
/// tests and by tools that produce goals lists; the gate itself reads display CTEs from the
/// goals list, never from the store (VER32 3.6: the store's names are whichever came first).
pub fn display_cte_from_store(store: &ExprStore, e: lean_kernel::Expr) -> Result<String, CteError> {
    Dterm::from_store(store, e)?.display()
}

/// The goal digest: SHA-256 of the binding CTE (G5).
pub fn goal_digest(store: &ExprStore, e: lean_kernel::Expr) -> Result<Digest, CteError> {
    Ok(Digest(sha256(binding_cte(store, e)?.as_bytes())))
}

// -----------------------------------------------------------------------------------------
// Names and strings.
// -----------------------------------------------------------------------------------------

/// `<comp,comp,...>` with string components quoted and numeric ones in decimal.
pub fn name_cte(n: &Name) -> String {
    let comps: Vec<String> = n
        .parts()
        .iter()
        .map(|p| match p {
            NamePart::Str(s) => quote(s),
            NamePart::Num(k) => k.to_string(),
        })
        .collect();
    format!("<{}>", comps.join(","))
}

/// A STRING token as PIF writes it: see the module documentation for the exact rules.
pub fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '!'..='~' => o.push(c),
            _ => o.push_str(&format!("\\u{{{:x}}}", c as u32)),
        }
    }
    o.push('"');
    o
}

// -----------------------------------------------------------------------------------------
// Parsing a display CTE (the goals list's second field).
// -----------------------------------------------------------------------------------------

/// What the parser still has to do. The stack of tasks replaces the call stack.
enum Task {
    Expr,
    Level,
    Lit(u8),
    Levels(Name, usize),
    Sort,
    App,
    Binder(bool, BinderInfo, Name),
    Let(Name),
    Proj(Name, u32),
    Succ,
    Max(bool),
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

type PR<T> = Result<T, CteError>;

impl<'a> P<'a> {
    fn err<T>(&self, why: &'static str) -> PR<T> {
        Err(CteError::Syntax(why, self.i))
    }
    fn next(&mut self) -> PR<u8> {
        let b = *self
            .s
            .get(self.i)
            .ok_or(CteError::Syntax("unexpected end", self.i))?;
        self.i += 1;
        Ok(b)
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn expect(&mut self, c: u8) -> PR<()> {
        if self.next()? == c {
            Ok(())
        } else {
            self.i -= 1;
            self.err("unexpected character")
        }
    }
    /// `0 | [1-9][0-9]*`: returns the digits.
    fn digits(&mut self) -> PR<&'a str> {
        let start = self.i;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.i += 1;
        }
        let t = &self.s[start..self.i];
        if t.is_empty() || (t[0] == b'0' && t.len() > 1) {
            self.i = start;
            return self.err("bad numeral");
        }
        std::str::from_utf8(t).or_else(|_| self.err("bad numeral"))
    }
    fn nat_u(&mut self, max: u64) -> PR<u64> {
        let at = self.i;
        let v = self.digits()?.parse::<u64>().ok().filter(|v| *v <= max);
        v.ok_or(CteError::Syntax("number out of range", at))
    }
    /// A STRING token, strict: only the canonical spellings of [`quote`].
    fn string(&mut self) -> PR<String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.next()? {
                b'"' => return Ok(out),
                b'\\' => match self.next()? {
                    c @ (b'"' | b'\\') => out.push(char::from(c)),
                    b'u' => {
                        self.expect(b'{')?;
                        let start = self.i;
                        while self
                            .peek()
                            .is_some_and(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                        {
                            self.i += 1;
                        }
                        let h = &self.s[start..self.i];
                        let ok = !h.is_empty() && h.len() <= 6 && (h[0] != b'0' || h.len() == 1);
                        self.expect(b'}')?;
                        if !ok {
                            return self.err("bad escape");
                        }
                        let v = h.iter().fold(0u32, |a, &b| {
                            a * 16 + char::from(b).to_digit(16).unwrap_or(0)
                        });
                        // Canonical only: a character with a direct spelling is never escaped.
                        let c = char::from_u32(v).filter(|c| !('!'..='~').contains(c));
                        out.push(c.ok_or(CteError::Syntax("non-canonical escape", self.i))?);
                    }
                    _ => return self.err("bad escape"),
                },
                b @ b'!'..=b'~' => out.push(char::from(b)),
                _ => return self.err("bad string byte"),
            }
            if out.len() > MAX_NAME_BYTES.max(MAX_CTE_BYTES) {
                return Err(CteError::TooLarge);
            }
        }
    }
    /// `<` components `>`.
    fn name(&mut self) -> PR<Name> {
        self.expect(b'<')?;
        let (mut n, mut count, mut bytes) = (Name::anonymous(), 0usize, 0usize);
        if self.peek() == Some(b'>') {
            self.i += 1;
            return Ok(n);
        }
        loop {
            count += 1;
            if self.peek() == Some(b'"') {
                let s = self.string()?;
                bytes += s.len();
                n = Name::str(&n, &s);
            } else {
                n = Name::num(&n, self.nat_u(u64::MAX)?);
                bytes += 8;
            }
            if count > MAX_NAME_COMPONENTS || bytes > MAX_NAME_BYTES {
                return self.err("name too long");
            }
            match self.next()? {
                b',' => {}
                b'>' => return Ok(n),
                _ => {
                    self.i -= 1;
                    return self.err("expected , or >");
                }
            }
        }
    }
}

impl Dterm {
    /// Parse a display CTE. **Strict**: the text must be exactly what [`Dterm::display`] emits
    /// for some tree (no spaces, canonical numerals and escapes, nothing after the term), so
    /// `parse(display(t)) = t` and two different texts never parse to one tree.
    pub fn parse(text: &str) -> Result<Dterm, CteError> {
        if text.len() > MAX_CTE_BYTES {
            return Err(CteError::TooLarge);
        }
        let mut p = P {
            s: text.as_bytes(),
            i: 0,
        };
        let (mut nodes, mut levels): (Vec<Node>, Vec<Lv>) = (vec![], vec![]);
        let (mut ex, mut lv): (Vec<u32>, Vec<u32>) = (vec![], vec![]);
        let mut tasks = vec![Task::Expr];
        // Push tasks so that the first listed runs first.
        fn queue(tasks: &mut Vec<Task>, ts: Vec<Task>) {
            tasks.extend(ts.into_iter().rev());
        }
        macro_rules! add {
            ($v:expr, $n:expr) => {{
                $v.push($n);
                u32::try_from($v.len() - 1).map_err(|_| CteError::TooLarge)?
            }};
        }
        while let Some(t) = tasks.pop() {
            if too_big(nodes.len(), levels.len()) {
                return Err(CteError::TooLarge);
            }
            match t {
                Task::Lit(c) => p.expect(c)?,
                Task::Expr => match p.next()? {
                    b'v' => {
                        let k = p.nat_u(u64::from(lean_kernel::MAX_BVAR_INDEX - 1))?;
                        ex.push(add!(nodes, Node::Var(k as u32)));
                    }
                    b'S' => queue(&mut tasks, vec![Task::Level, Task::Sort]),
                    b'C' => {
                        let n = p.name()?;
                        p.expect(b'[')?;
                        if p.peek() == Some(b']') {
                            p.i += 1;
                            ex.push(add!(nodes, Node::Const(n, vec![])));
                        } else {
                            queue(&mut tasks, vec![Task::Level, Task::Levels(n, 1)]);
                        }
                    }
                    b'A' => {
                        p.expect(b'(')?;
                        queue(
                            &mut tasks,
                            vec![
                                Task::Expr,
                                Task::Lit(b','),
                                Task::Expr,
                                Task::Lit(b')'),
                                Task::App,
                            ],
                        );
                    }
                    b'N' => {
                        let d = p.digits()?;
                        if d.len() > MAX_NAT_DIGITS {
                            return p.err("nat literal too long");
                        }
                        ex.push(add!(nodes, Node::Nat(d.to_string())));
                    }
                    b'T' => {
                        let s = p.string()?;
                        ex.push(add!(nodes, Node::Str(s)));
                    }
                    b'J' => {
                        p.expect(b'(')?;
                        let n = p.name()?;
                        p.expect(b',')?;
                        let k = p.nat_u(65_535)? as u32;
                        p.expect(b',')?;
                        queue(
                            &mut tasks,
                            vec![Task::Expr, Task::Lit(b')'), Task::Proj(n, k)],
                        );
                    }
                    tag @ (b'L' | b'P') => {
                        let Some(info) = info_of(p.next()?) else {
                            p.i -= 1;
                            return p.err("bad binder info");
                        };
                        p.expect(b'(')?;
                        let n = p.name()?;
                        p.expect(b',')?;
                        queue(
                            &mut tasks,
                            vec![
                                Task::Expr,
                                Task::Lit(b','),
                                Task::Expr,
                                Task::Lit(b')'),
                                Task::Binder(tag == b'L', info, n),
                            ],
                        );
                    }
                    b'Z' => {
                        p.expect(b'(')?;
                        let n = p.name()?;
                        p.expect(b',')?;
                        queue(
                            &mut tasks,
                            vec![
                                Task::Expr,
                                Task::Lit(b','),
                                Task::Expr,
                                Task::Lit(b','),
                                Task::Expr,
                                Task::Lit(b')'),
                                Task::Let(n),
                            ],
                        );
                    }
                    _ => {
                        p.i -= 1;
                        return p.err("not an expression");
                    }
                },
                Task::Level => match p.next()? {
                    b'z' => lv.push(add!(levels, Lv::Zero)),
                    b's' => {
                        p.expect(b'(')?;
                        queue(&mut tasks, vec![Task::Level, Task::Lit(b')'), Task::Succ]);
                    }
                    tag @ (b'm' | b'i') => {
                        p.expect(b'(')?;
                        queue(
                            &mut tasks,
                            vec![
                                Task::Level,
                                Task::Lit(b','),
                                Task::Level,
                                Task::Lit(b')'),
                                Task::Max(tag == b'i'),
                            ],
                        );
                    }
                    b'p' => {
                        let n = p.name()?;
                        lv.push(add!(levels, Lv::Param(n)));
                    }
                    _ => {
                        p.i -= 1;
                        return p.err("not a level");
                    }
                },
                Task::Levels(n, count) => match p.next()? {
                    b',' => queue(&mut tasks, vec![Task::Level, Task::Levels(n, count + 1)]),
                    b']' => {
                        let at = lv
                            .len()
                            .checked_sub(count)
                            .ok_or(CteError::Syntax("level stack", p.i))?;
                        let ids = lv.split_off(at);
                        ex.push(add!(nodes, Node::Const(n, ids)));
                    }
                    _ => {
                        p.i -= 1;
                        return p.err("expected , or ]");
                    }
                },
                Task::Sort => {
                    let l = lv.pop().ok_or(CteError::Syntax("level stack", p.i))?;
                    ex.push(add!(nodes, Node::Sort(l)));
                }
                Task::Succ => {
                    let l = lv.pop().ok_or(CteError::Syntax("level stack", p.i))?;
                    lv.push(add!(levels, Lv::Succ(l)));
                }
                Task::Max(imax) => {
                    let (Some(b), Some(a)) = (lv.pop(), lv.pop()) else {
                        return p.err("level stack");
                    };
                    lv.push(add!(
                        levels,
                        if imax { Lv::IMax(a, b) } else { Lv::Max(a, b) }
                    ));
                }
                Task::App => {
                    let (Some(a), Some(f)) = (ex.pop(), ex.pop()) else {
                        return p.err("term stack");
                    };
                    ex.push(add!(nodes, Node::App(f, a)));
                }
                Task::Binder(lam, info, n) => {
                    let (Some(b), Some(ty)) = (ex.pop(), ex.pop()) else {
                        return p.err("term stack");
                    };
                    ex.push(add!(
                        nodes,
                        if lam {
                            Node::Lam(info, n, ty, b)
                        } else {
                            Node::Pi(info, n, ty, b)
                        }
                    ));
                }
                Task::Let(n) => {
                    let (Some(b), Some(v), Some(ty)) = (ex.pop(), ex.pop(), ex.pop()) else {
                        return p.err("term stack");
                    };
                    ex.push(add!(nodes, Node::Let(n, ty, v, b)));
                }
                Task::Proj(n, k) => {
                    let x = ex.pop().ok_or(CteError::Syntax("term stack", p.i))?;
                    ex.push(add!(nodes, Node::Proj(n, k, x)));
                }
            }
        }
        if p.i != p.s.len() {
            return p.err("trailing bytes");
        }
        match ex.as_slice() {
            [root] => Ok(Dterm {
                nodes,
                levels,
                root: *root,
            }),
            _ => p.err("term stack"),
        }
    }

    /// Every constant name that occurs in the tree (levels and binder names excluded).
    pub fn constants(&self) -> HashSet<Name> {
        self.nodes
            .iter()
            .filter_map(|n| match n {
                Node::Const(c, _) | Node::Proj(c, ..) => Some(c.clone()),
                _ => None,
            })
            .collect()
    }
}
