---
category: Compiler / VM / language pipeline
---

# Check flat operator spines before building a recursively dropped typed AST

The native JavaScript division repair passed ordinary tests and Node oracles.
A 5,000-term division chain within the source cap then aborted with a Windows
stack overflow. The grammar repetition was shallow, but the typed AST bridge
left-folded it into thousands of nested boxes. The compiler's iterative depth
check rejected the AST; recursive destruction of that rejected AST still
overflowed before returning its diagnostic.

Check prospective operator/member/call spines on the grammar tree before the
typed bridge allocates them. Grammar depth and source length alone do not bound
the depth of a tree constructed from a flat repetition. Exercise hostile input
through a child process to distinguish a recoverable error from process abort,
and include the repeated-operator path in the focused regression suite.

A broader child-process probe then exposed a different stage: 200 `**`
operands overflowed during ASI parsing, before the grammar-tree check. The
independent security reviewer reproduced this failure. Exponentiation is
outside this native numeric subset, so reject its actual operator tokens before
parsing. A post-parse guard cannot protect recursion that happens during parsing;
probe unsupported recursive operators too, and identify the actual failing stage.
