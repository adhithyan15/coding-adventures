-- =============================================================================
-- Core — complete processor core integrating pipeline + memory + register file
-- =============================================================================
--
-- The Core is the integration point.  It wires together:
--   - Pipeline (from cpu_pipeline)    — instruction flow management
--   - Memory (from cpu_simulator)     — backing store for instructions + data
--   - RegisterFile (from cpu_simulator) — fast operand storage
--   - ISA Decoder (injected)          — instruction semantics
--
-- DESIGN: ISA Injection
-- =====================
-- The Core knows HOW to move instructions through a pipeline, but not WHAT
-- any instruction means.  That knowledge lives in the ISA decoder, which the
-- caller injects at construction time:
--
--   local decoder = MyDecoder.new()     -- implements decode/execute/instruction_size
--   local result  = Core.new(CoreConfig.simple(), decoder)
--   local core    = result.core
--
-- The ISA decoder must implement three methods:
--   decode(raw_instruction, token) → token   — fill opcode/rs1/rs2/rd/flags
--   execute(token, reg_file)       → token   — compute alu_result/branch_taken
--   instruction_size()             → int     — bytes per instruction (usually 4)
--
-- SHARED STATE VIA CLOSURES
-- =========================
-- Lua is not functional (unlike Elixir).  We pass a shared `state` table by
-- reference into the pipeline callbacks.  The callbacks close over `state` and
-- mutate it as instructions flow through the stages.  This is safe because the
-- pipeline callbacks are always called sequentially (not concurrently).

local cpu_pipeline = require("coding_adventures.cpu_pipeline")
local cpu_sim      = require("coding_adventures.cpu_simulator")

local Pipeline       = cpu_pipeline.Pipeline
local PipelineConfig = cpu_pipeline.PipelineConfig
local Memory         = cpu_sim.Memory
local RegisterFile   = cpu_sim.RegisterFile

-- ---------------------------------------------------------------------------
-- MemoryController — thin wrapper around Memory with latency tracking
-- ---------------------------------------------------------------------------
-- In a multi-core system the memory controller serialises requests from all
-- cores.  This single-core implementation is a direct pass-through with a
-- stored latency value for metadata purposes.

local MemoryController = {}
MemoryController.__index = MemoryController

-- new(size, latency) — create a MemoryController backed by `size` bytes of RAM
function MemoryController.new(size, latency)
    size    = size    or 65536
    latency = latency or 100
    local self = setmetatable({}, MemoryController)
    self.memory  = Memory.new(size)
    self.latency = latency
    return self
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

-- ---------------------------------------------------------------------------
-- CoreConfig — micro-architecture parameters
-- ---------------------------------------------------------------------------

local CoreConfig = {}
CoreConfig.__index = CoreConfig

-- new(opts)
--   opts.name           — human-readable name (default "Default")
--   opts.pipeline       — PipelineConfig (default classic_5_stage)
--   opts.num_registers  — number of GPRs (default 16)
--   opts.register_width — bits per register (default 32)
--   opts.memory_size    — bytes of RAM (default 65536)
--   opts.memory_latency — DRAM latency in cycles (default 100)
function CoreConfig.new(opts)
    opts = opts or {}
    local self = setmetatable({}, CoreConfig)
    self.name           = opts.name           or "Default"
    self.pipeline       = opts.pipeline       or PipelineConfig.classic_5_stage()
    self.num_registers  = opts.num_registers  or 16
    self.register_width = opts.register_width or 32
    self.memory_size    = opts.memory_size    or 65536
    self.memory_latency = opts.memory_latency or 100
    return self
end

-- simple() — 5-stage, 16 registers, 64KB RAM — good for teaching
function CoreConfig.simple()
    return CoreConfig.new({
        name           = "Simple",
        pipeline       = PipelineConfig.classic_5_stage(),
        num_registers  = 16,
        register_width = 32,
        memory_size    = 65536,
        memory_latency = 1,
    })
end

-- performance() — 13-stage, 31 registers, 256KB — inspired by ARM Cortex-A78
function CoreConfig.performance()
    return CoreConfig.new({
        name           = "Performance",
        pipeline       = PipelineConfig.deep_13_stage(),
        num_registers  = 31,
        register_width = 64,
        memory_size    = 262144,
        memory_latency = 100,
    })
end

-- ---------------------------------------------------------------------------
-- CoreStats — aggregate execution statistics
-- ---------------------------------------------------------------------------

local CoreStats = {}
CoreStats.__index = CoreStats

function CoreStats.new(instructions_completed, total_cycles, pipeline_stats)
    local self = setmetatable({}, CoreStats)
    self.instructions_completed = instructions_completed or 0
    self.total_cycles           = total_cycles           or 0
    self.pipeline_stats         = pipeline_stats         or nil
    return self
end

-- ipc() — Instructions Per Cycle
function CoreStats:ipc()
    if self.total_cycles == 0 then return 0.0 end
    return self.instructions_completed / self.total_cycles
end

-- cpi() — Cycles Per Instruction
function CoreStats:cpi()
    if self.instructions_completed == 0 then return 0.0 end
    return self.total_cycles / self.instructions_completed
end

function CoreStats:to_string()
    return string.format(
        "CoreStats{instr=%d cycles=%d ipc=%.3f cpi=%.3f}",
        self.instructions_completed, self.total_cycles,
        self:ipc(), self:cpi()
    )
end

-- ---------------------------------------------------------------------------
-- Core — the wired-together processor
-- ---------------------------------------------------------------------------

local Core = {}
Core.__index = Core

-- Core.new(config, decoder) → {ok=true, core=<Core>} | {ok=false, err="reason"}
--
-- decoder must implement:
--   decoder:decode(raw_instruction, token)  → token
--   decoder:execute(token, reg_file)        → token
--   decoder:instruction_size()              → int
function Core.new(config, decoder)
    -- Validate config
    if config == nil then
        return {ok = false, err = "config is nil"}
    end
    if decoder == nil then
        return {ok = false, err = "decoder is nil"}
    end

    -- 1. Create shared mutable state — passed by reference into callbacks
    --    The pipeline callbacks close over this table.
    local state = {
        reg_file = RegisterFile.new(config.num_registers, config.register_width),
        mem_ctrl = MemoryController.new(config.memory_size, config.memory_latency),
    }

    -- 2. Define the five pipeline callbacks.
    --    Each callback reads/writes `state` — this is safe because callbacks
    --    are invoked sequentially by the single-threaded pipeline.

    local function fetch_fn(pc)
        return state.mem_ctrl:read_word(pc)
    end

    local function decode_fn(raw, token)
        return decoder:decode(raw, token)
    end

    local function execute_fn(token)
        return decoder:execute(token, state.reg_file)
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

    -- 3. Build the pipeline
    local pipeline_config = config.pipeline or PipelineConfig.classic_5_stage()
    local pipeline_result = Pipeline.new(
        pipeline_config,
        fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn
    )
    if not pipeline_result.ok then
        return {ok = false, err = pipeline_result.err}
    end

    -- 4. Set predict function (PC + instruction_size)
    local p = pipeline_result.pipeline
    p:set_predict_fn(function(pc)
        return pc + decoder:instruction_size()
    end)

    -- 5. Assemble the Core struct
    local self = setmetatable({}, Core)
    self.config   = config
    self.decoder  = decoder
    self.pipeline = p
    self.state    = state   -- shared with callbacks
    self.cycle    = 0
    self.halted   = false

    return {ok = true, core = self}
end

-- load_program(bytes, start_address)
-- Loads machine code into memory and resets the PC.
function Core:load_program(bytes, start_address)
    start_address = start_address or 0
    self.state.mem_ctrl:load_program(bytes, start_address)
    self.pipeline:set_pc(start_address)
end

-- step() → Snapshot
-- Execute one clock cycle.
function Core:step()
    if self.halted then
        return self.pipeline:snapshot()
    end
    self.cycle = self.cycle + 1
    local snap = self.pipeline:step()
    if self.pipeline:is_halted() then
        self.halted = true
    end
    return snap
end

-- run(max_cycles) → CoreStats
-- Run until halt or max_cycles reached.
function Core:run(max_cycles)
    max_cycles = max_cycles or 100000
    while not self.halted and self.cycle < max_cycles do
        self:step()
    end
    return self:get_stats()
end

-- is_halted() → bool
function Core:is_halted()
    return self.halted
end

-- get_cycle() → int
function Core:get_cycle()
    return self.cycle
end

-- get_stats() → CoreStats
function Core:get_stats()
    local p_stats = self.pipeline:get_stats()
    return CoreStats.new(
        p_stats.instructions_completed,
        p_stats.total_cycles,
        p_stats
    )
end

-- read_register(index) → int
function Core:read_register(index)
    return self.state.reg_file:read(index)
end

-- write_register(index, value)
function Core:write_register(index, value)
    self.state.reg_file:write(index, value)
end

-- read_memory_word(address) → int
function Core:read_memory_word(address)
    return self.state.mem_ctrl:read_word(address)
end

-- write_memory_word(address, value)
function Core:write_memory_word(address, value)
    self.state.mem_ctrl:write_word(address, value)
end

-- get_trace() → list of Snapshot
function Core:get_trace()
    return self.pipeline:get_trace()
end

return {
    Core               = Core,
    CoreConfig         = CoreConfig,
    CoreStats          = CoreStats,
    MemoryController   = MemoryController,
}
