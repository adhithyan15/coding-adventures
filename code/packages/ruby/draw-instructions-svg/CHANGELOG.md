# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-31

### Added

- SvgRenderer class implementing duck-typed render(scene) interface
- render_svg convenience method for direct scene-to-SVG conversion
- Support for rect (with stroke), text (with font_weight), line, group, clip
- Clip regions using unique IDs (counter resets each render)
- Metadata serialized as data-* attributes
- XML escaping for all user-provided text and attributes
- Complete SVG document output with viewBox, role, aria-label
