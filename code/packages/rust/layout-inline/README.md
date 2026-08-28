# layout-inline

Reusable inline formatting contexts for the shared Layout IR.

`layout_inline_run` consumes producer-neutral inline `LayoutNode` siblings and
returns positioned fragments. Text is split at line-break opportunities,
line boxes align mixed fonts and replaced content, and semantic inline
containers are reconstructed once per occupied line. This lets HTML links,
document annotations, and future UI spans retain accurate geometry without
putting producer-specific behavior in the layout engine.

Formatting-context owners pass inherited `whiteSpace` and `wordBreak`
properties through `InlineOptions`; producers do not need to duplicate those
properties onto every descendant leaf.

Text fragmentation delegates to `text-flow`: normal wrapping uses shared
Unicode line-break opportunities, while `word-break: break-all` advances by
extended grapheme cluster rather than scalar value. This keeps combining marks,
emoji ZWJ sequences, and regional-indicator flags intact and lets CJK content
wrap without spaces. Explicit `dir` metadata is also forwarded to the analyzer.

The caller supplies an atomic-layout callback for replaced content. This keeps
image/intrinsic sizing policy in the composing layout algorithm and avoids a
dependency cycle with `layout-block`.
