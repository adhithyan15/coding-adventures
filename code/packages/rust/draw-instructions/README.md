# draw-instructions

Backend-neutral 2D draw instructions for reusable scene generation.

This crate is the seam between producer logic and renderer logic. A barcode
crate can emit rectangles and text without knowing SVG syntax, and a renderer
crate can serialize those instructions without knowing barcode rules.
