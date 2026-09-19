---
category: Rust
---

# Place Rust production helpers before the test module

Appending encode_ldc_i8 below the builder's test module passed cargo test but
failed Clippy with items_after_test_module. Keep public production helpers above
the cfg(test) module and run Clippy before committing a publication candidate.
