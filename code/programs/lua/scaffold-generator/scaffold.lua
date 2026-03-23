#!/usr/bin/env lua
-- scaffold.lua — Generate CI-ready Lua package scaffolding
-- ==========================================================
--
-- This tool generates correctly-structured, CI-ready Lua package directories
-- for the coding-adventures monorepo. It creates all required files:
-- rockspec, BUILD, BUILD_windows, source stubs, test stubs, README, CHANGELOG.
--
-- === Why This Tool Exists ===
--
-- Hand-crafting packages leads to recurring CI failures: missing BUILD files,
-- wrong test patterns, missing dependency installs. This tool eliminates
-- those failures by generating packages that compile, lint, and pass tests
-- out of the box. Then you fill in the business logic.
--
-- === Usage ===
--
--   lua scaffold.lua PACKAGE_NAME [options]
--
--     --depends-on dep1,dep2    Comma-separated list of sibling packages
--     --layer N                 Layer number for README context
--     --description "text"      One-line package description
--     --type library|program    Package type (default: library)
--     --dry-run                 Print what would be generated
--     --help                    Show help

local VALID_NAME_PATTERN = "^[a-z][a-z0-9]*%-?[a-z0-9%-]*$"

-- =========================================================================
-- Name normalization
-- =========================================================================

--- Convert kebab-case to snake_case: "logic-gates" → "logic_gates"
local function to_snake_case(kebab)
    return (kebab:gsub("%-", "_"))
end

--- Convert kebab-case to PascalCase: "logic-gates" → "LogicGates"
local function to_pascal_case(kebab)
    return (kebab:gsub("(%a)([%w]*)", function(first, rest)
        return first:upper() .. rest
    end):gsub("%-", ""))
end

-- =========================================================================
-- File I/O helpers
-- =========================================================================

--- Create a directory and all parent directories.
local function mkdir_p(path)
    -- Normalize to forward slashes for the command.
    local normalized = path:gsub("\\", "/")
    local sep = package.config:sub(1, 1)
    if sep == "\\" then
        -- Windows: use mkdir with backslashes
        os.execute('mkdir "' .. path:gsub("/", "\\") .. '" 2>nul')
    else
        os.execute('mkdir -p "' .. normalized .. '"')
    end
end

--- Write content to a file, creating parent directories as needed.
local function write_file(path, content)
    local dir = path:match("(.+)[/\\][^/\\]+$")
    if dir then
        mkdir_p(dir)
    end
    local f = io.open(path, "w")
    if not f then
        error("Could not write to: " .. path)
    end
    f:write(content)
    f:close()
end

--- Check if a directory exists.
local function dir_exists(path)
    local lfs_ok, lfs = pcall(require, "lfs")
    if lfs_ok then
        local attr = lfs.attributes(path)
        return attr ~= nil and attr.mode == "directory"
    end
    local sep = package.config:sub(1, 1)
    if sep == "\\" then
        local ok = os.execute('if exist "' .. path:gsub("/", "\\") .. '" (exit 0) else (exit 1)')
        return ok == true or ok == 0
    else
        local ok = os.execute('test -d "' .. path .. '"')
        return ok == true or ok == 0
    end
end

--- Check if a file exists.
local function file_exists(path)
    local f = io.open(path, "r")
    if f then f:close() return true end
    return false
end

-- =========================================================================
-- Dependency resolution
-- =========================================================================

--- Read direct dependencies of a Lua package from its rockspec file.
--
-- Scans the dependencies = { ... } block for entries matching
-- "coding-adventures-*", strips version specifiers, and converts
-- rockspec names back to kebab-case package names.
--
-- @param pkg_dir string Path to the package directory.
-- @return table List of kebab-case dependency names.
local function read_lua_deps(pkg_dir)
    -- Find rockspec file.
    local rockspec_path = nil
    local lfs_ok, lfs = pcall(require, "lfs")
    if lfs_ok then
        local ok, iter, state = pcall(lfs.dir, pkg_dir)
        if ok then
            for name in iter, state do
                if name:match("%.rockspec$") then
                    rockspec_path = pkg_dir .. "/" .. name
                    break
                end
            end
        end
    else
        local sep = package.config:sub(1, 1)
        local cmd
        if sep == "\\" then
            cmd = 'dir /b "' .. pkg_dir:gsub("/", "\\") .. '\\*.rockspec" 2>nul'
        else
            cmd = 'ls "' .. pkg_dir .. '"/*.rockspec 2>/dev/null'
        end
        local handle = io.popen(cmd)
        if handle then
            local result = handle:read("*l")
            handle:close()
            if result and result ~= "" then
                local name = result:match("([^/\\]+)$") or result
                rockspec_path = pkg_dir .. "/" .. name:match("^%s*(.-)%s*$")
            end
        end
    end

    if not rockspec_path then return {} end

    local f = io.open(rockspec_path, "r")
    if not f then return {} end
    local text = f:read("*a")
    f:close()

    local deps = {}
    local in_deps = false
    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")
        if not in_deps then
            if stripped:match("dependencies") and stripped:match("=") and stripped:match("{") then
                in_deps = true
                if stripped:match("}") then
                    for dep_str in stripped:gmatch('"(coding%-adventures%-[^"]+)"') do
                        local name = dep_str:match("^coding%-adventures%-(.+)$")
                        if name then
                            name = name:match("^([%w%-]+)") -- strip version
                            deps[#deps + 1] = name
                        end
                    end
                    break
                end
            end
        else
            if stripped:match("}") then break end
            for dep_str in stripped:gmatch('"(coding%-adventures%-[^"]+)"') do
                local name = dep_str:match("^coding%-adventures%-(.+)$")
                if name then
                    name = name:match("^([%w%-]+)") -- strip version
                    deps[#deps + 1] = name
                end
            end
        end
    end
    return deps
end

--- Compute all transitive dependencies via BFS.
--
-- @param direct_deps table List of direct dependency names (kebab-case).
-- @param base_dir string Path to code/packages/lua/.
-- @return table Sorted list of all transitive dependency names.
local function transitive_closure(direct_deps, base_dir)
    local visited = {}
    local visited_set = {}
    local queue = {}

    for _, dep in ipairs(direct_deps) do
        if not visited_set[dep] then
            queue[#queue + 1] = dep
        end
    end

    while #queue > 0 do
        local dep = table.remove(queue, 1)
        if not visited_set[dep] then
            visited_set[dep] = true
            visited[#visited + 1] = dep
            local dep_dir = base_dir .. "/" .. to_snake_case(dep)
            local sub_deps = read_lua_deps(dep_dir)
            for _, dd in ipairs(sub_deps) do
                if not visited_set[dd] then
                    queue[#queue + 1] = dd
                end
            end
        end
    end

    table.sort(visited)
    return visited
end

--- Topological sort via Kahn's algorithm. Returns leaf-first order.
--
-- @param all_deps table List of all dependency names.
-- @param base_dir string Path to code/packages/lua/.
-- @return table Dependencies in install order (leaves first).
local function topological_sort(all_deps, base_dir)
    local dep_set = {}
    for _, dep in ipairs(all_deps) do
        dep_set[dep] = true
    end

    -- Build adjacency: graph[dep] = list of deps it depends on (within our set)
    local graph = {}
    local in_degree = {}
    for _, dep in ipairs(all_deps) do
        graph[dep] = {}
        in_degree[dep] = 0
    end

    for _, dep in ipairs(all_deps) do
        local dep_dir = base_dir .. "/" .. to_snake_case(dep)
        local sub_deps = read_lua_deps(dep_dir)
        for _, dd in ipairs(sub_deps) do
            if dep_set[dd] then
                graph[dep][#graph[dep] + 1] = dd
                in_degree[dep] = in_degree[dep] + 1
            end
        end
    end

    -- Start with leaves (in_degree == 0)
    local queue = {}
    for _, dep in ipairs(all_deps) do
        if in_degree[dep] == 0 then
            queue[#queue + 1] = dep
        end
    end
    table.sort(queue)

    local result = {}
    while #queue > 0 do
        local node = table.remove(queue, 1)
        result[#result + 1] = node
        -- Decrease in-degree for nodes that depend on this one.
        for _, dep in ipairs(all_deps) do
            for _, dd in ipairs(graph[dep]) do
                if dd == node then
                    in_degree[dep] = in_degree[dep] - 1
                    if in_degree[dep] == 0 then
                        queue[#queue + 1] = dep
                        table.sort(queue)
                    end
                    break
                end
            end
        end
    end

    if #result ~= #all_deps then
        error(string.format("circular dependency detected: resolved %d of %d", #result, #all_deps))
    end

    return result
end

-- =========================================================================
-- File generation
-- =========================================================================

--- Generate the rockspec file content.
local function generate_rockspec(pkg_name, description, direct_deps)
    local snake = to_snake_case(pkg_name)

    local deps_lines = '    "lua >= 5.4",\n'
    for _, dep in ipairs(direct_deps) do
        deps_lines = deps_lines .. string.format('    "coding-adventures-%s >= 0.1.0",\n', dep)
    end

    return string.format([[package = "coding-adventures-%s"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "%s",
    license = "MIT",
}
dependencies = {
%s}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.%s"] = "src/coding_adventures/%s/init.lua",
    },
}
]], pkg_name, description, deps_lines, snake, snake)
end

--- Generate the init.lua content.
local function generate_init_lua(pkg_name, description, layer_ctx)
    local snake = to_snake_case(pkg_name)
    return string.format([[-- %s — %s
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- %s

local %s = {}

%s.VERSION = "0.1.0"

return %s
]], pkg_name, description, layer_ctx, snake, snake, snake)
end

--- Generate the test file content.
local function generate_test_lua(pkg_name)
    local snake = to_snake_case(pkg_name)

    -- Build a require path. Tests run from the tests/ directory, so we need
    -- to add the parent's src/ to the path.
    return string.format([[-- Tests for %s.

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local %s = require("coding_adventures.%s")

describe("%s", function()
    it("has a version", function()
        assert.are.equal("0.1.0", %s.VERSION)
    end)
end)
]], pkg_name, snake, snake, pkg_name, snake)
end

--- Generate the BUILD file content.
--
-- Dependencies are resolved at test time via package.path in the test files
-- (adding sibling package src/ directories). No luarocks install needed —
-- the packages aren't published to LuaRocks, they're local siblings.
local function generate_build(direct_deps, ordered_deps, pkg_name)
    return "cd tests && busted . --verbose --pattern=test_\n"
end

--- Generate the README.md content.
local function generate_readme(pkg_name, description, layer, direct_deps)
    local lines = {
        "# " .. pkg_name,
        "",
        description,
        "",
    }

    if layer > 0 then
        lines[#lines + 1] = "## Layer " .. layer
        lines[#lines + 1] = ""
        lines[#lines + 1] = "This package is part of Layer " .. layer .. " of the coding-adventures computing stack."
        lines[#lines + 1] = ""
    end

    if #direct_deps > 0 then
        lines[#lines + 1] = "## Dependencies"
        lines[#lines + 1] = ""
        for _, dep in ipairs(direct_deps) do
            lines[#lines + 1] = "- " .. dep
        end
        lines[#lines + 1] = ""
    end

    lines[#lines + 1] = "## Development"
    lines[#lines + 1] = ""
    lines[#lines + 1] = "```bash"
    lines[#lines + 1] = "# Run tests"
    lines[#lines + 1] = "bash BUILD"
    lines[#lines + 1] = "```"
    lines[#lines + 1] = ""

    return table.concat(lines, "\n")
end

--- Generate the CHANGELOG.md content.
local function generate_changelog()
    local today = os.date("%Y-%m-%d")
    return string.format([[# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - %s

### Added

- Initial package scaffolding generated by scaffold-generator
]], today)
end

-- =========================================================================
-- Main scaffold logic
-- =========================================================================

--- Find the monorepo root by walking up from cwd looking for .git/.
local function find_repo_root()
    -- Try to get current directory.
    local handle = io.popen("pwd 2>/dev/null || cd 2>nul")
    if not handle then return nil end
    local cwd = handle:read("*l")
    handle:close()
    if not cwd then return nil end
    cwd = cwd:match("^%s*(.-)%s*$")

    -- Walk up looking for .git.
    local d = cwd
    while true do
        if dir_exists(d .. "/.git") then
            return d
        end
        local parent = d:match("^(.+)[/\\][^/\\]+$")
        if not parent or parent == d then
            return nil
        end
        d = parent
    end
end

--- Scaffold a single Lua package.
local function scaffold_one(opts)
    local pkg_name = opts.pkg_name
    local description = opts.description
    local layer = opts.layer
    local direct_deps = opts.direct_deps
    local dry_run = opts.dry_run
    local repo_root = opts.repo_root
    local pkg_type = opts.pkg_type

    local snake = to_snake_case(pkg_name)
    local base_category = pkg_type == "library" and "packages" or "programs"
    local base_dir = repo_root .. "/code/" .. base_category .. "/lua"
    local target_dir = base_dir .. "/" .. snake

    -- Check if directory already exists.
    if dir_exists(target_dir) then
        error("directory already exists: " .. target_dir)
    end

    -- Check that all direct dependencies exist.
    for _, dep in ipairs(direct_deps) do
        local dep_dir = base_dir .. "/" .. to_snake_case(dep)
        if not dir_exists(dep_dir) then
            error("dependency '" .. dep .. "' not found at " .. dep_dir)
        end
    end

    -- Compute transitive deps and install order.
    local all_deps = transitive_closure(direct_deps, base_dir)
    local ordered_deps = topological_sort(all_deps, base_dir)

    local layer_ctx = layer > 0 and ("Layer " .. layer .. " in the computing stack.") or ""

    if dry_run then
        print("[dry-run] Would create Lua package at: " .. target_dir)
        print("  Direct deps: " .. table.concat(direct_deps, ", "))
        print("  All transitive deps: " .. table.concat(all_deps, ", "))
        print("  Install order: " .. table.concat(ordered_deps, ", "))
        return
    end

    -- Generate all files.
    write_file(
        target_dir .. "/coding-adventures-" .. pkg_name .. "-0.1.0-1.rockspec",
        generate_rockspec(pkg_name, description, direct_deps)
    )
    write_file(
        target_dir .. "/src/coding_adventures/" .. snake .. "/init.lua",
        generate_init_lua(pkg_name, description, layer_ctx)
    )
    write_file(
        target_dir .. "/tests/test_" .. snake .. ".lua",
        generate_test_lua(pkg_name)
    )
    write_file(target_dir .. "/BUILD", generate_build(direct_deps, ordered_deps, pkg_name))
    write_file(target_dir .. "/BUILD_windows", 'echo "Skipping Lua on Windows CI"\n')
    write_file(target_dir .. "/README.md", generate_readme(pkg_name, description, layer, direct_deps))
    write_file(target_dir .. "/CHANGELOG.md", generate_changelog())

    print("Created Lua package at: " .. target_dir)
end

-- =========================================================================
-- Argument parsing
-- =========================================================================

local function parse_args()
    local opts = {
        pkg_name = nil,
        pkg_type = "library",
        direct_deps = {},
        layer = 0,
        description = "",
        dry_run = false,
    }

    if #arg == 0 or arg[1] == "--help" or arg[1] == "-h" then
        print("Usage: lua scaffold.lua PACKAGE_NAME [options]")
        print()
        print("Generate a CI-ready Lua package for the coding-adventures monorepo.")
        print()
        print("Arguments:")
        print("  PACKAGE_NAME             Kebab-case package name (e.g., logic-gates)")
        print()
        print("Options:")
        print("  --depends-on dep1,dep2   Comma-separated sibling package names")
        print("  --layer N                Layer number for README context")
        print("  --description \"text\"     One-line package description")
        print("  --type library|program   Package type (default: library)")
        print("  --dry-run                Print what would be generated")
        print("  --help                   Show this help")
        os.exit(0)
    end

    opts.pkg_name = arg[1]

    local i = 2
    while i <= #arg do
        if arg[i] == "--depends-on" or arg[i] == "-d" then
            i = i + 1
            if arg[i] then
                for dep in arg[i]:gmatch("([^,]+)") do
                    dep = dep:match("^%s*(.-)%s*$")
                    if dep ~= "" then
                        opts.direct_deps[#opts.direct_deps + 1] = dep
                    end
                end
            end
        elseif arg[i] == "--layer" then
            i = i + 1
            opts.layer = tonumber(arg[i]) or 0
        elseif arg[i] == "--description" then
            i = i + 1
            opts.description = arg[i] or ""
        elseif arg[i] == "--type" or arg[i] == "-t" then
            i = i + 1
            opts.pkg_type = arg[i] or "library"
        elseif arg[i] == "--dry-run" then
            opts.dry_run = true
        end
        i = i + 1
    end

    -- Validate package name.
    if not opts.pkg_name:match(VALID_NAME_PATTERN) then
        io.stderr:write("scaffold-generator: invalid package name '" .. opts.pkg_name .. "' (must be kebab-case)\n")
        os.exit(1)
    end

    return opts
end

-- =========================================================================
-- Main
-- =========================================================================

local function main()
    local opts = parse_args()

    local repo_root = find_repo_root()
    if not repo_root then
        io.stderr:write("scaffold-generator: not inside a git repository\n")
        os.exit(1)
    end

    opts.repo_root = repo_root

    local ok, err = pcall(scaffold_one, opts)
    if not ok then
        io.stderr:write("scaffold-generator: " .. tostring(err) .. "\n")
        os.exit(1)
    end
end

main()
