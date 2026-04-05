# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added

- `MosaicWebComponentRenderer` — implements `MosaicRenderer` from `mosaic-vm`
- Emits typed Custom Element classes (`.ts`) from `MosaicIR`
- Shadow DOM rendering via `attachShadow({ mode: 'open' })` + `innerHTML` assignment
- Fragment-tree architecture: accumulates `RenderFragment` objects during VM
  traversal, then serializes to `html +=` statements in `_render()`
- Single-quoted JS string literals in generated `html +=` statements so that
  standard HTML attribute double-quotes (`role="button"`) appear unescaped
- Custom element naming conventions: `ProfileCard` → tag `mosaic-profile-card`,
  class `MosaicProfileCardElement`
- Backing private fields for all slot types with TypeScript types:
  `text→string`, `number→number`, `bool→boolean`, `image→string`, `list<T>→T[]`
- `static get observedAttributes()` lists all primitive slot names
- `attributeChangedCallback` with per-slot type coercion
- Typed getters and setters that call `this._render()` on update
- `when @flag { ... }` → `if (this._flag) { html += ...; }` conditional rendering
- `each @items as item { ... }` → `this._items.forEach((item, _index) => { ... })`
  iteration; node-typed slot lists use indexed slot names
- Light DOM projection for `node`-typed slots: inserts `<slot name="..."></slot>`
  into shadow DOM; manages dynamic re-projection via `connectedCallback` /
  `disconnectedCallback` observers
- `_escapeHtml()` helper: guards all dynamic text content from XSS
- Image `src` setter: validates against `javascript:` URL scheme
- Inline CSS generation: kebab-case property names, `dp`/`sp` → `px`, `%` → `%`,
  `rgba(r, g, b, alpha)` color format
- Accessibility: `a11y-role` → `role` attribute, `a11y-hidden` → `aria-hidden`
- `customElements.define(tag, class)` at end of file
- 78 tests, 100% coverage
