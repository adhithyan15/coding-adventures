---
category: Rust
---

# Advance past the entire whitespace scalar before slicing edge identifiers

Swimlane edge IDs on main b1f4e459 used rfind(char::is_whitespace) plus one as
a UTF-8 slice boundary. The API returns a byte offset, not a scalar index.
Actual public parse at a09d81cb failed with a byte-boundary panic for U+00A0;
independent review identified the same fault for U+2003. Retain that one-test
red and advance by the matched character's len_utf8. Compare complete edges
with no-ID controls over two-byte and three-byte separators and all styled
operators, keeping the prior resource-count controls and authored metadata.

The first full downstream build then caught the existing resource-test fixture's
missing new id field. Its exit101 with zero executed targets is retained as a
compile rejection, not a test success. Give every duplicate constraint a distinct
ID and assert that layout preserves them in order alongside the original exact
operation counts, labels, kinds, markers and lengths. Rerun the full affected cone.
