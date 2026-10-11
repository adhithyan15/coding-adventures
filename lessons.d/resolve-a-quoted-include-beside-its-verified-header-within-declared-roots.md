---
category: Compiler / VM / language pipeline
---

# Resolve a quoted include beside its verified header, within declared roots

A C translation unit can include `sub/a.h`, which then includes `"b.h"`.
Searching only the declared root can silently select an unrelated root-level
`b.h` even when `sub/b.h` exists. Use the verified including file's canonical
parent as the first quoted-include search directory, then try declared roots.
Canonicalise every candidate and enforce the same root containment and
opened-handle checks; an unknown origin must not fall back silently. Test
precedence with two same-named files and run the real C file-input frontend.
