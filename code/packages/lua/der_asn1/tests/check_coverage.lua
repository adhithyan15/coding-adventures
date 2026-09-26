local handle = assert(io.open("luacov.report.out", "rb"))
local report = handle:read("*a")
handle:close()

local coverage
for line in report:gmatch("[^\r\n]+") do
    local value = line:match("^Total%s+%d+%s+%d+%s+(%d+%.%d+)%%$")
    if value ~= nil then coverage = tonumber(value) end
end

assert(coverage ~= nil, "unable to read total coverage")
assert(coverage >= 95, "coverage below 95%")
print(string.format("Production line coverage: %.2f%%", coverage))
