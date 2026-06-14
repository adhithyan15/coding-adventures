local M = {}
M.VERSION = "0.1.0"

local function value_tostring(value)
    if type(value) == "string" then return string.format("%q", value) end
    if value == nil then return "nil" end
    return tostring(value)
end

local function format_instr(instr)
    local args = {}
    for i, src in ipairs(instr.srcs or {}) do
        args[i] = value_tostring(src)
    end
    local dest = instr.dest and (instr.dest .. " = ") or ""
    local type_text = instr.type_hint and (" : " .. instr.type_hint) or ""
    return dest .. instr.op .. "(" .. table.concat(args, ", ") .. ")" .. type_text
end

local TextBackend = {}
TextBackend.__index = TextBackend

function TextBackend.new(target)
    return setmetatable({ target = target }, TextBackend)
end

function TextBackend:compile(mod)
    local lines = {
        "; LANG target=" .. self.target .. " module=" .. mod.name .. " language=" .. mod.language,
        ".entry " .. mod.entry_point,
    }
    for _, fn in ipairs(mod.functions) do
        local params = {}
        for i, param in ipairs(fn.params) do
            params[i] = param.name .. ":" .. param.type
        end
        lines[#lines + 1] = ""
        lines[#lines + 1] = ".function " .. fn.name .. (#params > 0 and (" " .. table.concat(params, " ")) or "") .. " -> " .. fn.return_type
        for index, instr in ipairs(fn.instructions) do
            lines[#lines + 1] = string.format("  %04d  %s", index - 1, format_instr(instr))
        end
        lines[#lines + 1] = ".end"
    end
    return {
        target = self.target,
        format = self.target .. "-lang-ir-text",
        body = table.concat(lines, "\n") .. "\n",
        metadata = { functions = mod:function_names(), entry_point = mod.entry_point },
    }
end

M.TextBackend = TextBackend

local BackendRegistry = {}
BackendRegistry.__index = BackendRegistry
BackendRegistry.default_targets = { "pure_vm", "jvm", "clr", "wasm" }

function BackendRegistry.new()
    return setmetatable({ backends = {}, order = {} }, BackendRegistry)
end

function BackendRegistry.default()
    local registry = BackendRegistry.new()
    for _, target in ipairs(BackendRegistry.default_targets) do
        registry:register(TextBackend.new(target))
    end
    return registry
end

function BackendRegistry:register(backend)
    if self.backends[backend.target] == nil then
        self.order[#self.order + 1] = backend.target
    end
    self.backends[backend.target] = backend
end

function BackendRegistry:fetch(target)
    local backend = self.backends[target]
    if backend == nil then error("unknown backend target: " .. tostring(target)) end
    return backend
end

function BackendRegistry:compile(mod, target)
    return self:fetch(target):compile(mod)
end

function BackendRegistry:targets()
    local out = {}
    for i, target in ipairs(self.order) do out[i] = target end
    return out
end

M.BackendRegistry = BackendRegistry

return M
