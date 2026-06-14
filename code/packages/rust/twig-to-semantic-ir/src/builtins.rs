//! The v0 Twig builtin table.
//!
//! Matches TW00's surface plus `global_get` / `global_set` (used by
//! the synthesised `_init` and value-global access).  Each entry
//! lists the builtin name, its arity (`None` = variadic), and its
//! pre-tabulated [`EffectSet`].
//!
//! Lowering uses [`is_builtin`] for call-site dispatch and
//! [`effects_for`] to populate the emitted `BuiltinCall` node.

use semantic_ir::{Effect, EffectSet};

/// One entry in the builtin table.
///
/// `arity` is part of the public surface for future arity-checking
/// in the lowerer; v0 lowering doesn't yet enforce arity (the Twig
/// parser already validates argument *counts* against the grammar
/// where applicable, and runtime errors surface mismatches for the
/// rest).  The field is preserved so callers can inspect it.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct BuiltinSig {
    pub name: &'static str,
    /// `None` means variadic.  Twig's `+`, `-`, `*`, `/` accept ≥ 1
    /// argument.
    pub arity: Option<usize>,
    pub effects: EffectSet,
}

/// The full v0 builtin table, in source-order from TW00.
///
/// We expose this as a fn returning an array rather than a const
/// because `EffectSet::with` is not const yet.
pub fn table() -> [BuiltinSig; 17] {
    [
        BuiltinSig { name: "+", arity: None, effects: EffectSet::PURE },
        BuiltinSig { name: "-", arity: None, effects: EffectSet::PURE },
        BuiltinSig { name: "*", arity: None, effects: EffectSet::PURE },
        BuiltinSig { name: "/", arity: None, effects: EffectSet::PURE },
        BuiltinSig { name: "=", arity: Some(2), effects: EffectSet::PURE },
        BuiltinSig { name: "<", arity: Some(2), effects: EffectSet::PURE },
        BuiltinSig { name: ">", arity: Some(2), effects: EffectSet::PURE },
        BuiltinSig { name: "cons", arity: Some(2), effects: EffectSet::PURE.with(Effect::MayAllocate) },
        BuiltinSig { name: "car", arity: Some(1), effects: EffectSet::PURE },
        BuiltinSig { name: "cdr", arity: Some(1), effects: EffectSet::PURE },
        BuiltinSig { name: "null?", arity: Some(1), effects: EffectSet::PURE },
        BuiltinSig { name: "pair?", arity: Some(1), effects: EffectSet::PURE },
        BuiltinSig { name: "number?", arity: Some(1), effects: EffectSet::PURE },
        BuiltinSig { name: "symbol?", arity: Some(1), effects: EffectSet::PURE },
        BuiltinSig { name: "print", arity: Some(1), effects: EffectSet::PURE.with(Effect::MayPrint) },
        BuiltinSig { name: "global_get", arity: Some(1), effects: EffectSet::PURE },
        BuiltinSig { name: "global_set", arity: Some(2), effects: EffectSet::PURE },
    ]
}

/// Look up a builtin by name.
pub fn lookup(name: &str) -> Option<BuiltinSig> {
    table().into_iter().find(|b| b.name == name)
}

/// `true` iff the name is in the v0 builtin table.
pub fn is_builtin(name: &str) -> bool {
    lookup(name).is_some()
}

/// Effects for `name`, or `EffectSet::PURE` if unknown (callers
/// should have already verified the name is a builtin).
pub fn effects_for(name: &str) -> EffectSet {
    lookup(name).map(|b| b.effects).unwrap_or(EffectSet::PURE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_unique() {
        let mut seen = std::collections::HashSet::new();
        for b in table() {
            assert!(seen.insert(b.name), "duplicate {}", b.name);
        }
    }

    #[test]
    fn plus_is_pure() {
        assert!(effects_for("+").is_pure());
    }

    #[test]
    fn print_may_print() {
        assert!(effects_for("print").contains(Effect::MayPrint));
    }

    #[test]
    fn cons_may_allocate() {
        assert!(effects_for("cons").contains(Effect::MayAllocate));
    }

    #[test]
    fn unknown_name_is_not_builtin() {
        assert!(!is_builtin("hello"));
        assert!(lookup("hello").is_none());
    }
}
