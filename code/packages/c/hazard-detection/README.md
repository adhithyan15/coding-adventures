# hazard-detection (C)

Pipeline hazard detection for a classic 5-stage CPU, in **pure ISO C17**. A
faithful port of the Rust [`hazard-detection`](../../rust/hazard-detection)
crate.

## What it does

Detects **data**, **control**, and **structural** hazards in an in-order
5-stage pipeline and decides the action — *forward*, *stall*, or *flush*:

- **Data (RAW):** an ID-stage source register is being written by EX or MEM →
  forward from EX/MEM, or stall on a load-use hazard.
- **Control:** a branch in EX was mispredicted → flush IF and ID.
- **Structural:** ID and EX need the same execution unit, or IF and MEM share a
  single-cache memory port → stall.

Priority (most severe wins): `FLUSH > STALL > FORWARD_EX > FORWARD_MEM > NONE`.

## API

- `HdPipelineSlot` — an ISA-independent stage snapshot; zero-initialise (`{0}`)
  for an empty bubble, then set fields. `source_regs` is a borrowed array.
- `hd_data_detect` / `hd_control_detect` / `hd_structural_detect` — the three
  stateless detectors (each returns a value-type `HdHazardResult`).
- `hd_pick_higher_priority`, `hd_priority`.
- `HdHazardUnit` (`hd_unit_init` / `hd_unit_free` / `hd_unit_check`) — runs all
  three, keeps the highest-priority result, and records history for
  `hd_unit_stall_count` / `hd_unit_flush_count` / `hd_unit_forward_count`.

## Design notes

- **No allocation in the hot path.** `HdHazardResult` is a plain value type with
  inline fixed-size `reason` / `forwarded_from` buffers; `Option`s become
  `has_*` flags. The only heap owner is the unit's history (paired
  init/free, overflow-guarded growth).
- **Borrowed inputs.** `source_regs` and the slot pointers are never owned;
  `if_stage` / `mem_stage` may be `NULL` (the Rust `Option`).

## Usage

```c
#include "hazard_detection.h"

uint32_t srcs[1] = {1};
HdPipelineSlot id = {0}; id.valid = 1; id.source_regs = srcs; id.num_source_regs = 1;
HdPipelineSlot ex = {0}; ex.valid = 1; ex.has_dest_reg = 1; ex.dest_reg = 1;
ex.has_dest_value = 1; ex.dest_value = 42;
HdPipelineSlot mem = {0};
HdHazardResult r = hd_data_detect(&id, &ex, &mem);   /* ForwardFromEX, value 42 */
```

## Building

```sh
sh BUILD           # POSIX: GCC and/or Clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
