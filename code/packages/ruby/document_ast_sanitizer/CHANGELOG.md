# Changelog

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `CodingAdventures::DocumentAstSanitizer.sanitize(doc, policy)`.
- `SanitizationPolicy` as a `Data.define` value object with `with()` support for
  deriving custom policies.
- Three named presets: `STRICT`, `RELAXED`, `PASSTHROUGH`.
- Complete truth-table implementation for all 18 Document AST node types:
  - Block nodes: DocumentNode, HeadingNode, ParagraphNode, CodeBlockNode,
    BlockquoteNode, ListNode, ListItemNode, ThematicBreakNode, RawBlockNode
  - Inline nodes: TextNode, EmphasisNode, StrongNode, CodeSpanNode, LinkNode,
    ImageNode, AutolinkNode, RawInlineNode, HardBreakNode, SoftBreakNode
- URL scheme sanitization with C0 control character and zero-width character
  stripping (closes `java\x00script:` and `\u200Bjavascript:` bypasses).
- Empty-children pruning: container nodes with zero surviving children are
  themselves dropped (prevents empty `<p></p>` etc.).
- Link children promotion: `drop_links: true` preserves text content.
- Image to alt-text transformation: `transform_image_to_text: true`.
- Heading level clamping: `min_heading_level`, `max_heading_level` (includes
  `"drop"` option to remove all headings).
- 59 unit tests covering all policy options, XSS vectors from the spec,
  immutability guarantee, and integration scenarios.
- 95.93% test coverage via SimpleCov.
- Passes `standardrb` linting.
