---
category: Mosaic compiler pipeline
---

# A supported base transform does not prove state transform support

The initial SwiftUI degree-rotation patch (#17502) accepted and reported base
values correctly, but its public degradation report inspected only base props.
A valid base rotation plus an unsupported state scale therefore lost the state
transform without a report. State property names are validated; free-form
transform values are not guaranteed to match the base value's supported shape.

Audit state transform values through the same parser used by emission, and test
the public reporter with a supported base plus an unsupported state. The
regression returned zero drops before the fix and one afterward. A low-level
modifier-chain test alone does not prove the public report includes its losses.
