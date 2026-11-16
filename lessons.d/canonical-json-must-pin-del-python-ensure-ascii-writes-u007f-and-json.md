---
category: Testing & coverage
---

# Canonical JSON must pin DEL: Python ensure_ascii writes u007f and JSON.stringify writes the raw byte

Python's `json.dumps(ensure_ascii=True)` escapes everything outside
`U+0020..U+007E`, so DEL (U+007F) is written `\\u007f`; `JSON.stringify` and most
other writers emit it raw. Both are "ASCII only", and VER04i was silent, so the
lanes disagreed on any manifest that contained a DEL. VER04i 9.1 now says every
character outside `U+0020..U+007E` is escaped (lowercase hex); cases
`json/string-raw-del` and `json/string-escape-del` pin it. Pin any byte-level
choice the spec does not make before a second lane is written.
