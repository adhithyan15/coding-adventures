# draw-instructions-png

PNG encoder for draw-instructions pixel buffers.

Takes a `PixelBuffer` from any renderer (Metal, Vulkan, Direct2D) and encodes
it as a PNG file.  Uses the pure-Rust `png` crate — no system dependencies.
