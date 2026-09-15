---
category: Mosaic compiler pipeline
---

# Grammar alternation order matters: more specific alternatives must come first

In `slot_type`, `list_type` must appear before `KEYWORD`; otherwise `list<text>` is lexed as just the keyword `list` and the `<` causes a parse error. The rule of thumb: try the longest / most specific match first in any alternation. This applies in both `.grammar` files and the generated `_grammar.rs`.
