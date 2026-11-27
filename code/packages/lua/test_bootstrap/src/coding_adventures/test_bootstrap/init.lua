--- coding_adventures.test_bootstrap: the Lua lane of the property-based testing
-- engine (VER05o).  Pure Lua 5.4, no dependencies, no I/O.
--
--   local tb = require("coding_adventures.test_bootstrap")
--   local verdict = tb.Runner.check(
--     42,                              -- seed (an Int)
--     tb.Gens.integer(0, 1000),        -- generator
--     function(x) return x <= 100 end, -- holds only on exactly `true`
--     tb.Runner.Options.new({ num_tests = 100, ci = "off" }))
--   -- verdict.failure_id == "property-counterexample", verdict.minimal == 101

local prefix = "coding_adventures.test_bootstrap."

local M = {
  Int = require(prefix .. "int"),
  Prng = require(prefix .. "prng"),
  Limits = require(prefix .. "limits"),
  FailureIds = require(prefix .. "failure_ids"),
  Hooks = require(prefix .. "hooks"),
  Budget = require(prefix .. "budget"),
  Gen = require(prefix .. "gen"),
  Gens = require(prefix .. "gens"),
  Shrink = require(prefix .. "shrink"),
  Runner = require(prefix .. "runner"),
  Safe = require(prefix .. "safe"),
  Canon = require(prefix .. "canon"),
  StrictJson = require(prefix .. "strict_json"),
  Values = require(prefix .. "values"),
  Tfv = require(prefix .. "tfv"),
  Corpus = require(prefix .. "corpus"),
  Fuzz = require(prefix .. "fuzz"),
}

M.VERSION = "0.1.0"

return M
