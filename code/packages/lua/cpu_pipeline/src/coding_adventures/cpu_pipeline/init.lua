-- init.lua — CodingAdventures CPU Pipeline
--
-- Entry point for the cpu_pipeline Lua package.
--
-- The CPU pipeline is the mechanism that allows a processor to overlap
-- the execution of multiple instructions — while one instruction is
-- executing, the next is being decoded, and the one after is being fetched.
-- This is the same principle as a factory assembly line: each workstation
-- handles one task, then passes the work to the next.
--
-- This package provides:
--
--   Pipeline         — the configurable N-stage pipeline engine
--   PipelineConfig   — configuration (stages, execution width)
--   PipelineStage    — individual stage definition (name, category)
--   PipelineStats    — performance counters (IPC, CPI, stalls, flushes)
--   Snapshot         — point-in-time view of pipeline state
--   HazardResponse   — what the hazard detector tells the pipeline to do
--   Token            — a unit of work flowing through the pipeline
--
-- Quick start example:
--
--   local cpu_pipeline = require("coding_adventures.cpu_pipeline")
--   local Pipeline = cpu_pipeline.Pipeline
--   local PipelineConfig = cpu_pipeline.PipelineConfig
--
--   -- Minimal callbacks for a no-op simulation
--   local function fetch(pc)    return 0 end
--   local function decode(r, t) return t end
--   local function execute(t)   return t end
--   local function memory(t)    return t end
--   local function writeback(t) end
--
--   local result = Pipeline.new(
--       PipelineConfig.classic_5_stage(),
--       fetch, decode, execute, memory, writeback
--   )
--   assert(result.ok)
--   local p = result.pipeline
--   p:set_pc(0)
--   local snap = p:step()
--   print(snap:to_string())  -- "[cycle 1] PC=4 stalled=false flushing=false"

local Token      = require("coding_adventures.cpu_pipeline.token")
local config_mod = require("coding_adventures.cpu_pipeline.config")
local Pipeline   = require("coding_adventures.cpu_pipeline.pipeline")

return {
    -- Core types
    Pipeline       = Pipeline,
    Token          = Token,

    -- Configuration types
    PipelineConfig = config_mod.PipelineConfig,
    PipelineStage  = config_mod.PipelineStage,
    HazardResponse = config_mod.HazardResponse,
    PipelineStats  = config_mod.PipelineStats,
    Snapshot       = config_mod.Snapshot,
}
