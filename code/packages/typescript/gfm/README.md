# GFM

GitHub Flavored Markdown facade for TypeScript, composing `gfm-parser` with
the shared Document AST HTML renderer. Supports CommonMark syntax and GFM
tables, task lists and strikethrough.

```typescript
import { parse, toHtml } from "@coding-adventures/gfm";
const document = parse("~~~ts linenos\nx\n~~~");
const html = toHtml(document);
```

The existing literal CommonMark corpus is tested separately from GFM extension
witnesses. This package does not claim the complete language-neutral GFM
conformance profile, which remains separate parity work.

## TE04 fenced-block compatibility

The GFM facade exposes the corrected fenced AST through `parse`, re-exports `FencedBlockNode`, and uses the shared HTML renderer. Callers matching fenced input as `code_block` must handle `fenced_block`; indented and legacy producer nodes are unchanged.

See [TE04](../../../specs/TE04-gfm-extensions.md) for normalization,
source retention, compatibility and validation boundaries. Run package-local
`npm run build` when supplied and `npm test -- --coverage` after installing
local dependencies; BUILD fronts also install required sibling packages.
