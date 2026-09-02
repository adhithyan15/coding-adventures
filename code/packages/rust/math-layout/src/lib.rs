//! # TEX-1 — a TeX math list, with atom classes
//!
//! Turns a [`MathExpr`] into the structure TeX actually typesets: a **math
//! list** of atoms, each carrying a *class*.
//!
//! ## Why the class is the whole substance
//!
//! TeX's inter-atom spacing is not a set of aesthetic rules. It is a table
//! indexed by the classes of two adjacent atoms:
//!
//! ```text
//!            Ord    Op    Bin   Rel   Open  Close Punct Inner
//!   Ord      .      thin  med   thick .     .     .     thin
//!   Op       thin   thin  --    thick .     .     .     thin
//!   Bin      med    med   --    --    med   --    --    med
//!   Rel      thick  thick --    .     thick .     .     thick
//!   Open     .      .     --    .     .     .     .     .
//!   Close    .      thin  med   thick .     .     .     thin
//!   Punct    thin   thin  thin  thin  thin  thin  thin  thin
//!   Inner    thin   thin  med   thick thin  .     thin  thin
//! ```
//!
//! In the script styles some of these are dropped — and which ones is a
//! per-cell property, not a rule about the size of the space: an `Op` followed
//! by an `Ord` keeps its thin space, while an `Inner` followed by an `Ord`
//! loses the identical one.
//!
//! Classify correctly and `a+b`, `f(x)` and `a=b` space correctly with no
//! tuning. Classify wrongly and no tuning can fix them, because the difference
//! is structural. This is most of what makes output look like TeX rather than
//! like a browser's guess.
//!
//! ## The demotion rules, which are where the subtlety lives
//!
//! A `Bin` is only binary if it has something on both sides to bind. TeX
//! rewrites the list before spacing it:
//!
//! 1. A `Bin` becomes `Ord` if it is **first**, or follows a
//!    `Bin`/`Op`/`Rel`/`Open`/`Punct`.
//! 2. A `Bin` becomes `Ord` if it is **last**, or is followed by a
//!    `Rel`/`Close`/`Punct`.
//!
//! So the minus in `-x` is not the minus in `a-x`: the first is an `Ord`
//! (a sign) and the second a `Bin` (a subtraction), and they space differently
//! because they *are* different.
//!
//! Rule 2 is the one that is easy to miss, and it is why the TeXbook's table
//! marks `Bin`-followed-by-`Rel` as impossible: by the time spacing is chosen,
//! that `Bin` is an `Ord`.
//!
//! ## Where the table came from
//!
//! Not from this crate's author. `code/scripts/extract_tex_spacing_table.py`
//! asks a real `tex` to typeset all 256 class pairs in all four styles and
//! reads back the glue it inserted — and TeX names the parameter it used
//! (`\glue(\medmuskip)`), so there is no width arithmetic and no threshold
//! deciding what counts as "thin". `tests/spacing_oracle.rs` checks this
//! implementation against all 256.
//!
//! ## Scope
//!
//! Pure data transformation: no font metrics, no geometry, no I/O. Turning
//! this list into positioned boxes needs font metrics and is TEX-3.

use math_frontend::MathExpr;

// ─────────────────────────────────────────────────────────────────────────────
// Atom classes
// ─────────────────────────────────────────────────────────────────────────────

/// The eight classes TeX sorts every atom into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AtomClass {
    /// An ordinary symbol: a variable, a digit, a letter.
    Ord,
    /// A large operator: `\sum`, `\int`, `\lim`.
    Op,
    /// A binary operation: `+`, `\times`. Demoted to [`AtomClass::Ord`] when it
    /// has nothing to bind on one side.
    Bin,
    /// A relation: `=`, `\leq`, `\to`.
    Rel,
    /// An opening delimiter: `(`, `[`.
    Open,
    /// A closing delimiter: `)`, `]`.
    Close,
    /// Punctuation: `,`, `;`.
    Punct,
    /// A sub-formula treated as a unit — a `\left…\right` group. Spaces
    /// differently from the `Ord` it would otherwise be.
    Inner,
}

impl AtomClass {
    /// The lowercase name used in the oracle fixture.
    pub fn name(self) -> &'static str {
        match self {
            AtomClass::Ord => "ord",
            AtomClass::Op => "op",
            AtomClass::Bin => "bin",
            AtomClass::Rel => "rel",
            AtomClass::Open => "open",
            AtomClass::Close => "close",
            AtomClass::Punct => "punct",
            AtomClass::Inner => "inner",
        }
    }

    /// All eight, in the fixture's order.
    pub const ALL: [AtomClass; 8] = [
        AtomClass::Ord,
        AtomClass::Op,
        AtomClass::Bin,
        AtomClass::Rel,
        AtomClass::Open,
        AtomClass::Close,
        AtomClass::Punct,
        AtomClass::Inner,
    ];

    /// Does a `Bin` *preceded* by this class stay binary?
    ///
    /// Nothing can bind to the left of these, so a `Bin` after one of them is
    /// a sign rather than an operation.
    fn allows_following_bin(self) -> bool {
        !matches!(
            self,
            AtomClass::Bin | AtomClass::Op | AtomClass::Rel | AtomClass::Open | AtomClass::Punct
        )
    }

    /// Does a `Bin` *followed* by this class stay binary?
    fn allows_preceding_bin(self) -> bool {
        !matches!(self, AtomClass::Rel | AtomClass::Close | AtomClass::Punct)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Styles and spacing
// ─────────────────────────────────────────────────────────────────────────────

/// TeX's four math styles.
///
/// The distinction matters here because **medium and thick spaces are dropped
/// in the script styles** while thin spaces survive. A subscript that spaces
/// like display text is visibly wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Display,
    Text,
    Script,
    ScriptScript,
}

impl Style {
    pub fn name(self) -> &'static str {
        match self {
            Style::Display => "display",
            Style::Text => "text",
            Style::Script => "script",
            Style::ScriptScript => "scriptscript",
        }
    }

    pub const ALL: [Style; 4] = [
        Style::Display,
        Style::Text,
        Style::Script,
        Style::ScriptScript,
    ];

    /// Script styles suppress the medium and thick spaces.
    fn is_script(self) -> bool {
        matches!(self, Style::Script | Style::ScriptScript)
    }
}

/// The space TeX inserts between two atoms.
///
/// Named rather than measured: TeX picks one of three glue parameters, and
/// their point values follow from the font size. Keeping the *name* means this
/// layer needs no font metrics, which is what makes TEX-1 buildable before
/// TEX-2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    /// No glue at all.
    None,
    /// `\thinmuskip`, 3mu. Survives in script styles.
    Thin,
    /// `\medmuskip`, 4mu. Suppressed in script styles.
    Med,
    /// `\thickmuskip`, 5mu. Suppressed in script styles.
    Thick,
}

impl Space {
    pub fn name(self) -> &'static str {
        match self {
            Space::None => "none",
            Space::Thin => "thin",
            Space::Med => "med",
            Space::Thick => "thick",
        }
    }

    /// The width in **mu** (math units, 1/18 em).
    pub fn mu(self) -> u8 {
        match self {
            Space::None => 0,
            Space::Thin => 3,
            Space::Med => 4,
            Space::Thick => 5,
        }
    }
}

/// The spacing table, indexed `[left][right]` in [`AtomClass::ALL`] order.
///
/// Each cell holds **two** values: the space in display/text style, and the
/// space in the script styles. The TeXbook prints the second by parenthesising
/// entries that apply "in display and text styles only", and the suppression
/// is genuinely **per cell** rather than per kind of space:
///
/// ```text
///   Op  followed by Ord   -> thin, and it SURVIVES in script styles
///   Inner followed by Ord -> thin, and it does NOT
/// ```
///
/// An earlier version of this file assumed "thin always survives, medium and
/// thick never do". That is wrong in 30 of the 256 combinations, and the
/// oracle test caught every one — which is the entire reason the table is
/// checked against a real TeX rather than transcribed and trusted.
///
/// Cells for pairs that cannot occur are never consulted: [`MathList::spacings`]
/// applies TeX's demotion rules first, so a `Bin` that cannot be binary has
/// already become an `Ord` by the time the lookup happens.
#[rustfmt::skip]
const BASE: [[(Space, Space); 8]; 8] = {
    use Space::{Med as M, None as N, Thick as K, Thin as T};
    //              Ord      Op       Bin      Rel      Open     Close    Punct    Inner
    [
        /* ord   */ [  (N,N),   (T,T),   (M,N),   (K,N),   (N,N),   (N,N),   (N,N),   (T,N)],
        /* op    */ [  (T,T),   (T,T),   (T,T),   (K,N),   (N,N),   (N,N),   (N,N),   (T,N)],
        /* bin   */ [  (M,N),   (M,N),   (M,N),   (K,N),   (M,N),   (N,N),   (N,N),   (M,N)],
        /* rel   */ [  (K,N),   (K,N),   (K,N),   (N,N),   (K,N),   (N,N),   (N,N),   (K,N)],
        /* open  */ [  (N,N),   (N,N),   (N,N),   (N,N),   (N,N),   (N,N),   (N,N),   (N,N)],
        /* close */ [  (N,N),   (T,T),   (M,N),   (K,N),   (N,N),   (N,N),   (N,N),   (T,N)],
        /* punct */ [  (T,N),   (T,N),   (T,N),   (T,N),   (T,N),   (T,N),   (T,N),   (T,N)],
        /* inner */ [  (T,N),   (T,T),   (M,N),   (K,N),   (T,N),   (N,N),   (T,N),   (T,N)],
    ]
};

/// The space between two **already-demoted** adjacent atoms.
///
/// Callers with a whole list should use [`MathList::spacings`], which applies
/// the demotion rules first. This is the raw lookup.
pub fn spacing(left: AtomClass, right: AtomClass, style: Style) -> Space {
    let (text, script) = BASE[left as usize][right as usize];
    if style.is_script() {
        script
    } else {
        text
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The math list
// ─────────────────────────────────────────────────────────────────────────────

/// What an atom is built from.
///
/// The nucleus is deliberately not "a string": a fraction's nucleus is two
/// whole math lists, and flattening that away is what forces a renderer to
/// re-parse its own output later.
#[derive(Clone, Debug, PartialEq)]
pub enum Nucleus {
    /// A leaf symbol, as written.
    Symbol(String),
    /// Prose set in a math list (`\text{…}`).
    Text(String),
    /// A nested list — a group, or a delimited sub-formula.
    List(MathList),
    /// A fraction: numerator over denominator.
    Fraction {
        numerator: MathList,
        denominator: MathList,
    },
    /// A binomial: like a fraction, without the rule.
    Binomial { top: MathList, bottom: MathList },
    /// A radical, with an optional degree.
    Radical {
        degree: Option<MathList>,
        radicand: MathList,
    },
    /// A delimited group, carrying which delimiters bracketed it.
    Delimited {
        open: String,
        body: MathList,
        close: String,
    },
    /// Rows of cells.
    Matrix(Vec<Vec<MathList>>),
    /// Something set centred above or below a base.
    Stacked {
        base: MathList,
        over: Option<MathList>,
        under: Option<MathList>,
    },
}

/// One atom: a nucleus, its class, and its scripts.
#[derive(Clone, Debug, PartialEq)]
pub struct Atom {
    pub class: AtomClass,
    pub nucleus: Nucleus,
    pub superscript: Option<MathList>,
    pub subscript: Option<MathList>,
    /// For an `Op`: whether its scripts are set above and below rather than to
    /// the side. TeX decides this by style — `\sum_{i=1}^{n}` puts the limits
    /// above and below in display style and beside in text style.
    pub limits: bool,
}

impl Atom {
    /// An atom with the given class and nucleus, no scripts.
    pub fn new(class: AtomClass, nucleus: Nucleus) -> Self {
        Self {
            class,
            nucleus,
            superscript: None,
            subscript: None,
            limits: false,
        }
    }

    /// An atom whose nucleus is a single symbol.
    ///
    /// Public because building a list by hand is how the spacing oracle
    /// reproduces TeX's own arrangement; a test that could only go through
    /// `lower` would be testing the lowering and the table at once, and could
    /// not isolate a disagreement to either.
    pub fn symbol(class: AtomClass, text: impl Into<String>) -> Self {
        Self::new(class, Nucleus::Symbol(text.into()))
    }
}

/// A sequence of atoms — TeX's math list.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MathList {
    pub atoms: Vec<Atom>,
}

impl MathList {
    pub fn new(atoms: Vec<Atom>) -> Self {
        Self { atoms }
    }

    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty()
    }

    pub fn len(&self) -> usize {
        self.atoms.len()
    }

    /// The classes of this list's atoms **after demotion**.
    ///
    /// This is what TeX spaces, and it is not what the atoms were built as: a
    /// `Bin` with nothing to bind on either side is an `Ord` by the time
    /// spacing happens.
    pub fn resolved_classes(&self) -> Vec<AtomClass> {
        let mut classes: Vec<AtomClass> = self.atoms.iter().map(|atom| atom.class).collect();
        for index in 0..classes.len() {
            if classes[index] != AtomClass::Bin {
                continue;
            }
            // Rule 1 — nothing to bind on the left. Note this reads the
            // ALREADY-DEMOTED predecessor, so a run of Bins collapses left to
            // right exactly as TeX's single pass does.
            let binds_left = match index.checked_sub(1) {
                None => false,
                Some(previous) => classes[previous].allows_following_bin(),
            };
            // Rule 2 — nothing to bind on the right. The successor has not
            // been demoted yet, but demotion never turns a class INTO one of
            // Rel/Close/Punct, so reading it raw is safe.
            let binds_right = match classes.get(index + 1) {
                None => false,
                Some(next) => next.allows_preceding_bin(),
            };
            if !(binds_left && binds_right) {
                classes[index] = AtomClass::Ord;
            }
        }
        classes
    }

    /// The space before each atom; the first is always [`Space::None`].
    ///
    /// Length equals [`MathList::len`], so `spacings()[i]` is the space
    /// *preceding* `atoms[i]` and the two can be zipped directly.
    pub fn spacings(&self, style: Style) -> Vec<Space> {
        let classes = self.resolved_classes();
        let mut spaces = Vec::with_capacity(classes.len());
        for (index, &class) in classes.iter().enumerate() {
            spaces.push(match index.checked_sub(1) {
                None => Space::None,
                Some(previous) => spacing(classes[previous], class, style),
            });
        }
        spaces
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Lowering
// ─────────────────────────────────────────────────────────────────────────────

/// How deeply expressions may nest before lowering refuses.
///
/// `MathExpr` is produced by parsing user-supplied LaTeX or AsciiMath, so its
/// depth is attacker-controlled: a few hundred kilobytes of `{{{{{…}}}}}` is a
/// hundred thousand levels. Lowering walks that tree recursively, so without a
/// cap the parser's output crashes the process — verified, not assumed, in
/// `deeply_nested_input_is_refused_rather_than_crashing`.
///
/// Real formulas nest to single digits. 256 is far past anything legible and
/// still returns instantly.
pub const MAX_NESTING_DEPTH: usize = 256;

/// Why a `MathExpr` could not be lowered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// Nesting exceeded [`MAX_NESTING_DEPTH`].
    ///
    /// An error rather than a truncation: quietly dropping the over-deep part
    /// would render a formula that is subtly not the one that was written,
    /// which is worse than refusing it.
    DepthExceeded,
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::DepthExceeded => {
                write!(f, "expression nests deeper than {MAX_NESTING_DEPTH} levels")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Lower a [`MathExpr`] into a math list.
///
/// The shape of the expression decides the classes: a `Rel` node's operator
/// becomes a `Rel` atom, a `Bin` node's a `Bin` atom, a `Call`'s function name
/// an `Op`. Demotion is *not* applied here — it is a property of the list, and
/// applying it during construction would mean a sub-list could not be spliced
/// into a larger one without re-deriving it.
pub fn lower(expr: &MathExpr) -> Result<MathList, LayoutError> {
    lower_at(expr, 0)
}

/// Lower a sub-expression, carrying the nesting depth.
fn lower_at(expr: &MathExpr, depth: usize) -> Result<MathList, LayoutError> {
    let mut atoms = Vec::new();
    lower_into(expr, &mut atoms, depth)?;
    Ok(MathList::new(atoms))
}

fn lower_into(expr: &MathExpr, out: &mut Vec<Atom>, depth: usize) -> Result<(), LayoutError> {
    if depth > MAX_NESTING_DEPTH {
        return Err(LayoutError::DepthExceeded);
    }
    let next = depth + 1;

    match expr {
        MathExpr::Number(number) => {
            out.push(Atom::symbol(AtomClass::Ord, number.as_written()));
        }
        MathExpr::Symbol(name) => {
            out.push(Atom::symbol(AtomClass::Ord, name.clone()));
        }
        MathExpr::Text(text) => {
            out.push(Atom::new(AtomClass::Ord, Nucleus::Text(text.clone())));
        }

        MathExpr::Bin(op, left, right) => {
            lower_into(left, out, next)?;
            out.push(Atom::symbol(AtomClass::Bin, bin_symbol(op)));
            lower_into(right, out, next)?;
        }

        MathExpr::Rel(op, left, right) => {
            lower_into(left, out, next)?;
            out.push(Atom::symbol(AtomClass::Rel, rel_symbol(op)));
            lower_into(right, out, next)?;
        }

        // A unary minus is written the same way as subtraction and is a
        // different atom class. Emitting it as `Bin` and letting demotion fix
        // it is not a shortcut -- it is what TeX does, and it means `-x` and
        // `a - x` agree without a special case here.
        MathExpr::Unary(op, operand) => {
            out.push(Atom::symbol(AtomClass::Bin, unary_symbol(op)));
            lower_into(operand, out, next)?;
        }

        MathExpr::Group(inner) => {
            out.push(Atom::new(
                AtomClass::Ord,
                Nucleus::List(lower_at(inner, next)?),
            ));
        }

        // `\left...\right` is Inner, not Ord: that is the whole reason the
        // class exists, and it spaces differently from the group it resembles.
        MathExpr::Fenced { open, body, close } => {
            out.push(Atom::new(
                AtomClass::Inner,
                Nucleus::Delimited {
                    open: open.clone(),
                    body: lower_at(body, next)?,
                    close: close.clone(),
                },
            ));
        }

        MathExpr::Frac(numerator, denominator) => {
            out.push(Atom::new(
                AtomClass::Ord,
                Nucleus::Fraction {
                    numerator: lower_at(numerator, next)?,
                    denominator: lower_at(denominator, next)?,
                },
            ));
        }

        MathExpr::Binom(top, bottom) => {
            out.push(Atom::new(
                AtomClass::Ord,
                Nucleus::Binomial {
                    top: lower_at(top, next)?,
                    bottom: lower_at(bottom, next)?,
                },
            ));
        }

        MathExpr::Root { degree, radicand } => {
            let degree = match degree {
                Some(d) => Some(lower_at(d, next)?),
                None => None,
            };
            out.push(Atom::new(
                AtomClass::Ord,
                Nucleus::Radical {
                    degree,
                    radicand: lower_at(radicand, next)?,
                },
            ));
        }

        // `sin`, `ln` and friends are Op atoms, which is why `\sin x` has a
        // thin space that `sinx` does not.
        MathExpr::Call { func, arg } => {
            out.push(Atom::symbol(
                AtomClass::Op,
                format!("{func:?}").to_lowercase(),
            ));
            lower_into(arg, out, next)?;
        }

        MathExpr::BigOp {
            op,
            lower: lower_bound,
            upper,
            body,
        } => {
            let mut atom = Atom::symbol(AtomClass::Op, format!("{op:?}").to_lowercase());
            atom.subscript = match lower_bound {
                Some(b) => Some(lower_at(b, next)?),
                None => None,
            };
            atom.superscript = match upper {
                Some(b) => Some(lower_at(b, next)?),
                None => None,
            };
            // Limits go above and below in display style; the renderer applies
            // the style, so this records only that they *are* limits.
            atom.limits = true;
            out.push(atom);
            lower_into(body, out, next)?;
        }

        MathExpr::Subscript(base, index) => {
            let mut base_atoms = Vec::new();
            lower_into(base, &mut base_atoms, next)?;
            attach_script(&mut base_atoms, Some(lower_at(index, next)?), None, out);
        }

        MathExpr::Accent { accent, body } => {
            out.push(Atom::new(
                AtomClass::Ord,
                Nucleus::Stacked {
                    base: lower_at(body, next)?,
                    over: Some(MathList::new(vec![Atom::symbol(
                        AtomClass::Ord,
                        accent.clone(),
                    )])),
                    under: None,
                },
            ));
        }

        MathExpr::Overset { over, base } => {
            out.push(Atom::new(
                AtomClass::Ord,
                Nucleus::Stacked {
                    base: lower_at(base, next)?,
                    over: Some(lower_at(over, next)?),
                    under: None,
                },
            ));
        }

        MathExpr::Underset { under, base } => {
            out.push(Atom::new(
                AtomClass::Ord,
                Nucleus::Stacked {
                    base: lower_at(base, next)?,
                    over: None,
                    under: Some(lower_at(under, next)?),
                },
            ));
        }

        // The commas are separators, not operations, so they are Punct atoms
        // and pick up Punct's thin space after each one.
        MathExpr::Sequence(items) => {
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(Atom::symbol(AtomClass::Punct, ","));
                }
                lower_into(item, out, next)?;
            }
        }

        MathExpr::Matrix(rows) => {
            let mut lowered_rows = Vec::with_capacity(rows.len());
            for row in rows {
                let mut cells = Vec::with_capacity(row.len());
                for cell in row {
                    cells.push(lower_at(cell, next)?);
                }
                lowered_rows.push(cells);
            }
            out.push(Atom::new(AtomClass::Ord, Nucleus::Matrix(lowered_rows)));
        }
    }

    Ok(())
}

fn attach_script(
    atoms: &mut Vec<Atom>,
    subscript: Option<MathList>,
    superscript: Option<MathList>,
    out: &mut Vec<Atom>,
) {
    let mut atom =
        if atoms.len() == 1 && atoms[0].subscript.is_none() && atoms[0].superscript.is_none() {
            atoms.pop().expect("checked length")
        } else {
            Atom::new(
                AtomClass::Ord,
                Nucleus::List(MathList::new(std::mem::take(atoms))),
            )
        };
    if subscript.is_some() {
        atom.subscript = subscript;
    }
    if superscript.is_some() {
        atom.superscript = superscript;
    }
    out.push(atom);
}

fn bin_symbol(op: &math_frontend::BinOp) -> String {
    format!("{op:?}").to_lowercase()
}

fn rel_symbol(op: &math_frontend::RelOp) -> String {
    format!("{op:?}").to_lowercase()
}

fn unary_symbol(op: &math_frontend::UnaryOp) -> String {
    format!("{op:?}").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ord(text: &str) -> Atom {
        Atom::symbol(AtomClass::Ord, text)
    }
    fn class_list(classes: &[AtomClass]) -> MathList {
        MathList::new(
            classes
                .iter()
                .map(|&c| Atom::symbol(c, "x"))
                .collect::<Vec<_>>(),
        )
    }

    /// `MathExpr` comes from parsing user-supplied LaTeX, so its nesting depth
    /// is attacker-controlled: a few hundred kilobytes of `{{{...}}}` is a
    /// hundred thousand levels. Before the cap this aborted the process with a
    /// stack overflow — which in Engram means a crafted flashcard crashes the
    /// app, not merely a bad render.
    #[test]
    fn deeply_nested_input_is_refused_rather_than_crashing() {
        // MathExpr comes from parsing user-supplied LaTeX, so its depth is
        // attacker-controlled. 100k nested groups is a few hundred KB of input.
        let mut expr = MathExpr::Symbol("x".to_string());
        for _ in 0..100_000 {
            expr = MathExpr::Group(Box::new(expr));
        }
        assert_eq!(lower(&expr).unwrap_err(), LayoutError::DepthExceeded);
    }

    /// The cap must not reject anything a person would actually write.
    #[test]
    fn ordinary_nesting_is_well_inside_the_cap() {
        let mut expr = MathExpr::Symbol("x".to_string());
        for _ in 0..32 {
            expr = MathExpr::Group(Box::new(expr));
        }
        assert!(lower(&expr).is_ok());
    }

    #[test]
    fn a_leading_bin_is_demoted_because_it_has_nothing_to_bind() {
        // `-x`: the minus is a sign, not a subtraction.
        let list = MathList::new(vec![Atom::symbol(AtomClass::Bin, "-"), ord("x")]);
        assert_eq!(
            list.resolved_classes(),
            vec![AtomClass::Ord, AtomClass::Ord]
        );
    }

    #[test]
    fn a_bin_between_two_ords_stays_binary() {
        // `a - x`: the same character, a different atom.
        let list = MathList::new(vec![ord("a"), Atom::symbol(AtomClass::Bin, "-"), ord("x")]);
        assert_eq!(
            list.resolved_classes(),
            vec![AtomClass::Ord, AtomClass::Bin, AtomClass::Ord]
        );
        assert_eq!(
            list.spacings(Style::Text),
            vec![Space::None, Space::Med, Space::Med]
        );
    }

    #[test]
    fn a_trailing_bin_is_demoted() {
        let list = MathList::new(vec![ord("a"), Atom::symbol(AtomClass::Bin, "+")]);
        assert_eq!(
            list.resolved_classes(),
            vec![AtomClass::Ord, AtomClass::Ord]
        );
    }

    #[test]
    fn a_bin_before_a_rel_is_demoted() {
        // The TeXbook marks Bin-then-Rel impossible; this is why.
        let list = class_list(&[
            AtomClass::Ord,
            AtomClass::Bin,
            AtomClass::Rel,
            AtomClass::Ord,
        ]);
        assert_eq!(list.resolved_classes()[1], AtomClass::Ord);
    }

    #[test]
    fn consecutive_bins_collapse_left_to_right() {
        // `a + + b`: the first binds, the second has a Bin before it and so
        // becomes a sign. Demoting right-to-left would give the opposite
        // answer, so this pins the direction.
        let list = class_list(&[
            AtomClass::Ord,
            AtomClass::Bin,
            AtomClass::Bin,
            AtomClass::Ord,
        ]);
        assert_eq!(
            list.resolved_classes(),
            vec![
                AtomClass::Ord,
                AtomClass::Bin,
                AtomClass::Ord,
                AtomClass::Ord
            ]
        );
    }

    /// Script-style suppression is per CELL, not per size of space.
    ///
    /// The tempting summary — "thin survives, medium and thick do not" — is
    /// wrong in 30 of the 256 combinations. `Op` then `Ord` keeps its thin
    /// space; `Inner` then `Ord` loses the identical one.
    #[test]
    fn script_style_suppression_is_per_cell_not_per_space_size() {
        assert_eq!(
            spacing(AtomClass::Ord, AtomClass::Bin, Style::Text),
            Space::Med
        );
        assert_eq!(
            spacing(AtomClass::Ord, AtomClass::Bin, Style::Script),
            Space::None
        );
        assert_eq!(
            spacing(AtomClass::Ord, AtomClass::Rel, Style::ScriptScript),
            Space::None
        );
        // This thin space survives...
        assert_eq!(
            spacing(AtomClass::Ord, AtomClass::Op, Style::Script),
            Space::Thin
        );
        // ...and this identical one does not.
        assert_eq!(
            spacing(AtomClass::Inner, AtomClass::Ord, Style::Text),
            Space::Thin
        );
        assert_eq!(
            spacing(AtomClass::Inner, AtomClass::Ord, Style::Script),
            Space::None
        );
    }

    #[test]
    fn spacings_align_one_to_one_with_atoms() {
        let list = class_list(&[AtomClass::Ord, AtomClass::Rel, AtomClass::Ord]);
        let spaces = list.spacings(Style::Display);
        assert_eq!(spaces.len(), list.len());
        assert_eq!(spaces[0], Space::None, "nothing precedes the first atom");
        assert_eq!(spaces[1], Space::Thick);
    }

    #[test]
    fn a_fenced_group_is_inner_not_ord() {
        let expr = MathExpr::Fenced {
            open: "(".to_string(),
            body: Box::new(MathExpr::Symbol("x".to_string())),
            close: ")".to_string(),
        };
        let list = lower(&expr).unwrap();
        assert_eq!(list.atoms[0].class, AtomClass::Inner);
    }

    #[test]
    fn a_plain_group_is_ord() {
        let expr = MathExpr::Group(Box::new(MathExpr::Symbol("x".to_string())));
        assert_eq!(lower(&expr).unwrap().atoms[0].class, AtomClass::Ord);
    }

    #[test]
    fn a_sequence_separates_its_items_with_punct_atoms() {
        let expr = MathExpr::Sequence(vec![
            MathExpr::Symbol("a".to_string()),
            MathExpr::Symbol("b".to_string()),
        ]);
        let list = lower(&expr).unwrap();
        assert_eq!(
            list.resolved_classes(),
            vec![AtomClass::Ord, AtomClass::Punct, AtomClass::Ord]
        );
        // Punct's thin space is what makes `(a, b)` read as a list.
        assert_eq!(list.spacings(Style::Text)[2], Space::Thin);
    }

    #[test]
    fn a_multi_atom_base_is_boxed_before_a_script_is_attached() {
        // `(a+b)_i` must not become `a + b_i`.
        let expr = MathExpr::Subscript(
            Box::new(MathExpr::Bin(
                math_frontend::BinOp::Add,
                Box::new(MathExpr::Symbol("a".to_string())),
                Box::new(MathExpr::Symbol("b".to_string())),
            )),
            Box::new(MathExpr::Symbol("i".to_string())),
        );
        let list = lower(&expr).unwrap();
        assert_eq!(list.len(), 1, "the base should be one boxed atom");
        assert!(matches!(list.atoms[0].nucleus, Nucleus::List(_)));
        assert!(list.atoms[0].subscript.is_some());
    }
}
