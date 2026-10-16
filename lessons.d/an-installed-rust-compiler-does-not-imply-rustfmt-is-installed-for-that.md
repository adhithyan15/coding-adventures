---
category: Rust
---

# An installed Rust compiler does not imply rustfmt is installed for that toolchain

Cargo1.99 compilation worked but rustfmt +1.99.0 refused because that optional
component was absent. The default installed rustfmt was available and formatted
the exact two owned files. Check each tool independently; do not label a failed
formatter command as a completed format check. Use the actual generated lesson
path printed by lessons.py: long titles are truncated in filenames, and guessing
the full slug caused a read-only missing-file error while documenting this case.
