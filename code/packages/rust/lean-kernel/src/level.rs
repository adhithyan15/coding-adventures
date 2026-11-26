//! Universe levels, and the equivalence and ordering checks the kernel needs on them.
//!
//! # What a universe level is
//!
//! Lean's types live in a hierarchy: `Prop = Sort 0`, `Type = Sort 1`, `Type 1 = Sort 2`
//! and so on, to stop `Type : Type` (Girard's paradox). A *universe level* is the
//! natural-number-valued expression that says which rung:
//!
//! ```text
//!   Level ::= 0                 zero
//!           | succ l            l + 1
//!           | max l1 l2         the larger of the two
//!           | imax l1 l2        0 if l2 = 0, otherwise max l1 l2     ("impredicative max")
//!           | u                 a universe parameter, a placeholder for any natural number
//! ```
//!
//! `imax` is what makes `Prop` impredicative: a function type `A -> B` with `B : Prop`
//! is itself a `Prop` regardless of how big `A` is, because
//! `Sort (imax u 0) = Sort 0`.
//!
//! # Why normalisation
//!
//! Two syntactically different levels can denote the same number for every assignment
//! of the parameters: `max u v` and `max v u`, or `max u u` and `u`, or `imax u 1` and
//! `max u 1`. The kernel must treat `Sort (max u v)` and `Sort (max v u)` as the same
//! type. Getting this wrong in one direction rejects valid proofs; getting it wrong in
//! the other (declaring two different levels equal) is a soundness bug, because it lets
//! a universe-inconsistent proof through. So [`Level::is_equiv`] only answers `true`
//! when the normal forms are identical, and [`Level::is_geq`] only answers `true` when
//! it can show the inequality for every parameter assignment; both may say `false` for
//! a true-but-hard fact (they are sound, deliberately not complete, as in Lean's kernel).
//!
//! # Normal form
//!
//! `normalize` rewrites a level to a canonical shape:
//!
//! * `succ` is pushed through `max` (`succ (max a b) = max (succ a) (succ b)`), so a
//!   `max` never sits under a `succ`;
//! * `max` is flattened, its arguments sorted, duplicates with the same base merged
//!   (`max u (u+2)` becomes `u+2`), and a numeral dropped when some other argument
//!   already dominates it (`max 1 (v+1)` becomes `v+1`, because `v+1 >= 1`);
//! * `imax a b` becomes `max a b` when `b` is certainly non-zero, `0` when `b` is `0`,
//!   `b` when `a` is `0`, and `a` when `a = b`; otherwise it stays an `imax` whose
//!   arguments are themselves normalised.
//!
//! The levels are immutable and shared (`Arc`), carry a cached structural hash and a
//! cached "mentions a parameter" flag, and compare structurally.

use crate::name::Name;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Largest `k` accepted by [`Level::offset`] / [`Level::of_nat`] (2^16): they build `k`
/// nodes, and real universe offsets are tiny.
pub const MAX_LEVEL_OFFSET: u32 = 1 << 16;

/// Deepest level nesting the kernel will normalise or accept in a term (`succ` counts as
/// one level of nesting). Real universe levels are a handful deep; the bound keeps every
/// recursive level operation within a few hundred stack frames, whatever the input.
pub const MAX_LEVEL_DEPTH: u32 = 128;

/// Largest level, counted as a *tree* (a shared child counts once per use), the kernel
/// will normalise or accept in a term. Normalisation flattens `max` into a list and
/// pushes `succ` onto every element, so its cost grows with the tree size, not with the
/// number of distinct nodes: a 100-node DAG can denote `2^100` paths. Bounding the tree
/// size bounds both time and output size, and every offset the normaliser computes
/// (at most the depth) stays far below [`MAX_LEVEL_OFFSET`].
pub const MAX_LEVEL_SIZE: u32 = 512;

/// Error: a level offset above [`MAX_LEVEL_OFFSET`] was requested.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelOffsetTooLarge(pub u32);

/// Error: parameter and level lists of different lengths were given to an instantiation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ArityMismatch {
    /// Number of parameter names.
    pub params: usize,
    /// Number of replacement levels.
    pub levels: usize,
}

/// A universe level. Construct with [`Level::zero`], [`Level::succ`], [`Level::max`],
/// [`Level::imax`], [`Level::param`]; inspect with [`Level::kind`].
#[derive(Clone)]
pub struct Level(Arc<LevelData>);

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct LevelData {
    kind: LevelKind,
    hash: u64,
    has_param: bool,
    /// Nesting depth (a leaf is 1), saturating.
    depth: u32,
    /// Tree size (nodes counted once per path), saturating.
    size: u32,
}

/// The shape of a level. The variant order is the sort order used by normalisation
/// (numerals first, then parameters, then `imax`).
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum LevelKind {
    /// `0`.
    Zero,
    /// `l + 1`.
    Succ(Level),
    /// `max l1 l2`.
    Max(Level, Level),
    /// `imax l1 l2`.
    IMax(Level, Level),
    /// A universe parameter.
    Param(Name),
}

fn hash_of(tag: u8, parts: &[u64]) -> u64 {
    let mut s = DefaultHasher::new();
    tag.hash(&mut s);
    parts.hash(&mut s);
    s.finish()
}

impl Level {
    fn mk(kind: LevelKind) -> Level {
        let (depth, size) = match &kind {
            LevelKind::Zero | LevelKind::Param(_) => (1, 1),
            LevelKind::Succ(a) => (a.0.depth.saturating_add(1), a.0.size.saturating_add(1)),
            LevelKind::Max(a, b) | LevelKind::IMax(a, b) => (
                a.0.depth.max(b.0.depth).saturating_add(1),
                a.0.size.saturating_add(b.0.size).saturating_add(1),
            ),
        };
        let (hash, has_param) = match &kind {
            LevelKind::Zero => (hash_of(0, &[]), false),
            LevelKind::Succ(a) => (hash_of(1, &[a.0.hash]), a.0.has_param),
            LevelKind::Max(a, b) => (
                hash_of(2, &[a.0.hash, b.0.hash]),
                a.0.has_param || b.0.has_param,
            ),
            LevelKind::IMax(a, b) => (
                hash_of(3, &[a.0.hash, b.0.hash]),
                a.0.has_param || b.0.has_param,
            ),
            LevelKind::Param(n) => {
                let mut s = DefaultHasher::new();
                n.hash(&mut s);
                (hash_of(4, &[s.finish()]), true)
            }
        };
        Level(Arc::new(LevelData {
            kind,
            hash,
            has_param,
            depth,
            size,
        }))
    }

    /// The level `0`.
    pub fn zero() -> Level {
        Level::mk(LevelKind::Zero)
    }
    /// `l + 1`.
    pub fn succ(l: &Level) -> Level {
        Level::mk(LevelKind::Succ(l.clone()))
    }
    /// `max a b`, built as written (no simplification; see [`Level::normalize`]).
    pub fn max(a: &Level, b: &Level) -> Level {
        Level::mk(LevelKind::Max(a.clone(), b.clone()))
    }
    /// `imax a b`, built as written.
    pub fn imax(a: &Level, b: &Level) -> Level {
        Level::mk(LevelKind::IMax(a.clone(), b.clone()))
    }
    /// A universe parameter.
    pub fn param(n: &Name) -> Level {
        Level::mk(LevelKind::Param(n.clone()))
    }
    /// `base + k` (apply `succ` `k` times). Panics if `k > MAX_LEVEL_OFFSET`; use
    /// [`Level::try_offset`] for untrusted `k`.
    pub fn offset(base: &Level, k: u32) -> Level {
        Level::try_offset(base, k).expect("level offset above MAX_LEVEL_OFFSET")
    }
    /// `base + k`, or an error if `k > MAX_LEVEL_OFFSET` (it builds `k` nodes, so an
    /// untrusted `k` could otherwise exhaust memory).
    pub fn try_offset(base: &Level, k: u32) -> Result<Level, LevelOffsetTooLarge> {
        if k > MAX_LEVEL_OFFSET {
            return Err(LevelOffsetTooLarge(k));
        }
        Ok((0..k).fold(base.clone(), |l, _| Level::succ(&l)))
    }
    /// The numeral `k`. Panics if `k > MAX_LEVEL_OFFSET`.
    pub fn of_nat(k: u32) -> Level {
        Level::offset(&Level::zero(), k)
    }
    /// The numeral `k`, or an error if `k > MAX_LEVEL_OFFSET`.
    pub fn try_of_nat(k: u32) -> Result<Level, LevelOffsetTooLarge> {
        Level::try_offset(&Level::zero(), k)
    }

    /// The shape of this level.
    pub fn kind(&self) -> &LevelKind {
        &self.0.kind
    }
    /// The cached structural hash.
    pub fn hash_value(&self) -> u64 {
        self.0.hash
    }
    /// Does the level mention any universe parameter? (Cached.)
    pub fn has_param(&self) -> bool {
        self.0.has_param
    }
    /// Nesting depth (cached, saturating).
    pub fn depth(&self) -> u32 {
        self.0.depth
    }
    /// Tree size (cached, saturating).
    pub fn size(&self) -> u32 {
        self.0.size
    }
    /// Is the level within [`MAX_LEVEL_DEPTH`] and [`MAX_LEVEL_SIZE`]? Operations that
    /// recurse over structure (normalisation, comparison of normal forms) only run on
    /// such levels; the type checker rejects any other level with `ResourceLimit`.
    pub fn is_within_limits(&self) -> bool {
        self.0.depth <= MAX_LEVEL_DEPTH && self.0.size <= MAX_LEVEL_SIZE
    }
    /// Is this exactly `0`?
    pub fn is_zero(&self) -> bool {
        matches!(self.kind(), LevelKind::Zero)
    }

    /// Split `succ^k base` into `(base, k)` where `base` is not a `succ`.
    pub fn to_offset(&self) -> (Level, u32) {
        let (mut cur, mut k) = (self, 0);
        while let LevelKind::Succ(inner) = cur.kind() {
            cur = inner;
            k += 1;
        }
        (cur.clone(), k)
    }

    /// Is this a numeral (`succ^k 0`)?
    pub fn is_explicit(&self) -> bool {
        self.to_offset().0.is_zero()
    }

    /// Is this level provably `>= 1` for every parameter assignment? (Sound, not complete.)
    /// `imax a b` is non-zero exactly when `b` is.
    pub fn is_never_zero(&self) -> bool {
        // Explicit stack (levels can be a million deep) and a visited set (a shared DAG
        // would otherwise be walked once per path).
        let mut stack = vec![self];
        let mut seen = HashSet::new();
        while let Some(l) = stack.pop() {
            if !seen.insert(Arc::as_ptr(&l.0)) {
                continue;
            }
            match l.kind() {
                LevelKind::Succ(_) => return true,
                LevelKind::Max(a, b) => {
                    stack.push(a);
                    stack.push(b);
                }
                LevelKind::IMax(_, b) => stack.push(b),
                LevelKind::Zero | LevelKind::Param(_) => {}
            }
        }
        false
    }

    /// Replace parameters: `params[i]` becomes `levels[i]`. Parameters not listed are
    /// kept. The slices must have equal length, otherwise `ArityMismatch` (a silent
    /// `zip` truncation would leave parameters uninstantiated). The result is rebuilt as
    /// written, with no simplification.
    pub fn instantiate(&self, params: &[Name], levels: &[Level]) -> Result<Level, ArityMismatch> {
        if params.len() != levels.len() {
            return Err(ArityMismatch {
                params: params.len(),
                levels: levels.len(),
            });
        }
        Ok(self.instantiate_unchecked(params, levels))
    }

    /// `instantiate` after the arity check. Iterative with a memo per shared node, so
    /// neither depth nor sharing can blow the stack or the running time.
    pub(crate) fn instantiate_unchecked(&self, params: &[Name], levels: &[Level]) -> Level {
        enum Task<'a> {
            Visit(&'a Level),
            Succ(*const LevelData),
            Max(*const LevelData),
            IMax(*const LevelData),
        }
        let mut memo: HashMap<*const LevelData, Level> = HashMap::new();
        let mut tasks = vec![Task::Visit(self)];
        let mut out: Vec<Level> = Vec::new();
        while let Some(t) = tasks.pop() {
            match t {
                Task::Visit(l) => {
                    let key = Arc::as_ptr(&l.0);
                    if !l.has_param() {
                        out.push(l.clone());
                    } else if let Some(r) = memo.get(&key) {
                        out.push(r.clone());
                    } else {
                        match l.kind() {
                            LevelKind::Zero => out.push(l.clone()),
                            LevelKind::Param(n) => {
                                let r = match params.iter().zip(levels).find(|(p, _)| *p == n) {
                                    Some((_, r)) => r.clone(),
                                    None => l.clone(),
                                };
                                out.push(r);
                            }
                            LevelKind::Succ(a) => {
                                tasks.push(Task::Succ(key));
                                tasks.push(Task::Visit(a));
                            }
                            LevelKind::Max(a, b) => {
                                tasks.push(Task::Max(key));
                                tasks.push(Task::Visit(b));
                                tasks.push(Task::Visit(a));
                            }
                            LevelKind::IMax(a, b) => {
                                tasks.push(Task::IMax(key));
                                tasks.push(Task::Visit(b));
                                tasks.push(Task::Visit(a));
                            }
                        }
                    }
                }
                Task::Succ(key) => {
                    if let Some(a) = out.pop() {
                        let r = Level::succ(&a);
                        memo.insert(key, r.clone());
                        out.push(r);
                    }
                }
                Task::Max(key) => {
                    if let (Some(b), Some(a)) = (out.pop(), out.pop()) {
                        let r = Level::max(&a, &b);
                        memo.insert(key, r.clone());
                        out.push(r);
                    }
                }
                Task::IMax(key) => {
                    if let (Some(b), Some(a)) = (out.pop(), out.pop()) {
                        let r = Level::imax(&a, &b);
                        memo.insert(key, r.clone());
                        out.push(r);
                    }
                }
            }
        }
        out.pop().unwrap_or_else(|| self.clone())
    }

    /// The canonical form described in the module documentation. A level outside
    /// [`MAX_LEVEL_DEPTH`] / [`MAX_LEVEL_SIZE`] is returned unchanged (never a wrong
    /// answer: an un-normalised level only makes `is_equiv` say "no"), and so is any level
    /// for which an offset would pass [`MAX_LEVEL_OFFSET`]. No input can panic or overflow.
    pub fn normalize(&self) -> Level {
        if !self.is_within_limits() {
            return self.clone();
        }
        self.try_normalize().unwrap_or_else(|| self.clone())
    }

    /// `normalize` for a level already known to be within the limits; `None` if an offset
    /// would leave the supported range (impossible within the limits, but checked).
    fn try_normalize(&self) -> Option<Level> {
        let (base, k) = self.to_offset();
        match base.kind() {
            LevelKind::Zero | LevelKind::Param(_) | LevelKind::Succ(_) => Some(self.clone()),
            LevelKind::Max(a, b) => normalize_max(vec![a.clone(), b.clone()], k),
            LevelKind::IMax(a, b) => {
                let (a, b) = (a.try_normalize()?, b.try_normalize()?);
                if b.is_never_zero() {
                    normalize_max(vec![a, b], k)
                } else if b.is_zero() {
                    // imax a 0 = 0, so succ^k (imax a 0) is the numeral k.
                    Level::try_of_nat(k).ok()
                } else if a.is_zero() {
                    // `b` may be a `max`; under the outer `succ`s it must be re-normalised
                    // so the `succ` is pushed through (else normalize is not idempotent).
                    Level::try_offset(&b, k).ok()?.try_normalize()
                } else if a == b {
                    Level::try_offset(&a, k).ok()?.try_normalize()
                } else {
                    Level::try_offset(&Level::imax(&a, &b), k).ok()
                }
            }
        }
    }

    /// Are the two levels equal for every assignment of parameters? Compares normal
    /// forms, so it is sound; it is the check used when two `Sort`s must agree.
    pub fn is_equiv(&self, other: &Level) -> bool {
        self == other
            || (self.is_within_limits()
                && other.is_within_limits()
                && self.normalize() == other.normalize())
    }

    /// Is `self >= other` for every assignment of parameters? Sound, not complete.
    pub fn is_geq(&self, other: &Level) -> bool {
        if !(self.is_within_limits() && other.is_within_limits()) {
            return self == other; // sound: only the trivial case is claimed
        }
        geq_core(&self.normalize(), &other.normalize())
    }
}

/// Equivalence of two level lists (the arguments of two `Const`s), pointwise.
pub fn levels_equiv(a: &[Level], b: &[Level]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.is_equiv(y))
}

/// Flatten nested `max`es into their leaves.
fn max_leaves(l: &Level, out: &mut Vec<Level>) {
    match l.kind() {
        LevelKind::Max(a, b) => {
            max_leaves(a, out);
            max_leaves(b, out);
        }
        _ => out.push(l.clone()),
    }
}

/// Normalise `succ^k (max parts...)`. Steps (each guards against a classic bug):
/// 1. normalise every part and flatten, so `max` is associative in the result;
/// 2. sort by (base, offset): equal bases become adjacent, giving commutativity;
/// 3. keep only the largest offset per base (`max u (u+2) = u+2`, idempotence);
/// 4. drop the numeral if another argument dominates it: `u + j >= j` always.
///    Dropping it when nothing dominates would be wrong (`max 3 u` is not `u`);
/// 5. push the outer `k` onto every argument and rebuild a right-nested `max`.
fn normalize_max(parts: Vec<Level>, k: u32) -> Option<Level> {
    let mut leaves = Vec::new();
    for p in &parts {
        max_leaves(&p.try_normalize()?, &mut leaves);
    }
    let mut args: Vec<(Level, u32)> = leaves.iter().map(Level::to_offset).collect();
    args.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
    let mut kept: Vec<(Level, u32)> = Vec::new();
    for arg in args {
        match kept.last_mut() {
            Some(last) if last.0 == arg.0 => *last = arg, // ascending offsets: the later is larger
            _ => kept.push(arg),
        }
    }
    // After the sort the numeral (base `0`) is first, if present.
    if kept.len() > 1 && kept[0].0.is_zero() {
        let numeral = kept[0].1;
        if kept[1..].iter().any(|(_, off)| *off >= numeral) {
            kept.remove(0);
        }
    }
    let mut built = Vec::with_capacity(kept.len());
    for (b, off) in kept.into_iter().rev() {
        built.push(Level::try_offset(&b, off.checked_add(k)?).ok()?);
    }
    let mut it = built.into_iter();
    let last = it.next()?;
    Some(it.fold(last, |acc, l| Level::max(&l, &acc)))
}

fn geq_core(a: &Level, b: &Level) -> bool {
    if a == b || b.is_zero() {
        return true;
    }
    if let LevelKind::Max(x, y) = b.kind() {
        return geq_core(a, x) && geq_core(a, y);
    }
    if let LevelKind::Max(x, y) = a.kind() {
        if geq_core(x, b) || geq_core(y, b) {
            return true;
        }
    }
    if let LevelKind::IMax(x, y) = b.kind() {
        // imax x y <= max x y, so it suffices to dominate both.
        return geq_core(a, x) && geq_core(a, y);
    }
    if let LevelKind::IMax(_, y) = a.kind() {
        // imax x y >= y.
        return geq_core(y, b);
    }
    let ((ba, ka), (bb, kb)) = (a.to_offset(), b.to_offset());
    if ba == bb || bb.is_zero() {
        return ka >= kb; // same base, or `b` is a numeral that `a`'s offset covers
    }
    ka == kb && ka > 0 && geq_core(&ba, &bb) // cancel a common `succ^k`
}

/// Drop iteratively: a long `succ` chain would otherwise recurse once per node and
/// overflow the stack. Children of a uniquely owned node are moved onto a work list.
impl Drop for Level {
    fn drop(&mut self) {
        fn take(d: &mut LevelData, stack: &mut Vec<Level>) {
            match std::mem::replace(&mut d.kind, LevelKind::Zero) {
                LevelKind::Succ(a) => stack.push(a),
                LevelKind::Max(a, b) | LevelKind::IMax(a, b) => {
                    stack.push(a);
                    stack.push(b);
                }
                LevelKind::Zero | LevelKind::Param(_) => {}
            }
        }
        let mut stack = Vec::new();
        if let Some(d) = Arc::get_mut(&mut self.0) {
            take(d, &mut stack);
        }
        while let Some(mut l) = stack.pop() {
            if let Some(d) = Arc::get_mut(&mut l.0) {
                take(d, &mut stack);
            }
        }
    }
}

/// Structural three-way comparison, by variant order then contents (the order the derived
/// `Ord` on [`LevelKind`] would give). Iterative, with a visited set of pointer pairs
/// once the walk gets long: a level a million deep, or a DAG denoting `2^100` paths,
/// compares in linear time and constant stack. Equal results short-circuit on shared
/// pointers, so comparing a level with itself is O(1).
fn compare(x: &Level, y: &Level) -> Ordering {
    fn rank(l: &Level) -> u8 {
        match l.kind() {
            LevelKind::Zero => 0,
            LevelKind::Succ(_) => 1,
            LevelKind::Max(..) => 2,
            LevelKind::IMax(..) => 3,
            LevelKind::Param(_) => 4,
        }
    }
    let mut stack = vec![(x, y)];
    let mut seen: HashSet<(*const LevelData, *const LevelData)> = HashSet::new();
    let mut steps = 0u32;
    while let Some((a, b)) = stack.pop() {
        if Arc::ptr_eq(&a.0, &b.0) {
            continue;
        }
        steps = steps.saturating_add(1);
        if steps > 64 && !seen.insert((Arc::as_ptr(&a.0), Arc::as_ptr(&b.0))) {
            continue; // this exact pair was already compared (and was equal so far)
        }
        match (a.kind(), b.kind()) {
            (LevelKind::Zero, LevelKind::Zero) => {}
            (LevelKind::Succ(p), LevelKind::Succ(q)) => stack.push((p, q)),
            (LevelKind::Max(p, r), LevelKind::Max(q, t))
            | (LevelKind::IMax(p, r), LevelKind::IMax(q, t)) => {
                stack.push((r, t));
                stack.push((p, q));
            }
            (LevelKind::Param(m), LevelKind::Param(n)) => match m.cmp(n) {
                Ordering::Equal => {}
                o => return o,
            },
            _ => return rank(a).cmp(&rank(b)),
        }
    }
    Ordering::Equal
}

impl PartialEq for Level {
    fn eq(&self, other: &Level) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0.hash == other.0.hash && compare(self, other) == Ordering::Equal)
    }
}
impl Eq for Level {}
impl Hash for Level {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash);
    }
}
impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Level) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Level {
    fn cmp(&self, other: &Level) -> Ordering {
        compare(self, other)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Printing recurses once per nesting: refuse to for levels beyond the limits.
        if !self.is_within_limits() {
            return write!(f, "<level of depth {}>", self.depth());
        }
        let (base, k) = self.to_offset();
        match (base.kind(), k) {
            (LevelKind::Zero, _) => write!(f, "{k}"),
            (LevelKind::Param(n), 0) => write!(f, "{n}"),
            (LevelKind::Param(n), _) => write!(f, "{n}+{k}"),
            (LevelKind::Max(a, b), 0) => write!(f, "(max {a} {b})"),
            (LevelKind::IMax(a, b), 0) => write!(f, "(imax {a} {b})"),
            (_, _) => write!(f, "({base})+{k}"),
        }
    }
}
impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Debug for LevelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LevelKind::Zero => write!(f, "Zero"),
            LevelKind::Succ(a) => write!(f, "Succ({a})"),
            LevelKind::Max(a, b) => write!(f, "Max({a}, {b})"),
            LevelKind::IMax(a, b) => write!(f, "IMax({a}, {b})"),
            LevelKind::Param(n) => write!(f, "Param({n})"),
        }
    }
}
