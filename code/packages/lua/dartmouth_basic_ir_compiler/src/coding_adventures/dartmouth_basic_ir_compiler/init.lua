local codegen = require("coding_adventures.codegen_core")
local ir = require("coding_adventures.interpreter_ir")
local jit = require("coding_adventures.jit_core")
local vm_core = require("coding_adventures.vm_core")

local M = {}
M.VERSION = "0.1.0"

local ops = { ["+"] = "add", ["-"] = "sub", ["*"] = "mul", ["/"] = "div" }
local cmps = { ["="] = "cmp_eq", ["<>"] = "cmp_ne", ["<"] = "cmp_lt", ["<="] = "cmp_le", [">"] = "cmp_gt", [">="] = "cmp_ge" }

local function trim(s) return (s:gsub("^%s+", ""):gsub("%s+$", "")) end
local function line_label(n) return "_line_" .. tostring(n) end

function M.parse_basic_lines(source)
    local lines = {}
    for raw in (source .. "\n"):gmatch("([^\n]*)\n") do
        local line = trim(raw:gsub("\r$", ""))
        if line ~= "" then
            local number, text = line:match("^(%d+)%s*(.*)$")
            if not number then error("missing BASIC line number: " .. line) end
            lines[#lines + 1] = { number = tonumber(number), text = trim(text or "") }
        end
    end
    table.sort(lines, function(a, b) return a.number < b.number end)
    return lines
end

local function validate_var(name)
    local upper = string.upper(name)
    if not upper:match("^[A-Z][A-Z0-9]?$") then error("invalid BASIC variable: " .. name) end
    return upper
end

function M.tokenize_basic_expr(source)
    local tokens, i = {}, 1
    while i <= #source do
        local c = source:sub(i, i)
        if c:match("%s") then
            i = i + 1
        elseif c:match("%d") then
            local s = i
            while i <= #source and source:sub(i, i):match("%d") do i = i + 1 end
            tokens[#tokens + 1] = source:sub(s, i - 1)
        elseif c:match("%a") then
            local s = i
            while i <= #source and source:sub(i, i):match("%w") do i = i + 1 end
            tokens[#tokens + 1] = validate_var(source:sub(s, i - 1))
        elseif ("()+-*/"):find(c, 1, true) then
            tokens[#tokens + 1] = c
            i = i + 1
        else
            error("unexpected BASIC expression character: " .. c)
        end
    end
    return tokens
end

local ExprParser = {}
ExprParser.__index = ExprParser
function ExprParser.new(tokens) return setmetatable({ tokens = tokens, p = 1 }, ExprParser) end
function ExprParser:peek() return self.tokens[self.p] end
function ExprParser:next() local t = self.tokens[self.p]; if t == nil then error("unexpected end of BASIC expression") end; self.p = self.p + 1; return t end
function ExprParser:expect(t) if self:next() ~= t then error("expected " .. t) end end
function ExprParser:parse() local e = self:add(); if self:peek() ~= nil then error("unexpected expression token: " .. self:peek()) end; return e end
function ExprParser:add() local e = self:mul(); while self:peek() == "+" or self:peek() == "-" do local op = self:next(); e = { kind = "binary", op = op, left = e, right = self:mul() } end; return e end
function ExprParser:mul() local e = self:primary(); while self:peek() == "*" or self:peek() == "/" do local op = self:next(); e = { kind = "binary", op = op, left = e, right = self:primary() } end; return e end
function ExprParser:primary()
    if self:peek() == "-" then self:next(); return { kind = "binary", op = "-", left = { kind = "number", value = 0 }, right = self:primary() } end
    if self:peek() == "(" then local e; self:next(); e = self:add(); self:expect(")"); return e end
    local t = self:next()
    return t:match("^%d+$") and { kind = "number", value = tonumber(t) } or { kind = "var", name = validate_var(t) }
end

local function parse_basic_expr(source) return ExprParser.new(M.tokenize_basic_expr(source)):parse() end

local Ctx = {}
Ctx.__index = Ctx
function Ctx.new() return setmetatable({ instructions = {}, var_names = {}, for_loops = {}, n = 0 }, Ctx) end
function Ctx:temp() local t = "t" .. tostring(self.n); self.n = self.n + 1; return t end
function Ctx:var_register(name) local v = validate_var(name); self.var_names[v] = true; return "v_" .. v end
function Ctx:emit(op, options) self.instructions[#self.instructions + 1] = ir.IirInstr.of(op, options or {}) end
function Ctx:loop_depth() return #self.for_loops end
function Ctx:register_count() local count = 0; for _ in pairs(self.var_names) do count = count + 1 end; local total = self.n + count; return total > 64 and total or 64 end

local compile_expr
compile_expr = function(e, c)
    if e.kind == "number" then local d = c:temp(); c:emit("const", { dest = d, srcs = { e.value }, type_hint = ir.Types.U64 }); return d end
    if e.kind == "var" then return c:var_register(e.name) end
    local d = c:temp()
    c:emit(ops[e.op] or "add", { dest = d, srcs = { compile_expr(e.left, c), compile_expr(e.right, c) }, type_hint = ir.Types.U64 })
    return d
end

local function split_condition(condition)
    for _, op in ipairs({ "<=", ">=", "<>", "=", "<", ">" }) do
        local s, e = condition:find(op, 1, true)
        if s then return trim(condition:sub(1, s - 1)), op, trim(condition:sub(e + 1)) end
    end
    error("missing comparison operator: " .. condition)
end

local compile_line

local function compile_print(rest, c)
    if rest == "" then
        local d = c:temp(); c:emit("const", { dest = d, srcs = { "" }, type_hint = ir.Types.Str }); c:emit("call_builtin", { srcs = { "__basic_print", d }, type_hint = ir.Types.Nil }); return
    end
    if rest:sub(1, 1) == '"' and rest:sub(-1) == '"' and #rest >= 2 then
        local d = c:temp(); c:emit("const", { dest = d, srcs = { rest:sub(2, -2) }, type_hint = ir.Types.Str }); c:emit("call_builtin", { srcs = { "__basic_print", d }, type_hint = ir.Types.Nil }); return
    end
    c:emit("call_builtin", { srcs = { "__basic_print", compile_expr(parse_basic_expr(rest), c) }, type_hint = ir.Types.Nil })
end

local function compile_if(rest, c)
    local upper = string.upper(rest)
    local then_pos = upper:find("THEN", 1, true)
    if not then_pos then error("IF requires THEN") end
    local left, op, right = split_condition(trim(rest:sub(1, then_pos - 1)))
    local target = tonumber(trim(rest:sub(then_pos + 4)))
    local d = c:temp()
    c:emit(cmps[op] or "cmp_eq", { dest = d, srcs = { compile_expr(parse_basic_expr(left), c), compile_expr(parse_basic_expr(right), c) }, type_hint = ir.Types.Bool })
    c:emit("jmp_if_true", { srcs = { d, line_label(target) } })
end

local function compile_assignment(text, c)
    local body = text
    if string.upper(body):sub(1, 4) == "LET " then body = trim(body:sub(5)) end
    local eq = body:find("=", 1, true)
    if not eq then error("expected assignment: " .. text) end
    c:emit("move", { dest = c:var_register(trim(body:sub(1, eq - 1))), srcs = { compile_expr(parse_basic_expr(trim(body:sub(eq + 1))), c) }, type_hint = ir.Types.U64 })
end

local function compile_for(line_number, rest, c)
    local eq = rest:find("=", 1, true); if not eq then error("FOR requires =") end
    local variable = validate_var(trim(rest:sub(1, eq - 1)))
    local after_eq = trim(rest:sub(eq + 1))
    local to = string.upper(after_eq):find(" TO ", 1, true); if not to then error("FOR requires TO") end
    local start_text = trim(after_eq:sub(1, to - 1))
    local after_to = trim(after_eq:sub(to + 4))
    local step = string.upper(after_to):find(" STEP ", 1, true)
    local limit_text = step and trim(after_to:sub(1, step - 1)) or after_to
    local step_text = step and trim(after_to:sub(step + 6)) or "1"
    local var_reg = c:var_register(variable)
    c:emit("move", { dest = var_reg, srcs = { compile_expr(parse_basic_expr(start_text), c) }, type_hint = ir.Types.U64 })
    local label = "for_" .. tostring(line_number) .. "_" .. tostring(c:loop_depth())
    c:emit("label", { srcs = { label } })
    c.for_loops[#c.for_loops + 1] = { variable = variable, label = label, limit = compile_expr(parse_basic_expr(limit_text), c), step = compile_expr(parse_basic_expr(step_text), c), descending = step_text:match("^%s*-") ~= nil }
end

local function compile_next(rest, c)
    local expected = trim(rest) == "" and nil or validate_var(trim(rest))
    local loop = table.remove(c.for_loops)
    if not loop then error("NEXT without FOR") end
    if expected ~= nil and expected ~= loop.variable then error("NEXT " .. expected .. " does not match FOR " .. loop.variable) end
    local reg = c:var_register(loop.variable)
    c:emit("add", { dest = reg, srcs = { reg, loop.step }, type_hint = ir.Types.U64 })
    local keep = c:temp()
    c:emit(loop.descending and "cmp_ge" or "cmp_le", { dest = keep, srcs = { reg, loop.limit }, type_hint = ir.Types.Bool })
    c:emit("jmp_if_true", { srcs = { keep, loop.label } })
end

compile_line = function(line, c)
    local text, upper = trim(line.text), string.upper(trim(line.text))
    if text == "" or upper:sub(1, 3) == "REM" then return end
    if upper == "END" or upper == "STOP" then c:emit("ret_void"); return end
    if upper:sub(1, 5) == "PRINT" then compile_print(trim(text:sub(6)), c); return end
    if upper:sub(1, 4) == "GOTO" then c:emit("jmp", { srcs = { line_label(tonumber(trim(text:sub(5)))) } }); return end
    if upper:sub(1, 2) == "IF" then compile_if(trim(text:sub(3)), c); return end
    if upper:sub(1, 3) == "FOR" then compile_for(line.number, trim(text:sub(4)), c); return end
    if upper:sub(1, 4) == "NEXT" then compile_next(trim(text:sub(5)), c); return end
    compile_assignment(text, c)
end

function M.compile_dartmouth_basic(source, module_name)
    module_name = module_name or "dartmouth-basic"
    local c = Ctx.new()
    for _, line in ipairs(M.parse_basic_lines(source)) do
        c:emit("label", { srcs = { line_label(line.number) } })
        compile_line(line, c)
    end
    c:emit("ret_void")
    local names = {}
    for name in pairs(c.var_names) do names[#names + 1] = name end
    table.sort(names)
    local mod = ir.IirModule.new({ name = module_name, functions = { ir.IirFunction.new({ name = "main", return_type = ir.Types.Void, instructions = c.instructions, register_count = c:register_count(), type_status = ir.FunctionTypeStatus.PartiallyTyped }) }, entry_point = "main", language = "dartmouth-basic" })
    mod:validate()
    return { module = mod, var_names = names }
end

function M.run_dartmouth_basic(source, use_jit)
    local compiled, output = M.compile_dartmouth_basic(source), ""
    local vm = vm_core.VMCore.new()
    vm:register_builtin("__basic_print", function(args) output = output .. tostring(args[1] or "") .. "\n"; return nil end)
    if use_jit then jit.JITCore.new(vm):execute_with_jit(compiled.module) else vm:execute(compiled.module) end
    return output
end

function M.emit_dartmouth_basic(source, target)
    return codegen.BackendRegistry.default():compile(M.compile_dartmouth_basic(source).module, target)
end

return M
