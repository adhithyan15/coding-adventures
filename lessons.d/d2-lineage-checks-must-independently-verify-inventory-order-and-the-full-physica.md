---
category: Testing & coverage
---

# D2 lineage checks must independently verify inventory order and the full physical census

The initial D2 graph tests selected exact operand IDs by raw spans but their
inventory helper checked only availability and uniqueness. Reversing piece_ids
left those operand queries unchanged. A path-only guard against old parser
origins also missed unused namespace-free lexer rows with another origin name.
Independent test review identified this before any D2 green acceptance.

Retain the compiled initial semantic red. Strengthen the oracle with exact
piece/token ordinals, closed descriptor/counts, contiguous raw coordinates and
one terminal EOF, plus an independent whole-graph legacy/reserved physical
census against all root inventories. Add synthetic reversed-list and extra
namespace-free-token negatives. Earlier D1 tests remain; do not infer that
passing those makes a successor's weaker oracle complete.
