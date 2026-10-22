---
category: Compiler / VM / language pipeline
---

# Never re-strip quotes from a GrammarLexer STRING token value

The GrammarLexer already removes a STRING token's delimiters, so a consumer
that strips "if it starts and ends with a quote" is stripping the *content*.
`'"'` arrives as the one-char value `"`, and `&raw[1..raw.len() - 1]` becomes
`raw[1..0]` and panics; `'"abc"'` silently decodes to `abc`. The javascript-parser
bridge had exactly this (`unquote_string`, fixed in 0.64.0). Treat a STRING
token's value as the interior: decode escapes, never trim. Test quote-only and
quote-wrapped literals (`'"'`, `"'"`, `'"abc"'`) whenever you touch string decoding.
