# draw-instructions-pixels

Shared RGBA pixel buffer type that sits between GPU renderers and image encoders.

Any renderer (Metal, Vulkan, Direct2D) can produce a `PixelBuffer`, and any
encoder (PNG, JPEG, WebP) can consume one.  They never need to know about
each other — this crate is the universal interchange format.
