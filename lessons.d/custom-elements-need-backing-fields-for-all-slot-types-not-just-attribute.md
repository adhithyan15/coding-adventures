---
category: Mosaic compiler pipeline
---

# Custom Elements need backing fields for ALL slot types, not just attribute-observed ones

Scalar primitive slots (text, number, bool, image, color) appear in `observedAttributes` and get string backing fields. List and node slots must NOT appear in `observedAttributes` (the browser would stringify them); instead, expose them through explicit JavaScript property setters and initialize their backing fields as `[]` or `null` in the constructor.
