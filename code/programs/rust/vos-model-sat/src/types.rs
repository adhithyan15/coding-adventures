//! # The encoder's value types
//!
//! The `sched` contract model (`warrant/contract.part`) talks about five kinds of value.  This
//! module gives each one a representation made of AIG edges, so that a model function becomes a
//! circuit.
//!
//! ```text
//!   Warrant type             Encoding here            Width in AIG edges
//!   ----------------------   ----------------------   -------------------
//!   Bool                     Edge                     1
//!   U8  (a thread id)        BitVec of width 8        8
//!   Ready (the ready set)    Ready(BitVec) width N    N        <- the ONLY width that is N
//!   Upd = Changed
//!       | Unchanged
//!       | OutOfRange         UpdEnc, one-hot          3
//!   Pick = Idle | Run(U8)    PickEnc { idle, id }     1 + 8
//! ```
//!
//! ## Why a `Pick` is `{ idle, id }` with an 8-bit id at every width
//!
//! A thread id is a `U8`, the idle sentinel of `pick_of` is 255, and N is at most 64 (see
//! [`crate::MAX_N`]), so `64 < 255` and an id never needs more than 8 bits whatever N is
//! (VOS31 section 4; section 16 item 2, which this crate settles by reading `pick_of`'s input
//! `k: U8` in `contract.part`).
//!
//! ## Idle has no id
//!
//! In Warrant `Idle` carries no payload, so two `Idle` values are equal whatever bits sit in the
//! `id` field of the encoding.  [`crate::SchedModel::pick_eq`] therefore compares the `idle` flags
//! first and the ids only when both are `Run`; and [`Out::read`] reads an idle pick as
//! [`Val::Pick`]`(None)`, so a don't-care id can never leak into a comparison.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv::BitVec;

/// The ready set `M`: bit `i` of the vector is "thread `i` is ready".  Width N.
///
/// In Warrant this is a record of eight `Bool` fields (`Ready(p0, ..., p7)`), so structural
/// equality is set equality.  Here it is N edges and equality is [`crate::SchedModel::ready_eq`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ready(pub BitVec);

impl Ready {
    /// The width N.
    pub fn width(&self) -> usize {
        self.0.width()
    }

    /// Thread `i`'s membership bit (a constant index, known at encoding time).
    pub fn bit(&self, i: usize) -> Edge {
        self.0.bits[i]
    }

    /// `n` fresh inputs, thread 0 first.
    pub fn input(aig: &mut Aig, n: usize) -> Ready {
        Ready(BitVec::input(aig, n))
    }
}

/// `Upd = Changed | Unchanged | OutOfRange`, one-hot: exactly one of the three edges is true on
/// every input of a well-formed circuit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdEnc {
    /// The value is `Changed`.
    pub changed: Edge,
    /// The value is `Unchanged`.
    pub unchanged: Edge,
    /// The value is `OutOfRange`.
    pub out_of_range: Edge,
}

/// `Pick = Idle | Run(t: U8)`: the dispatcher's choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickEnc {
    /// The value is `Idle` (and then `id` means nothing).
    pub idle: Edge,
    /// The thread id of `Run(id)`.  8 bits at every width.
    pub id: BitVec,
}

/// A `(Upd, Ready)` pair: the result of `abs_insert` and `abs_remove`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The `.0` of the Warrant pair.
    pub upd: UpdEnc,
    /// The `.1` of the Warrant pair.
    pub ready: Ready,
}

/// Any value a model function can return, as edges.  The cross-checks read these back as
/// concrete [`Val`]s, case by case.
#[derive(Clone, Debug)]
pub enum Out {
    /// A `Bool`.
    Bool(Edge),
    /// A `Ready`.
    Ready(Ready),
    /// An `Upd`.
    Upd(UpdEnc),
    /// A `Pick`.
    Pick(PickEnc),
}

/// A concrete value, the common currency between the encoder and the Warrant evaluator: both
/// are converted to this type and compared with `==`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Val {
    /// A `Bool`.
    Bool(bool),
    /// A `Ready`: bit `i` is thread `i` (only the low N bits are ever set).
    Ready(u64),
    /// An `Upd`: 0 `Changed`, 1 `Unchanged`, 2 `OutOfRange` (the declaration order).
    Upd(u8),
    /// A `Pick`: `None` is `Idle`, `Some(t)` is `Run(t)`.
    Pick(Option<u8>),
}

/// Read bit-vector `bv` from a case: LSB first, at most 64 bits.
fn word(bv: &[Edge], value: &dyn Fn(Edge) -> bool) -> u64 {
    bv.iter()
        .enumerate()
        .fold(0u64, |w, (i, &e)| w | (u64::from(value(e)) << i))
}

/// The `Upd` number of a one-hot encoding under `value`.
fn upd_number(u: &UpdEnc, value: &dyn Fn(Edge) -> bool) -> u8 {
    if value(u.changed) {
        0
    } else if value(u.unchanged) {
        1
    } else {
        2
    }
}

impl Out {
    /// The concrete value of this output on one case, `value` giving each edge's truth.
    pub fn read(&self, value: &dyn Fn(Edge) -> bool) -> Val {
        match self {
            Out::Bool(e) => Val::Bool(value(*e)),
            Out::Ready(r) => Val::Ready(word(&r.0.bits, value)),
            Out::Upd(u) => Val::Upd(upd_number(u, value)),
            Out::Pick(p) => {
                if value(p.idle) {
                    Val::Pick(None)
                } else {
                    Val::Pick(Some(word(&p.id.bits, value) as u8))
                }
            }
        }
    }
}
