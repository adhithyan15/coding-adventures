---
category: Testing & coverage
---

# An A2 chapter placed on a B1 spine node makes the track reach B1

Punjabi chapter 181 described itself as an A2 mock prerequisite and gave its
inline extension `stage: A2`, but its chapter, path segment, and eight lessons
pointed at `SPINE-HANDLE-TRAVEL`. That spine node is B1. The focused chapter
test repeated the same wrong node, so every focused check passed while the
repo-wide level summary correctly moved Punjabi from A2 reach to B1.

The fix was to place the bounded practical travel note on the same A2
`SPINE-READ-PRACTICAL-TEXTS` node as the three prerequisite chapters before it,
then pin that node in the focused test. The chapter teaches vocabulary for
reading and producing one short practical note; it does not claim the B1
ability to handle most situations while travelling.

Do differently: before selecting a spine node from its English label, inspect
the canonical spine shard and its `stage`. For a chapter with an explicit
level claim, assert both the extension stage and every lesson's canonical
spine node, then run the repo-wide level summary. A self-consistent chapter
test can preserve the wrong level just as effectively as it preserves the
right one.
