//! `pif-writer`: build Proof Interchange Format (PIF) files without hand-numbering records.
//!
//! # What this crate is, and what it is not
//!
//! A PIF file ([VER32](../../../specs/VER32-proof-interchange-format.md) section 4) is lines
//! of ASCII. Names, levels and expressions each live in a table, and a record may mention
//! only an index that an earlier record defined:
//!
//! ```text
//!   nstr 1 0 "Bool"        name 1  = anonymous . "Bool"
//!   lsucc 1 0              level 1 = succ zero
//!   esort 1 1              expr 1  = Sort level1
//!   econst 2 1 []          expr 2  = the constant named name1
//! ```
//!
//! Numbering those by hand is exactly the kind of off-by-one that a producer should not own.
//! [`Writer`] hands out typed handles ([`Name`], [`Level`], [`Expr`]) and does the numbering.
//!
//! This crate is an **untrusted producer**. The trusted reader (`lean_kernel::pif::read`)
//! rechecks every byte, so a bug here can make a file be *rejected* but can never make the
//! kernel accept something false. What this crate does promise is the converse: for
//! well-formed input (handles taken from the same writer, valid strings) the text it emits
//! is accepted by the reader's lexical and structural rules. The tests check that with the
//! real reader.
//!
//! # What "canonical" means here (VER32 4.5)
//!
//! * **Dense, backward-only indices.** Each new record takes the next index of its table,
//!   and a handle exists only after its record was emitted, so a forward reference cannot be
//!   expressed at all.
//! * **Maximal sharing.** Every distinct name, level and expression is stored once: asking
//!   for the same node again returns the same handle and emits no record. (Binder names and
//!   binder infos are part of a node's identity, because they are part of its record.)
//! * **Post-order, in creation order.** Children exist before their parent, and records appear
//!   in the order the caller first created them. The text is therefore a pure function of the
//!   call sequence: the same calls give byte-identical output. No table is ever iterated, so
//!   there is no hash-order nondeterminism.
//! * **Exact lexical form.** One space between fields, brackets as `[]`, `[1]`, `[1 2]`,
//!   strings with only the escapes `\"`, `\\` and `\u{h}` (lowercase hex, no leading zero; a
//!   space is `\u{20}`), LF line ends, and the trailer `end <count>`.
//!
//! # A taste
//!
//! ```
//! use pif_writer::Writer;
//! let mut w = Writer::new("native", "demo 1").unwrap();
//! let t = w.name("T").unwrap();
//! let one = w.level_succ(pif_writer::Level::ZERO);
//! let ty = w.sort(one);
//! w.axiom(t, &[], ty).unwrap();
//! assert!(w.to_text().starts_with("pif 1\nlogic 1\nprofile native \"demo\\u{20}1\"\n"));
//! ```

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

macro_rules! handle {
    ($(#[$m:meta])* $t:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $t(u32);
        impl $t {
            /// The record index this handle denotes (the number written in the file).
            pub fn index(self) -> u32 {
                self.0
            }
        }
    };
}

handle!(
    /// A name: index 0 is the anonymous name, every other index is an `nstr` or `nnum` record.
    Name
);
handle!(
    /// A universe level: index 0 is `zero`, every other index is a level record.
    Level
);
handle!(
    /// An expression; index 0 is never valid (expressions start at 1).
    Expr
);

impl Name {
    /// The anonymous name, parent of every top-level name. It may not name a declaration.
    pub const ANON: Name = Name(0);
}

impl Level {
    /// Universe level zero (predefined, so it needs no record).
    pub const ZERO: Level = Level(0);
}

/// The binder info of `elam` and `epi` (VER32 3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinderInfo {
    Default,
    Implicit,
    Strict,
    Inst,
}

/// The reducibility hint of a `def` (VER32 3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    Abbrev,
    Opaque,
    Regular(u32),
}

/// One type of an `inductive` block: its name, its type, and its constructors in order.
#[derive(Clone, Debug)]
pub struct IndType {
    pub name: Name,
    pub ty: Expr,
    pub ctors: Vec<(Name, Expr)>,
}

/// Why a builder refused an input. Each variant names a rule of VER32 5.3 the reader would
/// otherwise have rejected, so a producer finds the mistake at the call that made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// R-NAM-1 (and R-NAM-3 for an ASCII-only writer): a bad name component.
    BadNameComponent(String),
    /// A profile id, feature or role that is not a `FIDENT`.
    BadIdent(String),
    /// The `profile` label contains U+0000, which only an `estr` literal may spell.
    BadLabel,
    /// R-LIM-1: a name deeper than 32 components or longer than 4096 decoded bytes.
    NameTooLong,
    /// R-NAM-2: the anonymous name cannot name a declaration.
    AnonymousName,
    /// R-CNT-1: universe parameter names must be distinct.
    DuplicateParam,
    /// R-CNT-1: an `inductive` block needs at least one type.
    EmptyBlock,
    /// R-GOL-3: a claim id outside `[A-Za-z0-9_.-]{1,128}`.
    BadClaimId(String),
    /// A digest that is not 64 lowercase hex digits.
    BadDigest(String),
    /// A numeral that is not canonical decimal (no sign, no leading zero).
    BadNumeral(String),
    /// A bound-variable index at or above the kernel's bound (R-IDX-3).
    BVarTooLarge(u32),
}

/// The kernel's bound on `evar` indices (`MAX_BVAR_INDEX` in `lean-kernel`), restated here
/// because this crate does not depend on the kernel outside its tests.
pub const MAX_VAR_INDEX: u32 = u32::MAX / 2;

/// The reader's limits on one name (`PifLimits` defaults): components and total decoded bytes
/// (a numeric component counts as 8). Hashing and comparing a name costs its length, so the
/// reader caps it; the writer refuses to emit a name the reader would reject.
pub const MAX_NAME_COMPONENTS: u32 = 32;
pub const MAX_NAME_BYTES: usize = 4096;

#[derive(Clone, PartialEq, Eq, Hash)]
enum NameKey {
    Str(Name, String),
    Num(Name, u64),
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum LevelKey {
    Succ(Level),
    Max(Level, Level),
    IMax(Level, Level),
    Param(Name),
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum ExprKey {
    Var(u32),
    Sort(Level),
    Const(Name, Vec<Level>),
    App(Expr, Expr),
    Lam(BinderInfo, Name, Expr, Expr),
    Pi(BinderInfo, Name, Expr, Expr),
    Let(Name, Expr, Expr, Expr),
    Nat(String),
    Str(String),
    Proj(Name, u32, Expr),
}

/// A PIF file under construction. See the crate documentation for the guarantees.
pub struct Writer {
    profile: (String, String),
    features: BTreeSet<String>,
    imports: Vec<String>,
    ascii_names: bool,
    body: Vec<String>,
    names: HashMap<NameKey, Name>,
    /// (components, decoded bytes) of each name, indexed by name; entry 0 is the anonymous name.
    name_size: Vec<(u32, usize)>,
    levels: HashMap<LevelKey, Level>,
    exprs: HashMap<ExprKey, Expr>,
}

/// `FIDENT`: `[a-z][a-z0-9_.]*`, and never one of the four binder words, which the grammar
/// lexes as binder infos.
fn is_ident(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| c.is_ascii_lowercase())
        && cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
        && !["default", "implicit", "strict", "inst"].contains(&s)
}

/// VER32 3.8: a component is non-empty, has no `.`, and no control, format or private-use
/// character.
fn is_component(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| {
            let u = c as u32;
            !(c == '.'
                || u < 0x20
                || (0x7F..=0x9F).contains(&u)
                || (0x200B..=0x200F).contains(&u)
                || (0x202A..=0x202E).contains(&u)
                || (0x2060..=0x2064).contains(&u)
                || (0x2066..=0x206F).contains(&u)
                || u == 0xFEFF
                || (0xE000..=0xF8FF).contains(&u)
                || u >= 0xF0000)
        })
}

/// The ASCII class of 3.8: `[A-Za-z_][A-Za-z0-9_']*`.
fn is_ascii_class(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '\'')
}

/// The string field of the grammar: printable ASCII 0x21 to 0x7E is written as is, except
/// `"` and `\`, which are `\"` and `\\`; everything else (space, controls, non-ASCII) is
/// `\u{h}` with lowercase hex and no leading zero. U+0000 comes out as `\u{0}`, which only an
/// `estr` literal may carry.
fn quote(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\x21'..='\x7e' => o.push(c),
            _ => {
                let _ = write!(o, "\\u{{{:x}}}", c as u32);
            }
        }
    }
    o.push('"');
    o
}

fn is_digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn list<T: Copy>(items: &[T], idx: impl Fn(T) -> u32) -> String {
    let v: Vec<String> = items.iter().map(|&i| idx(i).to_string()).collect();
    format!("[{}]", v.join(" "))
}

impl BinderInfo {
    fn word(self) -> &'static str {
        match self {
            BinderInfo::Default => "default",
            BinderInfo::Implicit => "implicit",
            BinderInfo::Strict => "strict",
            BinderInfo::Inst => "inst",
        }
    }
}

impl Writer {
    /// Start a file for the given profile id (`native`, `lean4`, ...) and free-text label.
    pub fn new(profile_id: &str, label: &str) -> Result<Writer, WriteError> {
        if !is_ident(profile_id) {
            return Err(WriteError::BadIdent(profile_id.to_string()));
        }
        if label.contains('\0') {
            return Err(WriteError::BadLabel);
        }
        Ok(Writer {
            profile: (profile_id.to_string(), label.to_string()),
            features: BTreeSet::new(),
            imports: vec![],
            ascii_names: false,
            body: vec![],
            names: HashMap::new(),
            name_size: vec![(0, 0)],
            levels: HashMap::new(),
            exprs: HashMap::new(),
        })
    }

    /// Restrict every name component to the ASCII class of VER32 3.8. Use this for files a
    /// person reviews (the prelude, models, claims): the reader enforces R-NAM-3 on them.
    pub fn ascii_names_only(mut self) -> Writer {
        self.ascii_names = true;
        self
    }

    /// Declare a feature on the `features` line. Features used by records the writer emits
    /// (`lits`, `quotient`, `mutual`) are added automatically; `nested` must be declared here.
    pub fn feature(&mut self, id: &str) -> Result<(), WriteError> {
        if !is_ident(id) {
            return Err(WriteError::BadIdent(id.to_string()));
        }
        self.features.insert(id.to_string());
        Ok(())
    }

    /// Add an `import` line: the SHA-256 of a file earlier in the chain, as 64 lowercase hex digits.
    pub fn import(&mut self, hex_digest: &str) -> Result<(), WriteError> {
        if !is_digest(hex_digest) {
            return Err(WriteError::BadDigest(hex_digest.to_string()));
        }
        self.imports.push(format!("sha256:{hex_digest}"));
        Ok(())
    }

    fn push(&mut self, line: String) {
        self.body.push(line);
    }

    // ---- names -----------------------------------------------------------------------

    /// `parent` extended by one string component; the same request returns the same handle.
    pub fn name_str(&mut self, parent: Name, comp: &str) -> Result<Name, WriteError> {
        self.check_component(comp)?;
        let key = NameKey::Str(parent, comp.to_string());
        if let Some(&n) = self.names.get(&key) {
            return Ok(n);
        }
        let size = self.grow(parent, comp.len())?;
        let n = Name(self.names.len() as u32 + 1);
        self.push(format!("nstr {} {} {}", n.0, parent.0, quote(comp)));
        self.names.insert(key, n);
        self.name_size.push(size);
        Ok(n)
    }

    /// The size of `parent` extended by `bytes` more bytes, or `NameTooLong`.
    fn grow(&self, parent: Name, bytes: usize) -> Result<(u32, usize), WriteError> {
        let (c, b) = self.name_size[parent.0 as usize];
        if c + 1 > MAX_NAME_COMPONENTS || b + bytes > MAX_NAME_BYTES {
            return Err(WriteError::NameTooLong);
        }
        Ok((c + 1, b + bytes))
    }

    fn check_component(&self, comp: &str) -> Result<(), WriteError> {
        let ok = is_component(comp) && (!self.ascii_names || is_ascii_class(comp));
        ok.then_some(())
            .ok_or_else(|| WriteError::BadNameComponent(comp.to_string()))
    }

    /// `parent` extended by a numeric component (a different name from the string `"3"`).
    pub fn name_num(&mut self, parent: Name, num: u64) -> Result<Name, WriteError> {
        let key = NameKey::Num(parent, num);
        if let Some(&n) = self.names.get(&key) {
            return Ok(n);
        }
        let size = self.grow(parent, 8)?;
        let n = Name(self.names.len() as u32 + 1);
        self.push(format!("nnum {} {} {num}", n.0, parent.0));
        self.names.insert(key, n);
        self.name_size.push(size);
        Ok(n)
    }

    /// A dotted name such as `Bool.rec`, built one component at a time, parents first.
    pub fn name(&mut self, dotted: &str) -> Result<Name, WriteError> {
        // Validate every component first so a refusal leaves no half-built prefix behind.
        dotted
            .split('.')
            .try_for_each(|c| self.check_component(c))?;
        let parts = dotted.split('.').count() as u32;
        let bytes: usize = dotted.split('.').map(str::len).sum();
        if parts > MAX_NAME_COMPONENTS || bytes > MAX_NAME_BYTES {
            return Err(WriteError::NameTooLong);
        }
        dotted
            .split('.')
            .try_fold(Name::ANON, |p, c| self.name_str(p, c))
    }

    // ---- levels ----------------------------------------------------------------------

    fn level(&mut self, key: LevelKey, line: impl FnOnce(u32) -> String) -> Level {
        if let Some(&l) = self.levels.get(&key) {
            return l;
        }
        let l = Level(self.levels.len() as u32 + 1);
        self.push(line(l.0));
        self.levels.insert(key, l);
        l
    }

    pub fn level_succ(&mut self, l: Level) -> Level {
        self.level(LevelKey::Succ(l), |i| format!("lsucc {i} {}", l.0))
    }

    pub fn level_max(&mut self, a: Level, b: Level) -> Level {
        self.level(LevelKey::Max(a, b), |i| format!("lmax {i} {} {}", a.0, b.0))
    }

    pub fn level_imax(&mut self, a: Level, b: Level) -> Level {
        self.level(LevelKey::IMax(a, b), |i| {
            format!("limax {i} {} {}", a.0, b.0)
        })
    }

    pub fn level_param(&mut self, name: Name) -> Level {
        self.level(LevelKey::Param(name), |i| format!("lparam {i} {}", name.0))
    }

    // ---- expressions -----------------------------------------------------------------

    fn expr(&mut self, key: ExprKey, line: impl FnOnce(u32) -> String) -> Expr {
        if let Some(&e) = self.exprs.get(&key) {
            return e;
        }
        let e = Expr(self.exprs.len() as u32 + 1);
        self.push(line(e.0));
        self.exprs.insert(key, e);
        e
    }

    /// A bound variable (de Bruijn index). Panics above [`MAX_VAR_INDEX`]; use
    /// [`Writer::try_var`] when the index is not under the caller's control.
    pub fn var(&mut self, i: u32) -> Expr {
        self.try_var(i).expect("bound-variable index too large")
    }

    pub fn try_var(&mut self, i: u32) -> Result<Expr, WriteError> {
        if i >= MAX_VAR_INDEX {
            return Err(WriteError::BVarTooLarge(i));
        }
        Ok(self.expr(ExprKey::Var(i), |e| format!("evar {e} {i}")))
    }

    pub fn sort(&mut self, l: Level) -> Expr {
        self.expr(ExprKey::Sort(l), |e| format!("esort {e} {}", l.0))
    }

    pub fn constant(&mut self, n: Name, levels: &[Level]) -> Expr {
        let key = ExprKey::Const(n, levels.to_vec());
        self.expr(key, |e| {
            format!("econst {e} {} {}", n.0, list(levels, |l| l.0))
        })
    }

    pub fn app(&mut self, f: Expr, a: Expr) -> Expr {
        self.expr(ExprKey::App(f, a), |e| format!("eapp {e} {} {}", f.0, a.0))
    }

    /// `f a1 a2 ...`, left-nested, one `eapp` per argument.
    pub fn app_many(&mut self, f: Expr, args: &[Expr]) -> Expr {
        args.iter().fold(f, |acc, &a| self.app(acc, a))
    }

    /// `fun (name : ty) => body`; the binder is index 0 inside `body`.
    pub fn lam(&mut self, info: BinderInfo, name: Name, ty: Expr, body: Expr) -> Expr {
        let key = ExprKey::Lam(info, name, ty, body);
        self.expr(key, |e| {
            format!("elam {e} {} {} {} {}", info.word(), name.0, ty.0, body.0)
        })
    }

    /// `(name : ty) -> body`; the binder is index 0 inside `body`.
    pub fn pi(&mut self, info: BinderInfo, name: Name, ty: Expr, body: Expr) -> Expr {
        let key = ExprKey::Pi(info, name, ty, body);
        self.expr(key, |e| {
            format!("epi {e} {} {} {} {}", info.word(), name.0, ty.0, body.0)
        })
    }

    pub fn let_(&mut self, name: Name, ty: Expr, val: Expr, body: Expr) -> Expr {
        let key = ExprKey::Let(name, ty, val, body);
        self.expr(key, |e| {
            format!("elet {e} {} {} {} {}", name.0, ty.0, val.0, body.0)
        })
    }

    /// A natural-number literal given as canonical decimal; declares the `lits` feature.
    pub fn nat(&mut self, decimal: &str) -> Result<Expr, WriteError> {
        let canon = !decimal.is_empty()
            && decimal.bytes().all(|b| b.is_ascii_digit())
            && (decimal == "0" || !decimal.starts_with('0'));
        if !canon {
            return Err(WriteError::BadNumeral(decimal.to_string()));
        }
        self.features.insert("lits".to_string());
        Ok(self.expr(ExprKey::Nat(decimal.to_string()), |e| {
            format!("enat {e} {decimal}")
        }))
    }

    /// A string literal (U+0000 allowed); declares the `lits` feature.
    pub fn string(&mut self, s: &str) -> Expr {
        self.features.insert("lits".to_string());
        self.expr(ExprKey::Str(s.to_string()), |e| {
            format!("estr {e} {}", quote(s))
        })
    }

    /// `eproj`: field `idx` of structure `st` applied to `e`.
    pub fn proj(&mut self, st: Name, idx: u32, e: Expr) -> Expr {
        self.expr(ExprKey::Proj(st, idx, e), |i| {
            format!("eproj {i} {} {idx} {}", st.0, e.0)
        })
    }

    // ---- declarations ----------------------------------------------------------------

    /// The checks every declaration head shares: a real name, distinct universe parameters.
    fn head(name: Name, params: &[Name]) -> Result<String, WriteError> {
        if name == Name::ANON {
            return Err(WriteError::AnonymousName);
        }
        let distinct: BTreeSet<_> = params.iter().collect();
        if distinct.len() != params.len() {
            return Err(WriteError::DuplicateParam);
        }
        Ok(list(params, |n| n.0))
    }

    pub fn axiom(&mut self, name: Name, params: &[Name], ty: Expr) -> Result<(), WriteError> {
        let ps = Self::head(name, params)?;
        self.push(format!("axiom {} {ps} {}", name.0, ty.0));
        Ok(())
    }

    pub fn definition(
        &mut self,
        name: Name,
        params: &[Name],
        ty: Expr,
        val: Expr,
        hint: Hint,
    ) -> Result<(), WriteError> {
        let ps = Self::head(name, params)?;
        let h = match hint {
            Hint::Abbrev => "@abbrev".to_string(),
            Hint::Opaque => "@opaque".to_string(),
            Hint::Regular(n) => format!("@regular {n}"),
        };
        self.push(format!("def {} {ps} {} {} {h}", name.0, ty.0, val.0));
        Ok(())
    }

    pub fn theorem(
        &mut self,
        name: Name,
        params: &[Name],
        ty: Expr,
        val: Expr,
    ) -> Result<(), WriteError> {
        let ps = Self::head(name, params)?;
        self.push(format!("theorem {} {ps} {} {}", name.0, ty.0, val.0));
        Ok(())
    }

    pub fn opaque(
        &mut self,
        name: Name,
        params: &[Name],
        ty: Expr,
        val: Expr,
    ) -> Result<(), WriteError> {
        let ps = Self::head(name, params)?;
        self.push(format!("opaque {} {ps} {} {}", name.0, ty.0, val.0));
        Ok(())
    }

    /// An inductive block: `params` are the universe parameter names, `num_params` the number
    /// of type parameters. More than one type declares the `mutual` feature.
    pub fn inductive(
        &mut self,
        params: &[Name],
        num_params: u32,
        types: &[IndType],
    ) -> Result<(), WriteError> {
        if types.is_empty() {
            return Err(WriteError::EmptyBlock);
        }
        let ps = Self::head(types[0].name, params)?;
        if types.iter().any(|t| t.name == Name::ANON)
            || types
                .iter()
                .flat_map(|t| &t.ctors)
                .any(|c| c.0 == Name::ANON)
        {
            return Err(WriteError::AnonymousName);
        }
        if types.len() > 1 {
            self.features.insert("mutual".to_string());
        }
        self.push(format!("inductive {ps} {num_params} {}", types.len()));
        for t in types {
            self.push(format!("itype {} {} {}", t.name.0, t.ty.0, t.ctors.len()));
            for (n, ty) in &t.ctors {
                self.push(format!("ictor {} {}", n.0, ty.0));
            }
        }
        Ok(())
    }

    /// The quotient primitive; declares the `quotient` feature.
    pub fn quot(&mut self) {
        self.features.insert("quotient".to_string());
        self.push("quot".to_string());
    }

    /// Bind a primitive role (`nat.add`, ...) to a declared constant.
    pub fn prim(&mut self, role: &str, constant: Name) -> Result<(), WriteError> {
        if !is_ident(role) {
            return Err(WriteError::BadIdent(role.to_string()));
        }
        self.push(format!("prim {role} {}", constant.0));
        Ok(())
    }

    /// Bind a claim id to a theorem declared in this file. The digest is 64 lowercase hex
    /// digits; the gate recomputes it, so it is only a hint.
    pub fn goal(
        &mut self,
        claim_id: &str,
        theorem: Name,
        hex_digest: &str,
    ) -> Result<(), WriteError> {
        let ok_id = (1..=128).contains(&claim_id.len())
            && claim_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b));
        if !ok_id {
            return Err(WriteError::BadClaimId(claim_id.to_string()));
        }
        if !is_digest(hex_digest) {
            return Err(WriteError::BadDigest(hex_digest.to_string()));
        }
        self.push(format!(
            "goal {} {} sha256:{hex_digest}",
            quote(claim_id),
            theorem.0
        ));
        Ok(())
    }

    // ---- output ----------------------------------------------------------------------

    /// How many record lines have been emitted (the `k` of `end k`).
    pub fn record_count(&self) -> usize {
        self.body.len()
    }

    /// The finished file: header, imports, records and `end <count>`, LF line ends.
    pub fn to_text(&self) -> String {
        let mut o = String::from("pif 1\nlogic 1\n");
        let _ = writeln!(o, "profile {} {}", self.profile.0, quote(&self.profile.1));
        o.push_str("features");
        for f in &self.features {
            let _ = write!(o, " {f}");
        }
        o.push('\n');
        for i in &self.imports {
            let _ = writeln!(o, "import {i}");
        }
        for l in &self.body {
            o.push_str(l);
            o.push('\n');
        }
        let _ = writeln!(o, "end {}", self.body.len());
        o
    }
}
