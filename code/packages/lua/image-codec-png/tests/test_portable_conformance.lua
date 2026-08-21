package.path = package.path
    .. ";../src/?.lua;../src/?/init.lua"
    .. ";../../pixel-container/src/?.lua;../../pixel-container/src/?/init.lua"
    .. ";../../zip/src/?.lua;../../zip/src/?/init.lua"
    .. ";../../lzss/src/?.lua;../../lzss/src/?/init.lua"

local json = require("dkjson")
local LibDeflate = require("LibDeflate")
local png = require("coding_adventures.image_codec_png")

local function fixture_path()
    local candidates = {
        "../../../../specs/fixtures/image-codec-png-v1/cases.json",
        "code/specs/fixtures/image-codec-png-v1/cases.json",
    }
    for _, path in ipairs(candidates) do
        local handle = io.open(path, "rb")
        if handle then
            handle:close()
            return path
        end
    end
    error("could not locate IC18 portable fixture corpus")
end

local function read_fixture()
    local handle = assert(io.open(fixture_path(), "rb"))
    local text = handle:read("*a")
    handle:close()
    return assert(json.decode(text))
end

local function hex_to_bytes(value)
    local parts = {}
    for offset = 1, #value, 2 do
        local byte = tonumber(value:sub(offset, offset + 1), 16)
        if byte == nil then error("invalid fixture hex") end
        parts[#parts + 1] = string.char(byte)
    end
    return table.concat(parts)
end

local function data_to_string(data)
    local parts = {}
    local current = {}
    for index = 1, #data do
        current[#current + 1] = string.char(data[index])
        if #current == 4096 then
            parts[#parts + 1] = table.concat(current)
            current = {}
        end
    end
    if #current > 0 then parts[#parts + 1] = table.concat(current) end
    return table.concat(parts)
end

local function exact_dimension(value)
    if type(value) ~= "number" or value ~= value or value == math.huge
        or value == -math.huge or value % 1 ~= 0
    then
        error(png.PngError.new("invalid-image-dimensions"), 0)
    end
    return value
end

local function encode_input(input)
    local width = exact_dimension(input.width)
    local height = exact_dimension(input.height)
    local rgba = hex_to_bytes(input.rgba_hex)
    local data = {}
    for index = 1, #rgba do data[index] = rgba:byte(index) end
    return png.encode_png({width = width, height = height, data = data})
end

local function decode_case(case)
    local options
    if case.options then options = {max_pixels = case.options.max_pixels} end
    return png.decode_png(hex_to_bytes(case.png_hex), options)
end

local function expect_error(case, action)
    local ok, failure = pcall(action)
    assert.is_false(ok, case.id)
    assert.is_table(failure, case.id)
    assert.are.equal(case.expected.error_id, failure.code, case.id)
    assert.are.equal(case.expected.error_id, failure.message, case.id)
end

local function parse_chunks(encoded)
    local chunks = {}
    local position = 9
    while position <= #encoded do
        local length = string.unpack(">I4", encoded, position)
        local kind = encoded:sub(position + 4, position + 7)
        local payload = encoded:sub(position + 8, position + 7 + length)
        chunks[#chunks + 1] = {kind = kind, data = payload}
        position = position + 12 + length
    end
    return chunks
end

local function windows_real_png(encoded, width, height, expected_rgba)
    if package.config:sub(1, 1) ~= "\\" then return end
    local image_path = os.tmpname() .. ".png"
    local script_path = os.tmpname() .. ".ps1"
    local image_handle = assert(io.open(image_path, "wb"))
    image_handle:write(encoded)
    image_handle:close()
    local script_handle = assert(io.open(script_path, "wb"))
    script_handle:write([[
param([string]$Path)
Add-Type -AssemblyName System.Drawing
$bitmap = [System.Drawing.Bitmap]::new($Path)
$builder = [System.Text.StringBuilder]::new()
for ($y = 0; $y -lt $bitmap.Height; $y++) {
  for ($x = 0; $x -lt $bitmap.Width; $x++) {
    $pixel = $bitmap.GetPixel($x, $y)
    [void]$builder.AppendFormat('{0:x2}{1:x2}{2:x2}{3:x2}', $pixel.R, $pixel.G, $pixel.B, $pixel.A)
  }
}
'{0}x{1}:{2}' -f $bitmap.Width, $bitmap.Height, $builder.ToString()
$bitmap.Dispose()
]])
    script_handle:close()
    local command = string.format(
        'powershell -NoProfile -ExecutionPolicy Bypass -File "%s" -Path "%s" 2>&1',
        script_path, image_path)
    local process = assert(io.popen(command, "r"))
    local output = process:read("*a"):gsub("%s+$", "")
    local ok = process:close()
    os.remove(script_path)
    os.remove(image_path)
    assert.is_truthy(ok, output)
    assert.are.equal(string.format("%dx%d:%s", width, height, expected_rgba), output)
end

local document = read_fixture()

describe("IC18 portable fixture corpus", function()
    it("pins schema, profile, limits, taxonomy, and case count", function()
        assert.are.equal(1, document.schema_version)
        assert.are.equal("image-codec-png-v1", document.profile)
        assert.are.equal(85, #document.cases)
        assert.are.equal(png.PNG_MAX_DIMENSION, document.limits.max_dimension)
        assert.are.equal(png.PNG_MAX_PIXELS, document.limits.default_max_pixels)
        assert.are.same(document.error_ids, png.png_error_codes())
    end)

    for _, case in ipairs(document.cases) do
        it("consumes " .. case.id .. " through public APIs", function()
            if case.operation == "decode" then
                local actual = decode_case(case)
                assert.are.equal(case.expected.width, actual.width)
                assert.are.equal(case.expected.height, actual.height)
                assert.are.equal(hex_to_bytes(case.expected.rgba_hex), data_to_string(actual.data))
            elseif case.operation == "decode-error" then
                expect_error(case, function() decode_case(case) end)
            elseif case.operation == "encode-error" then
                expect_error(case, function() encode_input(case.input) end)
            elseif case.operation == "adler32" then
                assert.are.equal(tonumber(case.expected.adler32_hex, 16), png.adler32(hex_to_bytes(case.input_hex)))
            elseif case.operation == "encode" then
                local encoded = encode_input(case.input)
                local chunks = parse_chunks(encoded)
                local kinds = {}
                local idat = {}
                for _, item in ipairs(chunks) do
                    kinds[#kinds + 1] = item.kind
                    if item.kind == "IDAT" then idat[#idat + 1] = item.data end
                end
                assert.are.same(case.expected.chunk_types, kinds)
                assert.are.equal(case.expected.bit_depth, encoded:byte(25))
                assert.are.equal(case.expected.colour_type, encoded:byte(26))
                assert.are.equal(case.expected.interlace, encoded:byte(29))
                local zlib = table.concat(idat)
                local filtered, trailing = LibDeflate:DecompressDeflate(zlib:sub(3, -5))
                assert.is_string(filtered)
                assert.are.equal(0, trailing)
                local width = exact_dimension(case.input.width)
                local height = exact_dimension(case.input.height)
                local row_size = width * 4 + 1
                local filters = {}
                for row = 0, height - 1 do filters[#filters + 1] = filtered:byte(row * row_size + 1) end
                assert.are.same(case.expected.filter_types, filters)
                local round_trip = png.decode_png(encoded)
                assert.are.equal(hex_to_bytes(case.input.rgba_hex), data_to_string(round_trip.data))
                windows_real_png(encoded, width, height, case.input.rgba_hex)
            else
                error("unsupported fixture operation " .. tostring(case.operation))
            end
        end)
    end
end)
