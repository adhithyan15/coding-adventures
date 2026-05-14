//! Constant-propagation map for the refinement pass.
//!
//! # What we compute
//!
//! For each function we make a single forward scan of the instruction list and
//! build a mapping:
//!
//! ```text
//! ConstMap : HashMap<String, i128>
//! ```
//!
//! A variable name `v` is entered in the map if and only if **exactly one**
//! `const` instruction in the function writes an integer literal to `dest = v`.
//!
//! If a second `const` writes to the same destination we **evict** the entry
//! (conservative: we no longer know the compile-time value).  We also skip any
//! `const` whose source is not `Operand::Int`.
//!
//! # Why single-pass is enough
//!
//! The IIR produced by the Twig compiler is in SSA-style: each virtual register
//! is written at most once in normal code.  The eviction rule is a safety net
//! for hand-crafted test modules and potential future passes that do inline
//! substitution.
//!
//! # Example
//!
//! ```text
//! const  arg0 = 500      ; Operand::Int(500) → ConstMap["arg0"] = 500
//! call   callee arg0     ; call_checker can now use Evidence::Concrete(500)
//! ```

use std::collections::HashMap;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::Operand;

/// A compile-time integer-constant map for a single function.
///
/// Built by [`build_const_map`].  The checker uses it to turn
/// `Operand::Var(name)` into `Evidence::Concrete(v)` when the variable is
/// provably a constant integer.
pub type ConstMap = HashMap<String, i128>;

/// Scan `func`'s instruction list once and return every variable that is
/// unconditionally assigned a single compile-time integer literal.
///
/// # Algorithm
///
/// 1. Walk each instruction in definition order.
/// 2. When `op == "const"`, `dest == Some(name)`, and `srcs[0] == Operand::Int(v)`:
///    - First occurrence → insert `name → v as i128`.
///    - Second (or later) occurrence → evict `name` (conservative: treat as
///      unconstrained so we never claim a wrong value).
/// 3. All other instructions are ignored.
///
/// The result is a stable, read-only snapshot; the caller uses it while
/// checking call sites and return sites in the same function.
pub fn build_const_map(func: &IIRFunction) -> ConstMap {
    // Track which names we have already seen (for eviction on duplicates).
    let mut seen: HashMap<String, bool> = HashMap::new(); // true = evicted
    let mut map: ConstMap = HashMap::new();

    for instr in &func.instructions {
        // Only `const` instructions contribute to the literal map.
        if instr.op != "const" {
            continue;
        }

        // The destination register must be named.
        let name = match &instr.dest {
            Some(n) => n.clone(),
            None => continue,
        };

        // The source must be an integer literal.
        let value: i128 = match instr.srcs.first() {
            Some(Operand::Int(v)) => *v as i128,
            _ => continue,
        };

        // Eviction rule: a second const to the same dest removes the entry.
        match seen.get(&name) {
            None => {
                // First time: record and enter into map.
                seen.insert(name.clone(), false);
                map.insert(name, value);
            }
            Some(false) => {
                // Second time: evict — mark as seen-evicted, remove from map.
                seen.insert(name.clone(), true);
                map.remove(&name);
            }
            Some(true) => {
                // Already evicted; nothing to do.
            }
        }
    }

    map
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    /// Helper: build a minimal function with the given instructions.
    fn func_with_instrs(instrs: Vec<IIRInstr>) -> IIRFunction {
        IIRFunction {
            name: "test".into(),
            instructions: instrs,
            ..Default::default()
        }
    }

    #[test]
    fn single_int_const_is_captured() {
        // A single `const arg0 = 42` should appear in the map.
        let f = func_with_instrs(vec![
            IIRInstr::new("const", Some("arg0".into()), vec![Operand::Int(42)], "i64"),
        ]);
        let map = build_const_map(&f);
        assert_eq!(map.get("arg0"), Some(&42i128));
    }

    #[test]
    fn two_consts_same_dest_evicts() {
        // Writing twice to the same dest → evict; map should be empty.
        let f = func_with_instrs(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("const", Some("v".into()), vec![Operand::Int(2)], "i64"),
        ]);
        let map = build_const_map(&f);
        assert!(!map.contains_key("v"), "eviction should have removed 'v'");
    }

    #[test]
    fn non_int_const_is_skipped() {
        // A `const` with a Bool source should not appear in the map.
        let f = func_with_instrs(vec![
            IIRInstr::new("const", Some("b".into()), vec![Operand::Bool(true)], "bool"),
        ]);
        let map = build_const_map(&f);
        assert!(!map.contains_key("b"));
    }

    #[test]
    fn non_const_instructions_are_ignored() {
        // An `add` instruction should not contribute anything to the map.
        let f = func_with_instrs(vec![
            IIRInstr::new("add", Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
        ]);
        let map = build_const_map(&f);
        assert!(map.is_empty());
    }

    #[test]
    fn multiple_distinct_consts() {
        // Multiple consts with different dests all appear in the map.
        let f = func_with_instrs(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(10)], "i64"),
            IIRInstr::new("const", Some("y".into()), vec![Operand::Int(20)], "i64"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(30)], "i64"),
        ]);
        let map = build_const_map(&f);
        assert_eq!(map.get("x"), Some(&10i128));
        assert_eq!(map.get("y"), Some(&20i128));
        assert_eq!(map.get("z"), Some(&30i128));
    }

    #[test]
    fn negative_int_const() {
        // Negative literals must also be stored faithfully.
        let f = func_with_instrs(vec![
            IIRInstr::new("const", Some("neg".into()), vec![Operand::Int(-99)], "i64"),
        ]);
        let map = build_const_map(&f);
        assert_eq!(map.get("neg"), Some(&(-99i128)));
    }
}
