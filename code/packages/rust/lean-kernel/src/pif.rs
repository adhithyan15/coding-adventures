//! The trusted PIF reader (VER32 section 5, backlog item PIF-002).
//!
//! # What this module is, and why it is trusted
//!
//! A Proof Interchange Format (PIF) file is *text*; the kernel only understands
//! [`Declaration`]s and [`InductiveDecl`]s. This module is the one place that turns the
//! first into the second. It sits **between an untrusted producer and the kernel**, so a
//! bug here is a soundness bug of exactly the worst kind: the kernel would faithfully
//! check *something other than what the file says*. The classic failure is a `theorem`
//! record handed to the kernel as an `axiom` (no proof is ever checked), or a table index
//! off by one (every later declaration silently means something else). That is why the
//! reader is small, has no I/O, never recurses on input-controlled depth, and **fails
//! closed**: whenever it is unsure it returns an error, and an error means *nothing from
//! this file is offered as an environment*.
//!
//! # Shape of the input
//!
//! A file is lines of ASCII. Each line is a keyword followed by fields, every field
//! introduced by exactly **one** space (the grammar `code/grammars/pif/pif.tokens` carries
//! the space inside the tokens, so there is no whitespace pre-pass and the grammar *is*
//! the accepted language). The reader accepts exactly that language, and no more:
//!
//! ```text
//!   pif 1 / logic 1 / profile <id> <string> / features <ids> / import <digest>*   header
//!   nstr nnum lsucc lmax limax lparam evar esort econst eapp elam epi elet ...    records
//!   axiom def theorem opaque inductive(itype ictor*) goal quot prim enat estr eproj
//!   end <count>                                                                   trailer
//! ```
//!
//! Records define *tables* of names, levels and expressions. An index is the position of
//! a record in its table, the tables are **dense** (each record defines the next index),
//! and a record may only mention indices defined before it (**backward only**), so cycles
//! cannot be written and no depth is recursive. Depth is a number stored at insert time.
//!
//! # Fail closed on what the kernel cannot do yet
//!
//! This kernel has slices 1, 2 and 3a. Literals (`enat`, `estr`, feature `lits`), `quot`
//! (feature `quotient`), mutual and nested blocks (features `mutual`, `nested`), `prim`
//! roles and `eproj` need later slices: they are rejected with a clear
//! [`PifError::Unsupported`] or [`PifError::FeatureNotDeclared`], never skipped. A file
//! that declares any feature is rejected, because the kernel implements none.
//!
//! # The rules
//!
//! Every check is one rule of VER32 section 5.3 and is cited (`R-...`) at its code with
//! what goes wrong if it were missing. Rules that need an implementation the kernel does
//! not have yet (`R-PRM-*`, `R-LEX-4` for `nat` literals, `R-IMP-2`, which belongs to the
//! SHA-256 resolver) are listed in the README as deferred, never silently dropped.
//!
//! # All or nothing
//!
//! [`read`] takes the [`Environment`] **by value** and returns it inside the
//! [`ReadReport`] only on success. A caller therefore cannot keep using an environment
//! that holds a prefix of a rejected file: on any error the half-built environment is
//! dropped.

use crate::env::{DeclKind, Declaration, Environment, ReducibilityHints};
use crate::error::KernelError;
use crate::expr::{Binder, BinderInfo, Expr, ExprStore};
use crate::inductive::{ConstructorDecl, InductiveDecl, InductiveTypeDecl};
use crate::level::Level;
use crate::name::{Name, NamePart};
use std::collections::{HashMap, HashSet};

use PifError as E;
type R<T> = Result<T, PifError>;

/// One check: if `c` fails, return the named error (built only on failure). Every rule
/// below is a `need!`, written as a macro so that each check is one line.
macro_rules! need {
    ($c:expr, $e:expr) => {
        if !$c {
            return Err($e);
        }
    };
}

/// Why a file was rejected: one kind per rule of VER32 section 5.3, so a corpus can assert
/// the *reason* and not just the failure. The rule each variant belongs to is in the
/// comment beside it.
#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PifError {
    BadLexicalForm, BadEscape, NonCanonicalEscape, NumberOutOfRange, // R-LEX-1, 2, 3, 4
    UnknownVersion, WithdrawnLogicVersion,                            // R-VER-1
    UnknownFeature, FeatureNotAllowed, FeatureOrder,                  // R-VER-2, R-VER-2, R-VER-3
    ImportMismatch,                                                   // R-IMP-1
    IndexNotDense, ForwardReference, BVarIndexTooLarge,               // R-IDX-1, 2, 3
    BadNameComponent, AnonymousName, NameNotAscii,                    // R-NAM-1, 2, 3
    CountMismatch, TrailerMismatch, FeatureNotDeclared,               // R-CNT-1, R-CNT-2, R-FEA-1
    AxiomNotAllowed, DerivedNameTaken,                                // R-AX-1, R-DER-1
    GoalTargetNotTheorem, GoalTargetImported, DuplicateGoal, BadClaimId, GoalConstantRedeclared, // R-GOL-1, 1, 2, 3, 4
    /// R-LIM-1: which section 7 limit was exceeded.
    LimitExceeded(&'static str),
    /// A record or feature the kernel does not implement yet; rejected, never skipped.
    Unsupported(&'static str),
    /// R-DEC-1 and every typing rule: the kernel refused the declaration.
    Kernel(KernelError),
}

impl From<KernelError> for PifError {
    fn from(e: KernelError) -> PifError {
        E::Kernel(e)
    }
}

/// The section 7 limits, as configuration with defaults. Lowering one is always safe;
/// raising one is a reviewed decision. There is no `nat` digit limit because literals are
/// unsupported, and the level limit is the kernel's own (`Level::is_within_limits`: depth
/// 128, tree size 512), which is stricter than the 256 of the table.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
pub struct PifLimits {
    pub file_bytes: usize, pub line_bytes: usize, pub records: usize, pub name_components: u32, pub name_bytes: usize,
    pub string_bytes: usize, pub expr_depth: u32, pub universe_params: usize, pub const_levels: usize,
    pub decls: usize, pub ind_types: usize, pub ind_ctors: usize, pub ind_params: u32, pub imports: usize,
}

#[rustfmt::skip]
impl Default for PifLimits {
    fn default() -> PifLimits {
        PifLimits {
            file_bytes: 64 << 20, line_bytes: 1 << 20, records: 4_000_000, name_components: 32, name_bytes: 4096,
            string_bytes: 64 << 10, expr_depth: 1000, universe_params: 64, const_levels: 64,
            decls: 1_000_000, ind_types: 64, ind_ctors: 4096, ind_params: 256, imports: 64,
        }
    }
}

/// Which rules apply to the file (VER32 5.3, "file class gate-fed"). `Library` adds none;
/// `Reviewed` (model file, preludes) adds R-NAM-3, the ASCII class; `Claim` adds R-AX-1 and
/// R-GOL-4 as well and is the default, because the safe mistake is to apply too many rules.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileClass { Library, Reviewed, #[default] Claim }

/// The registry predicate of R-AX-1 (gate-fed): given an axiom's name, universe
/// parameters, type and the store, is it exactly a registered statement? Statement digests
/// (binding CTE, SHA-256) live in the gate, outside this module.
pub type AxiomCheck<'a> = &'a dyn Fn(&Name, &[Name], Expr, &ExprStore) -> bool;

/// Everything the gate supplies that the file itself cannot be trusted to say.
#[rustfmt::skip]
#[derive(Default)]
pub struct GateInputs<'a> {
    /// R-IMP-1: lowercase hex digests (64 digits, no `sha256:`) of all files before this one, in order.
    pub expected_imports: Vec<String>,
    /// R-VER-2: the features this environment may enable.
    pub allowed_features: Vec<String>,
    /// R-VER-1: `logic` versions the pin file has withdrawn.
    pub withdrawn_logic: Vec<u32>,
    /// Which rule set applies.
    pub class: FileClass,
    /// R-AX-1: matches a `Warrant.Assume.*` or `Src.*` axiom against the registry. `None` rejects every axiom.
    pub axiom_registry: Option<AxiomCheck<'a>>,
    /// R-GOL-4: the constants occurring in the goal terms of the claims being checked.
    pub goal_constants: HashSet<Name>,
    /// R-GOL-2: claim ids already used by earlier files of the environment.
    pub prior_claim_ids: HashSet<String>,
    /// R-LIM-1: the limits.
    pub limits: PifLimits,
}

/// The kind of an accepted declaration, read back from the environment (never from the record).
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Axiom, Definition, Theorem, Opaque, Inductive, Constructor, Recursor }

/// One declaration the kernel accepted: what PIF-007's differential compares. `ty` is a
/// handle into the report's environment.
#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accepted { pub kind: Kind, pub name: Name, pub ty: Expr }

/// An accepted `goal` record. The digest is only a hint: the gate recomputes it (G4).
#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goal { pub claim_id: String, pub theorem: Name, pub digest: String }

/// A successful read: the extended environment, the `profile` id and label (never
/// trusted), every accepted declaration in file order (an inductive block lists the type,
/// its constructors, then `T.rec`) and every accepted goal.
#[rustfmt::skip]
pub struct ReadReport { pub env: Environment, pub profile: (String, String), pub decls: Vec<Accepted>, pub goals: Vec<Goal> }

const KNOWN_FEATURES: &[&str] = &["lits", "mutual", "nested", "quotient"];

// ---------------------------------------------------------------------------------------
// Lexing: one cursor over the fields of a line. Every field is " " followed by the field,
// so a doubled, leading or trailing space, a tab, or a stray byte matches nothing.
// ---------------------------------------------------------------------------------------

/// The numeral shape `0 | [1-9][0-9]*` (no sign, no leading zero, no `_`).
fn shape(t: &[u8]) -> R<()> {
    need! { !t.is_empty() && t.iter().all(u8::is_ascii_digit) && (t[0] != b'0' || t.len() == 1), E::BadLexicalForm }
    Ok(())
}

/// R-LEX-4: a numeral that fits a `u64` (an `nnum` component) or a `u32` (everything else).
fn num64(t: &[u8]) -> R<u64> {
    shape(t)?;
    let step = |a: u64, &d: &u8| a.checked_mul(10)?.checked_add(u64::from(d - b'0'));
    t.iter().try_fold(0, step).ok_or(E::NumberOutOfRange)
}

fn num32(t: &[u8]) -> R<u32> {
    u32::try_from(num64(t)?).map_err(|_| E::NumberOutOfRange)
}

/// A string token with its escapes decoded. R-LEX-2 and R-LEX-3 live in `escape`.
fn unescape(t: &[u8], lits: bool, max: usize) -> R<String> {
    let s = t
        .strip_prefix(b"\"")
        .and_then(|s| s.strip_suffix(b"\""))
        .ok_or(E::BadLexicalForm)?;
    let (mut out, mut i) = (String::new(), 0);
    while i < s.len() {
        let (c, used) = match (s[i], s.get(i + 1)) {
            (b'\\', Some(&q @ (b'"' | b'\\'))) => (char::from(q), 2),
            (b'\\', Some(b'u')) => escape(&s[i + 2..], lits)?,
            (b @ 0x21..=0x7e, _) if b != b'"' && b != b'\\' => (char::from(b), 1),
            _ => return Err(E::BadLexicalForm),
        };
        out.push(c);
        i += used;
    }
    need! { out.len() <= max, E::LimitExceeded("string length") }
    Ok(out)
}

/// The `{h}` after a `\u`: 1 to 6 lowercase hex digits, no leading zero. Returns the
/// character and the bytes the whole escape used (`\u` included).
///
/// * R-LEX-2: `h` must be a Unicode scalar value. A surrogate or a value above 10FFFF is no
///   `char`; accepting it would let a name hold something no producer can print.
/// * R-LEX-3: an escape is used only for a character with no direct spelling (not 0x21 to
///   0x7E; `\u{0}` only inside `estr`). Otherwise one name has two spellings, and a digest
///   of the bytes would not identify the name.
fn escape(s: &[u8], lits: bool) -> R<(char, usize)> {
    let hex = s.strip_prefix(b"{").ok_or(E::BadLexicalForm)?;
    let k = hex
        .iter()
        .take_while(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
        .count();
    need! { (1..=6).contains(&k) && hex.get(k) == Some(&b'}') && (hex[0] != b'0' || k == 1), E::BadLexicalForm }
    let v = hex[..k].iter().fold(0u32, |a, &b| {
        a * 16 + char::from(b).to_digit(16).unwrap_or(0)
    });
    need! { !((v == 0 && !lits) || (0x21..=0x7e).contains(&v)), E::NonCanonicalEscape }
    Ok((char::from_u32(v).ok_or(E::BadEscape)?, k + 4))
}

/// R-NAM-1 (VER32 3.8): a component must be non-empty, contain no `.`, and no control,
/// format or private-use character. Otherwise two different names could print alike.
fn bad_component(s: &str) -> bool {
    let bad = |c: char| {
        matches!(c as u32, 0..=0x1f | 0x7f..=0x9f | 0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x2064)
            || matches!(c as u32, 0x2066..=0x206f | 0xfeff | 0xe000..=0xf8ff | 0xf0000..=0xffffd | 0x100000..=0x10fffd)
    };
    s.is_empty() || s.contains('.') || s.chars().any(bad)
}

/// The ASCII class of 3.8, `[A-Za-z_][A-Za-z0-9_']*`.
fn ascii_class(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|x| x.is_ascii_alphabetic() || x == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || "_'".contains(x))
}

/// The unread fields of one line.
struct F<'a>(&'a [u8]);

impl<'a> F<'a> {
    fn tok(&mut self) -> R<&'a [u8]> {
        let r = self.0.strip_prefix(b" ").ok_or(E::BadLexicalForm)?;
        let n = r.iter().position(|&b| b == b' ').unwrap_or(r.len());
        self.0 = &r[n..];
        Ok(&r[..n])
    }
    fn u32(&mut self) -> R<u32> {
        num32(self.tok()?)
    }
    fn string(&mut self, lits: bool, max: usize) -> R<String> {
        unescape(self.tok()?, lits, max)
    }
    /// An identifier `[a-z][a-z0-9_.]*` that is not one of the four binder words.
    fn ident(&mut self) -> R<String> {
        let t = self.tok()?;
        let chars = t
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.".contains(b));
        let word = [&b"default"[..], b"implicit", b"strict", b"inst"].contains(&t);
        need! { t.first().is_some_and(u8::is_ascii_lowercase) && chars && !word, E::BadLexicalForm }
        Ok(String::from_utf8_lossy(t).into_owned())
    }
    /// A digest field: `sha256:` and 64 lowercase hex digits; returns the digits.
    fn digest(&mut self) -> R<String> {
        let t = self
            .tok()?
            .strip_prefix(b"sha256:")
            .ok_or(E::BadLexicalForm)?;
        need! { t.len() == 64 && t.iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)), E::BadLexicalForm }
        Ok(String::from_utf8_lossy(t).into_owned())
    }
    /// A bracket list, byte-exact: `[]`, `[1]`, `[1 2]` (never `[ 1]`, `[1 ]` or `[1  2]`).
    fn list(&mut self) -> R<Vec<u32>> {
        let r = self.0.strip_prefix(b" [").ok_or(E::BadLexicalForm)?;
        let close = r.iter().position(|&b| b == b']').ok_or(E::BadLexicalForm)?;
        self.0 = &r[close + 1..];
        if close == 0 {
            return Ok(vec![]);
        }
        r[..close].split(|&b| b == b' ').map(num32).collect()
    }
    fn binfo(&mut self) -> R<BinderInfo> {
        Ok(match self.tok()? {
            b"default" => BinderInfo::Default,
            b"implicit" => BinderInfo::Implicit,
            b"strict" => BinderInfo::StrictImplicit,
            b"inst" => BinderInfo::InstImplicit,
            _ => return Err(E::BadLexicalForm),
        })
    }
    fn hint(&mut self) -> R<ReducibilityHints> {
        Ok(match self.tok()? {
            b"@abbrev" => ReducibilityHints::Abbrev,
            b"@opaque" => ReducibilityHints::Opaque,
            b"@regular" => ReducibilityHints::Regular(self.u32()?),
            _ => return Err(E::BadLexicalForm),
        })
    }
    fn done(&self) -> R<()> {
        need! { self.0.is_empty(), E::BadLexicalForm }
        Ok(())
    }
}

/// R-LEX-1 for the whole file, before anything else: only LF and 0x20 to 0x7E occur, and the
/// size and line-length limits hold before any line is buffered (R-LIM-1).
fn prescan(bytes: &[u8], l: &PifLimits) -> R<()> {
    need! { bytes.len() <= l.file_bytes, E::LimitExceeded("file size") }
    let mut run = 0usize;
    for &b in bytes {
        need! { b == b'\n' || (0x20..=0x7e).contains(&b), E::BadLexicalForm }
        run = if b == b'\n' { 0 } else { run + 1 };
        need! { run <= l.line_bytes, E::LimitExceeded("line length") }
    }
    Ok(())
}

fn at<T: Clone>(v: &[T], base: usize, id: u32) -> R<T> {
    let i = (id as usize).checked_sub(base);
    i.and_then(|i| v.get(i)).cloned().ok_or(E::ForwardReference)
}

#[rustfmt::skip]
fn kind_of(k: &DeclKind) -> Kind {
    match k {
        DeclKind::Axiom => Kind::Axiom, DeclKind::Definition { .. } => Kind::Definition,
        DeclKind::Theorem { .. } => Kind::Theorem, DeclKind::Opaque { .. } => Kind::Opaque,
        DeclKind::Inductive(_) => Kind::Inductive, DeclKind::Constructor(_) => Kind::Constructor,
        DeclKind::Recursor(_) => Kind::Recursor,
    }
}

// ---------------------------------------------------------------------------------------
// The reader proper.
// ---------------------------------------------------------------------------------------

/// The state of one read. `names` maps an index to (name, component count, in the ASCII
/// class) with entry 0 the anonymous name; `levels` has entry 0 = `zero`; `exprs` holds
/// (node, depth) for index `i + 1`.
#[rustfmt::skip]
struct Rd<'a> {
    g: &'a GateInputs<'a>, lim: PifLimits, rest: &'a [u8], env: Environment,
    names: Vec<(Name, u32, bool, usize)>, levels: Vec<Level>, exprs: Vec<(Expr, u32)>,
    lines: usize, ndecls: usize, profile: (String, String),
    decls: Vec<Accepted>, goals: Vec<Goal>, local: HashMap<Name, Kind>, claim_ids: HashSet<String>,
}

/// Read one PIF file into `env`.
///
/// `bytes` is the exact file; `gate` carries what only the gate knows. On success the report
/// holds the extended environment and every accepted declaration; on any error no
/// environment is returned (all or nothing, see the module documentation).
#[rustfmt::skip]
pub fn read(env: Environment, bytes: &[u8], gate: &GateInputs) -> Result<ReadReport, PifError> {
    let mut r = Rd {
        g: gate, lim: gate.limits, rest: bytes, env, names: vec![(Name::anonymous(), 0, true, 0)],
        levels: vec![Level::zero()], exprs: vec![], lines: 0, ndecls: 0,
        profile: Default::default(), decls: vec![], goals: vec![], local: HashMap::new(),
        claim_ids: HashSet::new(),
    };
    prescan(bytes, &gate.limits)?;
    r.header()?;
    r.body()?;
    Ok(ReadReport { env: r.env, profile: r.profile, decls: r.decls, goals: r.goals })
}

impl<'a> Rd<'a> {
    /// The next line as (keyword, fields). `None` at end of input; a last line with no LF is
    /// a lexical error (the grammar requires NEWLINE after every line).
    fn raw(&mut self) -> R<Option<(&'a [u8], F<'a>)>> {
        if self.rest.is_empty() {
            return Ok(None);
        }
        let i = self
            .rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or(E::BadLexicalForm)?;
        let l = &self.rest[..i];
        self.rest = &self.rest[i + 1..];
        let k = l.iter().position(|&b| b == b' ').unwrap_or(l.len());
        Ok(Some((&l[..k], F(&l[k..]))))
    }

    /// A body line; counts toward `end k` and the record limit (R-LIM-1).
    fn line(&mut self) -> R<Option<(&'a [u8], F<'a>)>> {
        self.lines += 1;
        need! { self.lines <= self.lim.records.saturating_add(1), E::LimitExceeded("records per file") }
        self.raw()
    }

    /// Is the next line's keyword `kw`?
    fn peek(&self, kw: &[u8]) -> bool {
        let end = self
            .rest
            .iter()
            .position(|&b| b == b' ' || b == b'\n')
            .unwrap_or(self.rest.len());
        self.rest[..end] == *kw
    }

    /// Consume the next body line only if its keyword is `kw`.
    fn take(&mut self, kw: &[u8]) -> R<Option<F<'a>>> {
        Ok(if self.peek(kw) {
            self.line()?.map(|(_, f)| f)
        } else {
            None
        })
    }

    /// A header line whose keyword must be `kw`.
    fn hdr(&mut self, kw: &[u8]) -> R<F<'a>> {
        match self.raw()? {
            Some((k, f)) if k == kw => Ok(f),
            _ => Err(E::BadLexicalForm),
        }
    }

    fn hnum(&mut self, kw: &[u8]) -> R<u32> {
        let mut f = self.hdr(kw)?;
        let v = f.u32()?;
        f.done()?;
        Ok(v)
    }

    /// R-VER-*, R-IMP-1 and the fixed header lines.
    fn header(&mut self) -> R<()> {
        // R-VER-1: a version the reader does not list is a hard error, never guessed at.
        need! { self.hnum(b"pif")? == 1, E::UnknownVersion }
        let v = self.hnum(b"logic")?;
        need! { !self.g.withdrawn_logic.contains(&v), E::WithdrawnLogicVersion }
        need! { v == 1, E::UnknownVersion }
        let mut f = self.hdr(b"profile")?;
        self.profile = (f.ident()?, f.string(false, self.lim.string_bytes)?);
        f.done()?;
        let mut f = self.hdr(b"features")?;
        let mut features: Vec<String> = vec![];
        while !f.0.is_empty() {
            let w = f.ident()?;
            // R-VER-3: distinct and ascending, so one set has one spelling.
            need! { features.last().is_none_or(|p| *p < w), E::FeatureOrder }
            // R-VER-2: the logic defines it, and the gate allows it for this environment.
            need! { KNOWN_FEATURES.contains(&w.as_str()), E::UnknownFeature }
            need! { self.g.allowed_features.contains(&w), E::FeatureNotAllowed }
            features.push(w);
        }
        // The kernel implements no feature in slices 1 to 3a: declaring one fails closed.
        need! { features.is_empty(), E::Unsupported("kernel features (lits, quotient, mutual, nested)") }
        let mut imports = vec![];
        while self.peek(b"import") {
            let mut f = self.hdr(b"import")?;
            imports.push(f.digest()?);
            f.done()?;
            need! { imports.len() <= self.lim.imports, E::LimitExceeded("imports") }
        }
        // R-IMP-1: the chain is fixed by the gate. A file naming other imports (or none, when
        // some are expected) would be read against a different environment than it was checked in.
        need! { imports == self.g.expected_imports, E::ImportMismatch }
        Ok(())
    }

    /// Records up to the trailer.
    fn body(&mut self) -> R<()> {
        loop {
            // R-CNT-2: no `end` line at all is truncation.
            let (kw, mut f) = self.line()?.ok_or(E::TrailerMismatch)?;
            if kw != b"end" {
                self.record(kw, f)?;
                continue;
            }
            // R-CNT-2: `end k` equals the number of record lines (it catches accidental
            // truncation; it is compared, never used as a size), and nothing follows (R-LEX-1).
            let k = f.u32()?;
            f.done()?;
            need! { self.rest.is_empty(), E::BadLexicalForm }
            need! { k as usize == self.lines - 1, E::TrailerMismatch }
            return Ok(());
        }
    }

    // ---- tables ------------------------------------------------------------------------

    /// R-IDX-1: a record defines exactly the next index of its table, so every index means one
    /// thing and a dropped or duplicated record is caught at once. (Name 0 and level 0 are
    /// predefined; expressions start at 1.)
    fn fresh(&self, f: &mut F, space: usize) -> R<()> {
        let next = [self.names.len(), self.levels.len(), self.exprs.len() + 1][space];
        let id = f.u32()? as usize;
        need! { id == next, E::IndexNotDense }
        Ok(())
    }

    // R-IDX-2: an operand must already be defined. Without it a record could mention itself
    // or a later record, and the table would not be a DAG.
    fn name(&self, id: u32) -> R<(Name, u32, bool, usize)> {
        at(&self.names, 0, id)
    }
    fn lvl(&self, id: u32) -> R<Level> {
        at(&self.levels, 0, id)
    }
    fn ex(&self, id: u32) -> R<(Expr, u32)> {
        at(&self.exprs, 1, id)
    }

    /// A name used as a binder, universe parameter or declared name. R-NAM-2: a declared
    /// name or parameter is never the anonymous name. R-NAM-3: in a reviewed file every such
    /// name is in the ASCII class, so a homoglyph cannot hide in what a person signs.
    fn nm(&self, id: u32, declared: bool) -> R<Name> {
        let (n, depth, ascii, _) = self.name(id)?;
        need! { !declared || depth > 0, E::AnonymousName }
        need! { self.g.class == FileClass::Library || (ascii && depth > 0), E::NameNotAscii }
        Ok(n)
    }

    /// A declared constant name. R-GOL-4: a claim file may not declare a constant the goals
    /// mention, or the proof could redefine what the goal says.
    fn dn(&self, id: u32) -> R<Name> {
        let n = self.nm(id, true)?;
        need! { self.g.class != FileClass::Claim || !self.g.goal_constants.contains(&n), E::GoalConstantRedeclared }
        Ok(n)
    }

    /// A universe parameter list. R-CNT-1: the names are distinct (a repeated one makes
    /// instantiation ambiguous). R-LIM-1: at most 64.
    fn params(&self, ids: Vec<u32>) -> R<Vec<Name>> {
        need! { ids.len() <= self.lim.universe_params, E::LimitExceeded("universe parameters") }
        let ps: Vec<Name> = ids.iter().map(|&i| self.nm(i, true)).collect::<R<_>>()?;
        need! { ps.iter().collect::<HashSet<_>>().len() == ps.len(), E::CountMismatch }
        Ok(ps)
    }

    /// R-LIM-1: an expression's depth is stored at insert (leaf 1, node 1 + max of its
    /// children), so no later pass recurses on it. A DAG of 1,000 records can denote 2^1000
    /// nodes, so the depth bounds nesting, not size; the kernel's `Limits` bound the work.
    fn put_e(&mut self, e: Expr, depth: u32) -> R<()> {
        need! { depth <= self.lim.expr_depth, E::LimitExceeded("expression depth") }
        self.exprs.push((e, depth));
        Ok(())
    }

    // ---- records -----------------------------------------------------------------------

    fn record(&mut self, kw: &[u8], mut f: F<'a>) -> R<()> {
        match kw {
            b"nstr" | b"nnum" => {
                self.fresh(&mut f, 0)?;
                let (p, depth, ascii, bytes) = self.name(f.u32()?)?;
                let (n, ok, add) = if kw == b"nstr" {
                    let s = f.string(false, self.lim.string_bytes)?;
                    need! { !bad_component(&s), E::BadNameComponent }
                    (Name::str(&p, &s), ascii_class(&s), s.len())
                } else {
                    (Name::num(&p, num64(f.tok()?)?), false, 8)
                };
                f.done()?;
                need! { depth < self.lim.name_components, E::LimitExceeded("name components") }
                // R-LIM-1: total decoded bytes of the whole name. Hashing and comparing a name
                // costs its length, so one 2 MiB legal name referenced millions of times is a
                // denial of service even though every component is within its own limit.
                need! { bytes + add <= self.lim.name_bytes, E::LimitExceeded("name bytes") }
                self.names.push((n, depth + 1, ascii && ok, bytes + add));
            }
            b"lsucc" | b"lmax" | b"limax" | b"lparam" => {
                self.fresh(&mut f, 1)?;
                let l = match kw {
                    b"lsucc" => Level::succ(&self.lvl(f.u32()?)?),
                    b"lparam" => Level::param(&self.nm(f.u32()?, true)?),
                    _ => {
                        let (a, b) = (self.lvl(f.u32()?)?, self.lvl(f.u32()?)?);
                        let mk = if kw == b"lmax" {
                            Level::max
                        } else {
                            Level::imax
                        };
                        mk(&a, &b)
                    }
                };
                f.done()?;
                // R-LIM-1: the kernel's own bound, cached at construction, so no level a later
                // pass recurses over can be deeper or wider than the kernel will normalise.
                need! { l.is_within_limits(), E::LimitExceeded("level depth or size") }
                self.levels.push(l);
            }
            b"evar" | b"esort" | b"econst" | b"eapp" | b"elam" | b"epi" | b"elet" => {
                self.fresh(&mut f, 2)?;
                let (e, d) = self.expr(kw, &mut f)?;
                f.done()?;
                self.put_e(e, d)?;
            }
            b"enat" | b"estr" | b"eproj" => {
                // Literals and projections need the primitive validation of later slices.
                self.fresh(&mut f, 2)?;
                match kw {
                    b"enat" => shape(f.tok()?)?,
                    b"estr" => drop(f.string(true, self.lim.string_bytes)?),
                    _ => (0..3).try_for_each(|_| f.u32().map(drop))?,
                }
                f.done()?;
                // R-FEA-1: `enat` and `estr` need `lits`, which no file can declare yet.
                return Err(if kw == b"eproj" {
                    E::Unsupported("projections (slice 3b)")
                } else {
                    E::FeatureNotDeclared
                });
            }
            b"axiom" | b"def" | b"theorem" | b"opaque" => {
                let name = self.dn(f.u32()?)?;
                let ps = self.params(f.list()?)?;
                let ty = self.ex(f.u32()?)?.0;
                let d = if kw == b"axiom" {
                    // R-AX-1: a claim file may declare only registered axioms. An axiom is an
                    // unchecked assumption; without this a proof could simply assume its goal.
                    need! { self.g.class != FileClass::Claim || self.axiom_ok(&name, &ps, ty), E::AxiomNotAllowed }
                    Declaration::axiom(name, ps, ty)
                } else {
                    let v = self.ex(f.u32()?)?.0;
                    // Kind fidelity: each record becomes the declaration kind of the SAME name. A
                    // `theorem` added as an axiom would be accepted with no proof ever checked.
                    match kw {
                        b"theorem" => Declaration::theorem(name, ps, ty, v),
                        b"opaque" => Declaration::opaque(name, ps, ty, v),
                        _ => Declaration::definition(name, ps, ty, v, f.hint()?),
                    }
                };
                f.done()?;
                self.count_decl()?;
                let n = d.name.clone();
                self.env.add_declaration(d)?;
                self.accept(&n)?;
            }
            b"inductive" => self.inductive(f)?,
            b"quot" => {
                f.done()?;
                return Err(E::FeatureNotDeclared); // R-FEA-1: `quot` needs `quotient`
            }
            b"prim" => {
                f.ident()?;
                f.u32()?;
                f.done()?;
                return Err(E::Unsupported("prim roles (slice 4)"));
            }
            b"goal" => self.goal(f)?,
            _ => return Err(E::BadLexicalForm),
        }
        Ok(())
    }

    /// Build one expression node; returns it with its depth (a leaf is 1).
    fn expr(&mut self, kw: &[u8], f: &mut F) -> R<(Expr, u32)> {
        Ok(match kw {
            // R-IDX-3: the checked constructor; the unchecked `bvar` panics above the bound.
            b"evar" => {
                let v = self.env.store.try_bvar(f.u32()?);
                (v.map_err(|_| E::BVarIndexTooLarge)?, 1)
            }
            b"esort" => (self.env.store.sort(&self.lvl(f.u32()?)?), 1),
            b"econst" => {
                let n = self.name(f.u32()?)?.0;
                let ids = f.list()?;
                need! { ids.len() <= self.lim.const_levels, E::LimitExceeded("levels per constant") }
                let ls = ids.iter().map(|&i| self.lvl(i)).collect::<R<Vec<_>>>()?;
                (self.env.store.constant(&n, &ls), 1)
            }
            b"eapp" => {
                let ((a, da), (b, db)) = (self.ex(f.u32()?)?, self.ex(f.u32()?)?);
                (self.env.store.app(a, b), 1 + da.max(db))
            }
            b"elet" => {
                let name = self.nm(f.u32()?, false)?;
                let ((t, dt), (v, dv), (b, db)) =
                    (self.ex(f.u32()?)?, self.ex(f.u32()?)?, self.ex(f.u32()?)?);
                let info = BinderInfo::Default;
                (
                    self.env.store.let_e(Binder { name, info }, t, v, b),
                    1 + dt.max(dv).max(db),
                )
            }
            _ => {
                let info = f.binfo()?;
                let name = self.nm(f.u32()?, false)?;
                let ((t, dt), (b, db)) = (self.ex(f.u32()?)?, self.ex(f.u32()?)?);
                let mk = if kw == b"elam" {
                    ExprStore::lam
                } else {
                    ExprStore::pi
                };
                (
                    mk(&mut self.env.store, Binder { name, info }, t, b),
                    1 + dt.max(db),
                )
            }
        })
    }

    fn count_decl(&mut self) -> R<()> {
        self.ndecls += 1;
        need! { self.ndecls <= self.lim.decls, E::LimitExceeded("declarations per file") }
        Ok(())
    }

    /// Record a declaration the kernel accepted, reading its kind back from the environment.
    fn accept(&mut self, n: &Name) -> R<()> {
        let d = self
            .env
            .get(n)
            .ok_or(KernelError::Internal("declaration missing"))?;
        let kind = kind_of(&d.kind);
        #[rustfmt::skip]
        self.decls.push(Accepted { kind, name: n.clone(), ty: d.ty });
        self.local.insert(n.clone(), kind);
        Ok(())
    }

    /// R-AX-1: the name is `Warrant.Assume.<id>` or `Src.<profile>.<name>`, and the gate's
    /// registry says the statement matches exactly.
    fn axiom_ok(&self, name: &Name, ps: &[Name], ty: Expr) -> bool {
        let p = name.parts();
        let s = |i: usize, t: &str| matches!(p.get(i), Some(NamePart::Str(x)) if x == t);
        let shaped = (s(0, "Warrant") && s(1, "Assume") || s(0, "Src")) && p.len() > 2;
        shaped
            && self
                .g
                .axiom_registry
                .is_some_and(|ok| ok(name, ps, ty, &self.env.store))
    }

    /// An inductive block: `inductive [ps] np nt`, then `nt` `itype` lines each followed by
    /// its `ictor` lines. R-CNT-1: the counts are validated, never trusted, and every loop is
    /// bounded by the declared count before anything is allocated.
    fn inductive(&mut self, mut f: F<'a>) -> R<()> {
        let level_params = self.params(f.list()?)?;
        let (np, nt) = (f.u32()?, f.u32()?);
        f.done()?;
        need! { np <= self.lim.ind_params && nt as usize <= self.lim.ind_types, E::LimitExceeded("inductive block size") }
        // R-FEA-1: more than one type is a mutual block.
        need! { nt <= 1, E::FeatureNotDeclared }
        let mut types: Vec<InductiveTypeDecl> = vec![];
        while let Some(mut f) = self.take(b"itype")? {
            need! { types.len() < nt as usize, E::CountMismatch }
            let (name, ty, nc) = (self.dn(f.u32()?)?, self.ex(f.u32()?)?.0, f.u32()?);
            f.done()?;
            need! { nc as usize <= self.lim.ind_ctors, E::LimitExceeded("constructors per type") }
            let mut ctors: Vec<ConstructorDecl> = vec![];
            while let Some(mut f) = self.take(b"ictor")? {
                need! { ctors.len() < nc as usize, E::CountMismatch }
                let (name, ty) = (self.dn(f.u32()?)?, self.ex(f.u32()?)?.0);
                f.done()?;
                ctors.push(ConstructorDecl { name, ty });
            }
            need! { ctors.len() == nc as usize, E::CountMismatch }
            types.push(InductiveTypeDecl { name, ty, ctors });
        }
        need! { nt > 0 && types.len() == nt as usize, E::CountMismatch }
        // R-DER-1: the recursor's name is derived by the kernel. If it is taken already (an
        // axiom `T.rec`, or a constructor called `T.rec`) a hostile `T.rec` would stand in
        // for the real one, so the block is refused rather than letting either name win.
        let mut names = vec![types[0].name.clone()];
        names.extend(types[0].ctors.iter().map(|c| c.name.clone()));
        let rec = Name::str(&names[0], "rec");
        need! { !self.env.contains(&rec) && !names.contains(&rec), E::DerivedNameTaken }
        need! { self.g.class != FileClass::Claim || !self.g.goal_constants.contains(&rec), E::GoalConstantRedeclared }
        names.push(rec);
        self.count_decl()?;
        #[rustfmt::skip]
        self.env.add_inductive(InductiveDecl { level_params, num_params: np as usize, types })?;
        names.iter().try_for_each(|n| self.accept(n))
    }

    /// `goal "<id>" <theorem> sha256:<hex>`.
    fn goal(&mut self, mut f: F<'a>) -> R<()> {
        let id = f.string(false, self.lim.string_bytes)?;
        let target = self.name(f.u32()?)?.0;
        let digest = f.digest()?;
        f.done()?;
        // R-GOL-3: the id is signed in a line-oriented list, so it may carry no space or newline.
        let charset = |b: u8| b.is_ascii_alphanumeric() || b"_.-".contains(&b);
        need! { !id.is_empty() && id.len() <= 128 && id.bytes().all(charset), E::BadClaimId }
        // R-GOL-2: one claim, one goal; otherwise a signature could cover the wrong theorem.
        need! { !self.g.prior_claim_ids.contains(&id) && self.claim_ids.insert(id.clone()), E::DuplicateGoal }
        // R-GOL-1: the target is a theorem declared in THIS file. An axiom `T : G` "exists"
        // with an empty footprint, and an imported theorem was not checked as part of this claim.
        match self.local.get(&target) {
            Some(Kind::Theorem) => {}
            None if self.env.contains(&target) => return Err(E::GoalTargetImported),
            _ => return Err(E::GoalTargetNotTheorem),
        }
        #[rustfmt::skip]
        self.goals.push(Goal { claim_id: id, theorem: target, digest });
        Ok(())
    }
}
