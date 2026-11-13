---
category: Native extensions & FFI
---

# An empty f64 sum is -0.0 in Rust and OTP 27 tells -0.0 from 0.0, so an Erlang smoke check that passed on OTP 25 failed on Windows OTP 27

`values.iter().sum::<f64>()` over an empty slice returns `-0.0` in current Rust (the
additive identity for floats is `-0.0`). Erlang/OTP 25 treats `-0.0 =:= 0.0` as true,
so `nif_smoke:sum_floats([])` compared equal to `0.0` on Linux with OTP 25; OTP 27
distinguishes them, and the same check failed (`expected 0.0, got -0.0`) when the
Windows DLL was run against the real OTP 27.3.4.11 that CI installs.

Write `0.0 + values.iter().sum::<f64>()` (`-0.0 + 0.0` is `0.0`) wherever an empty sum
crosses into a host that compares exactly, and run the smoke test against the same
major OTP that CI pins, not only the one `apt` provides. (Lua and Perl compare
`-0.0 == 0.0` as true and are unaffected.)
