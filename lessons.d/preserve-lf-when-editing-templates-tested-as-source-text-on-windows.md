---
category: C#
---

# Preserve LF when editing templates tested as source text on Windows

Python text writes translated LF to CRLF in a C# template included verbatim by Rust. Three existing source assertions failed despite unchanged code. Write explicit UTF-8 bytes with LF for these templates, then run both textual and compiled runtime checks.
