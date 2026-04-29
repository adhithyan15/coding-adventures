local ir = require("coding_adventures.interpreter_ir")

local M = {}
M.VERSION = "0.1.0"

local BuiltinRegistry = {}
BuiltinRegistry.__index = BuiltinRegistry

function BuiltinRegistry.new(register_defaults)
    if register_defaults == nil then register_defaults = true end
    local self = setmetatable({ handlers = {}, order = {} }, BuiltinRegistry)
    if register_defaults then
        self:register("noop", function() return nil end)
        self:register("assert_eq", function(args)
            if args[1] ~= args[2] then
                error("assert_eq failed: " .. tostring(args[1]) .. " != " .. tostring(args[2]))
            end
            return nil
        end)
    end
    return self
end

function BuiltinRegistry:register(name, handler)
    if self.handlers[name] == nil then
        self.order[#self.order + 1] = name
    end
    self.handlers[name] = handler
end

function BuiltinRegistry:call(name, args)
    local handler = self.handlers[name]
    if handler == nil then
        error("unknown builtin: " .. tostring(name))
    end
    return handler(args or {})
end

function BuiltinRegistry:names()
    local out = {}
    for i, name in ipairs(self.order) do out[i] = name end
    return out
end

function BuiltinRegistry:entries()
    local out = {}
    for _, name in ipairs(self.order) do
        out[#out + 1] = { name, self.handlers[name] }
    end
    return out
end

M.BuiltinRegistry = BuiltinRegistry

local function boxed(value)
    return { value = value }
end

local VMFrame = {}
VMFrame.__index = VMFrame

function VMFrame.new(fn, args)
    local self = setmetatable({ fn = fn, ip = 1, registers = {}, slots = {} }, VMFrame)
    args = args or {}
    for index, param in ipairs(fn.params) do
        local value = args[index]
        self.registers[param.name] = boxed(value)
        self.slots[param.name] = boxed(value)
    end
    return self
end

function VMFrame:resolve(value)
    if value == nil then return nil end
    if type(value) == "string" then
        local entry = self.registers[value]
        if entry ~= nil then
            return entry.value
        end
    end
    if type(value) == "table" then
        local out = {}
        for i, entry in ipairs(value) do
            out[i] = self:resolve(entry)
        end
        return out
    end
    return value
end

function VMFrame:write(name, value)
    if name ~= nil then
        self.registers[name] = boxed(value)
    end
end

function VMFrame:load_slot(name)
    local entry = self.slots[name]
    return entry and entry.value or nil
end

function VMFrame:store_slot(name, value)
    self.slots[name] = boxed(value)
end

M.VMFrame = VMFrame

local BranchStats = {}
BranchStats.__index = BranchStats

function BranchStats.new()
    return setmetatable({ taken_count = 0, not_taken_count = 0 }, BranchStats)
end

function BranchStats:record(taken)
    if taken then self.taken_count = self.taken_count + 1 else self.not_taken_count = self.not_taken_count + 1 end
end

M.BranchStats = BranchStats

local function new_metrics()
    return {
        function_call_counts = {},
        total_instructions_executed = 0,
        total_frames_pushed = 0,
        total_jit_hits = 0,
        branch_stats = {},
        loop_back_edge_counts = {},
    }
end

local VMCore = {}
VMCore.__index = VMCore

function VMCore.new(options)
    options = options or {}
    local input = options.input or ""
    local input_buffer = {}
    for i = 1, #input do
        input_buffer[#input_buffer + 1] = input:byte(i)
    end
    return setmetatable({
        builtins = options.builtins or BuiltinRegistry.new(true),
        memory = {},
        io_ports = {},
        output = "",
        max_frames = options.max_frames or options.maxFrames or 64,
        profiler_enabled = options.profiler_enabled ~= false and options.profilerEnabled ~= false,
        u8_wrap = options.u8_wrap or options.u8Wrap or false,
        frames = {},
        jit_handlers = {},
        metric_data = new_metrics(),
        coverage = {},
        coverage_enabled = false,
        module = nil,
        interrupted = false,
        input_buffer = input_buffer,
        trace = nil,
    }, VMCore)
end

function VMCore:execute(mod, fn, args)
    fn = fn or mod.entry_point
    args = args or {}
    mod:validate()
    self.module = mod
    self.interrupted = false
    return self:invoke_function(fn, args)
end

function VMCore:execute_traced(mod, fn, args)
    self.trace = {}
    local result = self:execute(mod, fn, args)
    local trace = self.trace
    self.trace = nil
    return { result = result, trace = trace }
end

function VMCore:metrics() return self.metric_data end
function VMCore:register_builtin(name, handler) self.builtins:register(name, handler) end
function VMCore:register_jit_handler(name, handler) self.jit_handlers[name] = handler end
function VMCore:unregister_jit_handler(name) self.jit_handlers[name] = nil end
function VMCore:enable_coverage() self.coverage_enabled = true end
function VMCore:disable_coverage() self.coverage_enabled = false end
function VMCore:interrupt() self.interrupted = true end

function VMCore:hot_functions(min_calls)
    min_calls = min_calls or 1
    local out = {}
    for name, calls in pairs(self.metric_data.function_call_counts) do
        if calls >= min_calls then out[#out + 1] = name end
    end
    table.sort(out, function(a, b) return self.metric_data.function_call_counts[a] > self.metric_data.function_call_counts[b] end)
    return out
end

function VMCore:invoke_function(name, args)
    if self.module == nil then error("no module loaded") end
    local fn = self.module:get_function(name)
    if fn == nil then error("unknown function: " .. tostring(name)) end
    fn.call_count = fn.call_count + 1
    if self.profiler_enabled then
        self.metric_data.function_call_counts[name] = (self.metric_data.function_call_counts[name] or 0) + 1
    end
    local jit = self.jit_handlers[name]
    if jit ~= nil then
        self.metric_data.total_jit_hits = self.metric_data.total_jit_hits + 1
        return jit(args or {})
    end
    if #self.frames >= self.max_frames then
        error("maximum frame depth exceeded: " .. tostring(self.max_frames))
    end
    local frame = VMFrame.new(fn, args)
    self.frames[#self.frames + 1] = frame
    self.metric_data.total_frames_pushed = self.metric_data.total_frames_pushed + 1
    local result = self:run_frame(frame)
    self.frames[#self.frames] = nil
    return result
end

function VMCore:run_frame(frame)
    local labels = frame.fn:label_index()
    while frame.ip <= #frame.fn.instructions do
        if self.interrupted then error("VM interrupted") end
        local instr = frame.fn.instructions[frame.ip]
        self:record_instruction(frame.fn.name, frame.ip, instr)
        local result = self:dispatch(frame, instr, labels)
        if result.kind == "return" then
            return result.value
        elseif result.kind == "jump" then
            frame.ip = result.ip
        else
            frame.ip = frame.ip + 1
        end
    end
    return nil
end

local function bit_op(a, b, op)
    a = math.floor(a) % 4294967296
    b = math.floor(b) % 4294967296
    local result, bit = 0, 1
    while a > 0 or b > 0 do
        local abit, bbit = a % 2, b % 2
        local out = 0
        if op == "and" then out = (abit == 1 and bbit == 1) and 1 or 0
        elseif op == "or" then out = (abit == 1 or bbit == 1) and 1 or 0
        elseif op == "xor" then out = (abit ~= bbit) and 1 or 0
        end
        result = result + out * bit
        a = math.floor(a / 2)
        b = math.floor(b / 2)
        bit = bit * 2
    end
    return result
end

function VMCore:dispatch(frame, instr, labels)
    local op = instr.op
    if op == "const" or op == "move" or op == "tetrad.move" then
        self:write_observed(frame, instr, frame:resolve(instr.srcs[1]))
        return { kind = "next" }
    elseif op == "add" or op == "sub" or op == "mul" or op == "div" or op == "mod" or
        op == "and" or op == "or" or op == "xor" or op == "shl" or op == "shr" or
        op == "cmp_eq" or op == "cmp_ne" or op == "cmp_lt" or op == "cmp_le" or op == "cmp_gt" or op == "cmp_ge" then
        self:write_observed(frame, instr, self:binary_op(op, frame:resolve(instr.srcs[1]), frame:resolve(instr.srcs[2])))
        return { kind = "next" }
    elseif op == "neg" then
        self:write_observed(frame, instr, -self:to_number(frame:resolve(instr.srcs[1])))
        return { kind = "next" }
    elseif op == "not" then
        self:write_observed(frame, instr, 4294967295 - (self:to_number(frame:resolve(instr.srcs[1])) % 4294967296))
        return { kind = "next" }
    elseif op == "cast" then
        self:write_observed(frame, instr, self:cast(frame:resolve(instr.srcs[1]), instr.type_hint or tostring(instr.srcs[2] or ir.Types.Any)))
        return { kind = "next" }
    elseif op == "type_assert" then
        self:assert_type(frame:resolve(instr.srcs[1]), instr.type_hint or tostring(instr.srcs[2] or ir.Types.Any))
        return { kind = "next" }
    elseif op == "label" then
        return { kind = "next" }
    elseif op == "jmp" then
        return { kind = "jump", ip = self:jump_target(frame, labels, tostring(instr.srcs[1] or "")) }
    elseif op == "jmp_if_true" then
        local taken = self:truthy(frame:resolve(instr.srcs[1]))
        self:record_branch(frame.fn.name, frame.ip, taken)
        if taken then return { kind = "jump", ip = self:jump_target(frame, labels, tostring(instr.srcs[2] or "")) } end
        return { kind = "next" }
    elseif op == "jmp_if_false" then
        local taken = not self:truthy(frame:resolve(instr.srcs[1]))
        self:record_branch(frame.fn.name, frame.ip, taken)
        if taken then return { kind = "jump", ip = self:jump_target(frame, labels, tostring(instr.srcs[2] or "")) } end
        return { kind = "next" }
    elseif op == "ret" then
        return { kind = "return", value = frame:resolve(instr.srcs[1]) }
    elseif op == "ret_void" then
        return { kind = "return", value = nil }
    elseif op == "call" then
        local args = {}
        for i = 2, #instr.srcs do args[#args + 1] = frame:resolve(instr.srcs[i]) end
        self:write_observed(frame, instr, self:invoke_function(tostring(instr.srcs[1] or ""), args))
        return { kind = "next" }
    elseif op == "call_builtin" then
        local args = {}
        for i = 2, #instr.srcs do args[#args + 1] = frame:resolve(instr.srcs[i]) end
        self:write_observed(frame, instr, self.builtins:call(tostring(instr.srcs[1] or ""), args))
        return { kind = "next" }
    elseif op == "load_reg" then
        self:write_observed(frame, instr, frame:resolve(instr.srcs[1]))
        return { kind = "next" }
    elseif op == "store_reg" then
        local target = instr.dest or tostring(instr.srcs[1] or "")
        local value = instr.dest and frame:resolve(instr.srcs[1]) or frame:resolve(instr.srcs[2])
        frame:write(target, value)
        return { kind = "next" }
    elseif op == "load_mem" then
        self:write_observed(frame, instr, self.memory[self:to_number(frame:resolve(instr.srcs[1]))] or 0)
        return { kind = "next" }
    elseif op == "store_mem" then
        self.memory[self:to_number(frame:resolve(instr.srcs[1]))] = self:wrap_value(frame:resolve(instr.srcs[2]), instr.type_hint)
        return { kind = "next" }
    elseif op == "io_in" then
        local value = table.remove(self.input_buffer, 1) or 0
        self:write_observed(frame, instr, value)
        return { kind = "next" }
    elseif op == "io_out" then
        local value = frame:resolve(instr.srcs[1])
        if type(value) == "string" then
            self.output = self.output .. value
        else
            self.output = self.output .. string.char(self:to_number(value) % 256)
        end
        return { kind = "next" }
    elseif op == "is_null" then
        self:write_observed(frame, instr, frame:resolve(instr.srcs[1]) == nil)
        return { kind = "next" }
    elseif op == "safepoint" then
        return { kind = "next" }
    end
    error("unknown opcode: " .. tostring(op))
end

function VMCore:record_instruction(function_name, ip, instr)
    self.metric_data.total_instructions_executed = self.metric_data.total_instructions_executed + 1
    if self.coverage_enabled then
        self.coverage[function_name] = self.coverage[function_name] or {}
        self.coverage[function_name][ip] = true
    end
    if self.trace ~= nil then
        self.trace[#self.trace + 1] = { function_name = function_name, ip = ip, instruction = instr:to_string() }
    end
end

function VMCore:record_branch(function_name, ip, taken)
    local key = function_name .. ":" .. tostring(ip)
    local stats = self.metric_data.branch_stats[key] or BranchStats.new()
    stats:record(taken)
    self.metric_data.branch_stats[key] = stats
end

function VMCore:jump_target(frame, labels, label)
    local target = labels[label]
    if target == nil then error(frame.fn.name .. " branches to undefined label " .. label) end
    if target < frame.ip then
        local key = frame.fn.name .. ":" .. label
        self.metric_data.loop_back_edge_counts[key] = (self.metric_data.loop_back_edge_counts[key] or 0) + 1
    end
    return target
end

function VMCore:write_observed(frame, instr, value)
    local wrapped = self:wrap_value(value, instr.type_hint)
    frame:write(instr.dest, wrapped)
    instr:record_observation(instr.type_hint or self:runtime_type(wrapped))
end

function VMCore:binary_op(op, left, right)
    if op == "add" then return self:to_number(left) + self:to_number(right) end
    if op == "sub" then return self:to_number(left) - self:to_number(right) end
    if op == "mul" then return self:to_number(left) * self:to_number(right) end
    if op == "div" then return math.floor(self:to_number(left) / self:to_number(right)) end
    if op == "mod" then return self:to_number(left) % self:to_number(right) end
    if op == "and" or op == "or" or op == "xor" then return bit_op(self:to_number(left), self:to_number(right), op) end
    if op == "shl" then return (self:to_number(left) * (2 ^ self:to_number(right))) % 4294967296 end
    if op == "shr" then return math.floor((self:to_number(left) % 4294967296) / (2 ^ self:to_number(right))) end
    if op == "cmp_eq" then return left == right end
    if op == "cmp_ne" then return left ~= right end
    if op == "cmp_lt" then return self:to_number(left) < self:to_number(right) end
    if op == "cmp_le" then return self:to_number(left) <= self:to_number(right) end
    if op == "cmp_gt" then return self:to_number(left) > self:to_number(right) end
    if op == "cmp_ge" then return self:to_number(left) >= self:to_number(right) end
    error("unknown opcode: " .. tostring(op))
end

function VMCore:cast(value, type_name)
    if type_name == ir.Types.U8 then return self:to_number(value) % 256 end
    if type_name == ir.Types.U16 then return self:to_number(value) % 65536 end
    if type_name == ir.Types.U32 then return self:to_number(value) % 4294967296 end
    if type_name == ir.Types.Bool then return self:truthy(value) end
    if type_name == ir.Types.Str then return tostring(value) end
    if type_name == ir.Types.Nil then return nil end
    return value
end

function VMCore:assert_type(value, type_name)
    if type_name ~= ir.Types.Any and self:runtime_type(value) ~= type_name then
        error("type assertion failed: expected " .. tostring(type_name) .. ", got " .. self:runtime_type(value))
    end
end

function VMCore:runtime_type(value)
    if value == nil then return ir.Types.Nil end
    if type(value) == "boolean" then return ir.Types.Bool end
    if type(value) == "string" then return ir.Types.Str end
    if type(value) == "number" then
        if value >= 0 and value <= 255 and math.floor(value) == value then return ir.Types.U8 end
        return ir.Types.U64
    end
    return ir.Types.Any
end

function VMCore:wrap_value(value, type_hint)
    if self.u8_wrap and type_hint == ir.Types.U8 and type(value) == "number" then
        return value % 256
    end
    return value
end

function VMCore:truthy(value)
    return not (value == nil or value == false or value == 0)
end

function VMCore:to_number(value)
    if type(value) == "number" then return value end
    if type(value) == "boolean" then return value and 1 or 0 end
    if type(value) == "string" then
        local parsed = tonumber(value)
        if parsed ~= nil then return parsed end
    end
    error("expected number, got " .. tostring(value))
end

M.VMCore = VMCore

return M
