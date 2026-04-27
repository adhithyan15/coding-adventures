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

local function normalize_path(path)
    local normalized = path:gsub("\\", "/")
    local prefix = ""
    local segments = {}

    if normalized:match("^[A-Za-z]:/") then
        prefix = normalized:sub(1, 2):lower()
        normalized = normalized:sub(3)
    elseif normalized:sub(1, 1) == "/" then
        prefix = "/"
        normalized = normalized:sub(2)
    end

    for segment in normalized:gmatch("[^/]+") do
        if segment == ".." then
            if #segments > 0 and segments[#segments] ~= ".." then
                table.remove(segments)
            elseif prefix == "" then
                segments[#segments + 1] = segment
            end
        elseif segment ~= "." and segment ~= "" then
            segments[#segments + 1] = segment
        end
    end

    local joined = table.concat(segments, "/")
    if prefix == "/" then
        return joined == "" and "/" or "/" .. joined
    elseif prefix ~= "" then
        return joined == "" and (prefix .. "/") or (prefix .. "/" .. joined)
    end
    return joined
end

-- =========================================================================
-- Helper: find a file by extension in a directory.
-- =========================================================================

local function find_file_by_extension(directory, extension)
    -- Try using lfs if available.
    local lfs_ok, lfs = pcall(require, "lfs")
    if lfs_ok then
        -- lfs.dir() throws if the directory doesn't exist, so wrap in pcall.
        local ok, iter, state = pcall(lfs.dir, directory)
        if not ok then return nil end
        for name in iter, state do
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

local function find_files_by_extensions(directory, extensions)
    local files = {}
    local seen = {}

    for _, extension in ipairs(extensions) do
        local filepath = find_file_by_extension(directory, extension)
        if filepath then
            local normalized = normalize_path(filepath)
            if not seen[normalized] then
                seen[normalized] = true
                files[#files + 1] = filepath
            end
        end
    end

    table.sort(files)
    return files
end

local function dependency_scope(language)
    if language == "csharp" or language == "fsharp" or language == "dotnet" then
        return "dotnet"
    elseif language == "wasm" then
        return "rust"
    end
    return language
end

local function in_dependency_scope(package_language, scope)
    if scope == "dotnet" then
        return package_language == "csharp" or package_language == "fsharp" or package_language == "dotnet"
    elseif scope == "rust" then
        return package_language == "rust" or package_language == "wasm"
    end
    return package_language == scope
end

local function read_cargo_package_name(pkg)
    local text = read_file(pkg.path .. "/Cargo.toml")
    if not text then return nil end

    for line in text:gmatch("[^\n]+") do
        local name = line:match('^%s*name%s*=%s*"([^"]+)"')
        if name then
            return name:lower()
        end
    end

    return nil
end

local function set_known(known, key, pkg)
    if not known[key] then
        known[key] = pkg.name
        return
    end

    local normalized = pkg.path:gsub("\\", "/"):lower()
    if not normalized:match("/programs/") then
        known[key] = pkg.name
    end
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
        local name = gem_name:lower()
        if known_names[name] then
            deps[#deps + 1] = known_names[name]
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
                for raw_npm_name in stripped:gmatch('"(@coding%-adventures/[^"]+)"') do
                    local npm_name = raw_npm_name:lower()
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

local function parse_swift_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/Package.swift")
    if not text then return {} end

    local deps = {}
    for dep_path in text:gmatch('%.package%s*%(%s*path%s*:%s*"([^"]+)"') do
        local cleaned = normalize_path(dep_path)
        if not cleaned:match("^[A-Za-z]:/") and cleaned:sub(1, 1) ~= "/" then
            local dep_dir = cleaned:match("([^/]+)$")
            if dep_dir and dep_dir ~= "." and dep_dir ~= ".." then
                dep_dir = dep_dir:lower()
                if known_names[dep_dir] then
                    deps[#deps + 1] = known_names[dep_dir]
                end
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
        local name = app_name:lower()
        if known_names[name] then
            deps[#deps + 1] = known_names[name]
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
-- Perl dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Perl cpanfile.
--
-- A cpanfile declares dependencies with one `requires` per line:
--
--     requires 'coding-adventures-logic-gates';
--     requires 'coding-adventures-bitset', '>= 0.01';
--
-- We scan for lines matching `requires 'coding-adventures-...'` and map
-- them to internal package names. External deps are silently skipped.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from CPAN dist name to package name.
-- @return table List of internal dependency package names.
local function parse_perl_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/cpanfile")
    if not text then return {} end

    local deps = {}

    for line in text:gmatch("[^\n]+") do
        local trimmed = line:match("^%s*(.-)%s*$")

        -- Skip blank lines and comments.
        if trimmed ~= "" and not trimmed:match("^#") then
            -- Match: requires 'coding-adventures-foo' or requires "coding-adventures-foo"
            local dep_kebab = trimmed:match("requires%s+['\"]coding%-adventures%-([^'\"]+)['\"]")
            if dep_kebab then
                local dep_name = "coding-adventures-" .. dep_kebab:lower()
                if known_names[dep_name] then
                    deps[#deps + 1] = known_names[dep_name]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Haskell dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Haskell .cabal file.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from cabal name to package name.
-- @return table List of internal dependency package names.
local function parse_haskell_deps(pkg, known_names)
    local cabal_path = find_file_by_extension(pkg.path, "cabal")
    if not cabal_path then return {} end

    local text = read_file(cabal_path)
    if not text then return {} end

    local deps = {}

    for dep_name in text:gmatch("(coding%-adventures%-[%a%d%-]+)") do
        local name = dep_name:lower()
        if known_names[name] and known_names[name] ~= pkg.name then
            deps[#deps + 1] = known_names[name]
        end
    end

    return deps
end

local function parse_dotnet_deps(pkg, known_names)
    local project_files = find_files_by_extensions(pkg.path, {"csproj", "fsproj"})
    if #project_files == 0 then return {} end

    local deps = {}
    for _, project_file in ipairs(project_files) do
        local text = read_file(project_file)
        if text then
            for include_path in text:gmatch('<ProjectReference%s+Include%s*=%s*"([^"]+)"') do
                local normalized = normalize_path(pkg.path .. "/" .. include_path)
                if known_names[normalized] then
                    deps[#deps + 1] = known_names[normalized]
                end
            end
        end
    end

    return deps
end

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
function Resolver.build_known_names(packages, language)
    local known = {}
    local scope = language or ""

    for _, pkg in ipairs(packages) do
        if scope == "" or in_dependency_scope(pkg.language, scope) then
            local basename = pkg.path:gsub("\\", "/"):match("([^/]+)$"):lower()

            if pkg.language == "python" then
                set_known(known, "coding-adventures-" .. basename, pkg)

            elseif pkg.language == "ruby" then
                set_known(known, "coding_adventures_" .. basename, pkg)

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
                set_known(known, "@coding-adventures/" .. basename, pkg)
                set_known(known, basename, pkg)

            elseif pkg.language == "rust" or pkg.language == "wasm" then
                set_known(known, basename, pkg)
                local cargo_name = read_cargo_package_name(pkg)
                if cargo_name then
                    set_known(known, cargo_name, pkg)
                end

            elseif pkg.language == "elixir" then
                set_known(known, "coding_adventures_" .. basename:gsub("%-", "_"), pkg)
                set_known(known, basename:gsub("%-", "_"), pkg)

            elseif pkg.language == "lua" then
                set_known(known, "coding-adventures-" .. basename:gsub("_", "-"), pkg)

            elseif pkg.language == "perl" then
                set_known(known, "coding-adventures-" .. basename, pkg)

            elseif pkg.language == "swift" then
                set_known(known, basename, pkg)

            elseif pkg.language == "haskell" then
                set_known(known, "coding-adventures-" .. basename:gsub("_", "-"), pkg)

            elseif pkg.language == "csharp" or pkg.language == "fsharp" or pkg.language == "dotnet" then
                for _, project_file in ipairs(find_files_by_extensions(pkg.path, {"csproj", "fsproj"})) do
                    known[normalize_path(project_file)] = pkg.name
                end
            end
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

    local known_names_by_scope = {}
    for _, pkg in ipairs(packages) do
        local scope = dependency_scope(pkg.language)
        if not known_names_by_scope[scope] then
            known_names_by_scope[scope] = Resolver.build_known_names(packages, scope)
        end
    end

    -- Dispatch table for language-specific parsers.
    local parsers = {
        python     = parse_python_deps,
        ruby       = parse_ruby_deps,
        go         = parse_go_deps,
        typescript = parse_typescript_deps,
        rust       = parse_rust_deps,
        wasm       = parse_rust_deps,
        elixir     = parse_elixir_deps,
        lua        = parse_lua_deps,
        perl       = parse_perl_deps,
        swift      = parse_swift_deps,
        haskell    = parse_haskell_deps,
        csharp     = parse_dotnet_deps,
        fsharp     = parse_dotnet_deps,
        dotnet     = parse_dotnet_deps,
    }

    -- Parse dependencies for each package and add edges.
    for _, pkg in ipairs(packages) do
        local parser = parsers[pkg.language]
        if parser then
            local known_names = known_names_by_scope[dependency_scope(pkg.language)] or {}
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
