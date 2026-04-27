# paint_vm_ascii

Lua terminal backend for `coding_adventures.paint_instructions`.

The current Lua paint scene model is rect-only, so this first version renders
filled rectangles as block-character output and raises an error for unsupported
instruction kinds.
