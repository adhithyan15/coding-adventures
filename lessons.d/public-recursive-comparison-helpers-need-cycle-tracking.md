---
category: Cryptography & security review
---

# Public recursive comparison helpers need cycle tracking

A `DeepEqual` that walks dicts/enumerables/properties without tracking visited reference pairs explodes on cyclic graphs. Always assume hostile input.
