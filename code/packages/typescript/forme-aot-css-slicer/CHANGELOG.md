# Changelog — @coding-adventures/forme-aot-css-slicer

## 0.1.0 — 2026-05-17

Initial release.  First package of the FM06 AOT compiler family —
the integration point that consumes the CSS translator's
`usedRuleIds` feature shipped across the FM04 family.

### Added

- `slicePerPage(doc, pages, options): SliceResult` — for each
  page, runs `translateToCss` twice (once unscoped for the
  content-addressed fingerprint, once scoped for the deliverable)
  and returns a `Map<pageId, CssArtifact>`.
- `defaultScopePrefix(pageId): string` — `"#p-" + first 8 hex chars
  of sha256(pageId)`.  Collapses arbitrary page id bytes into a
  safe CSS identifier without sanitisation; ~9k pages before 1%
  birthday-collision odds.
- Types: `PageSlice`, `SliceOptions`, `SliceResult`, `CssArtifact`.

### Spec adherence

Implements FM06 §3 (per-page slicing).  Consumes FM01 §2.3.6
(`usedStyle` accumulator).  No spec divergences.

### Behavioural notes

- **sha256 is over the UNSCOPED CSS bytes.**  Pages with identical
  `usedRuleIds` get identical fingerprints; downstream caches can
  deduplicate by content while still serving per-page-scoped CSS
  to the browser.  Distinct from the scoped `css` field which is
  per-page unique.
- **Two `translateToCss` calls per page is deliberate.**  The
  alternative (hash the scoped output) would couple the cache key
  to the `scopePrefix` choice — pages differing only in their
  scope function would be cache misses despite identical content.
  Per-page translate is O(rules-per-page), typically tiny.
- **`byteSize` is `Buffer.byteLength(css, "utf8")`** — exact UTF-8
  bytes, not character count.  Drives size budgets / reports.
- **Page iteration order = caller's array order.**  The returned
  `Map` preserves insertion order.  No sort.
- **Empty pages array → empty `artefacts` Map.**

### Security posture

- **Page ID sanitisation.**  The default scope hashes the page id
  through sha256 first, so the scope is always
  `[#][p][-][0-9a-f]{8}` regardless of what bytes the page id
  contains.  No CSS injection surface from hostile page ids
  under the default scope.  Tests pin survival for `"\x00\x1b[31m"`,
  `"<script>"`, `"💥"`, etc.
- **Scope isolation across pages.**  Two pages using the same rule
  emit CSS with different `#p-` prefixes (different sha256 of the
  page id), so concatenating per-page CSS files cannot produce
  cross-page selector collisions.  Tests pin both directions
  (scopeA in pageA's CSS only; scopeB in pageB's CSS only).
- **sha256, not a weak hash.**  Cache-key collisions are
  catastrophic for an incremental rebuilder; sha256 makes them
  computationally infeasible.

### Capabilities

`["hash"]` — `node:crypto.createHash("sha256")` for the
content-addressed fingerprint AND the default scope.  No I/O,
no network, no shell.

### Tests

22 tests in `slicer.test.ts`:

- Basic slicing (per-page artefact, emittedRules subset, byteSize,
  scope per page differs, empty page)
- Content-addressed sha256 (dedup-friendly equality across same-
  content pages; differs for different `usedRuleIds`; UNSCOPED
  fingerprint; stable across runs; 64 hex chars)
- `defaultScopePrefix` (`#p-XXXXXXXX` shape; deterministic;
  no obvious collisions; first 8 chars match `sha256(pageId)`;
  hostile page ids produce safe scopes)
- Custom `scopePrefix` (caller override; empty-string no-op)
- Warning propagation (per-page isolation)
- Page iteration order preserved
- Cross-page scope isolation
- Empty pages array → empty Map

Coverage: **100% line / 100% branch** — well above the FM04 §14.4
≥95% line target.

### v0 simplifications (documented)

- **No incremental cache.**  Recomputes every page on every call;
  FM06 §4 (incremental rebuilds) is the future package that wraps
  this one.
- **CSS only.**  LaTeX / terminal per-page slicing would follow
  the same pattern but isn't shipped yet.
