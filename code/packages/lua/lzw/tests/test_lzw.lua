-- Tests for coding_adventures.lzw — CMP03 LZW compression.

local lzw = require("coding_adventures.lzw")

local pass = 0
local fail = 0

local function eq_list(a, b)
  if #a ~= #b then return false end
  for i = 1, #a do
    if a[i] ~= b[i] then return false end
  end
  return true
end

local function assert_eq(got, expected, name)
  if got == expected then
    pass = pass + 1
    print("PASS: " .. name)
  else
    fail = fail + 1
    print("FAIL: " .. name)
    print("  expected: " .. tostring(expected))
    print("  got:      " .. tostring(got))
  end
end

local function assert_list_eq(got, expected, name)
  if eq_list(got, expected) then
    pass = pass + 1
    print("PASS: " .. name)
  else
    fail = fail + 1
    print("FAIL: " .. name)
    print("  expected: {" .. table.concat(expected, ", ") .. "}")
    print("  got:      {" .. table.concat(got, ", ") .. "}")
  end
end

local function assert_true(cond, name)
  assert_eq(cond, true, name)
end

-- ---------------------------------------------------------------------------
-- Constants
-- ---------------------------------------------------------------------------

assert_eq(lzw.CLEAR_CODE,        256, "CLEAR_CODE == 256")
assert_eq(lzw.STOP_CODE,         257, "STOP_CODE == 257")
assert_eq(lzw.INITIAL_NEXT_CODE, 258, "INITIAL_NEXT_CODE == 258")
assert_eq(lzw.INITIAL_CODE_SIZE, 9,   "INITIAL_CODE_SIZE == 9")
assert_eq(lzw.MAX_CODE_SIZE,     16,  "MAX_CODE_SIZE == 16")

-- ---------------------------------------------------------------------------
-- encode_codes
-- ---------------------------------------------------------------------------

do
  local codes, orig = lzw.encode_codes("")
  assert_eq(orig, 0, "encode empty: orig")
  assert_eq(codes[1], lzw.CLEAR_CODE, "encode empty: first = CLEAR")
  assert_eq(codes[#codes], lzw.STOP_CODE, "encode empty: last = STOP")
  assert_eq(#codes, 2, "encode empty: len=2")
end

do
  local codes, orig = lzw.encode_codes("A")
  assert_eq(orig, 1, "encode single: orig")
  assert_eq(codes[1], lzw.CLEAR_CODE, "encode single: first = CLEAR")
  assert_eq(codes[#codes], lzw.STOP_CODE, "encode single: last = STOP")
  local found = false
  for _, c in ipairs(codes) do if c == 65 then found = true end end
  assert_true(found, "encode single: contains 65")
end

do
  local codes, _ = lzw.encode_codes("AB")
  assert_list_eq(codes, {lzw.CLEAR_CODE, 65, 66, lzw.STOP_CODE}, "encode two distinct")
end

do
  local codes, _ = lzw.encode_codes("ABABAB")
  assert_list_eq(codes, {lzw.CLEAR_CODE, 65, 66, 258, 258, lzw.STOP_CODE}, "encode ABABAB")
end

do
  local codes, _ = lzw.encode_codes("AAAAAAA")
  assert_list_eq(codes, {lzw.CLEAR_CODE, 65, 258, 259, 65, lzw.STOP_CODE}, "encode AAAAAAA")
end

-- ---------------------------------------------------------------------------
-- decode_codes
-- ---------------------------------------------------------------------------

assert_eq(lzw.decode_codes({lzw.CLEAR_CODE, lzw.STOP_CODE}), "", "decode empty stream")
assert_eq(lzw.decode_codes({lzw.CLEAR_CODE, 65, lzw.STOP_CODE}), "A", "decode single byte")
assert_eq(lzw.decode_codes({lzw.CLEAR_CODE, 65, 66, lzw.STOP_CODE}), "AB", "decode two distinct")
assert_eq(lzw.decode_codes({lzw.CLEAR_CODE, 65, 66, 258, 258, lzw.STOP_CODE}), "ABABAB", "decode ABABAB")
assert_eq(lzw.decode_codes({lzw.CLEAR_CODE, 65, 258, 259, 65, lzw.STOP_CODE}), "AAAAAAA", "decode AAAAAAA tricky token")
assert_eq(lzw.decode_codes({lzw.CLEAR_CODE, 65, lzw.CLEAR_CODE, 66, lzw.STOP_CODE}), "AB", "decode clear mid-stream")

do
  local result = lzw.decode_codes({lzw.CLEAR_CODE, 9999, 65, lzw.STOP_CODE})
  assert_eq(result, "A", "decode invalid code skipped")
end

-- ---------------------------------------------------------------------------
-- pack / unpack codes
-- ---------------------------------------------------------------------------

do
  local packed = lzw.pack_codes({lzw.CLEAR_CODE, lzw.STOP_CODE}, 42)
  local b1, b2, b3, b4 = string.byte(packed, 1, 4)
  local stored = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4
  assert_eq(stored, 42, "pack: header stores original_length")
end

do
  local codes = {lzw.CLEAR_CODE, 65, 66, 258, 258, lzw.STOP_CODE}
  local packed = lzw.pack_codes(codes, 6)
  local unpacked, orig = lzw.unpack_codes(packed)
  assert_eq(orig, 6, "pack/unpack ABABAB: orig")
  assert_list_eq(unpacked, codes, "pack/unpack ABABAB: codes")
end

do
  local codes = {lzw.CLEAR_CODE, 65, 258, 259, 65, lzw.STOP_CODE}
  local packed = lzw.pack_codes(codes, 7)
  local unpacked, orig = lzw.unpack_codes(packed)
  assert_eq(orig, 7, "pack/unpack AAAAAAA: orig")
  assert_list_eq(unpacked, codes, "pack/unpack AAAAAAA: codes")
end

do
  local codes, orig = lzw.unpack_codes("\x00\x00")
  assert_true(type(codes) == "table", "unpack short: codes is table")
  assert_true(type(orig) == "number", "unpack short: orig is number")
end

-- ---------------------------------------------------------------------------
-- compress / decompress
-- ---------------------------------------------------------------------------

local function rt(data, name)
  local compressed = lzw.compress(data)
  local result = lzw.decompress(compressed)
  assert_eq(result, data, name)
end

rt("", "compress empty")
rt("A", "compress single byte")
rt("AB", "compress two distinct")
rt("ABABAB", "compress ABABAB")
rt("AAAAAAA", "compress AAAAAAA tricky token")
rt("AABABC", "compress AABABC")

do
  local data = string.rep("the quick brown fox jumps over the lazy dog ", 20)
  rt(data, "compress long string")
end

do
  local bytes = {}
  for b = 0, 255 do bytes[#bytes+1] = string.char(b) end
  local data = table.concat(bytes) .. table.concat(bytes)
  rt(data, "compress binary data")
end

do
  local data = string.rep("\x00", 100)
  rt(data, "compress all zeros")
end

do
  local data = string.rep("\xFF", 100)
  rt(data, "compress all 0xFF")
end

do
  local data = string.rep("ABCABC", 100)
  local compressed = lzw.compress(data)
  assert_true(#compressed < #data, "compress: repetitive data shrinks")
end

do
  local data = "hello world"
  local compressed = lzw.compress(data)
  local b1, b2, b3, b4 = string.byte(compressed, 1, 4)
  local stored = b1 * 16777216 + b2 * 65536 + b3 * 256 + b4
  assert_eq(stored, #data, "compress: header stores original_length")
end

-- ---------------------------------------------------------------------------
-- Summary
-- ---------------------------------------------------------------------------

print(string.format("\n%d passed, %d failed", pass, fail))
if fail > 0 then
  os.exit(1)
end
