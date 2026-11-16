---
category: Testing & coverage
---

# Store conformance input bytes as hex, because a JSON string cannot hold invalid UTF-8, a lone surrogate or a BOM

Conformance inputs include invalid UTF-8, overlong encodings, encoded surrogates,
a BOM and lone surrogate escapes. None of that survives being a JSON string value
or a text file on a Windows checkout (CRLF conversion, re-encoding). Store the
bytes as lowercase hex in `input_hex`, and add `.gitattributes` with `* -text` to
the fixture directory so the JSON itself is also never rewritten. Every language
can hex-decode; none has to trust its own JSON reader to load the hostile input.
