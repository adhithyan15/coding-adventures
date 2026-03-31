# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-31

### Added

- DrawRectInstruction with fill, stroke, and stroke_width support
- DrawTextInstruction with font_weight support
- DrawLineInstruction for straight line segments
- DrawGroupInstruction for hierarchical grouping
- DrawClipInstruction for rectangular clipping regions
- DrawScene top-level container
- Convenience constructors: draw_rect, draw_text, draw_group, draw_line, draw_clip, create_scene
- render_with delegation to duck-typed renderers
- Frozen structs for immutability
