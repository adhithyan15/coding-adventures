## 2026-10-08 (solid per-edge borders leave native degradation reports)

`border-{top,right,bottom,left}-style: solid` is no longer reported as
dropped when SwiftUI actually lowers the same positive-width edge. Unsupported,
zero-width, and style-only declarations remain explicit degradations (#17098).
