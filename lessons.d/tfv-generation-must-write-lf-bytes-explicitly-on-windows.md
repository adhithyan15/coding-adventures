---
category: Cross-platform & Windows BUILD_windows
---

# TFV generation must write LF bytes explicitly on Windows

While extending the TF01 self-corpus, its generator used Path.write_text with
the default newline policy. On Windows that writes CRLF, although strict TFV
rejects carriage returns. A checking read_text call normalizes CRLF back to LF,
so a text comparison alone can report that such a generated file is current.

Write with newline="\n" explicitly, and compare the generated file's bytes
against the UTF-8 encoding of the canonical transport in the regeneration test.
The independent validator and Rust embedded-corpus run must both pass. Do not
relax the TFV parser or the corpus to accommodate platform newline translation.
