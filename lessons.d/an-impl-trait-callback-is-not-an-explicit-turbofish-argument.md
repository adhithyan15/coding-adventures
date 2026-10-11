---
category: Rust
---

# An impl Trait callback is not an explicit turbofish argument

The panic fixture called with_pipeline::<(), (), _> even though the method
declares only T and E; its callback uses argument-position impl Trait. Use
with_pipeline::<(), ()> and infer the callback type. Inspect the explicit
generic parameters before supplying a placeholder for an opaque callback.
