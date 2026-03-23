-- reporter.lua -- Build Report Output
-- =====================================
--
-- This module prints a summary report after all builds complete, showing
-- which packages passed, which failed, and overall statistics.

local Reporter = {}

--- Print a summary report of build results.
--
-- @param results table List of result records from the executor.
function Reporter.print_report(results)
    if #results == 0 then
        print("\nNo packages were built.")
        return
    end

    local passed = 0
    local failed = 0
    local total_duration = 0
    local failures = {}

    for _, result in ipairs(results) do
        total_duration = total_duration + result.duration
        if result.status == "pass" then
            passed = passed + 1
        else
            failed = failed + 1
            failures[#failures + 1] = result
        end
    end

    print(string.format("\n%s", string.rep("=", 60)))
    print(string.format("BUILD REPORT"))
    print(string.format("%s", string.rep("=", 60)))
    print(string.format("Total:    %d packages", #results))
    print(string.format("Passed:   %d", passed))
    print(string.format("Failed:   %d", failed))
    print(string.format("Duration: %.1fs", total_duration))

    if #failures > 0 then
        print(string.format("\nFailed packages:"))
        for _, result in ipairs(failures) do
            print(string.format("  - %s", result.name))
        end
    end

    print(string.rep("=", 60))
end

return Reporter
