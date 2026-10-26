---
category: Security boundaries
---

# Linked-list length checks do not bound repeated alias work

**What went wrong.** A renderer checked each F# alignment list with
`List.length` and then converted it with `List.toArray`. Each individual list
was capped, but the same large immutable list could be reused by many tables,
causing repeated full traversals and allocations below the node and output
limits.

**Fix.** Traverse alignment metadata once per visit into a bounded builder and
charge every entry against a per-render aggregate budget before adding it.
Repeated aliases are charged again, the exact aggregate ceiling is accepted,
and the budget resets for each public render call.

**Do differently.**

- Bound aggregate work across a request, not only the size of each collection.
- For linked lists, avoid a full length pass followed by a second conversion
  pass when one bounded traversal can validate and collect.
- Add repeated-alias tests whenever immutable caller-owned metadata can be
  referenced from many otherwise-small nodes.
