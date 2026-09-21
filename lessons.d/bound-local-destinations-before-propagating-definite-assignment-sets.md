---
category: Security boundaries
---

# Bound local destinations before propagating definite assignment sets

A CFG prepass that clones assignment sets before checking the local budget can
turn a rejected module into quadratic hashing work. Security review caught this
in strict CLR forward flow: preflight destination count before graph allocation
and propagation so each assignment set is bounded by parameters plus locals.
