# paint-vm-gpu-core

Shared render-plan and tessellation core for GPU-flavoured Paint VM backends.

This crate does not talk to Vulkan, OpenGL, WGPU, Mesa, or OpenCL directly.
Instead, it converts `paint-instructions::PaintScene` into a backend-neutral
`GpuPaintPlan`:

- Solid vector primitives become indexed triangle meshes.
- Pixel images become texture-upload records plus textured quads; URI-backed
  images use an optional host-owned `GpuImageResolver`.
- Linear gradients become sampled ramp textures plus gradient UVs on meshes.
- Radial gradients become sampled 2D textures plus radial UVs on meshes.
- Rectangular clips become push/pop clip commands.
- Isolated layers become balanced begin/end commands carrying post-filter
  opacity, ordered filter chains, and non-normal blend metadata.
- Text and glyph runs are preserved as explicit commands so backend-specific
  glyph atlas and shaping strategies can evolve without losing IR fidelity.

The goal is to keep `paint-vm-vulkan`, `paint-vm-opengl`, `paint-vm-wgpu`, and
Mesa profiles convergent. They should differ in API plumbing, not in how every
backend interprets the PaintScene geometry.

## Current Coverage

| Paint instruction | GPU plan lowering |
|-------------------|-------------------|
| `PaintRect` | Filled mesh; solid and dashed stroked edge meshes |
| `PaintLine` | Stroke quad with butt, square, and round caps; dash patterns split into segment meshes |
| `PaintEllipse` | Filled fan, solid stroked ring tessellation, and dashed stroked segments |
| `PaintPath` | Flattened line/quad/cubic contours; simple fan fill; solid stroked segments with bevel, round, and bounded miter joins plus open-contour caps; dashed stroked segments with continuing-run joins |
| `PaintClip` | Push/pop axis-aligned clip bounding rect |
| `PaintGroup` | Transform and opacity folded into children |
| `PaintLayer` | Explicit isolated begin/end scope; transform folded into children while opacity, ordered filters, and blend mode stay on the layer command |
| `PaintImage` | Texture upload plus textured quad; URI resolution is caller-owned |
| `PaintText` | Preserved text command |
| `PaintGlyphRun` | Preserved positioned glyph command |
| `PaintGradient` | Linear fills become ramp textures; radial fills become sampled 2D textures |

## Next Steps

- Replace simple fan path filling with a robust tessellator.
- Carry dashed join topology across closed-contour seams.
- Add glyph atlas planning once text shaping/font metrics are finalized.
- Add robust non-convex path tessellation.
