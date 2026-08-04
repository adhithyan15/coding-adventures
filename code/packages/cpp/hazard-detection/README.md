# hazard-detection (C++)

Pipeline hazard detection for a classic 5-stage CPU, **header-only** in pure ISO
C++17 (namespace `ca::hazard_detection`). A faithful port of the Rust
[`hazard-detection`](../../rust/hazard-detection) crate.

## What it does

Detects **data**, **control**, and **structural** hazards in an in-order
5-stage pipeline and decides the action — *forward*, *stall*, or *flush* — with
priority `Flush > Stall > ForwardFromEX > ForwardFromMEM > None`.

## API

- `PipelineSlot` — an ISA-independent stage snapshot (`std::vector` source regs,
  `std::optional` dest reg/value, bool flags).
- `DataHazardDetector`, `ControlHazardDetector`,
  `StructuralHazardDetector(num_alus, num_fp_units, split_caches)` — each with a
  `detect(...)` returning a `HazardResult`.
- `pick_higher_priority`, `priority`.
- `HazardUnit(num_alus, num_fp_units, split_caches)` — `check(...)` runs all
  three and records history; `history()`, `stall_count()`, `flush_count()`,
  `forward_count()`.

## Design notes

- **Value semantics.** `PipelineSlot` / `HazardResult` are plain structs built
  from `std::optional` / `std::vector` / `std::string`; the `HazardUnit`'s
  `Option<&PipelineSlot>` structural arguments become nullable const pointers.
- **Header-only.** `#include "hazard_detection.hpp"` and go.

## Usage

```cpp
#include "hazard_detection.hpp"
using namespace ca::hazard_detection;

PipelineSlot id; id.valid = true; id.source_regs = {1};
PipelineSlot ex; ex.valid = true; ex.dest_reg = 1; ex.dest_value = 42;
PipelineSlot mem;  // empty
auto r = DataHazardDetector{}.detect(id, ex, mem);   // ForwardFromEX, value 42
```

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
