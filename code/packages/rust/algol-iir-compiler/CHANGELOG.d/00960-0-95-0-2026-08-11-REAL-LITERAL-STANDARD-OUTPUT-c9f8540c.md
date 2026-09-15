## 0.95.0 — 2026-08-11 — real-literal standard output

Implementation-defined `print` and `output` calls now accept a direct real
literal and print its deterministic source spelling through the shared string
path. Runtime `f64` values remain explicit type errors until a portable typed
formatter ABI is available.

