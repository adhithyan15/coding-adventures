local report_path, expected_file = arg[1], arg[2]
assert(report_path and expected_file, "usage: lua check_coverage.lua REPORT EXPECTED_FILE")
local handle = assert(io.open(report_path, "rb"))
local report = handle:read("*a")
assert(handle:close())

local hits, missed, percent
for line in report:gmatch("[^\r\n]+") do
    local file, hit_text, missed_text, percent_text = line:match("^(%S+)%s+(%d+)%s+(%d+)%s+([%d%.]+)%%$")
    if file == expected_file then
        hits, missed, percent = tonumber(hit_text), tonumber(missed_text), tonumber(percent_text)
    elseif file and file ~= "Total" then
        error("unexpected Lua source in coverage denominator: " .. file)
    end
end

assert(hits and missed and hits + missed > 0, "expected source file has no measurable executable lines")
assert(percent > 80, string.format("production executable-line coverage must exceed 80%%, got %.2f%%", percent))
print(string.format("Production executable-line coverage: %.2f%% (%d hit, %d missed)", percent, hits, missed))
