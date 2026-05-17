# @coding-adventures/forme-aot-css-slicer

Per-page CSS slicer for the Forme **AOT compiler**.  Takes a
`StyleDocument` and a per-page `usedRuleIds` map (the renderer's
`usedStyle` accumulator from [FM01 §2.3.6](../../specs/FM01-forme-kernel.md))
and emits one content-addressed CSS artefact per page.

Per [FM06 §3 (per-page slicing)](../../specs/FM06-forme-aot-compiler.md).
First package in the FM06 family — the integration point that
consumes the CSS translator's `usedRuleIds` feature we've been
building toward across the FM04 family.

## Quick start

```ts
import { slicePerPage, defaultScopePrefix } from "@coding-adventures/forme-aot-css-slicer";
import { writeFileSync } from "node:fs";

const { artefacts } = slicePerPage(doc, [
  { id: "/index.html",     usedRuleIds: ["body", "headline"] },
  { id: "/about.html",     usedRuleIds: ["body", "headline", "nav"] },
  { id: "/blog/post.html", usedRuleIds: ["body", "headline", "nav", "code"] },
], {
  activeContexts: ["screen"],
});

for (const [pageId, art] of artefacts) {
  console.log(pageId, art.byteSize, "bytes,", art.sha256.slice(0, 8));
  writeFileSync(routeToFilePath(pageId), art.css);
}
```

## What you get per page

```ts
interface CssArtifact {
  pageId: string;
  css: string;              // the scoped CSS text the page loads
  emittedRules: readonly StyleRuleId[];
  warnings: readonly StyleWarning[];
  byteSize: number;         // Buffer.byteLength(css, "utf8")
  sha256: string;           // hex sha256 of the UNSCOPED bytes
}
```

The **sha256 is over the *unscoped* canonical CSS bytes** — so
pages with identical `usedRuleIds` get identical fingerprints and a
downstream cache (FM06 §4) can deduplicate by content while still
serving per-page-scoped CSS to the browser.

## Per-page scoping

Every emitted rule's selector gets prefixed with a per-page scope.
Default: `defaultScopePrefix(pageId)` returns `#p-<8 hex chars>`
where the hex is the first 32 bits of `sha256(pageId)`.

| Why `#`?            | Higher CSS specificity than `.`; lets per-page scopes override unscoped descendants without `!important`. |
| Why 8 hex chars?    | Birthday collision odds: 1% at ~9k pages, 50% at ~65k — well above any static-site count. |
| Why `p-` prefix?    | CSS identifiers can't start with a digit; the `p-` prefix avoids the issue uniformly. |
| What if I need more?| Override via `options.scopePrefix: (pageId) => string`. |

The choice deliberately collapses the pageId byte range into the
alphabet `[0-9a-f]` — hostile page ids (`"\x1b[31m"`, `"<script>"`,
`" "`) all hash to safe CSS identifiers without sanitisation
gymnastics.

## Capabilities — `["hash"]`

Uses `node:crypto.createHash("sha256")` for (a) the content-addressed
fingerprint and (b) the default scope.  No file I/O, no network,
no shell.

## Architecture

```
        StyleDocument                  pages: [ { id, usedRuleIds }, ... ]
              │                                  │
              ▼                                  ▼
       ┌──────────────────────────────────────────────────┐
       │            slicePerPage(doc, pages, opts)        │
       │                                                  │
       │  for each page:                                  │
       │    1. translateToCss(doc, { usedRuleIds })       │  ← UNSCOPED
       │       → sha256(output) = page.sha256             │
       │    2. translateToCss(doc, { usedRuleIds, scope })│  ← SCOPED
       │       → page.css                                 │
       └──────────────────────────────────────────────────┘
              │
              ▼
         Map<pageId, CssArtifact>
```

**Why two `translateToCss` calls per page?**  The cheaper
"hash-the-scoped-CSS-and-strip-the-prefix" approach couples the
cache key to the scope choice — pages differing only in their
`scopePrefix` function would then be cache misses despite being
byte-identical at the rule level.  Per-page translate cost is
O(rules-per-page), typically tiny.

## Security posture

Three concerns explicitly addressed:

1. **Page ID sanitisation.**  The default scope hashes the page id
   through sha256 first, so the scope is always
   `[#][p][-][0-9a-f]{8}` regardless of what bytes the page id
   contains.  No CSS injection surface from hostile page ids
   under the default scope.
2. **Scope isolation.**  Two pages using the same rule emit CSS
   with *different* `#p-` prefixes (different sha256 of the page
   id), so concatenating per-page CSS files can never produce
   cross-page selector collisions.
3. **sha256 fingerprint (not a weak hash).**  Cache-key collisions
   are catastrophic — two pages that differ but share a fingerprint
   would silently serve one's CSS for the other.  sha256 makes this
   computationally infeasible without an attacker who controls the
   token-set namespace.

## Tests

22 tests in `slicer.test.ts`:

- Basic slicing (per-page artefact, emittedRules subset, byteSize,
  empty page)
- Content-addressed sha256 (dedup-friendly equality; unscoped
  fingerprint; stable across runs; 64 hex chars)
- `defaultScopePrefix` (shape, determinism, no obvious collisions,
  first-8-hex-matches-sha256, hostile-pageId survival)
- Custom `scopePrefix` (override; empty-string no-op)
- Warning propagation (per-page isolation)
- Page iteration order preserved (Map respects input array)
- Cross-page scope isolation
- Empty pages array → empty Map

Coverage: **100% line / 100% branch** — well above the FM04 §14.4
≥95% line target.

## Spec adherence

Implements FM06 §3 (per-page CSS slicing) and consumes FM01 §2.3.6
(`usedStyle` accumulator).  No spec divergences.

## v0 simplifications

- **No incremental cache.**  Recomputes every page on every call.
  FM06 §4 (incremental rebuilds) is a future package that wraps
  this one with a content-addressed store.
- **CSS only.**  LaTeX / terminal per-page slicing would follow
  the same pattern but isn't shipped yet.
