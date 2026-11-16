---
category: Python
---

# Python json.loads accepts floats, lone surrogate escapes and unbounded nesting unless the reader checks them

`json.loads` returns a `float` for `1.0` and `1e2`, accepts `1e400` as infinity,
builds a `str` containing a lone surrogate from `"\\ud800"` (which then cannot be
encoded as UTF-8), and nests as deep as the recursion limit allows. A reader shared
with runtimes whose numbers are doubles must accept the same documents everywhere:
pass `parse_float` and `parse_int` hooks (reject floats and `-0`, bound integers to
+-(2^53-1)), walk the result iteratively for surrogates and depth, and keep
`object_pairs_hook` for duplicate keys. The shared conformance fixture pins each.
