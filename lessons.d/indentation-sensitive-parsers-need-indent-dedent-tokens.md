---
category: Compiler / VM / language pipeline
---

# Indentation-sensitive parsers need INDENT/DEDENT tokens

`skip_newlines` must NOT skip DEDENT (block boundary). Use a separate `skip_whitespace` that drops NEWLINE+INDENT+DEDENT for contexts where indentation is noise.
