local protocol = require("coding_adventures.in_memory_data_store_protocol")
local HyperLogLog = require("coding_adventures.hyperloglog").HyperLogLog

local Response = protocol.EngineResponse
local M = { VERSION = "0.1.0" }

local EntryType = {
    STRING = "string",
    HASH = "hash",
    LIST = "list",
    SET = "set",
    ZSET = "zset",
    HLL = "hll",
}

local function system_now_ms()
    return os.time() * 1000
end

local function count_keys(values)
    local count = 0
    for _ in pairs(values) do
        count = count + 1
    end
    return count
end

local function sorted_keys(values)
    local keys = {}
    for key in pairs(values) do
        keys[#keys + 1] = key
    end
    table.sort(keys)
    return keys
end

local function glob_match(pattern, value)
    local pattern_index, value_index = 1, 1
    local star_index, retry_value_index
    while value_index <= #value do
        local pattern_byte = string.byte(pattern, pattern_index)
        local value_byte = string.byte(value, value_index)
        if pattern_index <= #pattern and (pattern_byte == string.byte("?") or pattern_byte == value_byte) then
            pattern_index = pattern_index + 1
            value_index = value_index + 1
        elseif pattern_index <= #pattern and pattern_byte == string.byte("*") then
            star_index = pattern_index
            retry_value_index = value_index
            pattern_index = pattern_index + 1
        elseif star_index ~= nil then
            retry_value_index = retry_value_index + 1
            value_index = retry_value_index
            pattern_index = star_index + 1
        else
            return false
        end
    end
    while pattern_index <= #pattern and string.byte(pattern, pattern_index) == string.byte("*") do
        pattern_index = pattern_index + 1
    end
    return pattern_index > #pattern
end

local Entry = {}
Entry.__index = Entry

function Entry.new(entry_type, value, expires_at_ms)
    return setmetatable({ type = entry_type, value = value, expires_at_ms = expires_at_ms }, Entry)
end

local SortedSet = {}
SortedSet.__index = SortedSet

function SortedSet.new()
    return setmetatable({ scores = {} }, SortedSet)
end

function SortedSet:insert(score, member)
    local is_new = self.scores[member] == nil
    self.scores[member] = score
    return is_new
end

function SortedSet:remove(member)
    if self.scores[member] == nil then
        return false
    end
    self.scores[member] = nil
    return true
end

function SortedSet:ordered_entries()
    local result = {}
    for member, score in pairs(self.scores) do
        result[#result + 1] = { member = member, score = score }
    end
    table.sort(result, function(left, right)
        return left.score < right.score or (left.score == right.score and left.member < right.member)
    end)
    return result
end

function SortedSet:rank(member)
    for index, item in ipairs(self:ordered_entries()) do
        if item.member == member then
            return index - 1
        end
    end
    return nil
end

function SortedSet:score(member)
    return self.scores[member]
end

function SortedSet:size()
    return count_keys(self.scores)
end

function SortedSet:range_by_index(start_index, end_index)
    local entries = self:ordered_entries()
    local length = #entries
    if length == 0 then
        return {}
    end
    if start_index < 0 then
        start_index = length + start_index
    end
    if end_index < 0 then
        end_index = length + end_index
    end
    if start_index < 0 or end_index < 0 or start_index >= length or start_index > end_index then
        return {}
    end
    end_index = math.min(end_index, length - 1)
    local result = {}
    for index = start_index + 1, end_index + 1 do
        result[#result + 1] = entries[index]
    end
    return result
end

function SortedSet:range_by_score(minimum, maximum)
    local result = {}
    for _, item in ipairs(self:ordered_entries()) do
        if item.score >= minimum and item.score <= maximum then
            result[#result + 1] = item
        end
    end
    return result
end

local Database = {}
Database.__index = Database

function Database.new(now_provider)
    return setmetatable({ entries = {}, now_provider = now_provider or system_now_ms }, Database)
end

function Database:get(key)
    local entry = self.entries[key]
    if entry ~= nil and entry.expires_at_ms ~= nil and entry.expires_at_ms <= self.now_provider() then
        self.entries[key] = nil
        return nil
    end
    return entry
end

function Database:set(key, entry)
    self.entries[key] = entry
end

function Database:delete(key)
    if self.entries[key] == nil then
        return false
    end
    self.entries[key] = nil
    return true
end

function Database:expire_lazy(key)
    self:get(key)
end

function Database:active_expire()
    local now = self.now_provider()
    for key, entry in pairs(self.entries) do
        if entry.expires_at_ms ~= nil and entry.expires_at_ms <= now then
            self.entries[key] = nil
        end
    end
end

function Database:keys(pattern)
    self:active_expire()
    local result = {}
    for key in pairs(self.entries) do
        if glob_match(pattern, key) then
            result[#result + 1] = key
        end
    end
    table.sort(result)
    return result
end

function Database:clear()
    self.entries = {}
end

local Store = {}
Store.__index = Store

function Store.new(database_count, now_provider)
    database_count = database_count == nil and 16 or database_count
    if type(database_count) ~= "number" or database_count % 1 ~= 0 or database_count <= 0 then
        error("database_count must be positive", 2)
    end
    local databases = {}
    for index = 1, database_count do
        databases[index] = Database.new(now_provider)
    end
    return setmetatable({ databases = databases, active_db = 0 }, Store)
end

function Store:active_database()
    return self.databases[self.active_db + 1]
end

function Store:select(index)
    self.active_db = index
end

function Store:flushdb()
    self:active_database():clear()
end

function Store:flushall()
    for _, database in ipairs(self.databases) do
        database:clear()
    end
end

local function bulk(value)
    return Response.bulk_string(value)
end

local function integer(value)
    return Response.integer(value)
end

local function array(values)
    return Response.array(values)
end

local function err(message)
    return Response.error(message)
end

local function wrong_arity(command)
    return err(string.format("ERR wrong number of arguments for '%s' command", command))
end

local function wrong_type()
    return err("WRONGTYPE Operation against a key holding the wrong kind of value")
end

local function integer_error()
    return err("ERR value is not an integer or out of range")
end

local function float_error()
    return err("ERR value is not a valid float")
end

local function parse_i64(value)
    if type(value) ~= "string" or not value:match("^[+-]?%d+$") then
        return nil
    end
    local parsed = tonumber(value)
    if parsed == nil or math.type(parsed) ~= "integer" then
        return nil
    end
    return parsed
end

local function parse_float(value)
    local parsed = tonumber(value)
    if parsed == nil or parsed ~= parsed or parsed == math.huge or parsed == -math.huge then
        return nil
    end
    return parsed
end

local function format_score(score)
    if score == math.floor(score) then
        return string.format("%.0f", score)
    end
    local text = string.format("%.15f", score)
    text = text:gsub("0+$", ""):gsub("%.$", "")
    return text
end

local DataStoreEngine = {}
DataStoreEngine.__index = DataStoreEngine

function DataStoreEngine.new(options)
    options = options or {}
    if getmetatable(options) == Store then
        options = { store = options }
    end
    local now_provider = options.time_provider or system_now_ms
    local self = setmetatable({ now_provider = now_provider }, DataStoreEngine)
    self.store = options.store or Store.new(options.database_count, now_provider)
    return self
end

function DataStoreEngine:current_time_ms()
    return self.now_provider()
end

function DataStoreEngine:key_entry(key)
    return self.store:active_database():get(key)
end

function DataStoreEngine:ensure_collection(key, entry_type, factory)
    local entry = self:key_entry(key)
    if entry == nil then
        entry = Entry.new(entry_type, factory())
        self.store:active_database():set(key, entry)
    end
    if entry.type ~= entry_type then
        return nil
    end
    return entry
end

function DataStoreEngine:execute_frame(frame)
    if frame == nil then
        return err("ERR protocol error: expected array of bulk strings")
    end
    self.store:active_database():active_expire()
    local command = string.upper(frame.command)
    local handler = self["command_" .. string.lower(command)]
    if handler == nil then
        return err(string.format("ERR unknown command '%s'", string.lower(frame.command)))
    end
    return handler(self, frame.args)
end

function DataStoreEngine:execute_parts(parts)
    return self:execute_frame(protocol.CommandFrame.from_parts(parts))
end

function DataStoreEngine:command_ping(args)
    if #args == 0 then return Response.simple_string("PONG") end
    if #args == 1 then return bulk(args[1]) end
    return wrong_arity("ping")
end

function DataStoreEngine:command_echo(args)
    return #args == 1 and bulk(args[1]) or wrong_arity("echo")
end

function DataStoreEngine:command_set(args)
    if #args ~= 2 then return wrong_arity("set") end
    self.store:active_database():set(args[1], Entry.new(EntryType.STRING, args[2]))
    return Response.ok()
end

function DataStoreEngine:command_get(args)
    if #args ~= 1 then return wrong_arity("get") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.null() end
    if entry.type ~= EntryType.STRING then return wrong_type() end
    return bulk(entry.value)
end

function DataStoreEngine:command_del(args)
    if #args == 0 then return wrong_arity("del") end
    local removed = 0
    for _, key in ipairs(args) do
        if self.store:active_database():delete(key) then removed = removed + 1 end
    end
    return integer(removed)
end

function DataStoreEngine:command_exists(args)
    if #args == 0 then return wrong_arity("exists") end
    local found = 0
    for _, key in ipairs(args) do
        if self:key_entry(key) ~= nil then found = found + 1 end
    end
    return integer(found)
end

function DataStoreEngine:command_keys(args)
    if #args ~= 1 then return wrong_arity("keys") end
    local result = {}
    for _, key in ipairs(self.store:active_database():keys(args[1])) do
        result[#result + 1] = bulk(key)
    end
    return array(result)
end

function DataStoreEngine:command_type(args)
    if #args ~= 1 then return wrong_arity("type") end
    local entry = self:key_entry(args[1])
    return Response.simple_string(entry and entry.type or "none")
end

function DataStoreEngine:command_rename(args)
    if #args ~= 2 then return wrong_arity("rename") end
    local entry = self:key_entry(args[1])
    if entry == nil then return err("ERR no such key") end
    if args[1] ~= args[2] then
        self.store:active_database():delete(args[1])
        self.store:active_database():set(args[2], entry)
    end
    return Response.ok()
end

function DataStoreEngine:command_append(args)
    if #args ~= 2 then return wrong_arity("append") end
    local entry = self:key_entry(args[1])
    if entry == nil then
        self.store:active_database():set(args[1], Entry.new(EntryType.STRING, args[2]))
        return integer(#args[2])
    end
    if entry.type ~= EntryType.STRING then return wrong_type() end
    entry.value = entry.value .. args[2]
    return integer(#entry.value)
end

function DataStoreEngine:incr_by(args, fixed_delta, command)
    local expected = fixed_delta == nil and 2 or 1
    if #args ~= expected then return wrong_arity(command) end
    local delta = fixed_delta or parse_i64(args[2])
    if delta == nil then return integer_error() end
    local entry = self:key_entry(args[1])
    if entry ~= nil and entry.type ~= EntryType.STRING then return wrong_type() end
    local current = entry == nil and 0 or parse_i64(entry.value)
    if current == nil then return integer_error() end
    if (delta > 0 and current > math.maxinteger - delta)
        or (delta < 0 and current < math.mininteger - delta)
    then
        return integer_error()
    end
    local result = current + delta
    self.store:active_database():set(args[1], Entry.new(EntryType.STRING, tostring(result), entry and entry.expires_at_ms))
    return integer(result)
end

function DataStoreEngine:command_incr(args) return self:incr_by(args, 1, "incr") end
function DataStoreEngine:command_decr(args) return self:incr_by(args, -1, "decr") end
function DataStoreEngine:command_incrby(args) return self:incr_by(args, nil, "incrby") end

function DataStoreEngine:command_decrby(args)
    if #args ~= 2 then return wrong_arity("decrby") end
    local delta = parse_i64(args[2])
    if delta == nil or delta == math.mininteger then return integer_error() end
    return self:incr_by({ args[1], tostring(-delta) }, nil, "decrby")
end

function DataStoreEngine:command_hset(args)
    if #args < 3 or #args % 2 == 0 then return wrong_arity("hset") end
    local entry = self:ensure_collection(args[1], EntryType.HASH, function() return {} end)
    if entry == nil then return wrong_type() end
    local added = 0
    for index = 2, #args, 2 do
        if entry.value[args[index]] == nil then added = added + 1 end
        entry.value[args[index]] = args[index + 1]
    end
    return integer(added)
end

function DataStoreEngine:command_hget(args)
    if #args ~= 2 then return wrong_arity("hget") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.null() end
    if entry.type ~= EntryType.HASH then return wrong_type() end
    return bulk(entry.value[args[2]])
end

function DataStoreEngine:command_hdel(args)
    if #args < 2 then return wrong_arity("hdel") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.HASH then return wrong_type() end
    local removed = 0
    for index = 2, #args do
        if entry.value[args[index]] ~= nil then
            entry.value[args[index]] = nil
            removed = removed + 1
        end
    end
    if count_keys(entry.value) == 0 then self.store:active_database():delete(args[1]) end
    return integer(removed)
end

function DataStoreEngine:hash_array(args, command, values_only)
    if #args ~= 1 then return wrong_arity(command) end
    local entry = self:key_entry(args[1])
    if entry == nil then return array({}) end
    if entry.type ~= EntryType.HASH then return wrong_type() end
    local result = {}
    for _, field in ipairs(sorted_keys(entry.value)) do
        if values_only ~= true then result[#result + 1] = bulk(field) end
        if values_only ~= false then result[#result + 1] = bulk(entry.value[field]) end
    end
    return array(result)
end

function DataStoreEngine:command_hgetall(args) return self:hash_array(args, "hgetall", nil) end
function DataStoreEngine:command_hkeys(args) return self:hash_array(args, "hkeys", false) end
function DataStoreEngine:command_hvals(args) return self:hash_array(args, "hvals", true) end

function DataStoreEngine:command_hlen(args)
    if #args ~= 1 then return wrong_arity("hlen") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.HASH then return wrong_type() end
    return integer(count_keys(entry.value))
end

function DataStoreEngine:command_hexists(args)
    if #args ~= 2 then return wrong_arity("hexists") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.HASH then return wrong_type() end
    return integer(entry.value[args[2]] ~= nil and 1 or 0)
end

function DataStoreEngine:push_list(args, left)
    local command = left and "lpush" or "rpush"
    if #args < 2 then return wrong_arity(command) end
    local entry = self:ensure_collection(args[1], EntryType.LIST, function() return {} end)
    if entry == nil then return wrong_type() end
    for index = 2, #args do
        if left then table.insert(entry.value, 1, args[index]) else entry.value[#entry.value + 1] = args[index] end
    end
    return integer(#entry.value)
end

function DataStoreEngine:command_lpush(args) return self:push_list(args, true) end
function DataStoreEngine:command_rpush(args) return self:push_list(args, false) end

function DataStoreEngine:pop_list(args, left)
    local command = left and "lpop" or "rpop"
    if #args ~= 1 then return wrong_arity(command) end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.null() end
    if entry.type ~= EntryType.LIST then return wrong_type() end
    local value = table.remove(entry.value, left and 1 or #entry.value)
    if #entry.value == 0 then self.store:active_database():delete(args[1]) end
    return bulk(value)
end

function DataStoreEngine:command_lpop(args) return self:pop_list(args, true) end
function DataStoreEngine:command_rpop(args) return self:pop_list(args, false) end

function DataStoreEngine:command_llen(args)
    if #args ~= 1 then return wrong_arity("llen") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.LIST then return wrong_type() end
    return integer(#entry.value)
end

function DataStoreEngine:command_lindex(args)
    if #args ~= 2 then return wrong_arity("lindex") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.null() end
    if entry.type ~= EntryType.LIST then return wrong_type() end
    local index = parse_i64(args[2])
    if index == nil then return integer_error() end
    if index < 0 then index = #entry.value + index end
    return bulk(entry.value[index + 1])
end

function DataStoreEngine:command_lrange(args)
    if #args ~= 3 then return wrong_arity("lrange") end
    local entry = self:key_entry(args[1])
    if entry == nil then return array({}) end
    if entry.type ~= EntryType.LIST then return wrong_type() end
    local start_index, stop_index = parse_i64(args[2]), parse_i64(args[3])
    if start_index == nil or stop_index == nil then return integer_error() end
    local length = #entry.value
    if start_index < 0 then start_index = length + start_index end
    if stop_index < 0 then stop_index = length + stop_index end
    start_index = math.max(0, start_index)
    stop_index = math.min(length - 1, stop_index)
    if length == 0 or start_index > stop_index or start_index >= length then return array({}) end
    local result = {}
    for index = start_index + 1, stop_index + 1 do result[#result + 1] = bulk(entry.value[index]) end
    return array(result)
end

function DataStoreEngine:command_sadd(args)
    if #args < 2 then return wrong_arity("sadd") end
    local entry = self:ensure_collection(args[1], EntryType.SET, function() return {} end)
    if entry == nil then return wrong_type() end
    local added = 0
    for index = 2, #args do
        if entry.value[args[index]] == nil then added = added + 1 end
        entry.value[args[index]] = true
    end
    return integer(added)
end

function DataStoreEngine:command_srem(args)
    if #args < 2 then return wrong_arity("srem") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.SET then return wrong_type() end
    local removed = 0
    for index = 2, #args do
        if entry.value[args[index]] then entry.value[args[index]], removed = nil, removed + 1 end
    end
    if count_keys(entry.value) == 0 then self.store:active_database():delete(args[1]) end
    return integer(removed)
end

function DataStoreEngine:command_sismember(args)
    if #args ~= 2 then return wrong_arity("sismember") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.SET then return wrong_type() end
    return integer(entry.value[args[2]] and 1 or 0)
end

function DataStoreEngine:command_smembers(args)
    if #args ~= 1 then return wrong_arity("smembers") end
    local entry = self:key_entry(args[1])
    if entry == nil then return array({}) end
    if entry.type ~= EntryType.SET then return wrong_type() end
    local result = {}
    for _, value in ipairs(sorted_keys(entry.value)) do result[#result + 1] = bulk(value) end
    return array(result)
end

function DataStoreEngine:command_scard(args)
    if #args ~= 1 then return wrong_arity("scard") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.SET then return wrong_type() end
    return integer(count_keys(entry.value))
end

function DataStoreEngine:set_operation(args, command, operation)
    if #args == 0 then return wrong_arity(command) end
    local result = {}
    local first = self:key_entry(args[1])
    if first ~= nil and first.type ~= EntryType.SET then return wrong_type() end
    if first ~= nil then for value in pairs(first.value) do result[value] = true end end
    if operation == "union" then
        for index = 2, #args do
            local entry = self:key_entry(args[index])
            if entry ~= nil and entry.type ~= EntryType.SET then return wrong_type() end
            if entry ~= nil then for value in pairs(entry.value) do result[value] = true end end
        end
    else
        for index = 2, #args do
            local entry = self:key_entry(args[index])
            if entry ~= nil and entry.type ~= EntryType.SET then return wrong_type() end
            if operation == "intersection" then
                for value in pairs(result) do
                    if entry == nil or not entry.value[value] then result[value] = nil end
                end
            elseif entry ~= nil then
                for value in pairs(entry.value) do result[value] = nil end
            end
        end
    end
    local responses = {}
    for _, value in ipairs(sorted_keys(result)) do responses[#responses + 1] = bulk(value) end
    return array(responses)
end

function DataStoreEngine:command_sunion(args) return self:set_operation(args, "sunion", "union") end
function DataStoreEngine:command_sinter(args) return self:set_operation(args, "sinter", "intersection") end
function DataStoreEngine:command_sdiff(args) return self:set_operation(args, "sdiff", "difference") end

function DataStoreEngine:command_zadd(args)
    if #args < 3 or #args % 2 == 0 then return wrong_arity("zadd") end
    local parsed = {}
    for index = 2, #args, 2 do
        local score = parse_float(args[index])
        if score == nil then return float_error() end
        parsed[#parsed + 1] = { score = score, member = args[index + 1] }
    end
    local entry = self:ensure_collection(args[1], EntryType.ZSET, SortedSet.new)
    if entry == nil then return wrong_type() end
    local added = 0
    for _, item in ipairs(parsed) do if entry.value:insert(item.score, item.member) then added = added + 1 end end
    return integer(added)
end

function DataStoreEngine:flatten_zset(values, with_scores)
    local result = {}
    for _, item in ipairs(values) do
        result[#result + 1] = bulk(item.member)
        if with_scores then result[#result + 1] = bulk(format_score(item.score)) end
    end
    return result
end

function DataStoreEngine:command_zrange(args)
    if #args ~= 3 and #args ~= 4 then return wrong_arity("zrange") end
    local start_index, end_index = parse_i64(args[2]), parse_i64(args[3])
    if start_index == nil or end_index == nil then return integer_error() end
    local entry = self:key_entry(args[1])
    if entry == nil then return array({}) end
    if entry.type ~= EntryType.ZSET then return wrong_type() end
    local with_scores = #args == 4 and string.upper(args[4]) == "WITHSCORES"
    return array(self:flatten_zset(entry.value:range_by_index(start_index, end_index), with_scores))
end

function DataStoreEngine:command_zrangebyscore(args)
    if #args ~= 3 and #args ~= 4 then return wrong_arity("zrangebyscore") end
    local minimum, maximum = parse_float(args[2]), parse_float(args[3])
    if minimum == nil or maximum == nil then return float_error() end
    local entry = self:key_entry(args[1])
    if entry == nil then return array({}) end
    if entry.type ~= EntryType.ZSET then return wrong_type() end
    local with_scores = #args == 4 and string.upper(args[4]) == "WITHSCORES"
    return array(self:flatten_zset(entry.value:range_by_score(minimum, maximum), with_scores))
end

function DataStoreEngine:command_zrank(args)
    if #args ~= 2 then return wrong_arity("zrank") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.null() end
    if entry.type ~= EntryType.ZSET then return wrong_type() end
    local rank = entry.value:rank(args[2])
    return rank == nil and Response.null() or integer(rank)
end

function DataStoreEngine:command_zscore(args)
    if #args ~= 2 then return wrong_arity("zscore") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.null() end
    if entry.type ~= EntryType.ZSET then return wrong_type() end
    local score = entry.value:score(args[2])
    return score == nil and Response.null() or bulk(format_score(score))
end

function DataStoreEngine:command_zcard(args)
    if #args ~= 1 then return wrong_arity("zcard") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.ZSET then return wrong_type() end
    return integer(entry.value:size())
end

function DataStoreEngine:command_zrem(args)
    if #args < 2 then return wrong_arity("zrem") end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    if entry.type ~= EntryType.ZSET then return wrong_type() end
    local removed = 0
    for index = 2, #args do if entry.value:remove(args[index]) then removed = removed + 1 end end
    if entry.value:size() == 0 then self.store:active_database():delete(args[1]) end
    return integer(removed)
end

function DataStoreEngine:command_pfadd(args)
    if #args < 2 then return wrong_arity("pfadd") end
    local entry = self:ensure_collection(args[1], EntryType.HLL, HyperLogLog.new)
    if entry == nil then return wrong_type() end
    local before = table.concat(entry.value:registers(), ",")
    for index = 2, #args do entry.value:add(args[index]) end
    return integer(before ~= table.concat(entry.value:registers(), ",") and 1 or 0)
end

function DataStoreEngine:command_pfcount(args)
    if #args == 0 then return wrong_arity("pfcount") end
    local aggregate
    for _, key in ipairs(args) do
        local entry = self:key_entry(key)
        if entry ~= nil and entry.type ~= EntryType.HLL then return wrong_type() end
        if entry ~= nil then aggregate = aggregate == nil and entry.value or aggregate:merge(entry.value) end
    end
    return integer(aggregate == nil and 0 or aggregate:count())
end

function DataStoreEngine:command_pfmerge(args)
    if #args < 2 then return wrong_arity("pfmerge") end
    local aggregate
    for index = 2, #args do
        local entry = self:key_entry(args[index])
        if entry ~= nil and entry.type ~= EntryType.HLL then return wrong_type() end
        if entry ~= nil then aggregate = aggregate == nil and entry.value or aggregate:merge(entry.value) end
    end
    local destination = self:key_entry(args[1])
    self.store:active_database():set(args[1], Entry.new(EntryType.HLL, aggregate or HyperLogLog.new(), destination and destination.expires_at_ms))
    return Response.ok()
end

function DataStoreEngine:expire(args, absolute)
    local command = absolute and "expireat" or "expire"
    if #args ~= 2 then return wrong_arity(command) end
    local entry = self:key_entry(args[1])
    if entry == nil then return Response.zero() end
    local seconds = parse_i64(args[2])
    if seconds == nil then return integer_error() end
    entry.expires_at_ms = absolute and seconds * 1000 or self.now_provider() + seconds * 1000
    return Response.one()
end

function DataStoreEngine:command_expire(args) return self:expire(args, false) end
function DataStoreEngine:command_expireat(args) return self:expire(args, true) end

function DataStoreEngine:command_ttl(args)
    if #args ~= 1 then return wrong_arity("ttl") end
    local entry = self:key_entry(args[1])
    if entry == nil then return integer(-2) end
    if entry.expires_at_ms == nil then return integer(-1) end
    return integer(math.max(-2, math.floor((entry.expires_at_ms - self.now_provider()) / 1000)))
end

function DataStoreEngine:command_pttl(args)
    if #args ~= 1 then return wrong_arity("pttl") end
    local entry = self:key_entry(args[1])
    if entry == nil then return integer(-2) end
    if entry.expires_at_ms == nil then return integer(-1) end
    return integer(math.max(-1, entry.expires_at_ms - self.now_provider()))
end

function DataStoreEngine:command_persist(args)
    if #args ~= 1 then return wrong_arity("persist") end
    local entry = self:key_entry(args[1])
    if entry == nil or entry.expires_at_ms == nil then return Response.zero() end
    entry.expires_at_ms = nil
    return Response.one()
end

function DataStoreEngine:command_select(args)
    if #args ~= 1 then return wrong_arity("select") end
    local index = parse_i64(args[1])
    if index == nil or index < 0 or index >= #self.store.databases then return err("ERR DB index is out of range") end
    self.store:select(index)
    return Response.ok()
end

function DataStoreEngine:command_flushdb(args)
    if #args ~= 0 then return wrong_arity("flushdb") end
    self.store:flushdb()
    return Response.ok()
end

function DataStoreEngine:command_flushall(args)
    if #args ~= 0 then return wrong_arity("flushall") end
    self.store:flushall()
    return Response.ok()
end

function DataStoreEngine:command_dbsize(args)
    if #args ~= 0 then return wrong_arity("dbsize") end
    self.store:active_database():active_expire()
    return integer(count_keys(self.store:active_database().entries))
end

function DataStoreEngine:command_info(args)
    if #args ~= 0 then return wrong_arity("info") end
    local text = string.format("# Server\r\nin_memory_data_store_version:0.1.0\r\nactive_db:%d\r\ndbsize:%d\r\n", self.store.active_db, count_keys(self.store:active_database().entries))
    return bulk(text)
end

M.EntryType = EntryType
M.Entry = Entry
M.SortedSet = SortedSet
M.Database = Database
M.Store = Store
M.DataStoreEngine = DataStoreEngine
M.new = DataStoreEngine.new

return M
