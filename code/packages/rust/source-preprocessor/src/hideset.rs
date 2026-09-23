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
//! Adding a name allocates one node at most once per distinct set, not once
//! per token. Membership is O(1) for a name that is absent — the common case,
//! via a 64-bit Bloom summary carried on each node — and O(chain) only when
//! the summary says the name may be present.
//!
//! The summary matters as much as the interning. Interning removed the
//! quadratic in memory; without the filter, `contains` walking the chain on
//! every token left expansion quadratic in *time*, with fuel and round
//! counters both linear and therefore blind to it.

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
    /// `nodes[i] = (name, parent, filter)` for `HideId(i + 1)`; `HideId(0)` is
    /// empty and has no node.
    ///
    /// `filter` is a 64-bit Bloom summary of every name on the chain from this
    /// node to the root: `parent.filter | bit(name)`. It makes the common case
    /// -- "this name is NOT hidden" -- a single AND rather than a walk.
    ///
    /// Without it `contains` walked the whole chain on every token, so
    /// expansion stayed quadratic in *time* even though interning had removed
    /// the quadratic in *memory*. A security review measured 838 KB of source
    /// at 1.59 s, quadrupling per doubling, with fuel and rounds both linear --
    /// "invisible to every counter", which is the exact phrase the spec uses
    /// to justify requiring shared hide sets in the first place. The spec's
    /// claim was about memory; the time cost survived it.
    ///
    /// A Bloom filter and not an exact set because exactness is what interning
    /// buys us: a false positive costs one chain walk (correct, just slower),
    /// and a false negative is impossible, so `contains` stays exact.
    nodes: Vec<(NameId, HideId, u64, u32)>,
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
        let filter = self.filter_of(set) | Self::bit(name);
        let depth = self.depth_of(set).saturating_add(1);
        self.nodes.push((name, set, filter, depth));
        self.interned.insert((name, set), id);
        id
    }

    /// One bit per name, folded into the 64-bit summary.
    fn bit(name: NameId) -> u64 {
        // Multiplicative hash then take six bits. Cheap and well spread enough
        // for a summary whose only job is to answer "definitely not present".
        1u64 << ((name.0.wrapping_mul(0x9E37_79B9) >> 26) & 63)
    }

    /// How many distinct names are painted on `set`.
    ///
    /// Exposed because it is a *bounded* quantity the engine has to check:
    /// membership walks this chain, so an unbounded chain is an unbounded
    /// per-token cost that no token or fuel counter can see.
    #[must_use]
    pub fn depth_of(&self, set: HideId) -> u32 {
        if set.is_empty() {
            0
        } else {
            self.nodes.get(set.0 as usize - 1).map_or(0, |n| n.3)
        }
    }

    fn filter_of(&self, set: HideId) -> u64 {
        if set.is_empty() {
            0
        } else {
            self.nodes.get(set.0 as usize - 1).map_or(0, |n| n.2)
        }
    }

    /// Is `name` hidden in `set`?
    ///
    /// Exact. The Bloom summary only short-circuits the negative answer; a
    /// positive summary still walks the chain to confirm, so a false positive
    /// costs time and never correctness.
    #[must_use]
    pub fn contains(&self, set: HideId, name: NameId) -> bool {
        if self.filter_of(set) & Self::bit(name) == 0 {
            return false;
        }
        let mut cur = set;
        while !cur.is_empty() {
            let Some(&(n, parent, _, _)) = self.nodes.get(cur.0 as usize - 1) else {
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
            let Some(&(n, parent, _, _)) = self.nodes.get(cur.0 as usize - 1) else { break };
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
    /// O(|a| × |b|). Chain length is bounded by the number of DISTINCT macro
    /// names painted along a path — bounded by
    /// `Bounds::hide_set_depth`, which exists precisely because walking this
    /// chain per token is what made expansion quadratic in time.
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
    fn the_summary_never_produces_a_false_negative() {
        // The correctness invariant the whole optimisation rests on.
        //
        // `contains` returns early when the Bloom summary says a name is
        // absent. A false POSITIVE costs one chain walk and stays exact. A
        // false NEGATIVE would be catastrophic and silent: it would drop the
        // blue-paint rule for that token, so a self-referential macro would
        // expand again, and again — the infinite expansion hide sets exist to
        // prevent, reintroduced as a performance optimisation.
        //
        // It cannot happen by construction (`filter = filter_of(parent) |
        // bit(name)`, so every name on a chain has its bit set), but the
        // construction is one line in `insert` and this is what makes breaking
        // it loud. 2,000 names on one chain is far past the 64-bit summary's
        // saturation point, which is exactly where a naive filter would fail.
        let mut h = HideSets::new();
        let names: Vec<NameId> = (0..2_000).map(|i| h.name(&format!("M{i}"))).collect();

        let mut set = HideId::EMPTY;
        for (i, n) in names.iter().enumerate() {
            set = h.insert(set, *n);
            // Everything inserted so far must still be reported as present.
            for earlier in &names[..=i] {
                assert!(
                    h.contains(set, *earlier),
                    "false negative after {} insertions — blue paint would be dropped",
                    i + 1
                );
            }
            if i > 40 {
                break; // O(n^2) check; past saturation is the interesting part
            }
        }

        // And a name never inserted is still absent, so the filter has not
        // simply been made to say "yes" to everything.
        let absent = h.name("NEVER_INSERTED");
        assert!(!h.contains(set, absent));
    }

    #[test]
    fn a_colliding_summary_bit_stays_exact() {
        // Two names sharing a summary bit is expected — six bits of hash over
        // an unbounded name space. The consequence must be a wasted walk, not
        // a wrong answer, and an attacker picks the names.
        let mut h = HideSets::new();
        let mut a = None;
        let mut b = None;
        // Find a genuine collision rather than assuming one exists.
        let ids: Vec<NameId> = (0..500).map(|i| h.name(&format!("N{i}"))).collect();
        'outer: for (i, x) in ids.iter().enumerate() {
            for y in &ids[i + 1..] {
                if HideSets::bit(*x) == HideSets::bit(*y) {
                    a = Some(*x);
                    b = Some(*y);
                    break 'outer;
                }
            }
        }
        let (a, b) = (a.expect("a collision must exist in 500 names"), b.unwrap());

        let set = h.insert(HideId::EMPTY, a);
        assert!(h.contains(set, a), "the inserted name is present");
        assert!(
            !h.contains(set, b),
            "a name that merely COLLIDES in the summary must not be reported present"
        );
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
