# paint-vm-cairo

Cairo backend for the Paint VM runtime.

This crate renders `paint-instructions::PaintScene` values through native
Cairo on Linux and BSD targets. Other desktop targets keep a deterministic
software smoke renderer so the runtime can still compile and exercise Cairo
selection behavior without requiring Cairo DLL/framework installation.

## Status

| Area | Status |
|------|--------|
| Native Cairo image surface | Implemented on Linux/BSD via `cairo-rs` |
| Rect, line, ellipse, path | Implemented, except SVG `ArcTo` lowering |
| Clip, group transform, layer opacity | Implemented natively |
| Image pixels | Implemented for `ImageSrc::Pixels` |
| Text | Degraded, uses Cairo toy text API |
| Glyph runs | Degraded, uses Cairo glyph APIs without full shaping integration |
| Gradients | Not implemented |
| Layer filters / blend modes | Not implemented |

## Linux Dependencies

Native builds require Cairo development headers:

```bash
sudo apt-get install -y libcairo2-dev
```

Pango/HarfBuzz integration is intentionally not part of this first slice. Text
is visible for smoke tests, but full text parity should come through the shared
font/shaping pipeline and `pangocairo`.

## Runtime Use

```rust
let backend = paint_vm_cairo::renderer();
let pixels = backend.render(&scene)?;
```

For backend selection, register this renderer with `paint-vm-runtime`. Exact
text selection will reject Cairo for now unless degraded text is allowed.
