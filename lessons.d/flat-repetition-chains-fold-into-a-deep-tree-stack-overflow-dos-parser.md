# Flat repetition chains fold into a deep tree → stack-overflow DoS (parser depth cap does NOT catch them)

**Context:** COBOL compound conditions (`A AND B AND C AND …`). The parser's
`with_max_depth` rule-depth cap bounds *rule-reference nesting* (e.g. parenthesised
`((((…))))`), so I assumed any crafted-deep condition was already refused before
the AST was built. It is not.

**Mistake:** A grammar `{ op tail }` repetition (a flat `AND`/`OR` chain) parses as
N flat *sibling* children at CONSTANT rule-depth — the depth cap never fires; it's
bounded only by source length. I then folded those N siblings into a left-leaning
binary tree (`And(Box, And(Box, …))`) and evaluated it with recursion
(`eval_cond(l) && eval_cond(r)`). A crafted `IF A=1 AND A=1 AND … (thousands)`
recurses N frames deep → uncatchable stack-overflow `abort` (and the recursive
`Drop` of the deep `Box` tree overflows too). The security review caught it; my
"parser caps depth" reasoning was the bug.

**Fix / rule:** Represent repetition-folded operators as a **flat n-ary list**
(`And(Vec<Cond>)` / `Or(Vec<Cond>)`), collected iteratively and evaluated with a
loop (`for part in parts { … }`, short-circuiting). Then recursion depth = only the
genuinely-nested (parenthesised) structure the parser *does* cap; a flat chain is
O(1) stack. Dropping a flat `Vec` is iterative too. Cross-ref
`feedback_depth_guards_dont_compose` and `feedback_verify_dos_guards_adversarially`:
when a construct can repeat unboundedly, add an ADVERSARIAL test (here a 5000-term
chain) that would overflow the naive version, and confirm it evaluates by
iteration. The compiler was already safe because it *emitted* the fold iteratively
(a flat loop over children) rather than building a tree — mirror that shape in the
interpreter.
