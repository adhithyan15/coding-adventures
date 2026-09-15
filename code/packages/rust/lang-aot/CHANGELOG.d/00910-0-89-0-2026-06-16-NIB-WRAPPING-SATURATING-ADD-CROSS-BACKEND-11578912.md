## 0.89.0 — 2026-06-16 — Nib `+%` wrapping / `+?` saturating add cross-backend (LANG-FULL N7)

Four new matrix programs prove Nib's N7 operators run on all 7 backends: `200u8 +% 100`
→ wraps to `44`, `200u8 +? 100` → saturates to `255`, `15u4 +? 1` → clamps to `15`,
and `3 +? 4` → `7` (no clamp). Comparison-based so they distinguish saturate/wrap from
the unclamped sum. (nib-iir-compiler 0.15.0.)

