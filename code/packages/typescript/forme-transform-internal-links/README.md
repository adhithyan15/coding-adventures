# @coding-adventures/forme-transform-internal-links

Resolve root-relative `/slug` references in `LinkNode`
destinations to caller-supplied canonical URLs.  FM00 v0 §5.3
transform.

Pure transform: walks a `DocumentNode`, calls
`(slug: string) => string | null` on every internal link,
**validates the resolved URL against an `http(s)://` accept-list**
to defend against a malicious or buggy resolver, returns a
transformed `DocumentNode` copy.

Eighth FM00 v0 stage package — joins
[`forme-feeds`](../forme-feeds),
[`forme-opengraph`](../forme-opengraph),
[`forme-index-renderer`](../forme-index-renderer),
[`forme-transforms`](../forme-transforms),
[`forme-transform-autolink-headings`](../forme-transform-autolink-headings),
[`forme-transform-toc`](../forme-transform-toc),
[`forme-transform-typography`](../forme-transform-typography).

## Quick start

```ts
import { rewriteInternalLinks } from "@coding-adventures/forme-transform-internal-links";

function resolve(slug: string): string | null {
  const entry = manifest.byPath.get(slug);
  return entry ? entry.canonicalUrl : null;
}

const linked = rewriteInternalLinks(doc, resolve);

// Stricter: treat unresolved links as content bugs.
const validated = rewriteInternalLinks(doc, resolve, { unresolved: "throw" });

// Drop unresolved links from the rendered output entirely.
const stripped = rewriteInternalLinks(doc, resolve, { unresolved: "strip" });
```

## What counts as an internal link?

`isInternalSlug(url)` accepts:

- `/about`
- `/blog/2026/post`
- `/` (bare site root)

…and rejects everything else:

- `http://example.com/x`, `https://...` (already absolute — no
  rewrite needed)
- `//host/path` (protocol-relative — ambiguous scheme)
- `relative/path`, `./about` (bare relative — author should
  normalise to `/about` first)
- `mailto:`, `tel:`, `javascript:`, `data:` (not internal)
- `#fragment-only` (handled by
  [`forme-transform-autolink-headings`](../forme-transform-autolink-headings))
- Empty string, non-string

## Resolver contract

```ts
type SlugResolver = (slug: string) => string | null;
```

Resolvers must be:

- **Pure.**  Same slug → same result every call (else
  reproducibility breaks).
- **Synchronous.**  No I/O — manifest lookups are in-memory.

The resolver's return value is validated by `assertResolvedUrl`:

- Accepted: `http(s)://...` (case-insensitive scheme),
  root-relative `/path`.
- Rejected (throws `TypeError`): `javascript:`, `data:`,
  `file:`, `vbscript:`, protocol-relative `//host`, bare
  relative, empty string, non-string.

This guards against a buggy or hostile resolver returning a
URL that would break out of `<a href="...">` into XSS territory.

## API

### `rewriteInternalLinks(doc, resolver, options?): DocumentNode`

Walks the document, returns a fresh copy with every internal
`LinkNode.destination` rewritten.

### `isInternalSlug(url): boolean`

Exposed sub-helper.

### `assertResolvedUrl(url): asserts url is string`

Exposed sub-helper.  Throws `TypeError` on unsafe URL.

### Types

```ts
type SlugResolver = (slug: string) => string | null;

type UnresolvedPolicy = "keep" | "strip" | "throw";

interface InternalLinksOptions {
  readonly unresolved?: UnresolvedPolicy;  // default "keep"
}
```

## Unresolved-link policy

When the resolver returns `null` (or `undefined`) for an
internal slug:

| Policy   | Behaviour                                                  |
|----------|------------------------------------------------------------|
| `"keep"` (default) | Preserve the original `/slug` in `LinkNode.destination`.  Browsers will follow it as a site-relative path. |
| `"strip"`| Replace the `LinkNode` with its inline children (drop the link wrapper).  Useful when the renderer refuses to emit broken `<a href>`s. |
| `"throw"`| Throw `Error` immediately.  Useful in pre-publish validation: an unresolvable link is a content bug. |

## Pass-through nodes (NOT rewritten)

| Node                  | Reason                                        |
|-----------------------|-----------------------------------------------|
| `ImageNode.destination` | Image rewrite is a separate spec transform |
| `AutolinkNode.destination` | User's explicit external URL            |
| `CodeBlockNode.value` | Verbatim source                               |
| `CodeSpanNode.value`  | Verbatim inline code                          |
| `RawBlockNode.value`  | Back-end-specific markup                      |
| `RawInlineNode.value` | Same                                          |

## Behavioural contract

| Aspect                          | Behaviour                              |
|---------------------------------|----------------------------------------|
| Input document                  | Never mutated                          |
| Output                          | Fresh tree (no shared refs with input) |
| Internal link, resolved         | New URL spliced into LinkNode          |
| Internal link, unresolved (keep)| Original `/slug` preserved             |
| Internal link, unresolved (strip)| `LinkNode` replaced by children       |
| Internal link, unresolved (throw)| Throws `Error` with slug in message   |
| Resolver returns unsafe URL     | Throws `TypeError`                     |
| External / non-link nodes       | Pass-through                           |
| Resolver call count             | Once per internal `LinkNode`           |

## Reproducibility (FM03)

Same input `DocumentNode` + same resolver → byte-identical
output.  Safe to use as cache key input given a pure resolver.

## Security posture

Four concerns explicitly addressed (pre-push review):

- **Hostile resolver output.**  The resolver is caller-supplied
  code we can't audit.  Every returned URL is validated against
  `^https?://` or root-relative `/path` before being spliced
  back into the AST.  `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative, bare-relative all rejected
  with `TypeError`.  Defence-in-depth: even if the renderer
  forgets to escape, the AST cannot contain a JS-URL link.
- **No AST mutation.**  Input `DocumentNode` never modified;
  every returned node is freshly constructed.  Fresh-tree
  guarantee holds even for passthrough sub-trees.
- **Deterministic.**  Single forward walk, no `Map`/`Set`
  iteration affecting output, no randomness.  Same input +
  pure resolver → byte-identical output.
- **Bounded computation.**  O(N) walk; resolver called exactly
  once per internal LinkNode (no quadratic re-lookups).  No
  regex backtracking surface — URL detection is character-class
  checks.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

72 tests across 2 files:

- `url.test.ts` (31) — `isInternalSlug` accept set (root-relative
  paths, bare /) and reject set (absolute http(s), protocol-
  relative, bare relative, ./about, mailto:, javascript:, empty,
  non-string, fragment-only); `assertResolvedUrl` accept set
  (http://, https://, case-insensitive scheme, port + query +
  fragment, root-relative) and reject set (javascript:, data:,
  file:, vbscript:, protocol-relative, bare relative, mailto:,
  empty / null / undefined / number with descriptive error
  messages, long URL truncation).
- `walk.test.ts` (41) — internal link resolution, title
  preservation, bare /, external pass-through, resolver-NOT-
  called for external, unresolved policy matrix (keep default
  + explicit, strip with single + multi-child, throw with
  slug in message, undefined treated same as null), validation
  rejecting each forbidden resolver output, walks every nested
  container (blockquote, list, task_item, heading, table cells,
  emphasis / strong / strikethrough, nested DocumentNode),
  pass-through (image, autolink, code_block, code_span,
  raw_block, raw_inline, breaks), defensive non-tree BlockNode
  variants, purity (no input mutation, fresh tree, byte-
  identical output, resolver-called-once-per-link).

Coverage: **97.1% line / 97.89% branch** across all source
files with logic.  Uncovered lines are TypeScript `never`
exhaustiveness guards (`walk.ts` 138-141, 210-213) that cannot
fire at runtime.

## Spec adherence

Implements FM00 v0 §5.3 `transform-internal-links`.  No spec
divergences.

## v0 simplifications

- **No image-src rewriting.**  Image destinations pass through
  unchanged — `transform-image-rewrite` is a separate spec
  transform deferred to its own package.
- **No internal-link predicate customisation.**  "Internal"
  hardcoded to "starts with `/` but not `//`".  Custom
  predicates (e.g. matching a specific origin like
  `https://example.com/...`) could be added as an option in v1.
- **No async resolver support.**  Resolvers are synchronous.
  Manifest lookup is in-memory anyway.  Async resolution
  (e.g. database lookup) would change the transform's purity
  contract significantly; v1 may add `rewriteInternalLinksAsync`.
- **No batch / multi-document optimisation.**  Each call
  re-walks the document.  Pipelines processing many docs share
  the resolver but not the walk state.
