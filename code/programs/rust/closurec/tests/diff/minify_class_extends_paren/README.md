# minify_class_extends_paren — gap-066: redundant parens after extends

Input: `class A extends(B){}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `class A extends B{};`

Captured by CLOC14.33 byte-identity exploration.
