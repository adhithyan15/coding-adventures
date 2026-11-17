# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `flash-display-list` (`SWF05`).
- Display object hierarchy: `Stage`, `MovieClip`, `SimpleButton`, `TextField`, and `MorphShape`.
- Depth-sorted child slot management and timeline state machine (`Play`, `Stop`, `gotoAndPlay`, `gotoAndStop`).
- SWF depth op parsers and applicators for `PlaceObject`, `PlaceObject2`, `PlaceObject3`, `RemoveObject`, `RemoveObject2`.
- Stencil mask clipping lowering to `PaintInstruction::PaintClip` (`P2D00`).
- Retained-mode `PaintScene` generation and diffing via `paint-palette-blend::diff_paint_scene_into` (`P2D10`).
- Strict security limits: `MAX_HIERARCHY_DEPTH = 64`, depth validation, and bounds checking.
- Conformance, microbenchmark, and runtime capability test suites.
