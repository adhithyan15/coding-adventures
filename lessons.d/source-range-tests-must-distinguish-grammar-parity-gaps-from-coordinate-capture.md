---
category: Compiler / VM / language pipeline
---

# Source-range tests must distinguish grammar parity gaps from coordinate capture

CV04a's first real JavaScript grammar controls made three incorrect assumptions:
all editions preserved the same string escape value, `@` was lexically invalid,
and ES2025 skipped bare U+2028/U+2029. The actual generated grammars disagree:
legacy escape processing varies by edition, `@` is an AT token, and bare LS/PS
are currently rejected. A lexical snapshot must preserve those actual tokens and
errors while measuring raw bytes independently of transformed token values.

Find the literal by its actual token type and assert its exact raw slice and all
coordinate widths. Compare malformed/candidate input with the legacy tokenizer;
use an actual unmatched NUL for malformed-source control. Test accepted Unicode
line terminators inside multiline token/trivia matches and all terminators with
a generic grammar that admits them. Preserve and record bare LS/PS rejection as
a compiler grammar parity gap; do not silently repair it in a tracing API or
claim that coordinate support means every grammar accepts those characters.
