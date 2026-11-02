---
category: Compiler / VM / language pipeline
---

# A comparison's type hint is its operand type, so a ref<any> hint asks the native backends for a reference compare they refuse; compare tagged words as i64

**What went wrong.** LANG83 PR7f-2's `__lisp_equal` helper compares two
atoms as words. The first draft wrote `cmp_eq a, b` with the type hint
`ref<any>`, because `a` and `b` are tagged values. The LLVM column accepted
it (`llvm_type_for("ref<any>")` is `i64`), and the helper's unit tests ran it
on a reference machine, so both passed. The native x86-64 column refused the
whole function: `backend refused function '__lisp_equal' (untyped or
unsupported op)`. In IIR a comparison's `type_hint` names its *operand* type
(iir-to-llvm's `lower_cmp` says so), and the native specialiser has no
reference comparison.

**Fix.** The words are compared with an `i64` hint. Parameters and pair
fields stay `ref<any>`, so the precise collector still sees them as roots;
only the comparison says "these are 64-bit words".

**Do differently.** Give a comparison the type its operands must be
compared as, not the type of the values they came from. Run a new helper
end to end on every column that will lower it — here LLVM *and* native —
not only on the backend whose type mapping is most forgiving.
