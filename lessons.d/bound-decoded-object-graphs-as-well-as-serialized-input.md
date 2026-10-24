---
category: Security boundaries
---

# Bound decoded object graphs as well as serialized input

The portable GFM validator capped file bytes and JSON depth, but its public
`validate_document(Any)` path accepted already-decoded objects. A shallow DAG
with the same list referenced many times made the preflight traversal expand
exponentially even though only a few containers existed. JSON cannot encode
aliases, so transport-focused tests missed the direct-call boundary.

For any validator that accepts both serialized and decoded inputs, bound both
paths explicitly. Reject cycles and shared container identities, cap total
scheduled items before adding children, and cap aggregate string bytes. Include
a regression built from a tiny, highly aliased DAG; nesting-depth tests alone
do not cover breadth amplification.
