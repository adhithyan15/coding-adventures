---
category: Compiler / VM / language pipeline
---

# A WIR checker that threads one environment through the block vector accepts reads no path can satisfy; check definite assignment along control flow

`warrant-ir`'s `check.rs` walked `func.blocks` in vector order with one growing environment. Vector order is storage order, not control-flow order, so a read that a path reached with the variable unbound passed (block 0 jumps to 2, block 1 binds `x`, block 2 reads `x`). It also wrongly rejected sound programs whose blocks were stored out of flow order. VER25's totality guarantee (section 8.5) rests on this check, so the hole was a semantic one, not cosmetic (KRN-006b).

The fix is a worklist fixpoint over the control-flow graph: IN(entry) is the parameters, IN(b) is the intersection of the predecessors' OUT sets (keeping the type and joining the label), and errors are reported only in a second pass once every IN is final. Reporting during the iteration gives false errors, because an early IN is a superset of the final one.

Do differently:
- Any rule of the form "X is available here" in an IR with branches must be a property of paths (dataflow or dominance), never of the order a container lists blocks in.
- Write the counterexample first and see it fail. Cover a join where both arms bind, and a loop that binds only in its body.
- Check structural assumptions the interpreter relies on (entry block id 0, ids exactly 0..n) in the checker, so the "checked" guarantee covers what the interpreter assumes.
- Run real lowered output through any new checker rule. The same exercise showed that `check_function` already rejects the lowering of every ct-compare function that has a tail expression (`TypeMismatch`, unit return against a typed one): the full checker had never been run on lowered code, so adopter tests must ask the specific question (`check_definite_assignment`) until that is fixed.
