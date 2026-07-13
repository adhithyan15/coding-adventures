# cpu-pipeline (C++)

A **configurable N-stage CPU instruction pipeline** simulator — header-only,
ISO C++17. A faithful port of the Rust [`cpu-pipeline`](../../rust/cpu-pipeline)
crate, in namespace `ca::cpu_pipeline`. Pairs with the ported
[`cpu-cache`](../cpu-cache).

## What it models

The pipeline manages the **flow** of instructions through stages (IF → ID → EX →
MEM → WB, or deeper variants); the ISA work is injected via `std::function`
callbacks. Each `step()` checks for hazards, advances tokens (inserting bubbles
on stalls/flushes), runs the per-stage callbacks, retires the last stage, and
records a snapshot plus statistics (IPC/CPI, stall/flush/bubble cycles).

This port mirrors the Rust structure directly: `PipelineToken` carries a
`std::unordered_map<std::string,int64_t>`, callbacks are `std::function`, and
pipeline slots are `std::optional<PipelineToken>`. Where the Rust
`Pipeline::new` returns `Result`, this port throws `std::invalid_argument`.

## API

```cpp
#include "cpu_pipeline.hpp"
namespace cp = ca::cpu_pipeline;

cp::Pipeline p(cp::PipelineConfig::classic_5_stage(), fetch, decode, execute,
               memory, writeback);   // callbacks are std::function
p.set_hazard_fn(hazard);             // optional
auto stats = p.run(100);             // step to HLT or 100 cycles
```

- `PipelineConfig::classic_5_stage()` / `deep_13_stage()`, `validate()`.
- `step()` / `run()`, `set_hazard_fn` / `set_predict_fn`, `stage_contents`,
  `snapshot()`, and `trace()` for the snapshot history. Value semantics
  throughout (RAII); no manual memory management.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan.
