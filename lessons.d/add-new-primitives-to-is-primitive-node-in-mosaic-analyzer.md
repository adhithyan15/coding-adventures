---
category: Mosaic compiler pipeline
---

# Add new primitives to `is_primitive_node()` in `mosaic-analyzer`

When a new layout element (e.g. `Grid`) is added to the Mosaic spec, it must be included in the `is_primitive_node()` function in `mosaic-analyzer/src/lib.rs` so that `is_primitive = true` is set correctly. Missing this causes the VM to emit it as a custom element tag (e.g. `<grid>`) instead of the intended HTML element.
