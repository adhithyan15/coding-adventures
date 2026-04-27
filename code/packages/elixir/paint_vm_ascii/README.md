# paint_vm_ascii

Elixir terminal backend for `CodingAdventures.PaintInstructions`.

The current Elixir paint instruction model is rect-only, so this first version
renders filled rectangles as block-character output and raises for unsupported
instruction kinds.
