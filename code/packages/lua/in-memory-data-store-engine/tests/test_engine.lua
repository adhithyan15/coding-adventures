local engine_module = require("coding_adventures.in_memory_data_store_engine")

local function execute(engine, ...)
    return engine:execute_parts({...})
end

local function values(response)
    assert.equals("array", response.kind)
    local result = {}
    for index, item in ipairs(response.value) do
        result[index] = item.value
    end
    return result
end

local function assert_error(response, text)
    assert.equals("error", response.kind)
    assert.is_truthy(response.value:find(text, 1, true))
end

describe("DataStoreEngine", function()
    it("handles strings, integers, and keyspace commands", function()
        local engine = engine_module.new()
        assert.equals("PONG", execute(engine, "PING").value)
        assert.equals("hello", execute(engine, "PING", "hello").value)
        assert.equals("\0binary", execute(engine, "ECHO", "\0binary").value)
        assert.equals("OK", execute(engine, "SET", "user:1", "40").value)
        assert.equals("40", execute(engine, "GET", "user:1").value)
        assert.equals(1, execute(engine, "EXISTS", "user:1", "missing").value)
        assert.equals("string", execute(engine, "TYPE", "user:1").value)
        assert.equals("none", execute(engine, "TYPE", "missing").value)
        assert.equals(41, execute(engine, "INCR", "user:1").value)
        assert.equals(43, execute(engine, "INCRBY", "user:1", "2").value)
        assert.equals(42, execute(engine, "DECR", "user:1").value)
        assert.equals(40, execute(engine, "DECRBY", "user:1", "2").value)
        assert.equals(3, execute(engine, "APPEND", "user:1", "!").value)
        assert.equals(3, execute(engine, "APPEND", "new", "abc").value)
        execute(engine, "SET", "user:2", "Lin")
        assert.same({"user:1", "user:2"}, values(execute(engine, "KEYS", "user:*")))
        assert.equals("OK", execute(engine, "RENAME", "user:2", "user:two").value)
        assert.equals("Lin", execute(engine, "GET", "user:two").value)
        execute(engine, "SET", "literal[1]", "yes")
        assert.same({"literal[1]"}, values(execute(engine, "KEYS", "literal[1]")))
        assert.equals(1, execute(engine, "DEL", "user:1", "missing").value)
        assert.is_nil(execute(engine, "GET", "user:1").value)
    end)

    it("handles hashes and lists", function()
        local engine = engine_module.new()
        assert.equals(2, execute(engine, "HSET", "user", "name", "Ada", "city", "London").value)
        assert.equals(0, execute(engine, "HSET", "user", "name", "Augusta").value)
        assert.equals("Augusta", execute(engine, "HGET", "user", "name").value)
        assert.equals(1, execute(engine, "HEXISTS", "user", "city").value)
        assert.equals(2, execute(engine, "HLEN", "user").value)
        assert.same({"city", "name"}, values(execute(engine, "HKEYS", "user")))
        assert.same({"London", "Augusta"}, values(execute(engine, "HVALS", "user")))
        assert.same({"city", "London", "name", "Augusta"}, values(execute(engine, "HGETALL", "user")))
        assert.equals(1, execute(engine, "HDEL", "user", "city", "missing").value)
        assert.equals(1, execute(engine, "HDEL", "user", "name").value)
        assert.equals(0, execute(engine, "HLEN", "user").value)
        assert.equals(2, execute(engine, "LPUSH", "queue", "b", "a").value)
        assert.equals(3, execute(engine, "RPUSH", "queue", "c").value)
        assert.equals(3, execute(engine, "LLEN", "queue").value)
        assert.equals("c", execute(engine, "LINDEX", "queue", "-1").value)
        assert.same({"a", "b", "c"}, values(execute(engine, "LRANGE", "queue", "0", "-1")))
        assert.equals("a", execute(engine, "LPOP", "queue").value)
        assert.equals("c", execute(engine, "RPOP", "queue").value)
        assert.equals("b", execute(engine, "RPOP", "queue").value)
        assert.is_nil(execute(engine, "LPOP", "queue").value)
    end)

    it("handles sets, sorted sets, and HyperLogLog", function()
        local engine = engine_module.new()
        assert.equals(3, execute(engine, "SADD", "left", "a", "b", "c", "a").value)
        assert.equals(3, execute(engine, "SADD", "right", "b", "c", "d").value)
        assert.equals(1, execute(engine, "SISMEMBER", "left", "b").value)
        assert.equals(3, execute(engine, "SCARD", "left").value)
        assert.same({"a", "b", "c"}, values(execute(engine, "SMEMBERS", "left")))
        assert.same({"a", "b", "c", "d"}, values(execute(engine, "SUNION", "left", "right")))
        assert.same({"b", "c"}, values(execute(engine, "SINTER", "left", "right")))
        assert.same({"a"}, values(execute(engine, "SDIFF", "left", "right")))
        assert.same({}, values(execute(engine, "SINTER", "left", "missing")))
        assert.equals(1, execute(engine, "SREM", "left", "a", "missing").value)
        assert.equals(3, execute(engine, "ZADD", "scores", "1", "alice", "2", "bob", "1.5", "cara").value)
        assert.equals(0, execute(engine, "ZADD", "scores", "3", "alice").value)
        assert.same({"cara", "bob", "alice"}, values(execute(engine, "ZRANGE", "scores", "0", "-1")))
        assert.same({"cara", "1.5", "bob", "2"}, values(execute(engine, "ZRANGE", "scores", "0", "1", "WITHSCORES")))
        assert.same({"cara", "bob"}, values(execute(engine, "ZRANGEBYSCORE", "scores", "1", "2")))
        assert.equals(1, execute(engine, "ZRANK", "scores", "bob").value)
        assert.equals("1.5", execute(engine, "ZSCORE", "scores", "cara").value)
        assert.equals(3, execute(engine, "ZCARD", "scores").value)
        assert.equals(1, execute(engine, "ZREM", "scores", "bob", "missing").value)
        assert.equals(1, execute(engine, "PFADD", "visitors", "alice", "bob").value)
        assert.equals(0, execute(engine, "PFADD", "visitors", "alice").value)
        assert.equals(1, execute(engine, "PFADD", "other", "cara").value)
        assert.is_true(execute(engine, "PFCOUNT", "visitors").value >= 2)
        assert.is_true(execute(engine, "PFCOUNT", "visitors", "other").value >= 3)
        assert.equals("OK", execute(engine, "PFMERGE", "all", "visitors", "other").value)
        assert.is_true(execute(engine, "PFCOUNT", "all").value >= 3)
    end)

    it("handles expiry, logical databases, and admin commands", function()
        local now = 2000000
        local engine = engine_module.new({time_provider = function() return now end})
        execute(engine, "SET", "temporary", "value")
        assert.equals(-1, execute(engine, "TTL", "temporary").value)
        assert.equals(0, execute(engine, "PERSIST", "temporary").value)
        assert.equals(1, execute(engine, "EXPIRE", "temporary", "10").value)
        assert.equals(10, execute(engine, "TTL", "temporary").value)
        assert.equals(10000, execute(engine, "PTTL", "temporary").value)
        assert.equals(1, execute(engine, "PERSIST", "temporary").value)
        assert.equals(1, execute(engine, "EXPIREAT", "temporary", "1999").value)
        assert.is_nil(execute(engine, "GET", "temporary").value)
        assert.equals(-2, execute(engine, "TTL", "temporary").value)
        execute(engine, "SET", "db0", "zero")
        assert.equals("OK", execute(engine, "SELECT", "1").value)
        execute(engine, "SET", "db1", "one")
        assert.equals(1, execute(engine, "DBSIZE").value)
        assert.is_truthy(execute(engine, "INFO").value:find("active_db:1", 1, true))
        assert.equals("OK", execute(engine, "FLUSHDB").value)
        assert.equals(0, execute(engine, "DBSIZE").value)
        execute(engine, "SET", "again", "one")
        assert.equals("OK", execute(engine, "FLUSHALL").value)
        execute(engine, "SELECT", "0")
        assert.equals(0, execute(engine, "DBSIZE").value)
    end)

    it("returns protocol, arity, type, and parse errors", function()
        local engine = engine_module.new()
        assert_error(engine:execute_frame(nil), "protocol error")
        assert_error(execute(engine, "NOPE"), "unknown command")
        for _, parts in ipairs({
            {"PING", "a", "b"}, {"ECHO"}, {"SET", "a"}, {"GET"}, {"DEL"},
            {"HSET", "a", "b"}, {"LPUSH", "a"}, {"SADD", "a"},
            {"ZADD", "a", "1"}, {"PFADD", "a"}, {"EXPIRE", "a"},
            {"SELECT"}, {"FLUSHDB", "x"}, {"INFO", "x"},
        }) do
            assert_error(engine:execute_parts(parts), "wrong number")
        end
        assert_error(execute(engine, "RENAME", "missing", "other"), "no such key")
        assert_error(execute(engine, "SELECT", "99"), "DB index")
        execute(engine, "SET", "string", "value")
        for _, parts in ipairs({
            {"HGET", "string", "field"}, {"LPUSH", "string", "value"},
            {"SADD", "string", "value"}, {"ZADD", "string", "1", "value"},
            {"PFADD", "string", "value"}, {"SUNION", "string"},
        }) do
            assert_error(engine:execute_parts(parts), "WRONGTYPE")
        end
        assert_error(execute(engine, "INCR", "string"), "integer")
        assert_error(execute(engine, "INCRBY", "n", "bad"), "integer")
        assert_error(execute(engine, "DECRBY", "n", "-9223372036854775808"), "integer")
        execute(engine, "SET", "max", "9223372036854775807")
        assert_error(execute(engine, "INCR", "max"), "integer")
        assert_error(execute(engine, "ZADD", "z", "nan", "a"), "float")
    end)

    it("exposes storage and sorted-set helpers", function()
        assert.has_error(function() engine_module.Store.new(0) end, "database_count must be positive")
        local store = engine_module.Store.new(2)
        store:select(1)
        assert.equals(1, store.active_db)
        local now = 50000
        local database = engine_module.Database.new(function() return now end)
        database:set("live", engine_module.Entry.new(engine_module.EntryType.STRING, "yes"))
        database:set("old", engine_module.Entry.new(engine_module.EntryType.STRING, "no", now - 1))
        assert.is_nil(database:get("old"))
        assert.same({"live"}, database:keys("l?ve"))
        database:clear()
        assert.same({}, database.entries)
        local sorted_set = engine_module.SortedSet.new()
        assert.is_true(sorted_set:insert(1, "b"))
        assert.is_true(sorted_set:insert(1, "a"))
        assert.is_false(sorted_set:insert(2, "b"))
        assert.equals(0, sorted_set:rank("a"))
        assert.same({{member = "a", score = 1}}, sorted_set:range_by_score(0, 1.5))
        assert.is_true(sorted_set:remove("a"))
    end)

    it("accepts public frames and preserves binary strings", function()
        local engine = engine_module.new({time_provider = function() return 1234 end})
        assert.equals(1234, engine:current_time_ms())
        local frame = require("coding_adventures.in_memory_data_store_protocol").CommandFrame.new("ping")
        assert.equals("PONG", engine:execute_frame(frame).value)
        local binary = "\0\255\1"
        execute(engine, "SET", binary, binary)
        assert.equals(binary, execute(engine, "GET", binary).value)
    end)

    it("matches a deterministic randomized string reference model", function()
        local engine = engine_module.new()
        local model = {}
        local state = 20260716
        local function next_random(limit)
            state = (state * 1103515245 + 12345) & 0x7fffffff
            return state % limit
        end
        for _ = 1, 5000 do
            local key = "key:" .. next_random(31)
            local choice = next_random(6)
            if choice == 0 then
                local value = tostring(next_random(10000))
                assert.equals("OK", execute(engine, "SET", key, value).value)
                model[key] = value
            elseif choice == 1 then
                assert.equals(model[key], execute(engine, "GET", key).value)
            elseif choice == 2 then
                local expected = model[key] == nil and 0 or 1
                assert.equals(expected, execute(engine, "DEL", key).value)
                model[key] = nil
            elseif choice == 3 then
                assert.equals(model[key] == nil and 0 or 1, execute(engine, "EXISTS", key).value)
            elseif choice == 4 then
                local suffix = string.char(97 + next_random(26))
                model[key] = (model[key] or "") .. suffix
                assert.equals(#model[key], execute(engine, "APPEND", key, suffix).value)
            else
                local current = model[key]
                if current == nil or current:match("^[+-]?%d+$") then
                    local expected = (tonumber(current) or 0) + 1
                    assert.equals(expected, execute(engine, "INCR", key).value)
                    model[key] = tostring(expected)
                else
                    assert_error(execute(engine, "INCR", key), "integer")
                end
            end
        end
    end)
end)
