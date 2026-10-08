---
category: Rust
---

# Thread the compatibility probe work into checked import and remove the obsolete fresh-budget wrapper

The compatibility probe initially started a fresh work counter at checked
import. Carry consumed Work into bounded parsing, conversion and independent
validation. Remove the obsolete wrapper instead of suppressing Clippy dead-code
errors. A nearly exhausted seeded-work test proves that the real boundary rejects
rather than restarting the allowance.
