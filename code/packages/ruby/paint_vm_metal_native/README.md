# coding_adventures_paint_vm_metal_native

Rust-backed Metal Paint VM bridge for Ruby.

```ruby
require "coding_adventures_paint_vm_metal_native"
require "coding_adventures_paint_instructions"

scene = CodingAdventures::PaintInstructions.paint_scene(
  width: 40,
  height: 20,
  instructions: [
    CodingAdventures::PaintInstructions.paint_rect(
      x: 10, y: 0, width: 20, height: 20, fill: "#000000"
    ),
  ],
  background: "#ffffff",
)

pixels = CodingAdventures::PaintVmMetalNative.render(scene)
```
