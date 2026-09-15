## 0.220.3 - 2026-07-30 (native records — run-verify the movable-alloc change)

Test-only. `e6d6_llvm_records` gains `records_run_on_native`: a real Twig record program
(`(record Point (x : int) (y : int)) (point-x (Point 42 7))` and the `point-y` sibling) is compiled
to a host executable and run, proving the record cells still round-trip after the native `alloc` op
switched to the movable `{0,8}` pair allocator (`__twig_gc_alloc_pair`; aarch64-backend 0.34.0 /
x86_64-backend 0.36.0). The GC-relocation proof itself is the twig-aot smoke differential
`end_to_end_gc_record_field_traced_and_relocated`. No production-source change.

