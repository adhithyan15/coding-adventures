---
category: Python
---

# A generic IR walker that reads node[1:] as the children silently drops the operands of a tuple literal and treats a match pattern as an expression

The warrant oracle had a dozen hand-written IR walkers (call counting, the wrap trace, the admit check,
`requires`, `free_vars`, the lowerer's `walk`), each looping `stack.extend(node[1:])` and skipping
anything that was not a list starting with a tag string. That is correct for `["add", a, b]` and wrong
for the two `ir.tuple` shapes: `["tuple", [e1, e2]]` keeps its operands in *one list* at position 1, so a
walker popped a list of nodes, saw that its first element was not a tag string, and silently skipped every
subject call inside a tuple literal; and `["match", s, [[pattern, arm], ..]]` contains patterns such as
`["ok", ["bind", "x"]]` and `["tuple", [..]]` that have exactly the shape of the `ok` and `tuple`
expressions, so a walker either missed the arms or visited a pattern as an expression. Nothing failed: the
counts were simply too small.

Fix: one function, `irtree.children(node)`, that says which parts of a node are sub-expressions (and
never yields a pattern), used by every walker; a walker that must *compile* a sub-expression (to learn a
type) uses `ir_eval.walk_scoped`, which extends the scope with the arm's pattern binders. When a new IR
shape is added, grep for `node[1:]` first: every hit is a walker that must learn the shape, and a walker
that skips what it does not recognise fails open.
