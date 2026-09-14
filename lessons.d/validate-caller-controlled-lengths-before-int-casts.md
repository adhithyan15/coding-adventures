---
category: Rust
---

# Validate caller-controlled lengths before `int` casts

Binary parsers must explicit-bounds-check `u4`/`u8` lengths against host capacity; never recursively decode nested structures unless the format requires it.
