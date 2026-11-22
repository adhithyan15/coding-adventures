-- ============================================================================
-- Line Coverage Harness using Lua's built-in debug.sethook
--
-- Zero network / Luarocks dependencies. Pure Lua 5.4 + luac -l -p.
-- Asserts >= 95% executable line coverage across all production source files.
-- ============================================================================

package.path = "./?.lua;../src/?.lua;../src/?/init.lua;" .. package.path

local source_files = {
  "../src/coding_adventures/starlark_compiler/constants.lua",
  "../src/coding_adventures/starlark_compiler/messages.lua",
  "../src/coding_adventures/starlark_compiler/errors.lua",
  "../src/coding_adventures/starlark_compiler/opcodes.lua",
  "../src/coding_adventures/starlark_compiler/init.lua",
}

local executable = {}
local total_executable = 0

for _, path in ipairs(source_files) do
  executable[path] = {}
  local p = io.popen("luac -l -p " .. path)
  if not p then
    error("failed to run luac on " .. path, 0)
  end
  for line in p:lines() do
    local line_num, op = line:match("^\t%d+\t%[(%d+)%]\t([%a_]+)")
    if line_num and op ~= "VARARGPREP" then
      local n = tonumber(line_num)
      if not executable[path][n] then
        executable[path][n] = true
        total_executable = total_executable + 1
      end
    end
  end
  p:close()
end

local executed = {}
for _, path in ipairs(source_files) do
  executed[path] = {}
end

local func_cache = {}

local function hook(event, line)
  if event == "line" then
    local finfo = debug.getinfo(2, "f")
    if not finfo then
      return
    end
    local fn = finfo.func
    local target = func_cache[fn]
    if target == nil then
      local sinfo = debug.getinfo(2, "S")
      local src = sinfo and sinfo.source or ""
      if src:sub(1, 1) == "@" then
        src = src:sub(2)
      end
      target = false
      for _, path in ipairs(source_files) do
        local filename = path:match("([^/]+%.lua)$")
        if src:find(filename, 1, true) and src:find("starlark_compiler", 1, true) then
          target = path
          break
        end
      end
      func_cache[fn] = target
    end
    if target and executable[target][line] then
      executed[target][line] = true
    end
  end
end

-- Clear package cache to ensure full module evaluation under hook:
package.loaded["coding_adventures.starlark_compiler"] = nil
package.loaded["coding_adventures.starlark_compiler.constants"] = nil
package.loaded["coding_adventures.starlark_compiler.messages"] = nil
package.loaded["coding_adventures.starlark_compiler.errors"] = nil
package.loaded["coding_adventures.starlark_compiler.opcodes"] = nil

debug.sethook(hook, "l")

-- Execute full test suite via busted.runner:
arg = { ".", "--pattern=test_" }
local runner = require("busted.runner")
runner({ standalone = false })

debug.sethook()

local total_executed = 0
print("\n" .. string.rep("=", 78))
print(string.format("%-55s %10s %10s", "File", "Lines", "Coverage"))
print(string.rep("-", 78))

for _, path in ipairs(source_files) do
  local exec_count = 0
  for _ in pairs(executable[path]) do
    exec_count = exec_count + 1
  end

  local hit_count = 0
  local missed = {}
  for line_num in pairs(executable[path]) do
    if executed[path][line_num] then
      hit_count = hit_count + 1
    else
      missed[#missed + 1] = line_num
    end
  end
  total_executed = total_executed + hit_count
  table.sort(missed)

  local pct = exec_count > 0 and (hit_count / exec_count * 100) or 100.0
  print(string.format("%-55s %4d/%-4d %8.2f%%", path, hit_count, exec_count, pct))
  if #missed > 0 then
    print("    Missed lines: " .. table.concat(missed, ", "))
  end
end

local overall_pct = total_executable > 0 and (total_executed / total_executable * 100) or 100.0
print(string.rep("=", 78))
print(string.format("Total Coverage: %d / %d executable lines (%.2f%%)", total_executed, total_executable, overall_pct))
print(string.rep("=", 78) .. "\n")

assert(overall_pct >= 95.0, string.format("Coverage %.2f%% is below required 95.0%%", overall_pct))
print("COVERAGE GATE PASSED (>= 95.0%)\n")
