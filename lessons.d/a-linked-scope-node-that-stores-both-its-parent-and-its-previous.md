---
category: Elixir
---

# A linked scope node that stores both its parent and its previous ambient node doubles its term size per level

The Elixir PBT budget tree (VER05m) first ported Ruby's `Budget` with both a
`parent` and a `previous` (the ambient value to restore on close). In memory the
two fields point at the same ancestor, but on the BEAM a term is a tree for every
operation that walks it: `==`, `Enum.uniq`, hashing, and copying the handle to
another process. Each level therefore doubled the work, so a depth-64 test never
finished (it hung, no error).

Fix: a node stores only `parent`; `close` restores the parent (a closed ambient
node resolves to its nearest open ancestor anyway).

Next time: when porting a linked structure to an immutable-term language, store
each ancestor reference once, and bound every test with an ExUnit timeout so a
blow-up fails instead of hanging.
