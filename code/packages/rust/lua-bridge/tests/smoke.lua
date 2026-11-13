-- tests/smoke.lua -- load the lua_bridge_smoke cdylib into a real Lua and drive
-- every part of lua-bridge it uses.
--
--   lua tests/smoke.lua <directory that holds the built cdylib>
--
-- BUILD builds the library with `cargo build --release --example
-- lua_bridge_smoke --target-dir target`, so the directory is
-- target/release/examples.  cargo names the file per platform, so look for each
-- name; `package.loadlib` takes a path as it is, which is why no copy or rename
-- (a macOS .dylib is not a .so) is needed.
--
-- A failed check is an `error`, so a bad declaration in the bridge fails the
-- BUILD; there is no skip path.

-- The bridge declares the Lua 5.4 ABI; a 5.1 `lua` on PATH would fail much later
-- with an unhelpful `undefined symbol: lua_rotate`.
if _VERSION ~= "Lua 5.4" then
  io.stderr:write("tests/smoke.lua needs Lua 5.4 on PATH, found " .. tostring(_VERSION) .. "\n")
  os.exit(2)
end

local dir = arg and arg[1]
if not dir then
  io.stderr:write("usage: lua tests/smoke.lua <cdylib directory>\n")
  os.exit(2)
end

local candidates = {
  dir .. "/liblua_bridge_smoke.so",     -- Linux
  dir .. "/liblua_bridge_smoke.dylib",  -- macOS
  dir .. "/lua_bridge_smoke.dll",       -- Windows
}
local path
for _, candidate in ipairs(candidates) do
  local handle = io.open(candidate, "rb")
  if handle then
    handle:close()
    path = candidate
    break
  end
end
if not path then
  error("no lua_bridge_smoke library in " .. dir .. " (tried " .. table.concat(candidates, ", ") .. ")")
end

local open, why = package.loadlib(path, "luaopen_lua_bridge_smoke")
if not open then
  error("cannot load " .. path .. ": " .. tostring(why))
end
local m = open()
assert(type(m) == "table", "luaopen_lua_bridge_smoke must return the module table")

local checks = 0
local function eq(actual, expected, label)
  checks = checks + 1
  if actual ~= expected then
    error(string.format("%s: expected %s, got %s", label, tostring(expected), tostring(actual)), 2)
  end
end
local function fails(fn, pattern, label, ...)
  checks = checks + 1
  local ok, err = pcall(fn, ...)
  if ok then
    error(label .. ": expected an error, got none", 2)
  end
  if not tostring(err):find(pattern, 1, true) then
    error(string.format("%s: error %q does not contain %q", label, tostring(err), pattern), 2)
  end
end

-- numbers
eq(m.add(1.5, 2.25), 3.75, "add")
eq(m.add("4", 6), 10.0, "add coerces numeric strings")
eq(m.add_int(40, 2), 42, "add_int")
eq(math.type(m.add_int(1, 2)), "integer", "add_int returns an integer")
do
  local v, ok = m.to_int(12)
  eq(v, 12, "to_int value"); eq(ok, true, "to_int ok")
  v, ok = m.to_int("x")
  eq(v, 0, "to_int non-number value"); eq(ok, false, "to_int non-number flag")
end
eq(m.truthy(nil), false, "truthy(nil)")
eq(m.truthy(false), false, "truthy(false)")
eq(m.truthy(0), true, "truthy(0): zero is true in Lua")
eq(m.truthy("x"), true, "truthy(string)")

-- strings
eq(m.echo("hello"), "hello", "echo")
eq(m.echo("a\0b\0c"), "a\0b\0c", "echo keeps embedded NULs")
eq(#m.echo("a\0b\0c"), 5, "echo length with NULs")
eq(m.echo(42), "42", "echo coerces a number to its string")
eq(m.echo({}), nil, "echo of a table is nil")
eq(m.cstring("abc"), "abc", "cstring")
eq(m.cstring("ab\0cd"), "ab", "cstring stops at the first NUL")
fails(m.cstring, "bad argument #1", "cstring rejects a table", {})

-- tables
eq(m.sum({ 1, 2, 3.5 }), 6.5, "sum")
eq(m.sum({}), 0.0, "sum of an empty array")
eq(m.sum({ 1, "x" }), nil, "sum rejects a non-number element")
eq(m.sum(5), nil, "sum rejects a non-table")
do
  local r = m.range(4)
  eq(#r, 4, "range length"); eq(r[1], 1.0, "range[1]"); eq(r[4], 4.0, "range[4]")
  eq(#m.range(0), 0, "range(0)")
end
do
  local pairs_n, len, rawlen = m.count_pairs({ 10, 20, 30, x = 1, y = 2 })
  eq(pairs_n, 5, "count_pairs"); eq(len, 3, "lua_len"); eq(rawlen, 3, "lua_rawlen")
end
do
  local t, k, answer = m.fields()
  eq(t.answer, 7, "lua_setfield"); eq(t.k, 3.5, "lua_settable")
  eq(k, 3.5, "lua_gettable"); eq(answer, 7, "lua_getfield")
end
-- LUA_REGISTRYINDEX: the registry slot 2 is the globals table only if the
-- constant is Lua 5.4's -1001000.
eq(m.registry_globals(), _G, "registry[LUA_RIDX_GLOBALS] is _G")

-- the stack
do
  local a, b, c = m.stack()
  eq(a, 3, "stack[1]"); eq(b, 2, "stack[2]"); eq(c, 3, "stack[3]")
end
do
  local cases = {
    { nil, "nil", "0000" },
    { true, "boolean", "1000" },
    { 3, "number", "0110" },
    { "s", "string", "0010" },
    { {}, "table", "0000" },
    { print, "function", "0000" },
    { io.stdout, "userdata", "0001" },
    { m.light, "function", "0000" },
  }
  for i, case in ipairs(cases) do
    local name, mask = m.type_name(case[1])
    eq(name, case[2], "type_name case " .. i)
    eq(mask, case[3], "type predicates case " .. i)
  end
  eq(select("#", m.type_name()), 2, "type_name() with no argument still returns two values")
  eq((m.type_name()), "none", "type_name() reports LUA_TNONE")
end
eq(m.make_function()(), 99, "lua_pushcfunction")

-- userdata, metatables, __gc
do
  local c = m.Counter(10)
  eq(type(c), "userdata", "Counter is a userdata")
  eq(c:bump(), 11, "counter:bump via __index"); eq(c:bump(), 12, "counter:bump again")
  eq(m.has_metatable(c), true, "userdata has a metatable")
  eq(m.has_metatable({}), false, "a fresh table has none")
  fails(m.Counter(0).bump, "bad argument #1", "luaL_checkudata rejects a non-Counter", {})
  local ty, same = m.light()
  eq(ty, 2, "LUA_TLIGHTUSERDATA"); eq(same, true, "light userdata round trip")
end
do
  local before = m.finalised()
  for i = 1, 25 do m.Counter(i) end
  collectgarbage("collect")
  collectgarbage("collect")
  local after = m.finalised()
  -- The earlier locals are out of scope too, so at least the 25 are finalised.
  checks = checks + 1
  if after - before < 25 then
    error(string.format("__gc ran for %d of 25 Counters", after - before))
  end
end

-- errors: lua_error, luaL_error (variadic) and luaL_argerror unwind to pcall
fails(m.raise, "boom from rust", "raise_error")
fails(m.raise_formatted, "bad thing 42", "luaL_error with varargs")
fails(m.raise_argerror, "bad argument #1 to", "luaL_argerror")
eq(m.add(1, 1), 2.0, "the state is still usable after errors")

print(string.format("lua-bridge smoke: %d checks passed against %s", checks, _VERSION))
