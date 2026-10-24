---
category: Testing & coverage
---

# A path-construction terminal predicate that accepts the target produces no adjacent issuer link

A negative constructed-adjacency test used `|_| true` as its terminal
predicate. The constructor therefore accepted the target itself as terminal,
selected no issuer, and correctly returned a path whose `adjacent_link(0)` was
`None`; the test then panicked before reaching the policy boundary it meant to
exercise. When a fixture needs one issuer link, make the terminal predicate
identify only the intended candidate certificate, even if the test's real
focus is a later cryptographic failure. Assert the issuer count or projected
link before relying on the fixture in a negative composition test.
