# Changelog

## 0.01 — 2026-03-31

### Added
- Initial release
- `render_svg($scene)` function that serialises a draw instruction scene to SVG
- Support for all instruction kinds: rect, text, line, circle, clip, group
- XML escaping for text content and attribute values
- Metadata serialisation as `data-*` attributes
- Accessible SVG output with `role="img"` and `aria-label`
- Stroke and stroke-width support on rect instructions
- Font-weight support on text instructions (omitted when "normal" or undef)
- Deterministic clip IDs via counter reset per render call
