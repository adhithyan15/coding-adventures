local M = {}

M.VERSION = "0.1.0"

M.Types = {
    U8 = "u8",
    U16 = "u16",
    U32 = "u32",
    U64 = "u64",
    Bool = "bool",
    Str = "str",
    Nil = "nil",
    Void = "void",
    Any = "any",
    Polymorphic = "polymorphic",
}

function M.Types.is_ref(type_name)
    return type(type_name) == "string" and type_name:sub(1, 4) == "ref<" and type_name:sub(-1) == ">"
end

function M.Types.unwrap_ref(type_name)
    if M.Types.is_ref(type_name) then
        return type_name:sub(5, -2)
    end
    return type_name
end

function M.Types.ref(type_name)
    return "ref<" .. tostring(type_name) .. ">"
end

function M.Types.is_concrete(type_name)
    return type_name ~= nil and type_name ~= M.Types.Any and type_name ~= M.Types.Polymorphic
end

M.Opcodes = {
    Arithmetic = { "add", "sub", "mul", "div", "mod", "neg" },
    Bitwise = { "and", "or", "xor", "not", "shl", "shr" },
    Cmp = { "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge" },
    Branch = { "jmp", "jmp_if_true", "jmp_if_false" },
    Control = { "label", "ret", "ret_void" },
    Memory = { "load_reg", "store_reg", "load_mem", "store_mem" },
    Call = { "call", "call_builtin" },
    Io = { "io_in", "io_out" },
    Coercion = { "cast", "type_assert" },
    Heap = { "alloc", "box", "unbox", "field_load", "field_store", "is_null", "safepoint" },
}

local function array_contains(items, value)
    for _, item in ipairs(items) do
        if item == value then
            return true
        end
    end
    return false
end

M.ValueOpcodes = {
    "add", "sub", "mul", "div", "mod", "neg",
    "and", "or", "xor", "not", "shl", "shr",
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
    "const", "load_reg", "load_mem", "call", "call_builtin", "io_in",
    "cast", "alloc", "box", "unbox", "field_load", "is_null",
    "tetrad.move", "move",
}

M.SideEffectOpcodes = {
    "jmp", "jmp_if_true", "jmp_if_false", "label", "ret", "ret_void",
    "store_reg", "store_mem", "io_out", "type_assert", "field_store", "safepoint",
}

M.SlotKind = {
    Uninitialized = "uninitialized",
    Monomorphic = "monomorphic",
    Polymorphic = "polymorphic",
    Megamorphic = "megamorphic",
}

local SlotState = {}
SlotState.__index = SlotState

function SlotState.new()
    return setmetatable({
        observations = {},
        order = {},
        kind = M.SlotKind.Uninitialized,
        count = 0,
    }, SlotState)
end

function SlotState:record(runtime_type)
    self.count = self.count + 1
    if self.observations[runtime_type] == nil then
        self.observations[runtime_type] = 0
        self.order[#self.order + 1] = runtime_type
    end
    self.observations[runtime_type] = self.observations[runtime_type] + 1
    local unique = #self.order
    if unique == 1 then
        self.kind = M.SlotKind.Monomorphic
    elseif unique <= 4 then
        self.kind = M.SlotKind.Polymorphic
    else
        self.kind = M.SlotKind.Megamorphic
    end
    return self
end

function SlotState:observed_types()
    local out = {}
    for i, value in ipairs(self.order) do
        out[i] = value
    end
    return out
end

function SlotState:is_monomorphic()
    return self.kind == M.SlotKind.Monomorphic
end

function SlotState:is_polymorphic()
    return self.kind == M.SlotKind.Polymorphic
end

M.SlotState = SlotState

local IirInstr = {}
IirInstr.__index = IirInstr

function IirInstr.new(options)
    options = options or {}
    return setmetatable({
        op = assert(options.op, "IirInstr requires op"),
        dest = options.dest,
        srcs = options.srcs or {},
        type_hint = options.type_hint or options.typeHint,
        observed_type = options.observed_type or options.observedType,
        observation_count = options.observation_count or options.observationCount or 0,
        observed_slot = options.observed_slot or options.observedSlot,
        deopt_anchor = options.deopt_anchor or options.deoptAnchor,
        may_alloc = options.may_alloc or options.mayAlloc or false,
    }, IirInstr)
end

function IirInstr.of(op, options)
    options = options or {}
    options.op = op
    return IirInstr.new(options)
end

function IirInstr:typed()
    return M.Types.is_concrete(self.type_hint)
end

function IirInstr:has_observation()
    return self.observed_type ~= nil or self.observation_count > 0 or self.observed_slot ~= nil
end

function IirInstr:polymorphic()
    return self.observed_slot ~= nil and self.observed_slot:is_polymorphic()
end

function IirInstr:effective_type()
    return self.type_hint or self.observed_type
end

function IirInstr:record_observation(runtime_type, slot)
    self.observed_type = runtime_type
    self.observation_count = self.observation_count + 1
    if slot ~= nil then
        self.observed_slot = slot:record(runtime_type)
    end
    return self
end

local function value_tostring(value)
    if type(value) == "string" then
        return string.format("%q", value)
    end
    if value == nil then
        return "nil"
    end
    return tostring(value)
end

function IirInstr:to_string()
    local args = {}
    for i, src in ipairs(self.srcs) do
        args[i] = value_tostring(src)
    end
    local dest = self.dest and (self.dest .. " = ") or ""
    local type_text = self:effective_type() and (" : " .. self:effective_type()) or ""
    return dest .. self.op .. "(" .. table.concat(args, ", ") .. ")" .. type_text
end

M.IirInstr = IirInstr
M.IIRInstr = IirInstr

M.FunctionTypeStatus = {
    FullyTyped = "fully_typed",
    PartiallyTyped = "partially_typed",
    Untyped = "untyped",
}

local IirFunction = {}
IirFunction.__index = IirFunction

function IirFunction.new(options)
    options = options or {}
    local self = setmetatable({
        name = assert(options.name, "IirFunction requires name"),
        params = options.params or {},
        return_type = options.return_type or options.returnType or M.Types.Any,
        instructions = options.instructions or {},
        register_count = options.register_count or options.registerCount or 0,
        type_status = options.type_status or options.typeStatus,
        call_count = options.call_count or options.callCount or 0,
        feedback_slots = options.feedback_slots or options.feedbackSlots or {},
        source_map = options.source_map or options.sourceMap or {},
    }, IirFunction)
    if self.type_status == nil then
        self.type_status = self:infer_type_status()
    end
    return self
end

function IirFunction:param_names()
    local out = {}
    for i, param in ipairs(self.params) do
        out[i] = param.name
    end
    return out
end

function IirFunction:param_types()
    local out = {}
    for i, param in ipairs(self.params) do
        out[i] = param.type
    end
    return out
end

function IirFunction:infer_type_status()
    local signature_typed = M.Types.is_concrete(self.return_type)
    for _, param in ipairs(self.params) do
        signature_typed = signature_typed and M.Types.is_concrete(param.type)
    end
    local values = 0
    local typed_values = 0
    for _, instr in ipairs(self.instructions) do
        if array_contains(M.ValueOpcodes, instr.op) then
            values = values + 1
            if instr:typed() then
                typed_values = typed_values + 1
            end
        end
    end
    if signature_typed and typed_values == values then
        return M.FunctionTypeStatus.FullyTyped
    end
    if signature_typed or typed_values > 0 then
        return M.FunctionTypeStatus.PartiallyTyped
    end
    return M.FunctionTypeStatus.Untyped
end

function IirFunction:label_index()
    local labels = {}
    for index, instr in ipairs(self.instructions) do
        if instr.op == "label" then
            local label = tostring(instr.srcs[1] or instr.dest or "")
            if #label > 0 then
                labels[label] = index
            end
        end
    end
    return labels
end

M.IirFunction = IirFunction
M.IIRFunction = IirFunction

local IirModule = {}
IirModule.__index = IirModule

function IirModule.new(options)
    options = options or {}
    return setmetatable({
        name = assert(options.name, "IirModule requires name"),
        functions = options.functions or {},
        entry_point = options.entry_point or options.entryPoint or "main",
        language = options.language or "unknown",
        metadata = options.metadata or {},
    }, IirModule)
end

function IirModule:get_function(name)
    for _, fn in ipairs(self.functions) do
        if fn.name == name then
            return fn
        end
    end
    return nil
end

function IirModule:function_names()
    local out = {}
    for i, fn in ipairs(self.functions) do
        out[i] = fn.name
    end
    return out
end

function IirModule:add_or_replace(fn)
    for i, existing in ipairs(self.functions) do
        if existing.name == fn.name then
            self.functions[i] = fn
            return
        end
    end
    self.functions[#self.functions + 1] = fn
end

function IirModule:validate()
    local seen = {}
    for _, fn in ipairs(self.functions) do
        if seen[fn.name] then
            error("duplicate function: " .. fn.name)
        end
        seen[fn.name] = true
    end
    if not seen[self.entry_point] then
        error("missing entry point: " .. self.entry_point)
    end
    for _, fn in ipairs(self.functions) do
        local labels = fn:label_index()
        for index, instr in ipairs(fn.instructions) do
            if instr.op == "jmp" or instr.op == "jmp_if_true" or instr.op == "jmp_if_false" then
                local label = instr.op == "jmp" and tostring(instr.srcs[1] or "") or tostring(instr.srcs[2] or "")
                if not labels[label] then
                    error(string.format("%s:%d branches to undefined label %s", fn.name, index, label))
                end
            end
        end
    end
end

M.IirModule = IirModule
M.IIRModule = IirModule

return M
