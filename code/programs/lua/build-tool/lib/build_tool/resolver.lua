-- resolver.lua -- Dependency Resolution from Package Metadata
-- ============================================================
--
-- This module reads package metadata files (pyproject.toml, .gemspec, go.mod,
-- package.json, Cargo.toml, mix.exs, .rockspec) and extracts internal
-- dependencies. It builds a directed graph where edges represent build
-- ordering: an edge from A to B means "A must be built before B".
--
-- Dependency mapping conventions
-- ------------------------------
--
-- Each language ecosystem uses a different naming convention:
--
--   Python:     "coding-adventures-logic-gates"      (hyphens, pyproject.toml)
--   Ruby:       "coding_adventures_logic_gates"      (underscores, .gemspec)
--   Go:         full module path                     (go.mod)
--   TypeScript: "@coding-adventures/logic-gates"     (npm scope, package.json)
--   Rust:       "logic-gates"                        (crate name, Cargo.toml)
--   Elixir:     "coding_adventures_logic_gates"      (underscores, mix.exs)
--   Lua:        "coding-adventures-logic-gates"      (hyphens, .rockspec)
--
-- External dependencies are silently skipped.

local DirectedGraph = require("build_tool.directed_graph")
local Discovery = require("build_tool.discovery")

local Resolver = {}

-- =========================================================================
-- Helper: read a file's contents as a string.
-- =========================================================================

local function read_file(path)
    local f = io.open(path, "r")
    if not f then return nil end
    local content = f:read("*a")
    f:close()
    return content
end

-- =========================================================================
-- Helper: find a file by extension in a directory.
-- =========================================================================

local function find_file_by_extension(directory, extension)
    -- Try using lfs if available.
    local lfs_ok, lfs = pcall(require, "lfs")
    if lfs_ok then
        for name in lfs.dir(directory) do
            if name:match("%." .. extension .. "$") then
                return directory .. "/" .. name
            end
        end
        return nil
    end

    -- Fallback: use ls or dir command.
    local sep = package.config:sub(1, 1)
    local cmd
    if sep == "\\" then
        cmd = 'dir /b "' .. directory .. '\\*.' .. extension .. '" 2>nul'
    else
        cmd = 'ls "' .. directory .. '"/*.' .. extension .. ' 2>/dev/null'
    end
    local handle = io.popen(cmd)
    if handle then
        local result = handle:read("*l")
        handle:close()
        if result then
            -- On Unix, ls returns the full path. On Windows, dir returns just the name.
            if sep == "\\" then
                return directory .. "/" .. result:match("^%s*(.-)%s*$")
            else
                return result:match("^%s*(.-)%s*$")
            end
        end
    end
    return nil
end

-- =========================================================================
-- Python dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Python pyproject.toml.
--
-- We look for the dependencies = [...] array and extract quoted strings
-- matching our internal naming convention.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from ecosystem name to package name.
-- @return table List of internal dependency package names.
local function parse_python_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/pyproject.toml")
    if not text then return {} end

    local deps = {}
    local in_deps = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if not in_deps then
            if stripped:match("^dependencies") and stripped:match("=") then
                local after_eq = stripped:match("=%s*(.*)$")
                if after_eq and after_eq:match("^%[") then
                    in_deps = true
                    if after_eq:match("%]") then
                        -- Single-line array.
                        for dep_str in after_eq:gmatch('"([^"]+)"') do
                            local name = dep_str:match("^([%w%-]+)"):lower()
                            if known_names[name] then
                                deps[#deps + 1] = known_names[name]
                            end
                        end
                        break
                    end
                end
            end
        else
            if stripped:match("%]") then
                break
            end
            for dep_str in stripped:gmatch('"([^"]+)"') do
                local name = dep_str:match("^([%w%-]+)"):lower()
                if known_names[name] then
                    deps[#deps + 1] = known_names[name]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Ruby dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Ruby .gemspec file.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from gem name to package name.
-- @return table List of internal dependency package names.
local function parse_ruby_deps(pkg, known_names)
    local gemspec_path = find_file_by_extension(pkg.path, "gemspec")
    if not gemspec_path then return {} end

    local text = read_file(gemspec_path)
    if not text then return {} end

    local deps = {}
    for gem_name in text:gmatch('spec%.add_dependency%s+"([^"]+)"') do
        gem_name = gem_name:lower()
        if known_names[gem_name] then
            deps[#deps + 1] = known_names[gem_name]
        end
    end

    return deps
end

-- =========================================================================
-- Go dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Go go.mod file.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from module path to package name.
-- @return table List of internal dependency package names.
local function parse_go_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/go.mod")
    if not text then return {} end

    local deps = {}
    local in_require = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if stripped == "require (" then
            in_require = true
        elseif stripped == ")" then
            in_require = false
        elseif in_require or stripped:match("^require ") then
            local clean = stripped:gsub("^require%s+", "")
            local module_path = clean:match("^(%S+)")
            if module_path then
                module_path = module_path:lower()
                if known_names[module_path] then
                    deps[#deps + 1] = known_names[module_path]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- TypeScript dependency parsing
-- =========================================================================

--- Extract internal dependencies from a TypeScript package.json.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from npm name to package name.
-- @return table List of internal dependency package names.
local function parse_typescript_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/package.json")
    if not text then return {} end

    local deps = {}
    local in_deps = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if not in_deps then
            if stripped:match('"dependencies"') and stripped:match("{") then
                in_deps = true
            end
        else
            if stripped:match("}") then
                in_deps = false
            else
                for npm_name in stripped:gmatch('"(@coding%-adventures/[^"]+)"') do
                    npm_name = npm_name:lower()
                    if known_names[npm_name] then
                        deps[#deps + 1] = known_names[npm_name]
                    end
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Rust dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Rust Cargo.toml.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from crate name to package name.
-- @return table List of internal dependency package names.
local function parse_rust_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/Cargo.toml")
    if not text then return {} end

    local deps = {}
    local in_deps = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if stripped:match("^%[") then
            in_deps = (stripped == "[dependencies]")
        elseif in_deps and stripped:match("path") and stripped:match("=") then
            local crate_name = stripped:match("^([%w%-_]+)%s*="):lower()
            if crate_name and known_names[crate_name] then
                deps[#deps + 1] = known_names[crate_name]
            end
        end
    end

    return deps
end

-- =========================================================================
-- Elixir dependency parsing
-- =========================================================================

--- Extract internal dependencies from an Elixir mix.exs.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from app name to package name.
-- @return table List of internal dependency package names.
local function parse_elixir_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/mix.exs")
    if not text then return {} end

    local deps = {}
    for app_name in text:gmatch("{:(coding_adventures_%w+)") do
        app_name = app_name:lower()
        if known_names[app_name] then
            deps[#deps + 1] = known_names[app_name]
        end
    end

    return deps
end

-- =========================================================================
-- Lua dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Lua .rockspec file.
--
-- LuaRocks rockspec files declare dependencies in a Lua table:
--
--     dependencies = {
--         "lua >= 5.4",
--         "coding-adventures-logic-gates >= 0.1.0",
--     }
--
-- We scan for quoted strings inside the dependencies block that match
-- known internal packages, stripping version specifiers.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from rockspec name to package name.
-- @return table List of internal dependency package names.
local function parse_lua_deps(pkg, known_names)
    local rockspec_path = find_file_by_extension(pkg.path, "rockspec")
    if not rockspec_path then return {} end

    local text = read_file(rockspec_path)
    if not text then return {} end

    local deps = {}
    local in_deps = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if not in_deps then
            if stripped:match("dependencies") and stripped:match("=") and stripped:match("{") then
                in_deps = true
                if stripped:match("}") then
                    -- Single-line block.
                    for dep_str in stripped:gmatch('"([^"]+)"') do
                        local name = dep_str:match("^([%w%-]+)"):lower()
                        if known_names[name] then
                            deps[#deps + 1] = known_names[name]
                        end
                    end
                    break
                end
            end
        else
            if stripped:match("}") then
                for dep_str in stripped:gmatch('"([^"]+)"') do
                    local name = dep_str:match("^([%w%-]+)"):lower()
                    if known_names[name] then
                        deps[#deps + 1] = known_names[name]
                    end
                end
                break
            end
            for dep_str in stripped:gmatch('"([^"]+)"') do
                local name = dep_str:match("^([%w%-]+)"):lower()
                if known_names[name] then
                    deps[#deps + 1] = known_names[name]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Rosetta Stone: ecosystem name → internal package name
-- =========================================================================

--- Build a mapping from ecosystem-specific dependency names to internal
-- package names.
--
-- This is the "Rosetta Stone" of the build system. Each language ecosystem
-- uses its own naming convention:
--
--   Python:     "coding-adventures-logic-gates"  → "python/logic-gates"
--   Ruby:       "coding_adventures_logic_gates"  → "ruby/logic_gates"
--   Go:         full module path                 → "go/module-name"
--   TypeScript: "@coding-adventures/logic-gates" → "typescript/logic-gates"
--   Rust:       "logic-gates"                    → "rust/logic-gates"
--   Elixir:     "coding_adventures_logic_gates"  → "elixir/logic_gates"
--   Lua:        "coding-adventures-logic-gates"  → "lua/logic_gates"
--
-- @param packages table List of package records.
-- @return table Mapping from ecosystem name to package name.
function Resolver.build_known_names(packages)
    local known = {}

    for _, pkg in ipairs(packages) do
        -- Extract the directory basename.
        local basename = pkg.path:gsub("\\", "/"):match("([^/]+)$"):lower()

        if pkg.language == "python" then
            local pypi_name = "coding-adventures-" .. basename
            known[pypi_name] = pkg.name

        elseif pkg.language == "ruby" then
            local gem_name = "coding_adventures_" .. basename
            known[gem_name] = pkg.name

        elseif pkg.language == "go" then
            local text = read_file(pkg.path .. "/go.mod")
            if text then
                for line in text:gmatch("[^\n]+") do
                    if line:match("^module ") then
                        local module_path = line:gsub("^module%s+", ""):match("^%s*(.-)%s*$"):lower()
                        known[module_path] = pkg.name
                        break
                    end
                end
            end

        elseif pkg.language == "typescript" then
            local npm_name = "@coding-adventures/" .. basename
            known[npm_name] = pkg.name

        elseif pkg.language == "rust" then
            known[basename] = pkg.name

        elseif pkg.language == "elixir" then
            local app_name = "coding_adventures_" .. basename:gsub("%-", "_")
            known[app_name] = pkg.name

        elseif pkg.language == "lua" then
            -- Lua dirs use underscores, rockspec names use hyphens.
            local rockspec_name = "coding-adventures-" .. basename:gsub("_", "-")
            known[rockspec_name] = pkg.name
        end
    end

    return known
end

-- =========================================================================
-- Main dependency resolution
-- =========================================================================

--- Parse package metadata to discover dependencies and build a graph.
--
-- The graph contains all discovered packages as nodes. Edges represent
-- build ordering: an edge from A to B means "A must be built before B"
-- (because B depends on A).
--
-- @param packages table List of package records from discovery.
-- @return DirectedGraph The dependency graph.
function Resolver.resolve_dependencies(packages)
    local graph = DirectedGraph.new()

    -- Add all packages as nodes first.
    for _, pkg in ipairs(packages) do
        graph:add_node(pkg.name)
    end

    -- Build the ecosystem-specific name mapping table.
    local known_names = Resolver.build_known_names(packages)

    -- Dispatch table for language-specific parsers.
    local parsers = {
        python     = parse_python_deps,
        ruby       = parse_ruby_deps,
        go         = parse_go_deps,
        typescript = parse_typescript_deps,
        rust       = parse_rust_deps,
        elixir     = parse_elixir_deps,
        lua        = parse_lua_deps,
    }

    -- Parse dependencies for each package and add edges.
    for _, pkg in ipairs(packages) do
        local parser = parsers[pkg.language]
        if parser then
            local deps = parser(pkg, known_names)
            for _, dep_name in ipairs(deps) do
                -- Edge direction: dep → pkg means "dep must be built before pkg".
                graph:add_edge(dep_name, pkg.name)
            end
        end
    end

    return graph
end

return Resolver
