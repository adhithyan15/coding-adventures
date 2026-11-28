-- coding_adventures.microbenchmark
-- Paired batch collection, exact-rank analysis, report replay, and the
-- indexed-array workload driver. Timing callbacks are injectable so the same
-- collector contract can be tested without asserting noisy speed thresholds.

local rank_statistics = require("coding_adventures.rank_statistics")
local json_parser = require("coding_adventures.json_parser")
local native_clock = require("coding_adventures.microbenchmark.clock")
local M = { VERSION = "0.1.0" }

local I64_MIN, I64_MAX = math.mininteger, math.maxinteger
local MAX_PAIRS, MAX_JSON_BYTES, MAX_JSON_DEPTH, MAX_JSON_NODES, MAX_JSON_TOKENS = 10000, 4 * 1024 * 1024, 256, 100000, 200000
local NULL = setmetatable({}, { __tostring = function() return "null" end })
local JSON_ARRAY = {}
M.null = NULL
local base_first, candidate_first = "baseline-first", "candidate-first"

local function is_finite(value)
    return type(value) == "number" and value == value and value ~= math.huge and value ~= -math.huge
end

local function integer64(value, name, minimum, maximum)
    assert(math.type(value) == "integer", name .. " must be an exact signed 64-bit integer")
    if minimum ~= nil then assert(value >= minimum, name .. " is below its allowed range") end
    if maximum ~= nil then assert(value <= maximum, name .. " is above its allowed range") end
    return value
end

local function factor(value)
    assert(is_finite(value) and value > 1, "minimum_factor must be finite and greater than 1")
    return value + 0.0
end

function M.defaults()
    return { pairs = 40, confidence_percent = 95, minimum_factor = 1.05, seed = 0, target_ns = 100000, max_batch = 65536 }
end

local config_keys = { pairs = true, confidence_percent = true, minimum_factor = true, seed = true, target_ns = true, max_batch = true }
local function validate_config(config)
    assert(type(config) == "table", "config must be an object")
    for key in pairs(config) do assert(config_keys[key], "config has unknown field " .. tostring(key)) end
    local pairs_count = integer64(config.pairs, "pairs", 1, MAX_PAIRS)
    local confidence = integer64(config.confidence_percent, "confidence_percent", 1, 99)
    factor(config.minimum_factor)
    integer64(config.seed, "seed")
    integer64(config.target_ns, "target_ns", 1)
    integer64(config.max_batch, "max_batch", 1)
    return pairs_count, confidence
end

local function checked_add(a, b, label)
    if b > 0 then assert(a <= I64_MAX - b, label .. " overflows signed 64-bit integer") end
    if b < 0 then assert(a >= I64_MIN - b, label .. " overflows signed 64-bit integer") end
    return a + b
end

local function checked_product(value, count, label)
    assert(count >= 0, "operation count must be non-negative")
    if count == 0 or value == 0 then return 0 end
    if value > 0 then
        assert(value <= I64_MAX // count, label .. " overflows signed 64-bit integer")
    else
        local quotient, remainder = I64_MIN // count, I64_MIN % count
        assert(value > quotient or (value == quotient and remainder == 0), label .. " overflows signed 64-bit integer")
    end
    return value * count
end

local function validate_values(values)
    assert(type(values) == "table", "values must be an array table")
    local count = #values
    local observed = 0
    for key in pairs(values) do
        assert(math.type(key) == "integer" and key >= 1 and key <= count, "values must be a dense array")
        observed = observed + 1
    end
    assert(observed == count, "values must be a dense array")
    for i = 1, count do integer64(values[i], "array value") end
    return count
end

function M.indexed_batch(values, operations)
    local count = validate_values(values)
    integer64(operations, "operations", 0)
    local total = 0
    for _ = 1, operations do
        for i = 1, count do total = checked_add(total, values[i], "array checksum") end
    end
    return total
end

function M.iterated_batch(values, operations)
    validate_values(values)
    integer64(operations, "operations", 0)
    local total = 0
    for _ = 1, operations do
        for _, value in ipairs(values) do total = checked_add(total, value, "array checksum") end
    end
    return total
end

function M.analyze(observations, confidence_percent, minimum_factor)
    assert(type(observations) == "table" and #observations > 0, "at least one pair is required")
    assert(#observations <= MAX_PAIRS, "at most 10000 pairs are supported")
    integer64(confidence_percent, "confidence_percent", 1, 99)
    minimum_factor = factor(minimum_factor)
    local ratios = {}
    for i, pair in ipairs(observations) do
        assert(type(pair) == "table", "pair " .. i .. " must be an object")
        integer64(pair.baseline_ns, "baseline_ns", 1)
        integer64(pair.candidate_ns, "candidate_ns", 1)
        assert(pair.order == base_first or pair.order == candidate_first, "pair order is invalid")
        ratios[i] = pair.candidate_ns / pair.baseline_ns -- Lua / converts each signed integer to binary64.
    end
    local ratio = rank_statistics.median(ratios)
    local interval = rank_statistics.median_interval(ratios, confidence_percent)
    local verdict = "inconclusive"
    if interval then
        local fast_boundary = 1 / minimum_factor
        if interval[2] < fast_boundary then verdict = "faster"
        elseif interval[1] > minimum_factor then verdict = "slower"
        elseif interval[1] >= fast_boundary and interval[2] <= minimum_factor then verdict = "equivalent" end
    end
    return { ratio = ratio, interval = interval or NULL, verdict = verdict }
end

local function verify(callback, expected, operations)
    local wanted = checked_product(expected, operations, "checksum")
    local got = integer64(callback(operations), "checksum")
    assert(got == wanted, "checksum mismatch")
end

function M.collect(baseline, candidate, expected_checksum, config, now)
    config = config or M.defaults()
    local count, confidence = validate_config(config)
    integer64(expected_checksum, "expected_checksum")
    assert(type(baseline) == "function" and type(candidate) == "function", "baseline and candidate callbacks are required")
    now = now or function() return native_clock.now_ns() end
    assert(type(now) == "function", "monotonic clock callback is required")
    verify(baseline, expected_checksum, 1)
    verify(candidate, expected_checksum, 1)

    local previous
    local function read_clock()
        local tick = integer64(now(), "clock tick", 0)
        assert(previous == nil or tick >= previous, "clock regressed")
        previous = tick
        return tick
    end
    local function measure(callback, size, calibration)
        local wanted = checked_product(expected_checksum, size, "checksum")
        local start = read_clock()
        local got = integer64(callback(size), "checksum")
        local finish = read_clock()
        assert(got == wanted, "checksum mismatch")
        assert(finish > start or (calibration and finish == start), "measured duration must be positive")
        return finish - start
    end

    local size = 1
    while true do
        local base_ns = measure(baseline, size, true)
        local candidate_ns = measure(candidate, size, true)
        if base_ns >= config.target_ns and candidate_ns >= config.target_ns then break end
        assert(size < config.max_batch, "calibration limit exhausted")
        size = size > config.max_batch // 2 and config.max_batch or size * 2
    end

    local schedule = {}
    for i = 1, count do schedule[i] = i <= (count + 1) // 2 and base_first or candidate_first end
    local rng = config.seed % 4294967296
    local function random(limit)
        rng = (1664525 * rng + 1013904223) % 4294967296
        return rng % limit
    end
    for i = count, 2, -1 do
        local j = random(i) + 1
        schedule[i], schedule[j] = schedule[j], schedule[i]
    end

    local observations = {}
    for i, order in ipairs(schedule) do
        local base_ns, candidate_ns
        if order == base_first then
            base_ns = measure(baseline, size, false)
            candidate_ns = measure(candidate, size, false)
        else
            candidate_ns = measure(candidate, size, false)
            base_ns = measure(baseline, size, false)
        end
        observations[i] = { baseline_ns = base_ns, candidate_ns = candidate_ns, order = order }
    end
    return { config = config, batch_size = size, pairs = observations,
        analysis = M.analyze(observations, confidence, config.minimum_factor) }
end

local json_null
local function utf8_valid(text)
    return utf8.len(text) ~= nil
end

local function decode_string(raw)
    local out, index = {}, 1
    local escapes = { ['"'] = '"', ['\\'] = '\\', ['/'] = '/', b = '\b', f = '\f', n = '\n', r = '\r', t = '\t' }
    while index <= #raw do
        local char = raw:sub(index, index)
        if char ~= '\\' then
            out[#out + 1] = char
            index = index + 1
        else
            local escape = raw:sub(index + 1, index + 1)
            if escape == 'u' then
                local hex = raw:sub(index + 2, index + 5)
                local codepoint = tonumber(hex, 16)
                assert(codepoint, "invalid Unicode escape")
                index = index + 6
                if codepoint >= 0xD800 and codepoint <= 0xDBFF then
                    assert(raw:sub(index, index + 1) == "\\u", "unpaired high surrogate")
                    local low_hex = raw:sub(index + 2, index + 5)
                    local low = tonumber(low_hex, 16)
                    assert(low and low >= 0xDC00 and low <= 0xDFFF, "invalid surrogate pair")
                    codepoint = 0x10000 + (codepoint - 0xD800) * 0x400 + (low - 0xDC00)
                    index = index + 6
                elseif codepoint >= 0xDC00 and codepoint <= 0xDFFF then
                    error("unpaired low surrogate")
                end
                out[#out + 1] = utf8.char(codepoint)
            else
                assert(escapes[escape] ~= nil, "invalid string escape")
                out[#out + 1] = escapes[escape]
                index = index + 2
            end
        end
    end
    local result = table.concat(out)
    assert(utf8_valid(result), "JSON string is not valid UTF-8")
    return result
end

local function preflight_json(text)
    local depth, tokens, in_string, escaped = 0, 0, false, false
    local function add_token()
        tokens = tokens + 1
        assert(tokens <= MAX_JSON_TOKENS, "report exceeds JSON token limit")
    end
    local index = 1
    while index <= #text do
        local char = text:sub(index, index)
        index = index + 1
        if in_string then
            if escaped then escaped = false
            elseif char == '\\' then escaped = true
            elseif char == '"' then in_string = false end
        elseif char == '"' then in_string = true; add_token()
        elseif char == '{' or char == '[' then
            add_token()
            depth = depth + 1
            assert(depth <= MAX_JSON_DEPTH, "report exceeds JSON depth limit")
        elseif char == '}' or char == ']' then add_token(); depth = depth - 1
        elseif char == ',' or char == ':' then add_token()
        elseif not char:match("%s") then
            add_token()
            while index <= #text do
                local next_char = text:sub(index, index)
                if next_char:match("%s") or next_char == "{" or next_char == "}" or next_char == "[" or next_char == "]"
                    or next_char == "," or next_char == ":" or next_char == '"' then break end
                index = index + 1
            end
        end
    end
end

local function decode_ast(node, state)
    state.nodes = state.nodes + 1
    assert(state.nodes <= MAX_JSON_NODES, "report exceeds JSON node limit")
    local rule = node.rule_name
    if rule == "value" then
        for _, child in ipairs(node.children) do
            if child.rule_name then return decode_ast(child, state) end
            if child.type == "STRING" then return decode_string(child.value) end
            if child.type == "NUMBER" then return assert(tonumber(child.value), "invalid JSON number") end
            if child.type == "TRUE" then return true end
            if child.type == "FALSE" then return false end
            if child.type == "NULL" then return NULL end
        end
        error("JSON value is empty")
    elseif rule == "object" then
        local out, seen = {}, {}
        for _, pair_node in ipairs(node.children) do
            if pair_node.rule_name == "pair" then
                state.nodes = state.nodes + 1
                assert(state.nodes <= MAX_JSON_NODES, "report exceeds JSON node limit")
                local key_token, value_node
                for _, child in ipairs(pair_node.children) do
                    if child.type == "STRING" and not key_token then key_token = child end
                    if child.rule_name == "value" then value_node = child end
                end
                assert(key_token and value_node, "malformed JSON object pair")
                local key = decode_string(key_token.value)
                assert(not seen[key], "duplicate JSON object member")
                seen[key] = true
                out[key] = decode_ast(value_node, state)
            end
        end
        return out
    elseif rule == "array" then
        local out = {}
        for _, child in ipairs(node.children) do
            if child.rule_name == "value" then out[#out + 1] = decode_ast(child, state) end
        end
        return setmetatable(out, JSON_ARRAY)
    end
    error("unknown JSON AST node " .. tostring(rule))
end

local function parse_json(text)
    assert(type(text) == "string", "report JSON must be a string")
    assert(#text <= MAX_JSON_BYTES, "report exceeds JSON byte limit")
    assert(utf8_valid(text), "report must be valid UTF-8")
    preflight_json(text)
    local ast = json_parser.parse(text) -- Existing parser handles complete JSON syntax and trailing input.
    return decode_ast(ast, { nodes = 0 })
end

local function exact_keys(value, expected, label)
    assert(type(value) == "table", label .. " must be an object")
    local seen = {}
    for _, key in ipairs(expected) do seen[key] = true end
    local found = 0
    for key in pairs(value) do
        assert(type(key) == "string" and seen[key], label .. " has unknown field " .. tostring(key))
        found = found + 1
    end
    assert(found == #expected, label .. " has missing fields")
end

local function finite_json_number(value, label)
    assert(is_finite(value), label .. " must be a finite JSON number")
    return value
end

local core_keys = { "config", "batch_size", "pairs", "analysis" }
local metadata_keys = { "workload", "invocation", "revision", "runtime", "os", "arch" }
local function valid_workload_identifier(value)
    if type(value) ~= "string" or #value == 0 then return false end
    local nonblank = false
    for index = 1, #value do
        local byte = value:byte(index)
        if byte == 0 then return false end
        if byte ~= 0x20 and not (byte >= 0x09 and byte <= 0x0d) then nonblank = true end
    end
    return nonblank
end
local function replay(text)
    local document = parse_json(text)
    assert(type(document) == "table" and document ~= NULL, "report must be an object")
    local seen, expected = {}, {}
    for _, key in ipairs(core_keys) do expected[key] = true end
    local metadata_count = 0
    for _, key in ipairs(metadata_keys) do
        if document[key] ~= nil then expected[key] = true; metadata_count = metadata_count + 1 end
    end
    if document.profile ~= nil then expected.profile = true end
    for key in pairs(document) do assert(expected[key], "report has unknown field " .. tostring(key)); seen[key] = true end
    for _, key in ipairs(core_keys) do assert(seen[key], "report missing " .. key) end
    assert(metadata_count == 0 or metadata_count == #metadata_keys, "standard metadata must be complete or absent")
    if document.profile ~= nil then assert(metadata_count == #metadata_keys, "profile requires full metadata") end
    if metadata_count == #metadata_keys then
        assert(valid_workload_identifier(document.workload), "workload must be nonblank and contain no NUL")
        for _, key in ipairs(metadata_keys) do
            if key == "invocation" then
                assert(type(document.invocation) == "table" and getmetatable(document.invocation) == JSON_ARRAY,
                    "invocation must be an array")
                for _, item in ipairs(document.invocation) do
                    assert(type(item) == "string" and not item:find("%z"), "invocation entries must be strings without NUL")
                end
            else
                assert(type(document[key]) == "string" and #document[key] > 0 and not document[key]:find("%z"),
                    key .. " must be a non-empty string without NUL")
            end
        end
    end
    if document.profile ~= nil then
        assert(type(document.profile) == "string" and #document.profile > 0 and not document.profile:find("%z"),
            "profile must be a non-empty string without NUL")
    end

    exact_keys(document.config, { "pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch" }, "config")
    local config = document.config
    validate_config(config)
    integer64(document.batch_size, "batch_size", 1, config.max_batch)
    assert(type(document.pairs) == "table" and #document.pairs == config.pairs, "pair count does not match config")
    local baseline_count, observations = 0, {}
    for i, pair in ipairs(document.pairs) do
        exact_keys(pair, { "baseline_ns", "candidate_ns", "order" }, "pair " .. i)
        integer64(pair.baseline_ns, "baseline_ns", 1)
        integer64(pair.candidate_ns, "candidate_ns", 1)
        assert(pair.order == base_first or pair.order == candidate_first, "invalid pair order")
        if pair.order == base_first then baseline_count = baseline_count + 1 end
        observations[i] = pair
    end
    assert(baseline_count == (config.pairs + 1) // 2, "pair order schedule is unbalanced")
    exact_keys(document.analysis, { "ratio", "interval", "verdict" }, "analysis")
    local computed = M.analyze(observations, config.confidence_percent, config.minimum_factor)
    assert(finite_json_number(document.analysis.ratio, "analysis.ratio") == computed.ratio, "analysis ratio differs from observations")
    assert(document.analysis.verdict == computed.verdict, "analysis verdict differs from observations")
    if computed.interval == NULL then assert(document.analysis.interval == NULL, "analysis interval must be null")
    else
        local interval = document.analysis.interval
        assert(type(interval) == "table" and #interval == 2, "analysis interval must have two bounds")
        assert(finite_json_number(interval[1], "interval low") == computed.interval[1] and
            finite_json_number(interval[2], "interval high") == computed.interval[2], "analysis interval differs from observations")
    end
    return document
end

M.from_json = replay

local function quote_json(text)
    assert(type(text) == "string" and utf8_valid(text), "JSON strings must be valid UTF-8")
    local out = { '"' }
    for i = 1, #text do
        local byte = text:byte(i)
        if byte == 34 then out[#out + 1] = '\\"'
        elseif byte == 92 then out[#out + 1] = '\\\\'
        elseif byte == 8 then out[#out + 1] = '\\b'
        elseif byte == 9 then out[#out + 1] = '\\t'
        elseif byte == 10 then out[#out + 1] = '\\n'
        elseif byte == 12 then out[#out + 1] = '\\f'
        elseif byte == 13 then out[#out + 1] = '\\r'
        elseif byte < 32 then out[#out + 1] = string.format('\\u%04x', byte)
        else out[#out + 1] = string.char(byte) end
    end
    out[#out + 1] = '"'
    return table.concat(out)
end

local function json_encode(value)
    if value == NULL then return "null" end
    local kind = type(value)
    if kind == "string" then return quote_json(value) end
    if kind == "boolean" then return value and "true" or "false" end
    if kind == "number" then
        assert(is_finite(value), "JSON numbers must be finite")
        if math.type(value) == "integer" then return tostring(value) end
        return string.format("%.17g", value)
    end
    assert(kind == "table", "unsupported JSON value")
    local count, array = #value, true
    for key in pairs(value) do if math.type(key) ~= "integer" or key < 1 or key > count then array = false; break end end
    if getmetatable(value) == JSON_ARRAY then array = true end
    if array then
        local items = {}; for i = 1, count do items[i] = json_encode(value[i]) end
        return "[" .. table.concat(items, ",") .. "]"
    end
    local keys = {}; for key in pairs(value) do assert(type(key) == "string", "object keys must be strings"); keys[#keys + 1] = key end
    table.sort(keys)
    local items = {}; for _, key in ipairs(keys) do items[#items + 1] = quote_json(key) .. ":" .. json_encode(value[key]) end
    return "{" .. table.concat(items, ",") .. "}"
end

function M.to_json(report)
    local document = {}; for key, value in pairs(report) do if value ~= nil then document[key] = value end end
    if type(document.invocation) == "table" then setmetatable(document.invocation, JSON_ARRAY) end
    return json_encode(document) .. "\n"
end

local ARRAY = {}
for i = 0, 4095 do ARRAY[i + 1] = (17 * i + 3) % 251 end
local ONE_PASS = 0; for _, value in ipairs(ARRAY) do ONE_PASS = ONE_PASS + value end

local function indexed(batch) return M.indexed_batch(ARRAY, batch) end
local function iterated(batch) return M.iterated_batch(ARRAY, batch) end

local function write_checked(writer, text)
    local ok, err = writer:write(text)
    assert(ok ~= nil, "stdout write failed: " .. tostring(err))
    local flushed, flush_error = writer:flush()
    assert(flushed ~= nil, "stdout flush failed: " .. tostring(flush_error))
end

function M.revision_from(environment)
    environment = environment or { GIT_REVISION = os.getenv("GIT_REVISION"), GITHUB_SHA = os.getenv("GITHUB_SHA") }
    local first = environment.GIT_REVISION
    if first and first:match("%S") then return first end
    local second = environment.GITHUB_SHA
    if second and second:match("%S") then return second end
    return "unknown"
end

local function parse_cli(argv)
    local config, revision, revision_seen, replay_path = M.defaults(), M.revision_from(), false, nil
    local seen = {}
    local i = 1
    while i <= #argv do
        local option = argv[i]
        if option == "--help" then return config, revision, nil, true end
        assert(not seen[option], "duplicate option " .. tostring(option)); seen[option] = true
        i = i + 1
        assert(i <= #argv, "missing value for " .. tostring(option))
        local value = argv[i]
        if option == "--pairs" then config.pairs = assert(tonumber(value)); integer64(config.pairs, "pairs", 1, MAX_PAIRS)
        elseif option == "--confidence" then config.confidence_percent = assert(tonumber(value)); integer64(config.confidence_percent, "confidence", 1, 99)
        elseif option == "--seed" then config.seed = assert(tonumber(value)); integer64(config.seed, "seed")
        elseif option == "--target-ns" then config.target_ns = assert(tonumber(value)); integer64(config.target_ns, "target_ns", 1)
        elseif option == "--max-batch" then config.max_batch = assert(tonumber(value)); integer64(config.max_batch, "max_batch", 1)
        elseif option == "--factor" then config.minimum_factor = factor(tonumber(value))
        elseif option == "--revision" then assert(type(value) == "string" and value:match("%S"), "revision must not be blank"); revision = value; revision_seen = true
        elseif option == "--replay" then replay_path = value
        else error("unknown option " .. tostring(option)) end
        i = i + 1
    end
    validate_config(config)
    return config, revision_seen and revision or revision, replay_path, false
end

local function run_impl(argv, output, error_output, now)
    local config, revision, replay_path, help = parse_cli(argv)
    if help then write_checked(output, "Usage: array-microbenchmark [--pairs N] [--confidence N] [--factor F] [--seed I64] [--target-ns N] [--max-batch N] [--revision REV] | --replay FILE\n"); return 0 end
    if replay_path then
        local file, err = io.open(replay_path, "rb"); assert(file, "cannot read replay file: " .. tostring(err))
        local text = file:read(MAX_JSON_BYTES + 1)
        local closed, close_error = file:close()
        assert(closed, "cannot close replay file: " .. tostring(close_error))
        assert(text and #text <= MAX_JSON_BYTES, "report exceeds JSON byte limit")
        local report = replay(text)
        write_checked(output, "replay valid: " .. #report.pairs .. " pairs, " .. report.analysis.verdict .. "\n")
        return 0
    end
    local report = M.collect(indexed, iterated, ONE_PASS, config, now)
    local invocation = { "array-microbenchmark", table.unpack(argv) }
    report.workload, report.invocation, report.revision = "indexed-array-sum-4096", invocation, revision
    report.runtime = _VERSION
    report.os = package.config:sub(1, 1) == "\\" and "Windows" or "Unix"
    report.arch = os.getenv("PROCESSOR_ARCHITECTURE") or os.getenv("HOSTTYPE") or "unknown"
    local text = M.to_json(report)
    replay(text) -- Never write a report this implementation cannot replay.
    write_checked(output, text)
    return 0
end

function M.run(argv, output, error_output, now)
    output, error_output = output or io.stdout, error_output or io.stderr
    local ok, result = pcall(run_impl, argv or arg, output, error_output, now)
    if ok then return result end
    pcall(function() error_output:write("benchmark failed: " .. tostring(result) .. "\n"); error_output:flush() end)
    return 1
end

return M
