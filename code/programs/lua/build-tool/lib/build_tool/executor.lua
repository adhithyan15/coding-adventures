-- executor.lua -- Build Command Execution
-- =========================================
--
-- This module executes the build commands for each package. It runs
-- commands sequentially within a package (each BUILD line runs in order),
-- and packages within the same dependency level can theoretically run in
-- parallel. However, since standard Lua has no built-in threading, this
-- implementation runs everything sequentially.
--
-- The executor changes directory to the package's path before running
-- commands, captures stdout/stderr, and records pass/fail status.

local Executor = {}

--- Execute the build commands for a single package.
--
-- Each command from the BUILD file is run in order. If any command fails
-- (non-zero exit code), the package is marked as failed and remaining
-- commands are skipped.
--
-- @param pkg table The package record from discovery.
-- @return table Result record with fields: name, status ("pass" or "fail"),
--               output (captured stdout/stderr), duration (seconds).
function Executor.build_package(pkg)
    local start_time = os.clock()
    local output_lines = {}

    for _, cmd in ipairs(pkg.build_commands) do
        -- Run the command in the package's directory, capturing output.
        local full_cmd = string.format('cd "%s" && %s 2>&1', pkg.path, cmd)
        local handle = io.popen(full_cmd)
        local cmd_output = ""
        if handle then
            cmd_output = handle:read("*a") or ""
            local ok, _, code = handle:close()
            -- In Lua 5.4, handle:close() returns (ok, exit_type, code).
            -- ok is true for exit code 0, false otherwise.
            local exit_code = code or (ok and 0 or 1)

            output_lines[#output_lines + 1] = cmd_output

            if exit_code ~= 0 then
                local duration = os.clock() - start_time
                return {
                    name = pkg.name,
                    status = "fail",
                    output = table.concat(output_lines, "\n"),
                    duration = duration,
                }
            end
        else
            output_lines[#output_lines + 1] = "ERROR: could not execute command: " .. cmd
            local duration = os.clock() - start_time
            return {
                name = pkg.name,
                status = "fail",
                output = table.concat(output_lines, "\n"),
                duration = duration,
            }
        end
    end

    local duration = os.clock() - start_time
    return {
        name = pkg.name,
        status = "pass",
        output = table.concat(output_lines, "\n"),
        duration = duration,
    }
end

--- Execute builds for a list of packages, respecting dependency levels.
--
-- Packages are organized into independent groups (levels) by the dependency
-- graph. Within each level, packages have no dependencies on each other
-- and could theoretically run in parallel. This implementation runs them
-- sequentially.
--
-- @param packages table List of package records.
-- @param groups table List of lists of package names (from independent_groups).
-- @param dry_run boolean If true, print what would run without running.
-- @return table List of result records.
function Executor.execute_builds(packages, groups, dry_run)
    -- Build a lookup table from name to package.
    local pkg_by_name = {}
    for _, pkg in ipairs(packages) do
        pkg_by_name[pkg.name] = pkg
    end

    local results = {}

    for level_num, level in ipairs(groups) do
        if dry_run then
            io.write(string.format("Level %d: %s\n", level_num, table.concat(level, ", ")))
        else
            for _, name in ipairs(level) do
                local pkg = pkg_by_name[name]
                if pkg then
                    io.write(string.format("Building %s...\n", name))
                    local result = Executor.build_package(pkg)
                    results[#results + 1] = result

                    if result.status == "pass" then
                        io.write(string.format("  ✓ %s (%.1fs)\n", name, result.duration))
                    else
                        io.write(string.format("  ✗ %s FAILED (%.1fs)\n", name, result.duration))
                        io.write(result.output .. "\n")
                    end
                end
            end
        end
    end

    return results
end

return Executor
