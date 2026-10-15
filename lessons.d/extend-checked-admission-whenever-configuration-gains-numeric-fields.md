---
category: Security boundaries
---

# Extend checked admission whenever configuration gains numeric fields

The Railroad style integration added font size, stroke width and marker radius,
but the existing checked layout still validated only its three spacing values.
A NaN or negative font reached invalid floating clamp bounds and panicked before
the derived geometry check. Validate every new numeric input before arithmetic,
including empty diagrams, and test precise typed refusal under catch_unwind.
Finite caps prevent numeric overflow; they do not establish visual enclosure
of arbitrary fonts, markers or strokes.
