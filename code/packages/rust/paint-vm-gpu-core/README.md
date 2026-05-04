# paint-vm-gpu-core

Shared render-plan and tessellation core for GPU-flavoured Paint VM backends.

This crate does not talk to Vulkan, OpenGL, WGPU, Mesa, or OpenCL directly.
Instead, it converts `paint-instructions::PaintScene` into a backend-neutral
`GpuPaintPlan`:

- Solid vector primitives become indexed triangle meshes.
- Pixel images become texture-upload records plus textured quads.
- Linear gradients become sampled ramp textures plus gradient UVs on meshes.
- Radial gradients become sampled 2D textures plus radial UVs on meshes.
- Rectangular clips become push/pop clip commands.
- Text and glyph runs are preserved as explicit commands so backend-specific
  glyph atlas and shaping strategies can evolve without losing IR fidelity.

The goal is to keep `paint-vm-vulkan`, `paint-vm-opengl`, `paint-vm-wgpu`, and
Mesa profiles convergent. They should differ in API plumbing, not in how every
backend interprets the PaintScene geometry.

## Current Coverage

| Paint instruction | GPU plan lowering |
|-------------------|-------------------|
| `PaintRect` | Filled mesh, simple stroked edge meshes |
| `PaintLine` | Stroke quad |
| `PaintEllipse` | Filled fan and stroked ring tessellation |
| `PaintPath` | Flattened line/quad/cubic contours; simple fan fill and stroked segments |
| `PaintClip` | Push/pop axis-aligned clip bounding rect |
| `PaintGroup` | Transform and opacity folded into children |
| `PaintLayer` | Transform and opacity folded into children; filters/blends diagnosed |
| `PaintImage` | Texture upload plus textured quad |
| `PaintText` | Preserved text command |
| `PaintGlyphRun` | Preserved positioned glyph command |
| `PaintGradient` | Linear fills become ramp textures; radial fills become sampled 2D textures |

## Next Steps

- Replace simple fan path filling with a robust tessellator.
- Add stroke joins, caps, and dashed stroke expansion.
- Add glyph atlas planning once text shaping/font metrics are finalized.
