# draw-instructions-metal

Metal GPU renderer for the draw-instructions scene model.

Takes a `DrawScene` and renders it to a `PixelBuffer` using Apple's Metal GPU
API.  Rectangles become triangle pairs, text is rasterized via CoreText, and
the final image is read back from the GPU to CPU memory.

macOS only.  Requires a Metal-capable GPU (any Mac from 2012 or later).
