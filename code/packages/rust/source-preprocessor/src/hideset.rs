//! # Hide sets — the thing that makes macro expansion terminate.
//!
//! A macro whose body mentions its own name must expand exactly once:
//!
//! ```text
//!     #define FOO  (1 + FOO)
//!     FOO          →  (1 + FOO)          and then STOPS
//! ```
//!
//! The naive rule — "don't expand a macro while expanding itself" — is wrong,
//! and wrong in a way that shows up only on inputs nobody writes by accident:
//!
//! ```text
//!     #define F(x)  G(x)
//!     #define G(x)  F(x)
//!     F(1)         →  G(1) → F(1) → …        never terminates
//! ```
//!
//! because neither macro is ever "expanding itself". The rule that works,
//! which the C standard describes and Dave Prosser's algorithm formalises, is
//! per-**token**: every token carries a set of macro names that must not be
//! expanded *for that token*, and substitution unions the macro's own name
//! into every token it produces. A token painted with `FOO` is blue forever —
//! hence the traditional name, "blue paint".
//!
//! ## Why the sets are interned
//!
//! Every token in an expansion carries a hide set. Cloning an owned set per
//! token makes expansion quadratic in *tokens × active macros* — at no extra
//! token count, so it is invisible to [`crate::bounds::Bounds::tokens_produced`]
//! and to fuel. A security review of slice 1 called this out specifically, and
//! [`crate::bounds`] requires sets to be shared.
//!
//! So a hide set is an immutable cons list in an arena, and identical sets are
//! interned to the same id:
//!
//! ```text
//!     HideId(0) = {}                      the empty set, always id 0
//!     HideId(3) = FOO :: HideId(0)        {FOO}
//!     HideId(7) = BAR :: HideId(3)        {FOO, BAR}
//!                        ▲
//!             many tokens share this id
//! ```
//!
//! Adding a name is O(1) amortised and allocates one node at most once per
//! distinct set, not once per token.

use std::collections::HashMap;

/// An interned macro name.
///
/// Comparing `u32`s rather than strings matters here: `contains` runs on every
/// token of every expansion, and the whole point of this module is that it not
/// become the expensive part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameId(pub u32);

/// A handle to an immutable set of hidden macro names.
///
/// [`HideId::EMPTY`] is always valid and always id 0, so a token that has been
/// through no expansion costs nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HideId(u32);

impl HideId {
    pub const EMPTY: HideId = HideId(0);

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// The arena of interned hide sets and macro names.
#[derive(Debug, Default)]
pub struct HideSets {
    /// `nodes[i] = (name, parent)` for `HideId(i + 1)`; `HideId(0)` is empty
    /// and has no node.
    nodes: Vec<(NameId, HideId)>,
    /// Interning table, so the same (name, parent) pair is one node.
    interned: HashMap<(NameId, HideId), HideId>,
    /// Macro-name interning.
    names: Vec<String>,
    by_name: HashMap<String, NameId>,
}

impl HideSets {
    #[must_use]
    pub fn new() -> HideSets {
        HideSets::default()
    }

    /// Intern a macro name.
    pub fn name(&mut self, text: &str) -> NameId {
        if let Some(id) = self.by_name.get(text) {
            return *id;
        }
        let id = NameId(self.names.len() as u32);
        self.names.push(text.to_string());
        self.by_name.insert(text.to_string(), id);
        id
    }

    /// The text of an interned name, for diagnostics.
    #[must_use]
    pub fn name_text(&self, id: NameId) -> &str {
        self.names.get(id.0 as usize).map_or("<unknown>", String::as_str)
    }

    /// `set ∪ {name}`, interned.
    ///
    /// Returns `set` unchanged when the name is already present, which keeps
    /// the chains short and makes repeated expansion of the same macro free
    /// rather than growing a node each time.
    pub fn insert(&mut self, set: HideId, name: NameId) -> HideId {
        if self.contains(set, name) {
            return set;
        }
        if let Some(id) = self.interned.get(&(name, set)) {
            return *id;
        }
        let id = HideId(self.nodes.len() as u32 + 1);
        self.nodes.push((name, set));
        self.interned.insert((name, set), id);
        id
    }

    /// Is `name` hidden in `set`?
    #[must_use]
    pub fn contains(&self, set: HideId, name: NameId) -> bool {
        let mut cur = set;
        while !cur.is_empty() {
            let Some(&(n, parent)) = self.nodes.get(cur.0 as usize - 1) else {
                // Unreachable for ids this arena minted; treated as "not
                // hidden" rather than panicking, because the no-panic contract
                // covers every path reachable from hostile input.
                return false;
            };
            if n == name {
                return true;
            }
            cur = parent;
        }
        false
    }

    /// Everything in `set`, innermost first.
    fn members(&self, set: HideId) -> Vec<NameId> {
        let mut out = Vec::new();
        let mut cur = set;
        while !cur.is_empty() {
            let Some(&(n, parent)) = self.nodes.get(cur.0 as usize - 1) else { break };
            out.push(n);
            cur = parent;
        }
        out
    }

    /// `a ∩ b`, interned.
    ///
    /// Needed for function-like macros, and the reason is worth stating
    /// because it looks arbitrary: the standard takes the intersection of the
    /// hide sets of the macro's *name* token and its *closing parenthesis*.
    /// The invocation spans both, so a name hidden in only one of them was not
    /// hidden across the whole invocation and must stay expandable. Using the
    /// name token's set alone over-hides and silently drops expansions.
    ///
    /// O(|a| × |b|), both bounded by the macro-depth budget.
    pub fn intersect(&mut self, a: HideId, b: HideId) -> HideId {
        if a == b {
            return a;
        }
        if a.is_empty() || b.is_empty() {
            return HideId::EMPTY;
        }
        let both: Vec<NameId> =
            self.members(a).into_iter().filter(|n| self.contains(b, *n)).collect();

        // Rebuild outermost-first so the interning table is hit for common
        // prefixes rather than producing a fresh chain per intersection.
        let mut out = HideId::EMPTY;
        for n in both.into_iter().rev() {
            out = self.insert(out, n);
        }
        out
    }

    /// Number of interned nodes — the memory story for this arena, and what
    /// makes "shared, not cloned per token" checkable rather than asserted.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_empty_set_hides_nothing_and_costs_nothing() {
        let mut h = HideSets::new();
        let foo = h.name("FOO");
        assert!(!h.contains(HideId::EMPTY, foo));
        assert_eq!(h.node_count(), 0);
    }

    #[test]
    fn a_name_is_hidden_once_inserted() {
        let mut h = HideSets::new();
        let foo = h.name("FOO");
        let bar = h.name("BAR");
        let s = h.insert(HideId::EMPTY, foo);
        assert!(h.contains(s, foo));
        assert!(!h.contains(s, bar));
    }

    #[test]
    fn identical_sets_intern_to_the_same_id() {
        // The property the memory bound rests on: two tokens that end up with
        // the same hide set must SHARE it, not each own a copy.
        let mut h = HideSets::new();
        let foo = h.name("FOO");
        let a = h.insert(HideId::EMPTY, foo);
        let b = h.insert(HideId::EMPTY, foo);
        assert_eq!(a, b);
        assert_eq!(h.node_count(), 1, "one node, not one per insertion");
    }

    #[test]
    fn reinserting_a_present_name_is_a_no_op() {
        let mut h = HideSets::new();
        let foo = h.name("FOO");
        let s = h.insert(HideId::EMPTY, foo);
        let again = h.insert(s, foo);
        assert_eq!(s, again);
        assert_eq!(h.node_count(), 1);
    }

    #[test]
    fn many_tokens_sharing_a_set_cost_one_node() {
        // 10_000 tokens in one expansion must not cost 10_000 sets. This is
        // the quadratic blow-up the bounds module warns about, which no token
        // counter can see.
        let mut h = HideSets::new();
        let foo = h.name("FOO");
        let shared = h.insert(HideId::EMPTY, foo);
        let ids: Vec<HideId> = (0..10_000).map(|_| shared).collect();
        assert!(ids.iter().all(|&i| i == shared));
        assert_eq!(h.node_count(), 1);
    }

    #[test]
    fn nested_sets_chain_rather_than_copy() {
        let mut h = HideSets::new();
        let a = h.name("A");
        let b = h.name("B");
        let sa = h.insert(HideId::EMPTY, a);
        let sab = h.insert(sa, b);
        assert!(h.contains(sab, a));
        assert!(h.contains(sab, b));
        // {A} plus {A,B} is two nodes, not three: the chain shares its tail.
        assert_eq!(h.node_count(), 2);
    }

    #[test]
    fn intersection_keeps_only_common_names() {
        let mut h = HideSets::new();
        let (a, b, c) = (h.name("A"), h.name("B"), h.name("C"));
        let ab = {
            let s = h.insert(HideId::EMPTY, a);
            h.insert(s, b)
        };
        let bc = {
            let s = h.insert(HideId::EMPTY, b);
            h.insert(s, c)
        };
        let i = h.intersect(ab, bc);
        assert!(!h.contains(i, a));
        assert!(h.contains(i, b), "B is in both and must survive");
        assert!(!h.contains(i, c));
    }

    #[test]
    fn intersection_with_empty_is_empty_and_with_self_is_self() {
        let mut h = HideSets::new();
        let a = h.name("A");
        let sa = h.insert(HideId::EMPTY, a);
        assert_eq!(h.intersect(sa, HideId::EMPTY), HideId::EMPTY);
        assert_eq!(h.intersect(HideId::EMPTY, sa), HideId::EMPTY);
        assert_eq!(h.intersect(sa, sa), sa);
    }

    #[test]
    fn intersection_is_commutative() {
        let mut h = HideSets::new();
        let (a, b, c) = (h.name("A"), h.name("B"), h.name("C"));
        let ab = { let s = h.insert(HideId::EMPTY, a); h.insert(s, b) };
        let bc = { let s = h.insert(HideId::EMPTY, b); h.insert(s, c) };
        assert_eq!(h.intersect(ab, bc), h.intersect(bc, ab));
    }

    #[test]
    fn names_intern_and_round_trip() {
        let mut h = HideSets::new();
        let foo = h.name("FOO");
        assert_eq!(h.name("FOO"), foo, "the same text must intern to one id");
        assert_ne!(h.name("BAR"), foo);
        assert_eq!(h.name_text(foo), "FOO");
    }
}
