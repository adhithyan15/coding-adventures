# cpu-pipeline (C)

A **configurable N-stage CPU instruction pipeline** simulator in pure ISO C17. A
faithful port of the Rust [`cpu-pipeline`](../../rust/cpu-pipeline) crate. Pairs
with the ported [`cpu-cache`](../cpu-cache) as a microarchitecture toolkit.

## What it models

The pipeline manages the **flow** of instructions through stages (IF → ID → EX →
MEM → WB, or deeper variants); it does not interpret instructions — the ISA work
is injected via callbacks. Each `step()` (one clock cycle):

- checks for hazards (a callback returns stall / flush / forward / none),
- advances tokens one stage, inserting bubbles on stalls and flushes,
- runs the per-stage callbacks (fetch, decode, execute, memory, writeback),
- retires the instruction in the last stage, and
- records a snapshot and updates statistics (IPC/CPI, stall/flush/bubble cycles).

Presets: `cp_config_classic_5_stage` (MIPS R2000-style) and
`cp_config_deep_13_stage` (Cortex-A78-style). Custom stage lists are supported.

## Port shape

The Rust `PipelineToken` uses a `HashMap<String,i64>` and `String` fields. To
keep the C port heap-light and memory-safe, `CpToken` is a fixed-size **plain
value type** (opcode/stage-entry data in bounded arrays), so tokens copy by
assignment with no per-token allocation. A pipeline may therefore have at most
`CP_MAX_STAGES` (16) stages — the deepest preset (13) fits — and
`cp_config_validate` rejects more. Callbacks take a `void *ctx` user-data
pointer (the C stand-in for Rust's captured closures).

## API

```c
#include "cpu_pipeline.h"

CpPipelineConfig cfg;
cp_config_classic_5_stage(&cfg);

char err[128];
CpPipeline *p = cp_pipeline_new(&cfg, fetch, &fctx, decode, NULL, execute, NULL,
                                memory, NULL, writeback, &wctx, err, sizeof err);
CpPipelineStats st = cp_pipeline_run(p, 100);   /* step to HLT or 100 cycles */
cp_pipeline_free(p);
```

- `cp_pipeline_new` validates the config (returns NULL + message on failure) /
  `cp_pipeline_free`.
- `cp_pipeline_step` / `cp_pipeline_run`, `cp_pipeline_set_hazard_fn` /
  `_set_predict_fn`, state accessors, `cp_pipeline_stage_contents`, and
  `cp_pipeline_trace` / `_trace_count` for the snapshot history.

The snapshot-history buffer guards `size_t` overflow. Verified clean under
ASan + UBSan and the macOS `leaks` tool (0 leaks).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
