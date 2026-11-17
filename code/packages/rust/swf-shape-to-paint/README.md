# swf-shape-to-paint

Dual-fill Bézier shape converter lowering SWF `DefineShape` records to `P2D00` `PaintInstructions`.

Part of the **Coding Adventures** Flash Player suite (`SWF02`, see `code/specs/SWF00-flash-runtime-roadmap.md`).

---

## Overview

`swf-shape-to-paint` implements safe, deterministic parsing, dual-fill topology reconstruction, and lowering of Adobe Shockwave Flash (`.swf`) vector shape definitions into universal 2D paint intermediate representations (`P2D00`):

1. **Shape Tag Parsing**:
   - Parses `DefineShape` (Tag 2), `DefineShape2` (Tag 22), `DefineShape3` (Tag 32), and `DefineShape4` (Tag 83).
   - Supports `StyleChangeRecord`, `StraightEdgeRecord`, `CurvedEdgeRecord`, and `EndShapeRecord`.
   - Parses solid color fills (RGB and RGBA), linear & radial gradient fills (up to 15 color stops, spread modes, focal points), and bitmap fills (clipped/tiled).
   - Parses line styles (`LINESTYLE` and `LINESTYLE2`) with widths in twips, cap styles (`Round`, `Butt`, `Square`), join styles (`Round`, `Bevel`, `Miter`), and miter limits.
2. **Coordinate System & Twips**:
   - Twips-to-pixel coordinate conversion ($1\text{ twip} = \frac{1}{20}\text{ pixel} = 0.05\text{ px}$).
   - Strict checked integer arithmetic guards against integer overflow (CWE-190).
3. **Dual-Fill Edge Loop Reconstruction**:
   - Reconstructs closed directed polygon contours from dual-tagged edge segments (`FillStyle0` left side, `FillStyle1` right side).
   - Pairs matching start/end points into closed contours, accurately separating exterior boundaries from interior island cutouts.
   - Preserves contour winding semantics (`FillRule::EvenOdd` for legacy shapes, `FillRule::NonZero` when `UsesFillWindingNos` is set in `DefineShape4`).
   - Gathers contiguous stroke edge runs into unified stroked paths.
4. **Lowering to `paint-instructions` (`P2D00`)**:
   - Lowers shapes to `PaintScene`, `PaintPath`, `PaintGradient`, `PaintImage`, and `PathCommand` (`MoveTo`, `LineTo`, `QuadTo`, `Close`).
   - Uses `bezier2d` (`G2D02`) for quadratic Bézier curve evaluation, bounding boxes, and polyline operations.
   - Outputs valid CSS RGBA colors for fills and strokes.

---

## Architectural Guarantees

- **`CAP001` (Zero Ambient Authority):** `"capabilities": []` in `required_capabilities.json`.
- **`CAP002` (Zero Unsafe):** `#![deny(unsafe_code)]`.
- **Zero Allocation Hot Paths (`VER07`):** Coordinate transforms, Bézier evaluations, and edge pairing helpers operate with zero heap allocation, verified via `CountingAllocator` TAP microbenchmarks.
- **CWE Weakness Closures (`VER15`):** Complete 26-CWE matrix in `assurance_report.json` with machine-checked defenses against CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, and CWE-787.
