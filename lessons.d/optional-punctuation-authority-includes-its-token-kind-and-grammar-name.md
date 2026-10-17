---
category: Security boundaries
---

# Optional punctuation authority includes its token kind and grammar name

Independent CLOC35 source review withheld8fc: an optional marker was recognized
only by value == "?.". Changing its generic kind or grammar type_name kept the
same endpoint IDs and reached graph mutation, bypassing the intended local
punctuation check. This was a private data-integrity gap, not a claimed CLI exploit.

Test optional dot and computed marker kind, wrong grammar name and missing name
before fixing code. Precharge spelling/type_name inspections, then require the
standard Name fallback with OPTIONAL_CHAIN. Preserve fail-before-mutation and
the independent source replay; neither check substitutes for the other.
