---
category: Rust
---

# Raw string delimiters must enclose quoted CSS colors

The new Venn theme tests used single-hash Rust raw strings around JSON with quoted hex colors. The quote-hash sequence ended the Rust literal and caused compilation failure before any behavioral assertion ran. Use two-hash delimiters for these fixtures. Preserve the failed compile receipt separately; only executable failed assertions establish a behavioral red baseline.

When deriving a matrix test from an existing nested-loop fixture, count both opening and closing loop scopes after changing the axes. An extra retained closing scope caused a second compile failure in the new source-to-scene test; this too provides no runtime behavioral evidence.

Use explicit namespaces for expected font queries: layout_ir::FontStretch belongs to authored layout options, while text_interfaces::FontStretch belongs to resolver queries. Similar variant names do not make the types interchangeable. The first matrix compile exposed this distinction; correct only the oracle types, preserving the production conversion.

The public Venn lexer accepts attached text through the current region, not the semantic parser branch for explicit comma-separated text targets. Source-to-scene fixtures must use the supported `union a,b` followed by `text note` syntax. Preserve the failed lexer receipt and do not broaden the grammar to rescue an invalid fixture.
