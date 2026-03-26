"""CPU Pipeline -- the assembly line at the heart of every CPU.

This package implements a configurable N-stage CPU instruction pipeline.
Instead of completing one instruction fully before starting the next,
a pipelined CPU overlaps instruction execution -- while one instruction
is being executed, the next is being decoded, and the one after that
is being fetched.

The classic 5-stage pipeline:

    Stage 1: IF  (Instruction Fetch)  -- read instruction from memory at PC
    Stage 2: ID  (Instruction Decode) -- decode opcode, read registers
    Stage 3: EX  (Execute)            -- ALU operation, branch resolution
    Stage 4: MEM (Memory Access)      -- load/store data from/to memory
    Stage 5: WB  (Write Back)         -- write result to register file

Quick start:

    from cpu_pipeline import Pipeline, classic_5_stage

    config = classic_5_stage()
    pipeline = Pipeline(
        config, fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn
    )
    stats = pipeline.run(max_cycles=100)
    print(f"IPC: {stats.ipc():.3f}")
"""

from cpu_pipeline.pipeline import (
    DecodeFunc,
    ExecuteFunc,
    FetchFunc,
    HazardAction,
    HazardFunc,
    HazardResponse,
    MemoryFunc,
    Pipeline,
    PredictFunc,
    WritebackFunc,
)
from cpu_pipeline.snapshot import PipelineSnapshot, PipelineStats
from cpu_pipeline.token import (
    PipelineConfig,
    PipelineStage,
    PipelineToken,
    StageCategory,
    classic_5_stage,
    deep_13_stage,
    new_bubble,
    new_token,
)

__all__ = [
    "DecodeFunc",
    "ExecuteFunc",
    "FetchFunc",
    "HazardAction",
    "HazardFunc",
    "HazardResponse",
    "MemoryFunc",
    "Pipeline",
    "PipelineConfig",
    "PipelineSnapshot",
    "PipelineStage",
    "PipelineStats",
    "PipelineToken",
    "PredictFunc",
    "StageCategory",
    "WritebackFunc",
    "classic_5_stage",
    "deep_13_stage",
    "new_bubble",
    "new_token",
]
