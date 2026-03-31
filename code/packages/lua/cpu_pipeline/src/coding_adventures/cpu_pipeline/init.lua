-- coding_adventures.cpu_pipeline — module entry point
--
-- Exports all public types so callers can do:
--   local cpu_pipeline = require("coding_adventures.cpu_pipeline")
--   local Pipeline = cpu_pipeline.Pipeline
--   local PipelineConfig = cpu_pipeline.PipelineConfig
--   ...

local config_mod = require("coding_adventures.cpu_pipeline.config")
local Token      = require("coding_adventures.cpu_pipeline.token")
local Pipeline   = require("coding_adventures.cpu_pipeline.pipeline")

return {
    Token          = Token,
    PipelineStage  = config_mod.PipelineStage,
    PipelineConfig = config_mod.PipelineConfig,
    HazardResponse = config_mod.HazardResponse,
    PipelineStats  = config_mod.PipelineStats,
    Snapshot       = config_mod.Snapshot,
    Pipeline       = Pipeline,
}
