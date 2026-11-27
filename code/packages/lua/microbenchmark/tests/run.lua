local roots = {
    "src", "../rank_statistics/src", "../json_parser/src", "../json_lexer/src",
    "../parser/src", "../lexer/src", "../grammar_tools/src", "../directed_graph/src", "../state_machine/src",
    "../json_value/src",
}
for _, root in ipairs(roots) do
    package.path = root .. "/?.lua;" .. root .. "/?/init.lua;" .. package.path
end
package.cpath = "../../../../work/lua-tools/module/?.dll;" .. package.cpath
local benchmark = require("coding_adventures.microbenchmark")
local clock_native = require("coding_adventures.microbenchmark.clock")
local json_parser = require("coding_adventures.json_parser")

local function equal(actual, expected, message)
    assert(actual == expected, (message or "values differ") .. ": got " .. tostring(actual) .. ", want " .. tostring(expected))
end
local function rejects(fn, fragment)
    local ok, err = pcall(fn)
    assert(not ok and tostring(err):find(fragment, 1, true), "expected error containing " .. fragment .. "; got " .. tostring(err))
end
local function fake_clock(step)
    local tick = 0
    return function() local previous = tick; tick = tick + step; return previous end
end

local fixture_file = assert(io.open("testdata/microbenchmark-pairs-v1.json", "rb"))
local fixture = fixture_file:read("*a")
assert(fixture_file:close())
local canonical_file = assert(io.open("../../../specs/fixtures/microbenchmark-pairs-v1.json", "rb"))
local canonical = canonical_file:read("*a")
assert(canonical_file:close())
equal(fixture, canonical, "local fixture bytes match canonical")
local fixture_doc = benchmark.from_json -- keep the report parser separate from fixture parsing
local fixture_ast = json_parser.parse(fixture)
assert(fixture_ast.rule_name == "value")

local json_value = require("coding_adventures.json_value")
local fixture_data = json_value.from_string(fixture)
for _, case in ipairs(fixture_data.cases) do
    local pairs = {}
    for _, candidate_ns in ipairs(case.candidate_ns) do
        pairs[#pairs + 1] = { baseline_ns = case.baseline_ns, candidate_ns = candidate_ns, order = "baseline-first" }
    end
    local result = benchmark.analyze(pairs, fixture_data.confidence_percent, fixture_data.minimum_factor)
    equal(result.ratio, case.ratio, case.name .. " ratio")
    equal(result.verdict, case.verdict, case.name .. " verdict")
    if json_value.is_null(case.interval) then equal(tostring(result.interval), "null", case.name .. " no interval")
    else
        equal(result.interval[1], case.interval[1], case.name .. " low")
        equal(result.interval[2], case.interval[2], case.name .. " high")
    end
end

local maximum_pairs = {}
for index = 1, 10000 do
    maximum_pairs[index] = {
        baseline_ns = 1,
        candidate_ns = 1,
        order = index % 2 == 1 and "baseline-first" or "candidate-first",
    }
end
equal(benchmark.analyze(maximum_pairs, 95, 1.05).ratio, 1.0, "analysis accepts 10000 pairs")
maximum_pairs[10001] = false
rejects(function() benchmark.analyze(maximum_pairs, 0, 1) end, "at most 10000 pairs")

local config = { pairs = 5, confidence_percent = 95, minimum_factor = 1.05, seed = -5, target_ns = 100, max_batch = 16 }
local first_tick = 0
local function work_clock() local old = first_tick; first_tick = first_tick + 100; return old end
local function batch(n) return 7 * n end
local first = benchmark.collect(batch, batch, 7, config, work_clock)
local second_tick = 0
local second = benchmark.collect(batch, batch, 7, config, function() local old = second_tick; second_tick = second_tick + 100; return old end)
equal(first.batch_size, 1, "one-operation calibration")
for i = 1, config.pairs do equal(first.pairs[i].order, second.pairs[i].order, "seeded order replay") end
local baseline_count = 0
for _, pair in ipairs(first.pairs) do if pair.order == "baseline-first" then baseline_count = baseline_count + 1 end end
equal(baseline_count, 3, "odd schedule's extra baseline-first pair")
equal(#first.pairs, config.pairs, "all observations retained")

local calibrated_tick = 0
local function advancing_clock() local old = calibrated_tick; return old end
local function time_base(n) calibrated_tick = calibrated_tick + 4 * n; return 7 * n end
local function time_candidate(n) calibrated_tick = calibrated_tick + 5 * n; return 7 * n end
local calibrated = benchmark.collect(time_base, time_candidate, 7,
    { pairs = 1, confidence_percent = 95, minimum_factor = 1.05, seed = 1, target_ns = 100, max_batch = 32 }, advancing_clock)
assert(calibrated.batch_size >= 16, "calibration doubles until both variants meet the target")

rejects(function() benchmark.collect(function(n) return 8 * n end, batch, 7, config, fake_clock(100)) end, "checksum mismatch")
rejects(function() benchmark.collect(function() error("callback failure") end, batch, 7, config, fake_clock(100)) end, "callback failure")
rejects(function() benchmark.collect(batch, batch, 7, config, function() return -1 end) end, "below its allowed range")
local regressing, reads = 10, 0
rejects(function() benchmark.collect(batch, batch, 7, config, function() reads = reads + 1; local old = regressing; regressing = regressing - 1; return old end) end, "clock regressed")
rejects(function() benchmark.collect(batch, batch, 7, { pairs = 1, confidence_percent = 95, minimum_factor = 1.05, seed = 0, target_ns = 1000, max_batch = 1 }, fake_clock(10)) end, "calibration limit exhausted")
local zero_values = { 0, 1, 2, 3, 4, 4 }; local zero_index = 0
rejects(function() benchmark.collect(batch, batch, 7, { pairs = 1, confidence_percent = 95, minimum_factor = 1.05, seed = 0, target_ns = 1, max_batch = 1 }, function() zero_index = zero_index + 1; return zero_values[zero_index] end) end, "duration must be positive")
local maxint = math.maxinteger
local overflow_tick = 0
rejects(function()
    local max_callback = function(n) return n == 1 and maxint or 0 end
    benchmark.collect(max_callback, max_callback, maxint,
        { pairs = 1, confidence_percent = 95, minimum_factor = 1.05, seed = 0, target_ns = 200, max_batch = 2 },
        function() local old = overflow_tick; overflow_tick = overflow_tick + 100; return old end)
end, "checksum overflows")

local native_a, native_b = clock_native.now_ns(), clock_native.now_ns()
assert(math.type(native_a) == "integer" and native_a >= 0 and native_b >= native_a, "native QPC clock is integer and monotonic")

local max_report = {
    config = { pairs = 1, confidence_percent = 95, minimum_factor = 1.05, seed = math.mininteger, target_ns = 1, max_batch = maxint },
    batch_size = maxint,
    pairs = { { baseline_ns = maxint, candidate_ns = maxint, order = "baseline-first" } },
    analysis = { ratio = 1, interval = benchmark.null, verdict = "inconclusive" },
}
local portable = benchmark.to_json(max_report)
assert(portable:find("9223372036854775807", 1, true), "serializer emits max int64 exactly")
assert(portable:find("-9223372036854775808", 1, true), "serializer emits min int64 exactly")
local decoded = benchmark.from_json(portable)
equal(decoded.config.seed, math.mininteger, "min seed exact replay")
equal(decoded.pairs[1].baseline_ns, maxint, "max observation exact replay")
equal(decoded.batch_size, maxint, "max batch exact replay")
equal(decoded.analysis.interval, benchmark.null, "valid null interval preserved")
rejects(function() benchmark.from_json(portable:gsub('"batch_size":9223372036854775807', '"batch_size":9223372036854775808', 1)) end, "exact signed 64-bit")
rejects(function() benchmark.from_json(portable:gsub('"seed":%-9223372036854775808', '"seed":1.0', 1)) end, "exact signed 64-bit")
rejects(function() benchmark.from_json(portable:gsub('"seed":%-9223372036854775808', '"seed":1,"\\u0073eed":2', 1)) end, "duplicate")
rejects(function() benchmark.from_json(portable:gsub('"analysis":{', '"analysis":{"ratio":1,', 1)) end, "duplicate")
rejects(function() benchmark.from_json(portable:gsub('"baseline_ns":9223372036854775807', '"baseline_ns":1.5', 1)) end, "exact signed 64-bit")
rejects(function() benchmark.from_json(portable .. "{}") end, "Unexpected token")
local metadata_report = benchmark.from_json(portable)
metadata_report.workload = "indexed-array-sum-4096"
metadata_report.invocation = { "bench" }
metadata_report.revision, metadata_report.runtime, metadata_report.os, metadata_report.arch = "abc", "Lua 5.4", "Windows", "x64"
local with_metadata = benchmark.to_json(metadata_report)
assert(benchmark.from_json(with_metadata).revision == "abc")
rejects(function() benchmark.from_json(with_metadata:gsub('"workload"', '"unknown":true,"workload"', 1)) end, "unknown field")
rejects(function() benchmark.from_json(with_metadata:gsub('"revision":"abc"', '"revision":null', 1)) end, "revision must be a non-empty string")
rejects(function() benchmark.from_json(with_metadata:gsub('"revision":"abc"', '"revision":"abc","profile":null', 1)) end, "profile must be a non-empty string")
metadata_report.profile = "native-go"
local generic_with_profile = benchmark.to_json(metadata_report)
local array_workload_json = '"workload":"indexed%-array%-sum%-4096"'
local function with_workload(token)
    local text, count = generic_with_profile:gsub(array_workload_json, '"workload":' .. token, 1)
    equal(count, 1, "replace workload token")
    return text
end
local empty_invocation = with_metadata:gsub('"invocation":%b[]', '"invocation":[]', 1)
equal(#benchmark.from_json(empty_invocation).invocation, 0, "empty invocation array is valid")
local empty_argument = with_metadata:gsub('"invocation":%b[]', '"invocation":[""]', 1)
equal(benchmark.from_json(empty_argument).invocation[1], "", "empty argv element is valid")
local whitespace_revision = with_metadata:gsub('"revision":"abc"', '"revision":" "', 1)
equal(benchmark.from_json(whitespace_revision).revision, " ", "whitespace-only revision remains valid metadata")
rejects(function() benchmark.from_json(with_metadata:gsub('"invocation":%b[]', '"invocation":{}', 1)) end, "invocation must be an array")
for _, field in ipairs({ "revision", "runtime", "os", "arch" }) do
    local nul_value = with_metadata:gsub('"' .. field .. '":"[^"]*"', '"' .. field .. '":"\\u0000"', 1)
    rejects(function() benchmark.from_json(nul_value) end, "NUL")
end
local nul_invocation = with_metadata:gsub('"invocation":%b[]', '"invocation":["\\u0000"]', 1)
rejects(function() benchmark.from_json(nul_invocation) end, "without NUL")
local nul_profile = generic_with_profile:gsub('"profile":"[^"]*"', '"profile":"\\u0000"', 1)
rejects(function() benchmark.from_json(nul_profile) end, "without NUL")
local empty_profile = generic_with_profile:gsub('"profile":"[^"]*"', '"profile":""', 1)
rejects(function() benchmark.from_json(empty_profile) end, "non-empty string")

local accepted_workloads = {
    { '"indexed-array-sum-4096"', "indexed-array-sum-4096" },
    { '"dependent-link-order"', "dependent-link-order" },
    { '"\\u2003"', "\226\128\131" },
    { '"\\ud83d\\udca1"', "\240\159\146\161" },
    { '"\\ufffd"', "\239\191\189" },
    { '"\\\\ud800"', "\\ud800" },
}
for _, item in ipairs(accepted_workloads) do
    equal(benchmark.from_json(with_workload(item[1])).workload, item[2], "workload accepted and preserved")
end
local invalid_workloads = {
    '""', '" "', '"\\t"', '"\\n"', '"\\u000b"', '"\\f"', '"\\r"',
    '"\\u0000"', '"bad\\u0000id"', 'null', '17', 'true', '[]', '{}',
}
for _, token in ipairs(invalid_workloads) do
    rejects(function() benchmark.from_json(with_workload(token)) end, "nonblank")
end
rejects(function() benchmark.from_json(with_workload('"\\ud800"')) end, "surrogate")
rejects(function() benchmark.from_json(with_workload('"\\udfff"')) end, "surrogate")
metadata_report.invocation = { [[brace { [ ], comma, quote " slash \]] }
equal(benchmark.from_json(benchmark.to_json(metadata_report)).invocation[1], [[brace { [ ], comma, quote " slash \]], "escaped string punctuation is not counted as structure")
local absent_metadata = benchmark.from_json(portable)
equal(absent_metadata.workload, nil, "metadata absence remains valid")
local maximum_pairs, large_pairs = 10000, {}
for i = 1, maximum_pairs do large_pairs[i] = { baseline_ns = 1, candidate_ns = 1, order = i % 2 == 1 and "baseline-first" or "candidate-first" } end
local large_report = { config = { pairs = maximum_pairs, confidence_percent = 95, minimum_factor = 1.05, seed = 0, target_ns = 1, max_batch = 1 },
    batch_size = 1, pairs = large_pairs, analysis = benchmark.analyze(large_pairs, 95, 1.05) }
equal(#benchmark.from_json(benchmark.to_json(large_report)).pairs, maximum_pairs, "valid maximum-pair report fits parser caps")

rejects(function() benchmark.from_json('{"a":1,"a":2}') end, "duplicate")
rejects(function() benchmark.from_json('{"a":1,"\\u0061":2}') end, "duplicate")
rejects(function() benchmark.from_json('{"a":"\\ud800"}') end, "surrogate")
local parser_parse = json_parser.parse
json_parser.parse = function() error("recursive parser should not run") end
rejects(function() benchmark.from_json(string.rep("[", 257) .. "0" .. string.rep("]", 257)) end, "depth")
rejects(function() benchmark.from_json("[" .. string.rep("0,", 100000) .. "0]") end, "token limit")
json_parser.parse = parser_parse
rejects(function() benchmark.from_json(string.rep(" ", 4 * 1024 * 1024 + 1)) end, "byte limit")

local values = { 2, 5, 8 }
equal(benchmark.indexed_batch(values, 1), 15, "indexed traversal checksum")
equal(benchmark.iterated_batch(values, 1), 15, "native iteration checksum")
equal(benchmark.indexed_batch(values, 0), 0, "zero operations accepted")
equal(benchmark.iterated_batch(values, 0), 0, "zero iterator operations accepted")
rejects(function() benchmark.indexed_batch(values, -1) end, "below its allowed range")
rejects(function() benchmark.iterated_batch(values, -1) end, "below its allowed range")
rejects(function() benchmark.indexed_batch({ maxint }, 2) end, "array checksum overflows")
rejects(function() benchmark.indexed_batch({ -1, math.mininteger }, 1) end, "array checksum overflows")
rejects(function() benchmark.iterated_batch({ -1, math.mininteger }, 1) end, "array checksum overflows")
values[1] = 50
equal(benchmark.indexed_batch(values, 1), 63, "indexed helper reads actual mutable input")
equal(benchmark.iterated_batch(values, 1), 63, "iterator helper reads same mutable input")

equal(benchmark.revision_from({ GIT_REVISION = "  ", GITHUB_SHA = "fallback" }), "fallback", "blank environment falls through")
equal(benchmark.revision_from({ GIT_REVISION = "", GITHUB_SHA = " " }), "unknown", "blank environment defaults unknown")
local output, errors = {}, {}
local writer = { write = function(self, text) self.text = (self.text or "") .. text; return self end,
    flush = function() return true end }
local error_writer = { write = function(self, text) self.text = (self.text or "") .. text; return self end,
    flush = function() return true end }
equal(benchmark.run({ "--pairs", "40", "--target-ns", "1", "--max-batch", "1", "--revision", "test" }, writer, error_writer, fake_clock(1000)), 0, "CLI run succeeds")
local cli_report = benchmark.from_json(writer.text)
equal(#cli_report.pairs, 40, "native CLI has 40 pairs")
equal(cli_report.revision, "test", "explicit CLI revision")
equal(benchmark.run({ "--revision", " " }, writer, error_writer, fake_clock(1000)), 1, "blank revision rejected")
equal(benchmark.run({ "--unknown", "x" }, writer, error_writer, fake_clock(1000)), 1, "unknown option rejected")
local oversized_path = os.tmpname()
local oversized_file = assert(io.open(oversized_path, "wb"))
assert(oversized_file:write(string.rep("x", 4 * 1024 * 1024 + 1)))
assert(oversized_file:close())
equal(benchmark.run({ "--replay", oversized_path }, writer, error_writer, fake_clock(1000)), 1,
    "CLI refuses a path above the parser limit while reading")
assert(error_writer.text:find("report exceeds JSON byte limit", 1, true))
assert(os.remove(oversized_path))
local broken_write = { write = function() return nil, "broken sink" end, flush = function() return true end }
equal(benchmark.run({ "--help" }, broken_write, error_writer, fake_clock(1000)), 1, "write failure propagated")
local broken_flush = { write = function() return true end, flush = function() return nil, "flush failed" end }
equal(benchmark.run({ "--help" }, broken_flush, error_writer, fake_clock(1000)), 1, "flush failure propagated")
local help_writer = { write = function(self, text) self.text = text; return self end, flush = function() return true end }
equal(benchmark.run({ "--help" }, help_writer, error_writer, fake_clock(1000)), 0, "help succeeds")
assert(help_writer.text:find("Usage:", 1, true))

print("Lua microbenchmark: fixture, fake-clock collection, strict replay, 40-pair CLI, native QPC, and array checks passed")
