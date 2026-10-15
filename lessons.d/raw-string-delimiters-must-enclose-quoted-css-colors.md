---
category: Rust
---

# Raw string delimiters must enclose quoted CSS colors

The new Venn theme tests used single-hash Rust raw strings around JSON with quoted hex colors. The quote-hash sequence ended the Rust literal and caused compilation failure before any behavioral assertion ran. Use two-hash delimiters for these fixtures. Preserve the failed compile receipt separately; only executable failed assertions establish a behavioral red baseline.
