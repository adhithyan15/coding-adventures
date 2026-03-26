-- GFM 0.31.2 Specification Test Suite
-- =============================================
--
-- Runs all 652 examples from the GFM 0.31.2 specification.
-- Each example consists of a Markdown input and expected HTML output.
-- The test verifies that our parser+renderer pipeline produces exactly
-- the expected output.
--
-- Spec source: https://spec.commonmark.org/0.31.2/spec.json
--
-- === How to interpret failures ===
--
-- When a test fails, busted will show:
--   Expected: <expected HTML from spec>
--   Got:      <actual HTML from our pipeline>
--
-- The section name tells you which GFM feature area needs fixing.
-- Start with failures in "Tabs", "Thematic breaks", and "ATX headings"
-- since those are foundational. Fix block-level issues before inline issues.
--
-- @module test_commonmark_spec

-- Set up package path so all four packages are findable when running from tests/
package.path = "../src/?.lua;../src/?/init.lua;"
  .. "../../document_ast_to_html/src/?.lua;../../document_ast_to_html/src/?/init.lua;"
  .. "../../gfm_parser/src/?.lua;../../gfm_parser/src/?/init.lua;"
  .. "../../document_ast/src/?.lua;../../document_ast/src/?/init.lua;"
  .. package.path

local commonmark = require("coding_adventures.commonmark")

-- Load the spec JSON using dkjson (available via luarocks)
local json = require("dkjson")

-- Find the spec file relative to this test file
-- When busted runs from the tests/ directory, the file is in the same dir
local spec_path = "commonmark_spec.json"
local f = io.open(spec_path, "r")
if not f then
  -- Try relative to script location
  spec_path = debug.getinfo(1, "S").source:sub(2):match("(.*/)")  .. "commonmark_spec.json"
  f = io.open(spec_path, "r")
end
if not f then
  error("Cannot find commonmark_spec.json — expected at: " .. spec_path)
end
local spec_json = f:read("*a")
f:close()

local examples, _, err = json.decode(spec_json)
if not examples then
  error("Failed to parse commonmark_spec.json: " .. tostring(err))
end

-- Group examples by section for organized test output
local sections = {}
local section_order = {}
for _, ex in ipairs(examples) do
  local s = ex.section
  if not sections[s] then
    sections[s] = {}
    section_order[#section_order + 1] = s
  end
  sections[s][#sections[s] + 1] = ex
end

-- Run all spec examples, grouped by section
describe("GFM 0.31.2 specification", function()
  for _, section in ipairs(section_order) do
    describe(section, function()
      for _, ex in ipairs(sections[section]) do
        -- Create a test name with the example number for easy identification
        local test_name = string.format("example %d", ex.example)
        it(test_name, function()
          local actual = commonmark.render(ex.markdown)
          assert.equals(ex.html, actual,
            string.format("\nExample %d (%s)\nMarkdown: %s\nExpected: %s\nActual:   %s",
              ex.example, ex.section,
              ex.markdown:gsub("\n", "↵"),
              ex.html:gsub("\n", "↵"),
              actual:gsub("\n", "↵")))
        end)
      end
    end)
  end
end)
