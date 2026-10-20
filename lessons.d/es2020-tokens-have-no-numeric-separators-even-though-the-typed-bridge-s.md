---
category: Compiler / VM / language pipeline
---

# ES2020 tokens have no numeric separators, even though the typed bridge's number decoder strips underscores

While porting `javascript-iir-compiler` to walk the CST, I wrote test
fixtures such as `console.log(1_000)` and `0b1_0…0`, reasoning from the
bridge's `parse_js_number`, which drops every `_` ("ES2021 numeric
separators"). The tests failed with parse errors: `es2020.tokens` defines
`INT_NUMBER = /[0-9]+/` and friends with no `_`, so `1_000` lexes as
`NUMBER 1` followed by `NAME _000`, and the parse fails long before any
decoder runs.

A decoder's tolerance says nothing about what the lexer will hand it. Before
writing a literal fixture for a grammar-driven frontend, read the edition's
`.tokens` file (or dump the tokens) to confirm the spelling lexes as one
token. The same applies to the regex-aware JavaScript parse: two `/` on one
line become a `REGEX` token, so a division fixture is only meaningful under
`parse_javascript_cst_division_only`.
