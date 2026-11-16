# flash-display-list

Interactive Flash display-object hierarchy, timeline FSM, and retained-mode `PaintScene` generation and diffing.

Part of the **Coding Adventures** Flash Player suite (`SWF05`, see `code/specs/SWF00-flash-runtime-roadmap.md`).

---

## Overview

`flash-display-list` implements the core interactive scene graph, timeline state machine, and retained-mode render call diffing for the Shockwave Flash (`.swf`) runtime:

1. **Display Object Hierarchy**:
   - `Stage`: Root rendering canvas containing stage dimensions, background color, frame rate, focus, and mouse input states.
   - `MovieClip`: Self-contained timeline entity with current frame, total frames, `PlayState::{Play, Stop}`, frame labels, depth-sorted child slots, local transforms (`SwfMatrix`), and color transforms (`SwfColorTransform`).
   - `SimpleButton`: 4-state interactive button (`Up`, `Over`, `Down`, `HitTest`) responding to mouse move, mouse down, and mouse up events.
   - `TextField`: Dynamic and input text renderer with ActionScript variable binding, font size, border, and background support.
   - `MorphShape`: Tweened vector shapes interpolated across frame ratios `0..=65535`, integrating `swf-shape-to-paint` (`SWF02`).
2. **Timeline Frame Tick & Bounds Enforcement**:
   - Depth-sorted child management with depth limits (`MIN_DEPTH = -16384`, `MAX_DEPTH = 65535`).
   - Parses and applies `PlaceObject`, `PlaceObject2`, `PlaceObject3`, `RemoveObject`, and `RemoveObject2` depth tags.
   - Advances playing movie clip timelines, with frame looping and label resolution (`gotoAndPlay`, `gotoAndStop`).
   - Enforces `MAX_HIERARCHY_DEPTH = 64`, terminating cyclic or excessively deep hierarchies with `DisplayListError::HierarchyDepthExceeded`.
   - Stencil mask clipping: `clip_depth: Some(D)` lowers to `PaintInstruction::Clip` (`paint-instructions` `P2D00`).
3. **Retained-Mode Diffing**:
   - Traverses visible display hierarchy in depth order, emitting `paint-instructions::PaintScene` with stable node identifiers.
   - Integrates `paint-palette-blend::diff_paint_scene_into` (`P2D10`) for zero-allocation patch generation across consecutive frames.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![forbid(unsafe_code)]`.
- **Zero Allocation Hot Paths (`VER07`):** Steady-state timeline frame ticking, hierarchy bounds checks, and scene diffing into pre-allocated patch buffers operate with zero heap allocation, verified via `CountingAllocator` TAP microbenchmarks.
- **CWE Weakness Closures (`VER15`):** Complete 26-CWE matrix in `assurance_report.json` with machine-checked defenses against CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, and CWE-787.
