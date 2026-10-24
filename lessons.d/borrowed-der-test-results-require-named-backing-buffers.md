---
category: Rust
---

# Borrowed DER test results require named backing buffers

Allocation-free decoders return views tied to their encoded input. Passing an
owned `Vec<u8>` test helper directly by reference creates a temporary that is
dropped at the end of the statement, so any decoded view cannot be inspected
afterward. Bind generated DER to a named variable before decoding it. Bind
generated expected encodings too, then compare borrowed slices by reference;
this makes both the lifetime and the intended exact bytes explicit.
