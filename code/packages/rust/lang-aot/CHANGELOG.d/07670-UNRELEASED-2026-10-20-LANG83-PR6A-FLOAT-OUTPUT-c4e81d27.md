## 0.582.0 (UNRELEASED) — LANG83 PR6a shortest round-trip float output

- LANG83 PR6a — `write_f64_py` / `write_f64_js` on every backend. `tests/op_conformance.rs` gains four cases:
  - a 90-value table through each writer, with one line per value. It holds the layout edges (`±0.0`, `±inf`, `nan`, `1.0`, `100.0`, `1e15`, `1e16`, `1e20`, `1e21`, `1e-4` through `1e-7`, `5e-324`, `±max`, the `2^-25` and `1072012167405248.25` round-half-even ties, `2e23`, `0.1 + 0.2`) plus 60 seeded random doubles. The expected strings are printed by host `python3` (`repr`) and `node` (`String(x)`);
  - an ordered-stream case that interleaves `write_i64`, `print_i64` and `putchar` with the writers (`-0.0` → `0` in JS, `-0.0` in Python, `1e21` → `1e+21`);
  - a case that prints computed floats (`0.1 + 0.2`, `int_to_real 7`).

  New gates:
  - `pr6a_ops_are_lowered_on_every_backend`: all nine backends lower both writers, and both table cases run and pass on WASM, VM, JIT, LLVM, native and JVM whenever their tools exist. Locally all six run.
  - `float_table_matches_host_python3_and_node`: re-runs both oracles on the checked-in table whenever they are installed.

  The VM and JIT runners route the writers to the shared ordered sink through `vm_core::float_format`. CLR and BEAM are compile-only here, and their crates' model tests execute the helpers.
