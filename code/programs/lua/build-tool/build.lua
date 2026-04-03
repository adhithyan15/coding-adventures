#!/usr/bin/env lua
-- build.lua -- Lua Build Tool CLI
-- =================================
--
-- This is the entry point for the Lua build tool. It ties together all the
-- modules: discovery, resolver, executor, and reporter.
--
-- Usage:
--
--     lua build.lua                          # Build all packages
--     lua build.lua --root /path/to/repo     # Specify root
--     lua build.lua --dry-run                # Show what would build
--     lua build.lua --language python        # Only build Python packages
--
-- The flow is:
--   1. Discover packages (walk recursive BUILD files)
--   2. Filter by language if specified
--   3. Resolve dependencies (parse metadata files)
--   4. Compute build levels (topological sort via Kahn's algorithm)
--   5. If --dry-run, print levels and exit
--   6. Execute builds level by level
--   7. Print report
--   8. Exit with code 1 if any builds failed
--
-- This is an educational implementation of the build tool in Lua. The
-- primary build tool is the Go implementation at code/programs/go/build-tool/.

-- Add lib/ to the Lua module search path so our modules can be found.
local script_dir = arg[0]:match("(.*/)")
    or arg[0]:match("(.+\\)") or "./"
package.path = script_dir .. "lib/?.lua;" .. script_dir .. "lib/?/init.lua;" .. package.path

local Discovery = require("build_tool.discovery")
local Resolver = require("build_tool.resolver")
local Executor = require("build_tool.executor")
local Reporter = require("build_tool.reporter")
local Validator = require("build_tool.validator")

-- =========================================================================
-- Argument parsing
-- =========================================================================

--- Parse command-line arguments.
--
-- Lua doesn't have argparse in its stdlib, so we parse manually. This
-- is intentionally simple — the primary build tool is Go.
--
-- @return table Parsed options: root, dry_run, language, force.
local function parse_args()
    local opts = {
        root = nil,
        dry_run = false,
        language = "all",
        force = false,
        validate_build_files = false,
    }

    local i = 1
    while i <= #arg do
        if arg[i] == "--root" then
            i = i + 1
            opts.root = arg[i]
        elseif arg[i] == "--dry-run" then
            opts.dry_run = true
        elseif arg[i] == "--language" then
            i = i + 1
            opts.language = arg[i]
        elseif arg[i] == "--force" then
            opts.force = true
        elseif arg[i] == "--validate-build-files" then
            opts.validate_build_files = true
        elseif arg[i] == "--help" or arg[i] == "-h" then
            print("Usage: lua build.lua [OPTIONS]")
            print()
            print("Options:")
            print("  --root DIR       Monorepo root directory")
            print("  --dry-run        Show what would build without building")
            print("  --language LANG  Only build packages for this language (default: all)")
            print("  --force          Rebuild everything")
            print("  --validate-build-files  Validate BUILD/CI metadata contracts before continuing")
            print("  --help           Show this help")
            os.exit(0)
        end
        i = i + 1
    end

    return opts
end

-- =========================================================================
-- Find the repository root
-- =========================================================================

--- Walk up from the current directory to find the repo root.
--
-- We look for a .git directory or a code/ directory as indicators.
--
-- @return string|nil The repo root path.
local function find_repo_root()
    -- Try the current directory first.
    if Discovery.dir_exists(".git") or Discovery.dir_exists("code") then
        -- Get the absolute path.
        local handle = io.popen("pwd 2>/dev/null || cd 2>nul")
        if handle then
            local result = handle:read("*l")
            handle:close()
            if result then
                return result:match("^%s*(.-)%s*$")
            end
        end
    end
    return nil
end

-- =========================================================================
-- Main
-- =========================================================================

local function main()
    local opts = parse_args()

    -- Determine root.
    local root = opts.root or find_repo_root()
    if not root then
        io.stderr:write("Error: could not determine monorepo root. Use --root.\n")
        os.exit(1)
    end

    print(string.format("Root: %s", root))

    -- Step 1: Discover packages.
    print("Discovering packages...")
    local packages = Discovery.discover_packages(root)
    print(string.format("Found %d packages", #packages))

    -- Step 2: Filter by language if specified.
    if opts.language ~= "all" then
        local filtered = {}
        for _, pkg in ipairs(packages) do
            if pkg.language == opts.language then
                filtered[#filtered + 1] = pkg
            end
        end
        packages = filtered
        print(string.format("Filtered to %d %s packages", #packages, opts.language))
    end

    if #packages == 0 then
        print("No packages to build.")
        os.exit(0)
    end

    if opts.validate_build_files then
        local validation_error = Validator.validate_ci_full_build_toolchains(root, packages)
        if validation_error then
            io.stderr:write("BUILD/CI validation failed:\n")
            io.stderr:write("  - " .. validation_error .. "\n")
            io.stderr:write("Fix the CI workflow so full-build toolchain setup stays correct.\n")
            os.exit(1)
        end
    end

    -- Step 3: Resolve dependencies.
    print("Resolving dependencies...")
    local graph = Resolver.resolve_dependencies(packages)

    -- Step 4: Compute build levels.
    local ok, groups = pcall(function()
        return graph:independent_groups()
    end)

    if not ok then
        io.stderr:write("Error: " .. tostring(groups) .. "\n")
        os.exit(1)
    end

    print(string.format("Build plan: %d levels", #groups))

    -- Step 5: Execute builds (or dry run).
    local results = Executor.execute_builds(packages, groups, opts.dry_run)

    if opts.dry_run then
        os.exit(0)
    end

    -- Step 6: Print report.
    Reporter.print_report(results)

    -- Step 7: Exit with code 1 if any failures.
    for _, result in ipairs(results) do
        if result.status == "fail" then
            os.exit(1)
        end
    end
end

main()
