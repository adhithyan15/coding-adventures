---
category: Rust
---

# Locate Rust methods by declaration rather than assumed source order

An integration repair script assumed the sign helper preceded the dedicated
outer-entier helper. Their actual order was reversed; a protective assertion
stopped the script before any file write. Match each declaration and its exact
method boundary independently, assert a unique match, and compare the restored
method bytes against the reviewed reference before committing. Do not infer
method order from a previous diff excerpt.
