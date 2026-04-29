local ir = require("coding_adventures.interpreter_ir")
local jit = require("coding_adventures.jit_core")
local vm_core = require("coding_adventures.vm_core")

local M = {}

local function mutate_cell(instructions, op, amount)
    instructions[#instructions + 1] = ir.IirInstr.of("load_mem", { dest = "cell", srcs = { "ptr" }, type_hint = ir.Types.U8 })
    instructions[#instructions + 1] = ir.IirInstr.of(op, { dest = "cell", srcs = { "cell", amount }, type_hint = ir.Types.U8 })
    instructions[#instructions + 1] = ir.IirInstr.of("store_mem", { srcs = { "ptr", "cell" }, type_hint = ir.Types.U8 })
end

function M.compile_to_iir(source, module_name)
    module_name = module_name or "brainfuck"
    local instructions = { ir.IirInstr.of("const", { dest = "ptr", srcs = { 0 }, type_hint = ir.Types.U32 }) }
    local loops, loop_id = {}, 0
    for i = 1, #source do
        local ch = source:sub(i, i)
        if ch == ">" then instructions[#instructions + 1] = ir.IirInstr.of("add", { dest = "ptr", srcs = { "ptr", 1 }, type_hint = ir.Types.U32 })
        elseif ch == "<" then instructions[#instructions + 1] = ir.IirInstr.of("sub", { dest = "ptr", srcs = { "ptr", 1 }, type_hint = ir.Types.U32 })
        elseif ch == "+" then mutate_cell(instructions, "add", 1)
        elseif ch == "-" then mutate_cell(instructions, "sub", 1)
        elseif ch == "." then
            instructions[#instructions + 1] = ir.IirInstr.of("load_mem", { dest = "cell", srcs = { "ptr" }, type_hint = ir.Types.U8 })
            instructions[#instructions + 1] = ir.IirInstr.of("io_out", { srcs = { "cell" } })
        elseif ch == "," then
            instructions[#instructions + 1] = ir.IirInstr.of("io_in", { dest = "cell", type_hint = ir.Types.U8 })
            instructions[#instructions + 1] = ir.IirInstr.of("store_mem", { srcs = { "ptr", "cell" }, type_hint = ir.Types.U8 })
        elseif ch == "[" then
            local labels = { start = "loop_" .. loop_id .. "_start", ["end"] = "loop_" .. loop_id .. "_end" }
            loop_id = loop_id + 1
            loops[#loops + 1] = labels
            instructions[#instructions + 1] = ir.IirInstr.of("label", { srcs = { labels.start } })
            instructions[#instructions + 1] = ir.IirInstr.of("load_mem", { dest = "cell", srcs = { "ptr" }, type_hint = ir.Types.U8 })
            instructions[#instructions + 1] = ir.IirInstr.of("cmp_eq", { dest = "is_zero", srcs = { "cell", 0 }, type_hint = ir.Types.Bool })
            instructions[#instructions + 1] = ir.IirInstr.of("jmp_if_true", { srcs = { "is_zero", labels["end"] } })
        elseif ch == "]" then
            local labels = table.remove(loops)
            if labels == nil then error("Unmatched ']' -- no matching '[' found") end
            instructions[#instructions + 1] = ir.IirInstr.of("jmp", { srcs = { labels.start } })
            instructions[#instructions + 1] = ir.IirInstr.of("label", { srcs = { labels["end"] } })
        end
    end
    if #loops > 0 then error("Unmatched '[' -- " .. tostring(#loops) .. " unclosed bracket(s)") end
    instructions[#instructions + 1] = ir.IirInstr.of("ret_void")
    local mod = ir.IirModule.new({ name = module_name, functions = { ir.IirFunction.new({ name = "main", return_type = ir.Types.Void, instructions = instructions, register_count = 8, type_status = ir.FunctionTypeStatus.PartiallyTyped }) }, entry_point = "main", language = "brainfuck" })
    mod:validate()
    return mod
end

function M.execute_on_lang_vm(source, input, use_jit)
    local mod = M.compile_to_iir(source)
    local vm = vm_core.VMCore.new({ input = input or "", u8_wrap = true })
    if use_jit then jit.JITCore.new(vm):execute_with_jit(mod) else vm:execute(mod) end
    return { output = vm.output, memory = vm.memory, vm = vm, module = mod }
end

return M
