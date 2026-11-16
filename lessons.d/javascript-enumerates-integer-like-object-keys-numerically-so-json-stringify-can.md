---
category: TypeScript / JavaScript
---

# JavaScript enumerates integer-like object keys numerically, so JSON.stringify cannot produce code-point-sorted canonical output

A canonical JSON writer that builds a plain object with keys inserted in sorted
order still emits `"2"` before `"10"`: JavaScript enumerates integer-like keys
(array indices) first, ascending, whatever the insertion order, and
`JSON.stringify` follows `Object.keys`. The conformance case
`json/key-integer-like-sorted` (`{"10":..,"9":..,"2":..}`) caught it; Python's
`sort_keys` gives `"10"`, `"2"`, `"9"`.

Serialise by hand from a sorted key list (or a `Map`) and never let an object's
own enumeration order decide the output. Applies to any canonical form two
runtimes must agree on byte for byte.
