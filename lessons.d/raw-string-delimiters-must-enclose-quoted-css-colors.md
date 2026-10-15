---
category: Rust
---

# Raw string delimiters must enclose quoted CSS colors

The new Venn theme tests used single-hash Rust raw strings around JSON with quoted hex colors. The quote-hash sequence ended the Rust literal and caused compilation failure before any behavioral assertion ran. Use two-hash delimiters for these fixtures. Preserve the failed compile receipt separately; only executable failed assertions establish a behavioral red baseline.

When deriving a matrix test from an existing nested-loop fixture, count both opening and closing loop scopes after changing the axes. An extra retained closing scope caused a second compile failure in the new source-to-scene test; this too provides no runtime behavioral evidence.
