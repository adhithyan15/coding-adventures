local ir = require("coding_adventures.interpreter_ir")
local codegen = require("coding_adventures.codegen_core")
local vm_core = require("coding_adventures.vm_core")

local M = {}
M.VERSION = "0.1.0"

local PureVmBackend = {}
PureVmBackend.__index = PureVmBackend

function PureVmBackend.new()
    return setmetatable({}, PureVmBackend)
end

function PureVmBackend:compile_callable(fn, mod, source_vm)
    local builtins = vm_core.BuiltinRegistry.new(false)
    for _, entry in ipairs(source_vm.builtins:entries()) do
        builtins:register(entry[1], entry[2])
    end
    return function(args)
        local child = vm_core.VMCore.new({
            builtins = builtins,
            profiler_enabled = false,
            u8_wrap = fn.return_type == ir.Types.U8,
        })
        return child:execute(mod, fn.name, args or {})
    end
end

M.PureVmBackend = PureVmBackend

local JITCore = {}
JITCore.__index = JITCore

function JITCore.new(vm, backend, registry)
    return setmetatable({
        vm = vm,
        backend = backend or PureVmBackend.new(),
        registry = registry or codegen.BackendRegistry.default(),
    }, JITCore)
end

function JITCore:execute_with_jit(mod, fn, args)
    self:compile_ready_functions(mod)
    return self.vm:execute(mod, fn or mod.entry_point, args or {})
end

function JITCore:compile_ready_functions(mod)
    local compiled = {}
    for _, fn in ipairs(mod.functions) do
        if self:should_compile(fn) then
            self.vm:register_jit_handler(fn.name, self.backend:compile_callable(fn, mod, self.vm))
            compiled[#compiled + 1] = fn.name
        end
    end
    return compiled
end

function JITCore:emit(mod, target)
    return self.registry:compile(mod, target)
end

function JITCore:should_compile(fn)
    if fn.type_status == ir.FunctionTypeStatus.FullyTyped then return true end
    if fn.type_status == ir.FunctionTypeStatus.PartiallyTyped then return fn.call_count >= 10 end
    return fn.call_count >= 100
end

M.JITCore = JITCore

return M
