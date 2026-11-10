//! # Pattern-matrix exhaustiveness and redundancy (VER04j section 4.3)
//!
//! A `match` is *exhaustive* when every value of the scrutinee's type is taken
//! by some arm, and an arm is *redundant* when every value it takes is already
//! taken by an earlier arm.  Both are decided here by the classic
//! pattern-matrix **usefulness** algorithm (Maranget, "Warnings for pattern
//! matching", 2007), over the finite constructor sets Warrant has: `Bool`,
//! `Option`, `Result`, the cases of an error type, tuples, records and
//! variants.
//!
//! ```text
//! A pattern vector q is USEFUL against a matrix of rows if some value matches
//! q and matches no row.
//!
//!   match is exhaustive   <=>  [_] is not useful against all arms
//!   arm i is redundant    <=>  row i is not useful against rows 0..i
//! ```
//!
//! The algorithm works on an abstract *shape* of the scrutinee ([`PTy`]) and an
//! abstract pattern ([`Pat`]): a wildcard, or a constructor given by its index
//! within the shape with sub-patterns for its fields.  The checker translates
//! its typed patterns into these (binders become wildcards) and the tests
//! drive the module directly, comparing it with brute-force enumeration.
//!
//! ## Constructor numbering
//!
//! ```text
//! Bool            0 = false           1 = true
//! Option<T>       0 = None            1 = Some(T)
//! Result<T, E>    0 = Ok(T)           1 = Err(E)
//! error cases     one nullary constructor per case, in declaration-set order
//! tuple           one constructor, 0, with one field per component
//! record/variant  constructor i of the declaration, fields in order
//! Opaque          integers, bytes, lists: no constructor pattern exists, so
//!                 only a wildcard matches (there are no literal patterns)
//! ```
//!
//! ## Worked example
//!
//! ```text
//! match (b, o) with  (true, None) -> 1 | (false, _) -> 2 | (_, Some(_)) -> 3
//! rows: [T, None] [F, _] [_, Some]
//! Is [_, _] useful?  Split on the first column's constructors:
//!   false: rows left [F,_] -> [_] ... wildcards cover it: not useful
//!   true : rows left [None] [Some] -> [_] against {None, Some}: both
//!          constructors covered: not useful
//! => exhaustive; and no row is redundant.
//! ```
//!
//! ## Cost
//!
//! Every call charges one unit per row it looks at plus one, against a budget
//! the caller supplies; running out is [`Exhausted`] (the checker reports
//! W0602).  This bounds the (worst case exponential) algorithm by the work
//! the checker is willing to spend.

/// The shape of a scrutinee, as far as patterns can see into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PTy {
    /// `Bool`.
    Bool,
    /// A type with no constructor patterns (`Int`, `Nat`, fixed-width integers,
    /// `Bytes`, `List<T>`): only `_` or a binder matches.
    Opaque,
    /// `Option<T>`.
    Option(Box<PTy>),
    /// `Result<T, E>`: the second shape is the error type, [`PTy::ErrCases`].
    Result(Box<PTy>, Box<PTy>),
    /// An error type with this many nullary cases.
    ErrCases(usize),
    /// A tuple.
    Tuple(Vec<PTy>),
    /// A record (one constructor) or variant: the field shapes of each
    /// constructor.
    Data(Vec<Vec<PTy>>),
}

impl PTy {
    /// The field shapes of every constructor, or `None` for [`PTy::Opaque`].
    pub fn ctors(&self) -> Option<Vec<Vec<PTy>>> {
        match self {
            PTy::Opaque => None,
            PTy::Bool => Some(vec![vec![], vec![]]),
            PTy::Option(t) => Some(vec![vec![], vec![(**t).clone()]]),
            PTy::Result(t, e) => Some(vec![vec![(**t).clone()], vec![(**e).clone()]]),
            PTy::ErrCases(n) => Some(vec![vec![]; *n]),
            PTy::Tuple(ts) => Some(vec![ts.clone()]),
            PTy::Data(cs) => Some(cs.clone()),
        }
    }

    /// Number of constructors (`None` when infinite / opaque).
    pub fn ctor_count(&self) -> Option<usize> {
        self.ctors().map(|c| c.len())
    }
}

/// An abstract pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pat {
    /// `_` or a binder.
    Wild,
    /// Constructor `index` of the shape with one sub-pattern per field.
    Ctor(usize, Vec<Pat>),
}

/// The work budget ran out (the checker reports W0602).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exhausted;

fn spend(budget: &mut usize, amount: usize) -> Result<(), Exhausted> {
    if *budget < amount {
        *budget = 0;
        return Err(Exhausted);
    }
    *budget -= amount;
    Ok(())
}

/// Rows of the matrix specialised to constructor `c` of arity `arity`: rows
/// starting with `c` expose their sub-patterns, rows starting with `_` expose
/// `arity` wildcards, other rows disappear.
fn specialize(rows: &[Vec<Pat>], c: usize, arity: usize) -> Vec<Vec<Pat>> {
    let mut out = Vec::new();
    for row in rows {
        match &row[0] {
            Pat::Ctor(k, args) if *k == c => {
                let mut r = args.clone();
                r.extend_from_slice(&row[1..]);
                out.push(r);
            }
            Pat::Ctor(..) => {}
            Pat::Wild => {
                let mut r = vec![Pat::Wild; arity];
                r.extend_from_slice(&row[1..]);
                out.push(r);
            }
        }
    }
    out
}

/// Is the pattern vector `q` useful against `rows`?  `tys` is the shape of each
/// column; every row and `q` have `tys.len()` entries.
pub fn useful(
    rows: &[Vec<Pat>],
    q: &[Pat],
    tys: &[PTy],
    budget: &mut usize,
) -> Result<bool, Exhausted> {
    spend(budget, rows.len() + 1 + tys.len())?;
    if q.is_empty() {
        // No columns left: q is useful exactly when no row already matches.
        return Ok(rows.is_empty());
    }
    // A row of wildcards matches everything `q` matches: not useful, and no
    // need to expand the shape at all.
    if rows.iter().any(|r| r.iter().all(|p| matches!(p, Pat::Wild))) {
        return Ok(false);
    }
    let ty = &tys[0];
    let all = ty.ctors();
    if let Some(all) = &all {
        // Cloning the sub-shapes is work too.
        spend(budget, all.iter().map(Vec::len).sum::<usize>())?;
    }
    match (&q[0], all) {
        (Pat::Ctor(c, args), Some(all)) => {
            let fields = &all[*c];
            let mut next_q = args.clone();
            next_q.extend_from_slice(&q[1..]);
            let mut next_tys = fields.clone();
            next_tys.extend_from_slice(&tys[1..]);
            let next_rows = specialize(rows, *c, fields.len());
            useful(&next_rows, &next_q, &next_tys, budget)
        }
        (Pat::Ctor(..), None) => Ok(false),
        (Pat::Wild, Some(all)) => {
            // A wildcard is useful if it is useful for at least one
            // constructor of the (finite, complete) signature.
            for (c, fields) in all.iter().enumerate() {
                let mut next_q = vec![Pat::Wild; fields.len()];
                next_q.extend_from_slice(&q[1..]);
                let mut next_tys = fields.clone();
                next_tys.extend_from_slice(&tys[1..]);
                let next_rows = specialize(rows, c, fields.len());
                if useful(&next_rows, &next_q, &next_tys, budget)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        (Pat::Wild, None) => {
            // Opaque column: only wildcards can appear; use the default matrix.
            let default: Vec<Vec<Pat>> = rows
                .iter()
                .filter(|r| matches!(r[0], Pat::Wild))
                .map(|r| r[1..].to_vec())
                .collect();
            useful(&default, &q[1..], &tys[1..], budget)
        }
    }
}

/// The verdict on a whole `match`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchReport {
    /// Some value reaches no arm.
    pub exhaustive: bool,
    /// Indexes of the arms every value of which an earlier arm already takes.
    pub redundant: Vec<usize>,
}

/// Check a list of single-column arms against the scrutinee shape `ty`.
pub fn analyze(arms: &[Pat], ty: &PTy, budget: &mut usize) -> Result<MatchReport, Exhausted> {
    let tys = std::slice::from_ref(ty);
    let mut rows: Vec<Vec<Pat>> = Vec::new();
    let mut redundant = Vec::new();
    for (i, arm) in arms.iter().enumerate() {
        let row = vec![arm.clone()];
        if !useful(&rows, &row, tys, budget)? {
            redundant.push(i);
        }
        rows.push(row);
    }
    let exhaustive = !useful(&rows, &[Pat::Wild], tys, budget)?;
    Ok(MatchReport {
        exhaustive,
        redundant,
    })
}

/// Does value `v` (a pattern with no wildcards: a constructor tree) match
/// pattern `p`?  Used by the brute-force oracle of the tests.
pub fn matches(p: &Pat, v: &Pat) -> bool {
    match (p, v) {
        (Pat::Wild, _) => true,
        (Pat::Ctor(a, ps), Pat::Ctor(b, vs)) => {
            a == b && ps.len() == vs.len() && ps.iter().zip(vs).all(|(p, v)| matches(p, v))
        }
        (Pat::Ctor(..), Pat::Wild) => false,
    }
}

/// Every value of `ty` as a constructor tree.  `None` for a type with
/// infinitely many values; the size is bounded by the caller choosing small
/// shapes (this is a test oracle, not part of the checker).
pub fn values(ty: &PTy) -> Option<Vec<Pat>> {
    let ctors = ty.ctors()?;
    let mut out = Vec::new();
    for (i, fields) in ctors.iter().enumerate() {
        let mut combos: Vec<Vec<Pat>> = vec![Vec::new()];
        for f in fields {
            let vs = values(f)?;
            let mut next = Vec::new();
            for c in &combos {
                for v in &vs {
                    let mut n = c.clone();
                    n.push(v.clone());
                    next.push(n);
                }
            }
            combos = next;
        }
        for c in combos {
            out.push(Pat::Ctor(i, c));
        }
    }
    Some(out)
}
