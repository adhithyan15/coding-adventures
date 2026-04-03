-- cli_builder — Declarative CLI Argument Parser
-- ================================================
--
-- This package is part of the coding-adventures monorepo.
-- It implements a complete CLI argument parsing library driven by a JSON spec.
--
-- # The Core Insight: CLI Syntax Is a Directed Graph
-- ====================================================
--
-- A CLI tool's valid syntax forms a directed graph.  Consider:
--
--   git remote add origin <url>
--
-- The user navigates: root → remote → add → (two positional args).
-- The valid invocations of any CLI tool are exactly the valid PATHS through
-- this graph from the root node to an accepting state.
--
-- Flag constraints (conflicts, requirements) form a SECOND graph layered on
-- top of the routing graph.  We detect cycles in that graph at load time.
--
-- # The State Machine Execution Engine
-- ======================================
--
-- Parsing argv is stateful.  After seeing --output, the next token is a
-- VALUE, not a flag or subcommand.  After --, ALL subsequent tokens are
-- positional.  After routing into "git commit", flags for "git add" are
-- no longer in scope.
--
-- CLI Builder drives parsing with a four-mode state machine:
--
--   ROUTING → SCANNING ↔ FLAG_VALUE
--                └→ END_OF_FLAGS
--
-- # Three-Phase Parsing
-- =====================
--
-- Phase 1 — ROUTING:
--   Walk the command graph depth-first.  Each token that matches a
--   subcommand name advances the routing pointer.  Non-subcommand tokens
--   switch to SCANNING mode.
--
-- Phase 2 — SCANNING:
--   Classify each token:
--     "--"              → switch to END_OF_FLAGS
--     "--name"          → long flag
--     "--name=value"    → long flag with inline value
--     "-x"              → short flag
--     "-xyz"            → stacked boolean flags
--     other             → positional token
--
-- Phase 3 — VALIDATION:
--   - Resolve positional tokens to argument slots (required, optional, variadic)
--   - Validate flag constraints (conflicts, requires, enum values, types)
--   - Apply defaults for missing flags
--
-- # Return Values
-- ================
--
--   { type = "result",  flags = {}, arguments = {}, command_path = {} }
--   { type = "help",    text  = "...", command_path = {} }
--   { type = "version", version = "1.0.0" }
--   { type = "error",   errors = { {error_type=..., message=...}, ... } }
--
-- # Quick Start
-- ==============
--
--   local cli = require("coding_adventures.cli_builder")
--   local result = cli.parse_table(spec_table, {"--verbose", "hello"})
--   if result.type == "result" then
--     print(result.flags["verbose"])  -- true
--   end

local json_ok, json = pcall(require, "dkjson")
if not json_ok then
  json_ok, json = pcall(require, "cjson")
end
if not json_ok then
  json_ok, json = pcall(require, "json")
end

local M = {}

-- ============================================================================
-- Token Classifier
-- ============================================================================
--
-- Classifies a single argv token into a typed token event.
-- Each token falls into exactly one category:
--
--   "end_of_flags"         → "--"
--   "long_flag"            → "--name"
--   "long_flag_value"      → "--name=value"
--   "short_flag"           → "-x"
--   "short_flag_value"     → "-xVALUE" (non-boolean x)
--   "stacked_flags"        → "-xyz" (all boolean)
--   "single_dash_long"     → "-classpath" (SDL flag)
--   "positional"           → anything else
--   "unknown_flag"         → looks like a flag but not recognized

local TokenClassifier = {}

--- Build lookup maps from a list of flag definitions.
-- @param flags  List of flag definition tables.
-- @return short_map, long_map, sdl_map
local function build_flag_maps(flags)
  local short_map = {}
  local long_map  = {}
  local sdl_map   = {}
  for _, f in ipairs(flags) do
    if f["short"]             then short_map[f["short"]]                = f end
    if f["long"]              then long_map[f["long"]]                  = f end
    if f["single_dash_long"]  then sdl_map[f["single_dash_long"]]       = f end
  end
  return short_map, long_map, sdl_map
end

--- Classify one argv token.
-- @param token       The raw string from argv.
-- @param flags       List of currently-in-scope flag definitions.
-- @return            A typed token table:
--                      { kind = "long_flag",     name = "verbose" }
--                      { kind = "long_flag_value", name = "output", value = "foo.txt" }
--                      { kind = "short_flag",    char = "v" }
--                      { kind = "short_flag_value", char = "o", value = "foo.txt" }
--                      { kind = "stacked_flags", chars = {"v","x"} }
--                      { kind = "single_dash_long", name = "classpath" }
--                      { kind = "end_of_flags" }
--                      { kind = "positional",    value = "hello" }
--                      { kind = "unknown_flag",  token = "--bogus" }
function TokenClassifier.classify(token, flags)
  local short_map, long_map, sdl_map = build_flag_maps(flags)

  -- "--" → end of flags
  if token == "--" then
    return { kind = "end_of_flags" }
  end

  -- Long flags: "--name" or "--name=value"
  if token:sub(1, 2) == "--" then
    local rest = token:sub(3)
    local eq   = rest:find("=", 1, true)
    if eq then
      return { kind = "long_flag_value", name = rest:sub(1, eq-1), value = rest:sub(eq+1) }
    else
      return { kind = "long_flag", name = rest }
    end
  end

  -- Bare "-" → positional (stdin/stdout convention)
  if token == "-" then
    return { kind = "positional", value = "-" }
  end

  -- Short flags: "-x..."
  if token:sub(1, 1) == "-" then
    local rest = token:sub(2)

    -- Rule 1: single-dash-long exact match
    if sdl_map[rest] then
      return { kind = "single_dash_long", name = rest }
    end

    -- Rule 2: first char is a known short flag
    local first = rest:sub(1, 1)
    if short_map[first] then
      local flag      = short_map[first]
      local remainder = rest:sub(2)

      if remainder == "" then
        return { kind = "short_flag", char = first }
      elseif flag["type"] == "boolean" or flag["type"] == "count" then
        -- Boolean/count flags: try to stack remaining chars
        local chars = { first }
        local ok = true
        for i = 1, #remainder do
          local c = remainder:sub(i, i)
          if short_map[c] then
            chars[#chars + 1] = c
          else
            ok = false
            break
          end
        end
        if ok then
          return { kind = "stacked_flags", chars = chars }
        else
          return { kind = "unknown_flag", token = token }
        end
      else
        -- Non-boolean: remainder is the value
        return { kind = "short_flag_value", char = first, value = remainder }
      end
    end

    -- No match → unknown flag
    return { kind = "unknown_flag", token = token }
  end

  -- Positional
  return { kind = "positional", value = token }
end

M.TokenClassifier = TokenClassifier

-- ============================================================================
-- SpecLoader
-- ============================================================================
--
-- Loads, validates, and normalizes a CLI spec from either a table (already
-- decoded) or a JSON string.  Raises on invalid specs.

local SpecLoader = {}

local VALID_TYPES = {
  boolean=true, string=true, integer=true, float=true,
  path=true, file=true, directory=true, enum=true, count=true
}

local function normalise_flag(f, scope_id)
  -- Required fields: id, description, type
  assert(f["id"],          scope_id .. ": flag missing 'id'")
  assert(f["description"], scope_id .. ": flag '" .. f["id"] .. "' missing 'description'")
  assert(f["type"],        scope_id .. ": flag '" .. f["id"] .. "' missing 'type'")
  assert(VALID_TYPES[f["type"]], scope_id .. ": flag '" .. f["id"] .. "' has invalid type '" .. f["type"] .. "'")

  if f["type"] == "enum" then
    assert(f["enum_values"] and #f["enum_values"] > 0,
      scope_id .. ": flag '" .. f["id"] .. "' type=enum requires non-empty enum_values")
  end

  return {
    id               = f["id"],
    short            = f["short"],
    long             = f["long"],
    single_dash_long = f["single_dash_long"],
    description      = f["description"],
    type             = f["type"],
    required         = f["required"] or false,
    default          = f["default"],
    value_name       = f["value_name"] or "VALUE",
    enum_values      = f["enum_values"] or {},
    conflicts_with   = f["conflicts_with"] or {},
    requires         = f["requires"] or {},
    required_unless  = f["required_unless"] or {},
    repeatable       = f["repeatable"] or false,
  }
end

local function normalise_argument(a, scope_id)
  assert(a["id"],          scope_id .. ": argument missing 'id'")
  assert(a["description"], scope_id .. ": argument '" .. a["id"] .. "' missing 'description'")
  return {
    id           = a["id"],
    display_name = a["display_name"] or a["id"]:upper(),
    description  = a["description"],
    required     = (a["required"] ~= false),  -- default true
    variadic     = a["variadic"] or false,
    type         = a["type"] or "string",
  }
end

local function normalise_commands(cmds, scope_id)
  if not cmds then return {} end
  local result = {}
  for _, cmd in ipairs(cmds) do
    assert(cmd["name"],        scope_id .. ": command missing 'name'")
    assert(cmd["description"], scope_id .. ": command '" .. cmd["name"] .. "' missing 'description'")
    result[#result + 1] = {
      name         = cmd["name"],
      description  = cmd["description"],
      flags        = (function()
        local fs = {}
        for _, f in ipairs(cmd["flags"] or {}) do
          fs[#fs + 1] = normalise_flag(f, cmd["name"])
        end
        return fs
      end)(),
      arguments    = (function()
        local as = {}
        for _, a in ipairs(cmd["arguments"] or {}) do
          as[#as + 1] = normalise_argument(a, cmd["name"])
        end
        return as
      end)(),
      commands     = normalise_commands(cmd["commands"], cmd["name"]),
    }
  end
  return result
end

--- Validate and normalize a raw spec table.
function SpecLoader.load_table(raw)
  assert(type(raw) == "table", "spec must be a table (decoded JSON object)")

  -- Required top-level fields
  local version = raw["cli_builder_spec_version"]
  assert(version == "1.0",
    "unsupported cli_builder_spec_version: " .. tostring(version))
  assert(raw["name"],        "spec missing 'name'")
  assert(raw["description"], "spec missing 'description'")

  local spec = {
    name          = raw["name"],
    display_name  = raw["display_name"] or raw["name"],
    description   = raw["description"],
    version       = raw["version"],
    parsing_mode  = raw["parsing_mode"] or "gnu",
    builtin_flags = {
      help    = (raw["builtin_flags"] or {})["help"]    ~= false,
      version = (raw["builtin_flags"] or {})["version"] ~= false,
    },
    global_flags  = {},
    flags         = {},
    arguments     = {},
    commands      = {},
  }

  for _, f in ipairs(raw["global_flags"] or {}) do
    spec.global_flags[#spec.global_flags + 1] = normalise_flag(f, "global")
  end
  for _, f in ipairs(raw["flags"] or {}) do
    spec.flags[#spec.flags + 1] = normalise_flag(f, "root")
  end
  for _, a in ipairs(raw["arguments"] or {}) do
    spec.arguments[#spec.arguments + 1] = normalise_argument(a, "root")
  end
  spec.commands = normalise_commands(raw["commands"], "root")

  return spec
end

--- Load spec from a JSON string.
function SpecLoader.load_string(json_str)
  assert(json, "No JSON library found. Install dkjson, cjson, or json via luarocks.")
  local raw, err
  if json.decode then
    raw, _, err = json.decode(json_str)
  else
    raw, err = json.decode(json_str)
  end
  assert(raw, "Invalid JSON: " .. tostring(err))
  return SpecLoader.load_table(raw)
end

M.SpecLoader = SpecLoader

-- ============================================================================
-- Help Generator
-- ============================================================================
--
-- Produces formatted help text for a command node.
-- The help format follows the spec §9:
--
--   USAGE
--     name [OPTIONS] [COMMAND] [ARGS...]
--
--   DESCRIPTION
--     ...
--
--   COMMANDS
--     subcommand    Description
--
--   OPTIONS
--     -s, --long <VAL>    Description [default: val]
--
--   GLOBAL OPTIONS
--     -h, --help    Show help

local HelpGenerator = {}

local function flag_usage(f)
  local parts = {}
  if f.short then parts[#parts + 1] = "-" .. f.short end
  if f.long  then
    if f.type == "boolean" or f.type == "count" then
      parts[#parts + 1] = "--" .. f.long
    else
      parts[#parts + 1] = "--" .. f.long .. " <" .. f.value_name .. ">"
    end
  end
  return table.concat(parts, ", ")
end

local function flag_line(f)
  local left  = "  " .. flag_usage(f)
  local right = f.description
  if f.default ~= nil then
    right = right .. " [default: " .. tostring(f.default) .. "]"
  end
  if f.required then
    right = right .. " (required)"
  end
  return string.format("  %-30s%s", flag_usage(f), right)
end

--- Generate help text for a command path.
-- @param spec          Normalised spec table.
-- @param command_path  List of command names, e.g. {"git", "remote", "add"}.
-- @return              Formatted help string.
function HelpGenerator.generate(spec, command_path)
  -- Navigate to the target command node
  local node = spec
  for i = 2, #command_path do
    local name = command_path[i]
    local found = nil
    for _, cmd in ipairs(node.commands or {}) do
      if cmd.name == name then found = cmd; break end
    end
    if found then node = found else break end
  end

  local lines = {}
  local name  = table.concat(command_path, " ")

  -- Build usage synopsis
  local usage_parts = { name }
  local all_flags = {}
  for _, f in ipairs(spec.global_flags or {}) do all_flags[#all_flags + 1] = f end
  for _, f in ipairs(node.flags or spec.flags or {}) do all_flags[#all_flags + 1] = f end
  if #all_flags > 0 then usage_parts[#usage_parts + 1] = "[OPTIONS]" end
  if node.commands and #node.commands > 0 then usage_parts[#usage_parts + 1] = "[COMMAND]" end
  for _, a in ipairs(node.arguments or spec.arguments or {}) do
    if a.variadic then
      usage_parts[#usage_parts + 1] = (a.required and "<" or "[") .. a.display_name .. "..." .. (a.required and ">" or "]")
    else
      usage_parts[#usage_parts + 1] = (a.required and "<" or "[") .. a.display_name .. (a.required and ">" or "]")
    end
  end

  lines[#lines + 1] = "USAGE"
  lines[#lines + 1] = "  " .. table.concat(usage_parts, " ")
  lines[#lines + 1] = ""

  -- Description
  local desc = node.description or spec.description
  lines[#lines + 1] = "DESCRIPTION"
  lines[#lines + 1] = "  " .. desc
  lines[#lines + 1] = ""

  -- Commands
  local cmds = node.commands or spec.commands or {}
  if #cmds > 0 then
    lines[#lines + 1] = "COMMANDS"
    for _, cmd in ipairs(cmds) do
      lines[#lines + 1] = string.format("  %-20s%s", cmd.name, cmd.description)
    end
    lines[#lines + 1] = ""
  end

  -- Options (node-level flags)
  local node_flags = node.flags or spec.flags or {}
  if #node_flags > 0 then
    lines[#lines + 1] = "OPTIONS"
    for _, f in ipairs(node_flags) do
      lines[#lines + 1] = flag_line(f)
    end
    lines[#lines + 1] = ""
  end

  -- Global Options
  local global = spec.global_flags or {}
  local builtins = {}
  if spec.builtin_flags and spec.builtin_flags.help then
    builtins[#builtins + 1] = {
      short = "h", long = "help", type = "boolean",
      description = "Show this help message and exit."
    }
  end
  if spec.builtin_flags and spec.builtin_flags.version and spec.version then
    builtins[#builtins + 1] = {
      long = "version", type = "boolean",
      description = "Show version and exit."
    }
  end
  local all_global = {}
  for _, f in ipairs(global)   do all_global[#all_global + 1] = f end
  for _, f in ipairs(builtins) do all_global[#all_global + 1] = f end
  if #all_global > 0 then
    lines[#lines + 1] = "GLOBAL OPTIONS"
    for _, f in ipairs(all_global) do
      lines[#lines + 1] = flag_line(f)
    end
  end

  return table.concat(lines, "\n")
end

M.HelpGenerator = HelpGenerator

-- ============================================================================
-- FlagValidator
-- ============================================================================
--
-- Validates a collected flags map against spec constraints:
--   - Required flags must be present
--   - Enum flags must have one of the declared values
--   - conflicts_with pairs must not both be set
--   - requires dependencies must be satisfied

local FlagValidator = {}

--- Validate flags against their specs.
-- @param flags     Table of { [flag_id] = value }
-- @param flag_defs List of flag definition tables.
-- @return errors   List of error strings (empty if valid).
function FlagValidator.validate(flags, flag_defs)
  local errors = {}

  for _, f in ipairs(flag_defs) do
    local id  = f.id
    local val = flags[id]

    -- Required
    if f.required and (val == nil or val == false) then
      if #f.required_unless > 0 then
        local has_one = false
        for _, other_id in ipairs(f.required_unless) do
          if flags[other_id] ~= nil and flags[other_id] ~= false then
            has_one = true; break
          end
        end
        if not has_one then
          errors[#errors + 1] = "required flag missing: --" .. (f.long or id)
        end
      else
        errors[#errors + 1] = "required flag missing: --" .. (f.long or id)
      end
    end

    -- Enum validation
    if f.type == "enum" and val ~= nil then
      local valid = false
      for _, ev in ipairs(f.enum_values) do
        if val == ev then valid = true; break end
      end
      if not valid then
        errors[#errors + 1] = string.format(
          "invalid value '%s' for --%s: must be one of: %s",
          tostring(val), f.long or id, table.concat(f.enum_values, ", "))
      end
    end

    -- Conflicts
    if val ~= nil and val ~= false and val ~= 0 then
      for _, other_id in ipairs(f.conflicts_with) do
        local ov = flags[other_id]
        if ov ~= nil and ov ~= false and ov ~= 0 then
          errors[#errors + 1] = string.format(
            "--%s conflicts with --%s", f.long or id, other_id)
        end
      end
    end

    -- Requires
    if val ~= nil and val ~= false and val ~= 0 then
      for _, req_id in ipairs(f.requires) do
        local rv = flags[req_id]
        if rv == nil or rv == false or rv == 0 then
          errors[#errors + 1] = string.format(
            "--%s requires --%s", f.long or id, req_id)
        end
      end
    end
  end

  return errors
end

M.FlagValidator = FlagValidator

-- ============================================================================
-- Parser
-- ============================================================================
--
-- Main parsing engine.  Given a normalized spec and argv, runs the
-- three-phase algorithm and returns a result table.

local Parser = {}

--- Coerce a raw string value to the flag's declared type.
local function coerce(raw, ftype, flag_id)
  if ftype == "boolean" then
    return raw == nil or raw == "true" or raw == true
  elseif ftype == "integer" or ftype == "count" then
    local n = tonumber(raw)
    assert(n, string.format("flag --%s expects an integer, got '%s'", flag_id, tostring(raw)))
    return math.floor(n)
  elseif ftype == "float" then
    local n = tonumber(raw)
    assert(n, string.format("flag --%s expects a number, got '%s'", flag_id, tostring(raw)))
    return n
  else
    return tostring(raw)
  end
end

--- Build a map from flag id to flag def, including both local and global flags.
local function all_flags_map(spec_node, global_flags)
  local map = {}
  for _, f in ipairs(global_flags or {}) do map[f.id] = f end
  for _, f in ipairs(spec_node.flags or {}) do map[f.id] = f end
  -- Also index by short, long, single_dash_long
  local by_short = {}
  local by_long  = {}
  local by_sdl   = {}
  for _, f in pairs(map) do
    if f.short            then by_short[f.short]            = f end
    if f.long             then by_long[f.long]              = f end
    if f.single_dash_long then by_sdl[f.single_dash_long]   = f end
  end
  return map, by_short, by_long, by_sdl
end

--- Apply default values for all flags not set by the user.
local function apply_defaults(flags, flag_defs)
  for _, f in ipairs(flag_defs) do
    if flags[f.id] == nil then
      if f.type == "boolean" then
        flags[f.id] = f.default or false
      elseif f.type == "count" then
        flags[f.id] = f.default or 0
      else
        flags[f.id] = f.default  -- nil if no default
      end
    end
  end
end

--- Parse argv against a normalized spec table.
-- @param spec  Normalized spec (from SpecLoader.load_table).
-- @param argv  List of argument strings (WITHOUT argv[0]).
-- @return      Result table.
function Parser.parse(spec, argv)
  local errors = {}

  -- ── Phase 1: Routing ─────────────────────────────────────────────────────
  -- Walk argv looking for subcommand tokens until we hit a non-subcommand.
  local command_path    = { spec.name }
  local current_node    = spec
  local remaining_argv  = {}

  local i = 1
  local in_routing = true

  while i <= #argv and in_routing do
    local token = argv[i]

    -- Flags (tokens starting with "-") are never subcommand names.
    -- Pass them to the scanner but continue routing for subsequent tokens.
    if token:match("^-") then
      remaining_argv[#remaining_argv + 1] = token
      i = i + 1
    else
      -- Check if this token is a subcommand name at the current level
      local matched = false
      for _, cmd in ipairs(current_node.commands or {}) do
        if cmd.name == token then
          command_path[#command_path + 1] = cmd.name
          current_node = cmd
          matched = true
          break
        end
      end

      if not matched then
        in_routing = false
        -- This token is not a subcommand — it and the rest go to the scanner
        for j = i, #argv do remaining_argv[#remaining_argv + 1] = argv[j] end
      else
        i = i + 1
      end
    end
  end
  -- remaining_argv already contains all flag tokens collected during routing.
  -- No need to clear it — flags like --help and -h must still be processed.

  -- ── Build flag lookups for current command scope ─────────────────────────
  local all_flag_defs = {}
  for _, f in ipairs(spec.global_flags or {}) do all_flag_defs[#all_flag_defs + 1] = f end
  for _, f in ipairs(current_node.flags or spec.flags or {}) do all_flag_defs[#all_flag_defs + 1] = f end

  -- Add builtin flags
  if spec.builtin_flags.help then
    all_flag_defs[#all_flag_defs + 1] = {
      id="__help__", short="h", long="help", type="boolean",
      description="Show help", required=false, default=false,
      value_name="", enum_values={}, conflicts_with={}, requires={},
      required_unless={}, repeatable=false, single_dash_long=nil,
    }
  end
  if spec.builtin_flags.version and spec.version then
    all_flag_defs[#all_flag_defs + 1] = {
      id="__version__", short=nil, long="version", type="boolean",
      description="Show version", required=false, default=false,
      value_name="", enum_values={}, conflicts_with={}, requires={},
      required_unless={}, repeatable=false, single_dash_long=nil,
    }
  end

  local _, by_short, by_long, by_sdl = all_flags_map(current_node, all_flag_defs)

  -- ── Phase 2: Scanning ────────────────────────────────────────────────────
  local flags          = {}
  local positionals    = {}
  local explicit_flags = {}
  local end_of_flags   = false
  local expecting_value_for = nil  -- flag def awaiting a value token

  local function set_flag(f, value)
    local id = f.id
    if f.type == "count" then
      flags[id] = (flags[id] or 0) + 1
    elseif f.repeatable then
      if flags[id] == nil then flags[id] = {} end
      if type(flags[id]) == "table" then
        flags[id][#flags[id] + 1] = coerce(value, f.type, id)
      else
        flags[id] = { flags[id], coerce(value, f.type, id) }
      end
    else
      flags[id] = coerce(value, f.type, id)
    end
    explicit_flags[#explicit_flags + 1] = id
  end

  local function handle_token(token)
    if expecting_value_for then
      local f = expecting_value_for
      expecting_value_for = nil
      set_flag(f, token)
      return
    end

    if end_of_flags then
      positionals[#positionals + 1] = token
      return
    end

    local classified = TokenClassifier.classify(token, all_flag_defs)

    if classified.kind == "end_of_flags" then
      end_of_flags = true

    elseif classified.kind == "long_flag" then
      local f = by_long[classified.name]
      if not f then
        errors[#errors + 1] = { error_type = "unknown_flag", message = "unknown flag: --" .. classified.name }
        return
      end
      if f.type == "boolean" then
        set_flag(f, true)
      elseif f.type == "count" then
        set_flag(f, 1)
      else
        expecting_value_for = f
      end
      -- Handle --help / --version
      if f.id == "__help__"    then return "help"    end
      if f.id == "__version__" then return "version" end

    elseif classified.kind == "long_flag_value" then
      local f = by_long[classified.name]
      if not f then
        errors[#errors + 1] = { error_type = "unknown_flag", message = "unknown flag: --" .. classified.name }
        return
      end
      set_flag(f, classified.value)

    elseif classified.kind == "short_flag" then
      local f = by_short[classified.char]
      if not f then
        errors[#errors + 1] = { error_type = "unknown_flag", message = "unknown flag: -" .. classified.char }
        return
      end
      if f.type == "boolean" then
        set_flag(f, true)
      elseif f.type == "count" then
        set_flag(f, 1)
      else
        expecting_value_for = f
      end
      if f.id == "__help__"    then return "help"    end
      if f.id == "__version__" then return "version" end

    elseif classified.kind == "short_flag_value" then
      local f = by_short[classified.char]
      if not f then
        errors[#errors + 1] = { error_type = "unknown_flag", message = "unknown flag: -" .. classified.char }
        return
      end
      set_flag(f, classified.value)

    elseif classified.kind == "stacked_flags" then
      for _, ch in ipairs(classified.chars) do
        local f = by_short[ch]
        if not f then
          errors[#errors + 1] = { error_type = "unknown_flag", message = "unknown flag in stack: -" .. ch }
          return
        end
        if f.type == "boolean" then
          set_flag(f, true)
        elseif f.type == "count" then
          set_flag(f, 1)
        end
        if f.id == "__help__"    then return "help"    end
        if f.id == "__version__" then return "version" end
      end

    elseif classified.kind == "single_dash_long" then
      local f = by_sdl[classified.name]
      if not f then
        errors[#errors + 1] = { error_type = "unknown_flag", message = "unknown flag: -" .. classified.name }
        return
      end
      if f.type == "boolean" then
        set_flag(f, true)
      else
        expecting_value_for = f
      end

    elseif classified.kind == "unknown_flag" then
      errors[#errors + 1] = { error_type = "unknown_flag", message = "unknown flag: " .. classified.token }

    elseif classified.kind == "positional" then
      positionals[#positionals + 1] = classified.value
    end
  end

  for _, token in ipairs(remaining_argv) do
    local signal = handle_token(token)
    if signal == "help" then
      return {
        type         = "help",
        text         = HelpGenerator.generate(spec, command_path),
        command_path = command_path,
      }
    elseif signal == "version" then
      return {
        type    = "version",
        version = spec.version or "",
      }
    end
  end

  -- If we were expecting a value but ran out of tokens
  if expecting_value_for then
    errors[#errors + 1] = {
      error_type = "missing_flag_value",
      message    = "flag --" .. (expecting_value_for.long or expecting_value_for.id) .. " requires a value"
    }
  end

  -- ── Phase 3: Validation ──────────────────────────────────────────────────
  -- Apply defaults first
  apply_defaults(flags, all_flag_defs)

  -- Resolve positionals to argument slots
  local arg_defs  = current_node.arguments or spec.arguments or {}
  local arguments = {}
  local pos_idx   = 1

  for _, a in ipairs(arg_defs) do
    if a.variadic then
      local vals = {}
      while pos_idx <= #positionals do
        vals[#vals + 1] = positionals[pos_idx]
        pos_idx = pos_idx + 1
      end
      if a.required and #vals == 0 then
        errors[#errors + 1] = { error_type = "missing_argument", message = "required argument <" .. a.display_name .. "> is missing" }
      end
      arguments[a.id] = vals
    else
      if positionals[pos_idx] then
        arguments[a.id] = positionals[pos_idx]
        pos_idx = pos_idx + 1
      elseif a.required then
        errors[#errors + 1] = { error_type = "missing_argument", message = "required argument <" .. a.display_name .. "> is missing" }
      else
        arguments[a.id] = nil
      end
    end
  end

  -- Extra positionals not consumed by argument slots
  if pos_idx <= #positionals and #arg_defs == 0 then
    for j = pos_idx, #positionals do
      errors[#errors + 1] = {
        error_type = "unexpected_argument",
        message    = "unexpected argument: " .. positionals[j]
      }
    end
  end

  -- Flag validation
  local flag_errors = FlagValidator.validate(flags, all_flag_defs)
  for _, e in ipairs(flag_errors) do
    errors[#errors + 1] = { error_type = "flag_error", message = e }
  end

  -- Remove builtin flags from result
  flags["__help__"]    = nil
  flags["__version__"] = nil

  if #errors > 0 then
    return { type = "error", errors = errors }
  end

  return {
    type           = "result",
    program        = spec.name,
    command_path   = command_path,
    flags          = flags,
    arguments      = arguments,
    explicit_flags = explicit_flags,
  }
end

M.Parser = Parser

-- ============================================================================
-- Public API
-- ============================================================================

--- Parse argv against a spec supplied as a Lua table.
-- @param spec_table  Raw spec table (will be normalized internally).
-- @param argv        List of argument strings (no argv[0]).
-- @return            Result table.
function M.parse_table(spec_table, argv)
  local spec = SpecLoader.load_table(spec_table)
  return Parser.parse(spec, argv)
end

--- Parse argv against a spec supplied as a JSON string.
-- @param spec_json  JSON string.
-- @param argv       List of argument strings.
-- @return           Result table.
function M.parse_string(spec_json, argv)
  local spec = SpecLoader.load_string(spec_json)
  return Parser.parse(spec, argv)
end

--- Parse argv against a spec file.
-- @param spec_path  Path to JSON spec file.
-- @param argv       List of argument strings.
-- @return           Result table.
function M.parse(spec_path, argv)
  local f = io.open(spec_path, "r")
  assert(f, "Cannot read spec file: " .. spec_path)
  local content = f:read("*a")
  f:close()
  return M.parse_string(content, argv)
end

return M
