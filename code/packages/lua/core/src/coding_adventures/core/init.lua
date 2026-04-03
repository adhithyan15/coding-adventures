-- init.lua — CPU Core: the integration point
--
-- The Core wires together all D-series sub-components into a working
-- processor. Think of it like a motherboard: the Core doesn't define
-- new behavior, it connects the parts and routes signals between them.
--
-- WHAT THE CORE PROVIDES:
--
--   Pipeline     — manages instruction flow through stages
--   RegisterFile — fast operand storage (16 registers, 32-bit)
--   MemoryController — access to backing memory
--   ISA Decoder  — INJECTED from outside (not part of the Core itself)
--
-- DESIGN PHILOSOPHY: The Core knows HOW to move instructions through a
-- pipeline, detect hazards, access caches. But it does NOT know WHAT
-- instructions mean. That's the ISA decoder's job.
--
-- This separation means the same Core works with any instruction set:
--   core = Core.new(config, arm_decoder)
--   core = Core.new(config, riscv_decoder)
--
-- THE ISA DECODER PROTOCOL:
-- An ISA decoder must provide:
--   decoder.decode(raw_instruction, token) → token
--   decoder.execute(token, register_file)  → token
--   decoder.instruction_size()             → 4 (bytes)
--
-- FUNCTIONAL DESIGN:
-- The Core is an immutable table. Each call to step() returns a new
-- Core with updated state. There is no mutation.
--
-- CORE CONFIGURATION:
--
--   CoreConfig.simple()        — 5-stage, no frills, good for learning
--   CoreConfig.performance()   — 13-stage, with forwarding and stall detection
--
-- LAYER POSITION:
--
--   ISA Simulators (ARM, RISC-V) ← inject decoder here
--          ↓
--   Core (D05) ← YOU ARE HERE
--          ↓
--   Pipeline (D04), Hazard Detection (D03), Cache (D01), CPU Simulator

local Pipeline   = require("coding_adventures.cpu_pipeline.pipeline")
local config_mod = require("coding_adventures.cpu_pipeline.config")
local cpu_sim    = require("coding_adventures.cpu_simulator")
local RegisterFile = cpu_sim.RegisterFile
local Memory       = cpu_sim.Memory

local PipelineConfig = config_mod.PipelineConfig
local HazardResponse = config_mod.HazardResponse

-- ========================================================================
-- MemoryController
-- ========================================================================
--
-- Wraps Memory with a latency model. In real hardware, a cache miss
-- causes the CPU to stall while waiting for DRAM. We model this with
-- a `latency` parameter (currently informational only — we don't inject
-- stalls from cache misses in this simplified model).

local MemoryController = {}
MemoryController.__index = MemoryController

function MemoryController.new(size, latency)
    return setmetatable({
        memory  = Memory.new(size),
        size    = size,
        latency = latency or 100,  -- DRAM latency in cycles (informational)
    }, MemoryController)
end

function MemoryController:read_word(address)
    return self.memory:read_word(address)
end

function MemoryController:write_word(address, value)
    self.memory:write_word(address, value)
end

function MemoryController:load_program(bytes, start_address)
    self.memory:load_bytes(start_address, bytes)
end

-- ========================================================================
-- CoreConfig
-- ========================================================================
--
-- CoreConfig describes the micro-architectural parameters of a core.
-- Every parameter is a tradeoff:
--
--   More pipeline stages → higher clock, worse misprediction penalty
--   Larger register file → fewer spills, but more chip area
--   More cache → fewer misses, but more area and power
--
-- Two presets are provided. You can create custom configs by constructing
-- CoreConfig directly.

local CoreConfig = {}
CoreConfig.__index = CoreConfig

--- Creates a CoreConfig.
--
-- @param opts  table  Optional configuration fields:
--   name             string  Human-readable name (default "Core")
--   pipeline_config  PipelineConfig  (default: classic_5_stage)
--   num_registers    number  (default 16)
--   register_width   number  bits per register (default 32)
--   memory_size      number  bytes (default 65536)
--   memory_latency   number  cycles (default 100)
-- @return CoreConfig
function CoreConfig.new(opts)
    opts = opts or {}
    return setmetatable({
        name           = opts.name           or "Core",
        pipeline_config = opts.pipeline_config or PipelineConfig.classic_5_stage(),
        num_registers  = opts.num_registers  or 16,
        register_width = opts.register_width or 32,
        memory_size    = opts.memory_size    or 65536,
        memory_latency = opts.memory_latency or 100,
    }, CoreConfig)
end

--- Simple 5-stage core — great for learning.
--
-- Equivalent to a 1980s microcontroller:
--   5-stage pipeline, 16 registers, 64KB memory
function CoreConfig.simple()
    return CoreConfig.new({
        name           = "Simple",
        pipeline_config = PipelineConfig.classic_5_stage(),
        num_registers  = 16,
        register_width = 32,
        memory_size    = 65536,
        memory_latency = 100,
    })
end

--- Performance 13-stage core — inspired by ARM Cortex-A78.
--
-- Higher clock speed, better throughput, but worse misprediction penalty.
function CoreConfig.performance()
    return CoreConfig.new({
        name           = "Performance",
        pipeline_config = PipelineConfig.deep_13_stage(),
        num_registers  = 31,
        register_width = 64,
        memory_size    = 65536,
        memory_latency = 100,
    })
end

-- ========================================================================
-- CoreStats
-- ========================================================================

local CoreStats = {}
CoreStats.__index = CoreStats

function CoreStats.new()
    return setmetatable({
        total_cycles           = 0,
        instructions_completed = 0,
        stall_cycles           = 0,
        flush_cycles           = 0,
    }, CoreStats)
end

function CoreStats:ipc()
    if self.total_cycles == 0 then return 0.0 end
    return self.instructions_completed / self.total_cycles
end

function CoreStats:to_string()
    return string.format(
        "CoreStats{cycles=%d, completed=%d, IPC=%.3f, stalls=%d, flushes=%d}",
        self.total_cycles, self.instructions_completed, self:ipc(),
        self.stall_cycles, self.flush_cycles
    )
end

-- ========================================================================
-- Core
-- ========================================================================
--
-- The Core ties together pipeline, register file, memory controller,
-- and an ISA decoder. It delegates all instruction semantics to the decoder.
--
-- USAGE:
--
--   1. Create a core:      local core = Core.new(config, decoder)
--   2. Load a program:     core:load_program(bytes, start_addr)
--   3. Step/run:           local snap = core:step()
--                          core:run(1000)
--   4. Read results:       core:read_register(0)
--
-- INTERNAL ARCHITECTURE:
--
--   The core holds a pipeline, register file, and memory controller.
--   The pipeline's callbacks are closures over the core's state tables.
--   Since Lua tables are passed by reference, callbacks can read/write
--   the register file and memory directly.
--
--   ┌─────────────────────────────────────────────────────────────┐
--   │  Core                                                        │
--   │  ┌──────────┐  fetch_fn  ┌────────────────────────┐         │
--   │  │ Memory   │←──────────→│                        │         │
--   │  │ Controller│           │  Pipeline (D04)        │         │
--   │  │          │  memory_fn │                        │         │
--   │  │          │←──────────→│  IF → ID → EX → MEM → WB        │
--   │  └──────────┘            │                        │         │
--   │                          │        ↑ writeback_fn  │         │
--   │  ┌──────────┐            │        │               │         │
--   │  │Register  │←───────────┘        │               │         │
--   │  │ File     │←────────────────────┘               │         │
--   │  └──────────┘  decode_fn / execute_fn              │         │
--   │                                     ↑              │         │
--   │  ┌──────────┐                       │              │         │
--   │  │ISA       │───────────────────────┘              │         │
--   │  │ Decoder  │  (injected)                          │         │
--   └──┴──────────┴──────────────────────────────────────┘         │
--   └─────────────────────────────────────────────────────────────┘

local Core = {}
Core.__index = Core

--- Creates a new CPU Core.
--
-- @param config   CoreConfig
-- @param decoder  table  Must implement decode(raw, token), execute(token, rf),
--                        and instruction_size() methods
-- @return {ok=true, core=...} | {ok=false, err="..."}
function Core.new(config, decoder)
    -- 1. Build the register file
    local reg_file = RegisterFile.new(config.num_registers, config.register_width)

    -- 2. Build the memory controller
    local mem_ctrl = MemoryController.new(config.memory_size, config.memory_latency)

    -- 3. Build state tables (passed by reference to callbacks)
    local state = {
        reg_file = reg_file,
        mem_ctrl = mem_ctrl,
        halted   = false,
        cycle    = 0,
        stats    = CoreStats.new(),
    }

    -- 4. Build the pipeline callbacks
    --    These closures capture `state` and `decoder` by reference.

    local function fetch_fn(pc)
        return state.mem_ctrl:read_word(pc)
    end

    local function decode_fn(raw, token)
        return decoder.decode(raw, token)
    end

    local function execute_fn(token)
        return decoder.execute(token, state.reg_file)
    end

    local function memory_fn(token)
        if token.mem_read then
            local data = state.mem_ctrl:read_word(token.alu_result)
            token.mem_data   = data
            token.write_data = data
        elseif token.mem_write then
            state.mem_ctrl:write_word(token.alu_result, token.write_data)
        end
        return token
    end

    local function writeback_fn(token)
        if token.reg_write and token.rd >= 0 then
            state.reg_file:write(token.rd, token.write_data)
        end
    end

    -- 5. Create the pipeline
    local result = Pipeline.new(
        config.pipeline_config,
        fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn
    )

    if not result.ok then
        return { ok = false, err = result.err }
    end

    local pipeline = result.pipeline

    -- Set predict function using the decoder's instruction size
    pipeline:set_predict_fn(function(pc)
        return pc + decoder.instruction_size()
    end)

    -- Build the core
    local core = setmetatable({
        config   = config,
        decoder  = decoder,
        pipeline = pipeline,
        state    = state,
    }, Core)

    return { ok = true, core = core }
end

--- Loads machine code into memory and sets the PC.
-- @param bytes          table   List of byte values
-- @param start_address  number  Address to load at (default 0)
function Core:load_program(bytes, start_address)
    start_address = start_address or 0
    self.state.mem_ctrl:load_program(bytes, start_address)
    self.pipeline:set_pc(start_address)
end

--- Advances the core by one clock cycle.
-- @return Snapshot  The pipeline state after this cycle
function Core:step()
    local snap = self.pipeline:step()
    self.state.cycle = self.state.cycle + 1

    -- Sync stats from pipeline
    local ps = self.pipeline:get_stats()
    self.state.stats.total_cycles           = ps.total_cycles
    self.state.stats.instructions_completed = ps.instructions_completed
    self.state.stats.stall_cycles           = ps.stall_cycles
    self.state.stats.flush_cycles           = ps.flush_cycles

    return snap
end

--- Runs the core until halted or max_cycles reached.
-- @param max_cycles  number  (default 10000)
-- @return CoreStats
function Core:run(max_cycles)
    max_cycles = max_cycles or 10000
    while not self.pipeline:is_halted() and self.state.cycle < max_cycles do
        self:step()
    end
    return self.state.stats
end

--- Returns true if the pipeline has halted.
function Core:is_halted()
    return self.pipeline:is_halted()
end

--- Returns the current cycle number.
function Core:get_cycle()
    return self.state.cycle
end

--- Returns current execution statistics.
function Core:get_stats()
    return self.state.stats
end

--- Reads a register value (0-based index).
function Core:read_register(index)
    return self.state.reg_file:read(index)
end

--- Writes a register value (0-based index).
function Core:write_register(index, value)
    self.state.reg_file:write(index, value)
end

--- Reads a word from memory.
function Core:read_memory_word(address)
    return self.state.mem_ctrl:read_word(address)
end

--- Writes a word to memory.
function Core:write_memory_word(address, value)
    self.state.mem_ctrl:write_word(address, value)
end

--- Returns the current program counter.
function Core:get_pc()
    return self.pipeline:get_pc()
end

--- Returns the complete pipeline snapshot history.
function Core:get_trace()
    return self.pipeline:get_trace()
end

-- ========================================================================
-- Module exports
-- ========================================================================

return {
    Core               = Core,
    CoreConfig         = CoreConfig,
    CoreStats          = CoreStats,
    MemoryController   = MemoryController,
}
