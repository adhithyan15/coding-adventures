---
category: Cross-platform & Windows BUILD_windows
---

# MSVC can see a conditionally parsed C output as uninitialized

The C HDD fixture adapter parses an expected timing structure only when a
probe is valid. A later comparison used one field only on that same valid
branch. MSVC's C4701 analysis did not preserve the correlation between the
parse-success condition and that later branch, so the warnings-as-errors build
rejected the test with a potentially uninitialized read.

Initialize the result structure at declaration and keep the parse-success
check before every semantic use. Initialization is a safe fallback for the
non-parsed branch; it does not replace malformed-input checks or the independent
model. When control flow can be restructured without obscuring the test, placing
the read directly in the success branch is also valid. Keep `/W4 /WX` enabled
and use the Windows native build to validate MSVC's flow analysis.
