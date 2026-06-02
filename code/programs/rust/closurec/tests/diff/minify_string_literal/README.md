# minify_string_literal — round-trip a string literal verbatim

Pins that a `"hi"` string literal survives the lex → parse →
emit round-trip without quote-flipping, escape-doubling, or
length change. Closure under WHITESPACE_ONLY does NOT optimize
quote choice, so the input quote style must be preserved.

Output is hand-traced for the simple case; a fresh capture
from upstream Closure would not differ for an already-minimal
`var x="hi";\n` input.
