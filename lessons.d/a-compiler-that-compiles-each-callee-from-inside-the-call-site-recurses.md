---
category: Compiler / VM / language pipeline
---

# A compiler that compiles each callee from inside the call site recurses through the whole call chain, so compile from a work list

**What went wrong.** The v2 runner's closure compiler compiled a `model_call` by compiling the callee's body *inside* the compile of the call site. A chain of 64 models (the spec's frame limit) whose bodies are 62 nodes deep (the spec's depth limit) then nested 64 compilers, each recursing over its body, and a spec-legal corpus died with `RangeError: Maximum call stack size exceeded` while 50 x 62 and 64 x 50 still worked. The limits are each checked on their own, so nothing flagged the product.

**The fix.** `compile` hands out a *cell* for the callee (`{ run }`) and puts the compile of its body on a work list that is drained after the current expression is compiled. The call-site closure calls `cell.run` at evaluation time. Compilation depth is then bounded by one body, whatever the chain, and a callee is compiled once per tainted-context flavour.

**What to do differently.** When two independent limits multiply (call-chain depth times expression depth), test the product, not each factor. A compiler that follows references while it compiles should queue them rather than recurse.
