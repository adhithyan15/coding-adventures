//! Effect tags carried on call nodes.
//!
//! Every call node (`DirectCall`, `IndirectCall`, `BuiltinCall`,
//! `Intrinsic`) and every `Function` carries an [`EffectSet`].
//! Frontends annotate; backends may use the information or ignore
//! it.  v0 effects are coarse-grained and orthogonal:
//!
//! | Effect       | Meaning                                       |
//! |--------------|-----------------------------------------------|
//! | `MayThrow`   | the call may raise an exception                |
//! | `MayPrint`   | the call writes to stdout/stderr               |
//! | `MayAllocate`| the call allocates heap memory                 |
//! | `MayBlock`   | the call may block on I/O                      |
//! | `Divergent`  | the call may not terminate                     |
//!
//! `Pure` is the absence of effects (empty bitset), not a flag.
//! It is the natural identity for set composition.

use std::fmt;

/// A bitset of effect tags.  v0 has 5 distinct effects; storing them
/// in a `u8` leaves plenty of room for future versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EffectSet(u8);

/// Individual effect tags.  Use [`EffectSet::with`] to compose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Effect {
    MayThrow = 1 << 0,
    MayPrint = 1 << 1,
    MayAllocate = 1 << 2,
    MayBlock = 1 << 3,
    Divergent = 1 << 4,
}

impl EffectSet {
    /// The empty set — synonymous with "pure".
    pub const PURE: EffectSet = EffectSet(0);

    /// Construct from a list of effects (any ordering, dedup is free).
    pub fn from_effects(effects: &[Effect]) -> Self {
        let mut s = 0u8;
        for e in effects {
            s |= *e as u8;
        }
        EffectSet(s)
    }

    /// Add one effect to the set (chainable).
    pub fn with(mut self, effect: Effect) -> Self {
        self.0 |= effect as u8;
        self
    }

    /// True iff the set contains `effect`.
    pub fn contains(&self, effect: Effect) -> bool {
        (self.0 & effect as u8) != 0
    }

    /// True iff the set is empty (i.e. the call is pure).
    pub fn is_pure(&self) -> bool {
        self.0 == 0
    }

    /// Union of two effect sets.
    pub fn union(&self, other: &EffectSet) -> EffectSet {
        EffectSet(self.0 | other.0)
    }

    /// Iterate the effects present in the set.  Order is the
    /// declaration order of the [`Effect`] enum.
    pub fn iter(&self) -> impl Iterator<Item = Effect> + '_ {
        const ALL: [Effect; 5] = [
            Effect::MayThrow,
            Effect::MayPrint,
            Effect::MayAllocate,
            Effect::MayBlock,
            Effect::Divergent,
        ];
        ALL.into_iter().filter(move |e| self.contains(*e))
    }
}

impl fmt::Display for EffectSet {
    /// Pure is rendered as `pure`; non-empty sets are
    /// space-separated tag names — matching the SIR text grammar.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_pure() {
            return write!(f, "pure");
        }
        let mut first = true;
        for e in self.iter() {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            let name = match e {
                Effect::MayThrow => "may-throw",
                Effect::MayPrint => "may-print",
                Effect::MayAllocate => "may-allocate",
                Effect::MayBlock => "may-block",
                Effect::Divergent => "divergent",
            };
            write!(f, "{}", name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_is_empty() {
        assert!(EffectSet::PURE.is_pure());
        assert_eq!(format!("{}", EffectSet::PURE), "pure");
    }

    #[test]
    fn with_adds_effect() {
        let s = EffectSet::PURE.with(Effect::MayPrint);
        assert!(s.contains(Effect::MayPrint));
        assert!(!s.contains(Effect::MayThrow));
    }

    #[test]
    fn from_effects_constructs() {
        let s = EffectSet::from_effects(&[Effect::MayPrint, Effect::MayAllocate]);
        assert!(s.contains(Effect::MayPrint));
        assert!(s.contains(Effect::MayAllocate));
        assert!(!s.contains(Effect::MayBlock));
    }

    #[test]
    fn duplicate_effects_idempotent() {
        let s = EffectSet::PURE
            .with(Effect::MayPrint)
            .with(Effect::MayPrint);
        // Iter should still show MayPrint exactly once.
        let v: Vec<_> = s.iter().collect();
        assert_eq!(v, vec![Effect::MayPrint]);
    }

    #[test]
    fn union_combines() {
        let a = EffectSet::PURE.with(Effect::MayPrint);
        let b = EffectSet::PURE.with(Effect::MayAllocate);
        let c = a.union(&b);
        assert!(c.contains(Effect::MayPrint));
        assert!(c.contains(Effect::MayAllocate));
    }

    #[test]
    fn display_multiple_effects() {
        let s = EffectSet::from_effects(&[Effect::MayThrow, Effect::Divergent]);
        // Display orders effects by the Effect enum declaration order.
        assert_eq!(format!("{}", s), "may-throw divergent");
    }

    #[test]
    fn iter_yields_effects() {
        let s = EffectSet::from_effects(&[
            Effect::MayPrint,
            Effect::MayBlock,
        ]);
        let v: Vec<_> = s.iter().collect();
        assert_eq!(v, vec![Effect::MayPrint, Effect::MayBlock]);
    }
}
