---
category: Python
---

# Decode Git source output explicitly as UTF-8 on Windows

The native documentation validator read Git's UTF-8 BUILD source with
subprocess text=True and Windows' default cp1252 decoder. Existing Unicode
arrows caused UnicodeDecodeError before any documentation checks executed.
Set encoding="utf-8" explicitly for source text, or read bytes and decode them
with the declared format. Keep the failed harness receipt separate from the
corrected validator's actual terminal results.
