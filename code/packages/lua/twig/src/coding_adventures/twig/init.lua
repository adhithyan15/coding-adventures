local codegen = require("coding_adventures.codegen_core")
local ir = require("coding_adventures.interpreter_ir")
local jit = require("coding_adventures.jit_core")
local vm_core = require("coding_adventures.vm_core")

local M = {}
M.VERSION = "0.1.0"

local builtins = { ["+"] = true, ["-"] = true, ["*"] = true, ["/"] = true, ["="] = true, ["<"] = true, [">"] = true,
    cons = true, car = true, cdr = true, ["null?"] = true, ["pair?"] = true, ["number?"] = true, print = true }

local function symbol(name) return { kind = "symbol", name = name } end
local function is_symbol(value) return type(value) == "table" and value.kind == "symbol" end
local function symbol_name(value)
    if not is_symbol(value) then error("expected symbol") end
    return value.name
end
local function symbol_name_or_nil(value) return is_symbol(value) and value.name or nil end

function M.tokenize_twig(source)
    local tokens, i = {}, 1
    while i <= #source do
        local c = source:sub(i, i)
        if c:match("%s") then
            i = i + 1
        elseif c == ";" then
            while i <= #source and source:sub(i, i) ~= "\n" do i = i + 1 end
        elseif c == "(" or c == ")" then
            tokens[#tokens + 1] = c
            i = i + 1
        else
            local s = i
            while i <= #source and not source:sub(i, i):match("%s") and source:sub(i, i) ~= "(" and source:sub(i, i) ~= ")" do
                i = i + 1
            end
            tokens[#tokens + 1] = source:sub(s, i - 1)
        end
    end
    return tokens
end

local Parser = {}
Parser.__index = Parser

function Parser.new(tokens) return setmetatable({ tokens = tokens, p = 1 }, Parser) end
function Parser:peek() return self.tokens[self.p] end
function Parser:next()
    local t = self.tokens[self.p]
    if t == nil then error("unexpected end of Twig source") end
    self.p = self.p + 1
    return t
end
function Parser:forms()
    local out = {}
    while self.p <= #self.tokens do out[#out + 1] = self:expr() end
    return out
end
function Parser:expr()
    local t = self:next()
    if t == "(" then
        local list = {}
        while self:peek() ~= ")" do
            if self:peek() == nil then error("unterminated Twig list") end
            list[#list + 1] = self:expr()
        end
        self:next()
        return list
    elseif t == ")" then
        error("unexpected )")
    elseif t:match("^-?%d+$") then
        return tonumber(t)
    elseif t == "#t" then
        return true
    elseif t == "#f" then
        return false
    elseif t == "nil" then
        return nil
    end
    return symbol(t)
end

function M.parse_twig(source)
    return Parser.new(M.tokenize_twig(source)):forms()
end

local Ctx = {}
Ctx.__index = Ctx
function Ctx.new() return setmetatable({ instructions = {}, n = 0, labels = 0 }, Ctx) end
function Ctx:temp() local t = "t" .. tostring(self.n); self.n = self.n + 1; return t end
function Ctx:label(prefix) local l = prefix .. "_" .. tostring(self.labels); self.labels = self.labels + 1; return l end
function Ctx:emit(op, options) self.instructions[#self.instructions + 1] = ir.IirInstr.of(op, options or {}) end
function Ctx:register_count() return self.n > 64 and self.n or 64 end

local compile_expr

local function compile_begin(exprs, c, locals)
    local r = nil
    for _, e in ipairs(exprs) do r = compile_expr(e, c, locals) end
    if r ~= nil then return r end
    local d = c:temp()
    c:emit("const", { dest = d, srcs = {}, type_hint = ir.Types.Nil })
    return d
end

local function copy_set(values)
    local out = {}
    for k, v in pairs(values) do out[k] = v end
    return out
end

local function compile_if(expr, c, locals)
    local cond = compile_expr(expr[2], c, locals)
    local else_label, end_label, d = c:label("else"), c:label("endif"), c:temp()
    c:emit("jmp_if_false", { srcs = { cond, else_label } })
    c:emit("move", { dest = d, srcs = { compile_expr(expr[3], c, locals) }, type_hint = ir.Types.Any })
    c:emit("jmp", { srcs = { end_label } })
    c:emit("label", { srcs = { else_label } })
    c:emit("move", { dest = d, srcs = { compile_expr(expr[4], c, locals) }, type_hint = ir.Types.Any })
    c:emit("label", { srcs = { end_label } })
    return d
end

local function compile_let(expr, c, locals)
    local bindings = expr[2]
    if type(bindings) ~= "table" then error("let requires a binding list") end
    local next_locals = copy_set(locals)
    for _, b in ipairs(bindings) do
        if type(b) ~= "table" or #b ~= 2 then error("let binding must be a pair") end
        local name = symbol_name(b[1])
        c:emit("move", { dest = name, srcs = { compile_expr(b[2], c, next_locals) }, type_hint = ir.Types.Any })
        next_locals[name] = true
    end
    local rest = {}
    for i = 3, #expr do rest[#rest + 1] = expr[i] end
    return compile_begin(rest, c, next_locals)
end

compile_expr = function(expr, c, locals)
    if is_symbol(expr) then
        if locals[expr.name] then return expr.name end
        local d = c:temp()
        c:emit("call_builtin", { dest = d, srcs = { "global_get", expr.name }, type_hint = ir.Types.Any })
        return d
    elseif type(expr) == "number" or type(expr) == "boolean" or expr == nil then
        local d = c:temp()
        c:emit("const", { dest = d, srcs = expr == nil and {} or { expr }, type_hint = expr == nil and ir.Types.Nil or (type(expr) == "boolean" and ir.Types.Bool or ir.Types.U64) })
        return d
    elseif #expr == 0 then
        local d = c:temp()
        c:emit("const", { dest = d, srcs = {}, type_hint = ir.Types.Nil })
        return d
    end
    local head = expr[1]
    if is_symbol(head) and head.name == "if" then return compile_if(expr, c, locals) end
    if is_symbol(head) and head.name == "begin" then
        local rest = {}
        for i = 2, #expr do rest[#rest + 1] = expr[i] end
        return compile_begin(rest, c, locals)
    end
    if is_symbol(head) and head.name == "let" then return compile_let(expr, c, locals) end
    if not is_symbol(head) then error("Twig applications require a symbol in operator position") end
    local srcs = { head.name }
    for i = 2, #expr do srcs[#srcs + 1] = compile_expr(expr[i], c, locals) end
    local d = c:temp()
    c:emit(builtins[head.name] and "call_builtin" or "call", { dest = d, srcs = srcs, type_hint = ir.Types.Any })
    return d
end

local function is_fn_define(expr) return type(expr) == "table" and symbol_name_or_nil(expr[1]) == "define" and type(expr[2]) == "table" and not is_symbol(expr[2]) end
local function is_value_define(expr) return type(expr) == "table" and symbol_name_or_nil(expr[1]) == "define" and is_symbol(expr[2]) end

local function compile_fn(form)
    local sig = form[2]
    if type(sig) ~= "table" or #sig == 0 then error("function define requires a signature list") end
    local name = symbol_name(sig[1])
    local params, locals = {}, {}
    for i = 2, #sig do
        local p = symbol_name(sig[i])
        params[#params + 1] = { name = p, type = ir.Types.Any }
        locals[p] = true
    end
    local c, result = Ctx.new(), nil
    for i = 3, #form do result = compile_expr(form[i], c, locals) end
    if result == nil then
        result = c:temp()
        c:emit("const", { dest = result, srcs = {}, type_hint = ir.Types.Nil })
    end
    c:emit("ret", { srcs = { result } })
    return ir.IirFunction.new({ name = name, params = params, return_type = ir.Types.Any, instructions = c.instructions, register_count = c:register_count(), type_status = ir.FunctionTypeStatus.Untyped })
end

function M.compile_twig(source, module_name)
    module_name = module_name or "twig"
    local forms, functions, body = M.parse_twig(source), {}, {}
    local main = Ctx.new()
    for _, form in ipairs(forms) do
        if is_fn_define(form) then
            functions[#functions + 1] = compile_fn(form)
        elseif is_value_define(form) then
            main:emit("call_builtin", { srcs = { "global_set", symbol_name(form[2]), compile_expr(form[3], main, {}) }, type_hint = ir.Types.Any })
        else
            body[#body + 1] = form
        end
    end
    local last = nil
    for _, form in ipairs(body) do last = compile_expr(form, main, {}) end
    if last == nil then
        last = main:temp()
        main:emit("const", { dest = last, srcs = {}, type_hint = ir.Types.Nil })
    end
    main:emit("ret", { srcs = { last } })
    functions[#functions + 1] = ir.IirFunction.new({ name = "main", return_type = ir.Types.Any, instructions = main.instructions, register_count = main:register_count(), type_status = ir.FunctionTypeStatus.Untyped })
    local mod = ir.IirModule.new({ name = module_name, functions = functions, entry_point = "main", language = "twig" })
    mod:validate()
    return mod
end

local function to_number(value)
    if type(value) ~= "number" then error("expected number, got " .. M.format_twig_value(value)) end
    return value
end

local function is_pair(value) return type(value) == "table" and value[1] == "cons" and #value >= 1 end
local function as_pair(value)
    if not is_pair(value) then error("expected pair, got " .. M.format_twig_value(value)) end
    return value
end

function M.format_twig_value(value)
    if value == nil then return "nil" end
    if value == true then return "#t" end
    if value == false then return "#f" end
    if is_pair(value) then return "(" .. M.format_twig_value(value[2]) .. " . " .. M.format_twig_value(value[3]) .. ")" end
    return tostring(value)
end

function M.install_twig_builtins(vm, globals, write)
    vm:register_builtin("+", function(args) local s = 0; for _, v in ipairs(args) do s = s + to_number(v) end; return s end)
    vm:register_builtin("-", function(args) local r = to_number(args[1] or 0); if #args == 1 then return -r end; for i = 2, #args do r = r - to_number(args[i]) end; return r end)
    vm:register_builtin("*", function(args) local p = 1; for _, v in ipairs(args) do p = p * to_number(v) end; return p end)
    vm:register_builtin("/", function(args) local r = to_number(args[1] or 0); for i = 2, #args do r = math.floor(r / to_number(args[i])) end; return r end)
    vm:register_builtin("=", function(args) return args[1] == args[2] end)
    vm:register_builtin("<", function(args) return to_number(args[1] or 0) < to_number(args[2] or 0) end)
    vm:register_builtin(">", function(args) return to_number(args[1] or 0) > to_number(args[2] or 0) end)
    vm:register_builtin("cons", function(args) return { "cons", args[1], args[2] } end)
    vm:register_builtin("car", function(args) return as_pair(args[1])[2] end)
    vm:register_builtin("cdr", function(args) return as_pair(args[1])[3] end)
    vm:register_builtin("null?", function(args) return args[1] == nil end)
    vm:register_builtin("pair?", function(args) return is_pair(args[1]) end)
    vm:register_builtin("number?", function(args) return type(args[1]) == "number" end)
    vm:register_builtin("print", function(args) write(M.format_twig_value(args[1]) .. "\n"); return nil end)
    vm:register_builtin("global_get", function(args) local name = tostring(args[1]); if globals[name] == nil then error("undefined global: " .. name) end; return globals[name] end)
    vm:register_builtin("global_set", function(args) globals[tostring(args[1])] = args[2]; return args[2] end)
    vm:register_builtin("_move", function(args) return args[1] end)
end

function M.run_twig_detailed(source, use_jit)
    local mod, globals, stdout = M.compile_twig(source), {}, ""
    local vm = vm_core.VMCore.new()
    M.install_twig_builtins(vm, globals, function(text) stdout = stdout .. text end)
    local value = use_jit and jit.JITCore.new(vm):execute_with_jit(mod) or vm:execute(mod)
    return { stdout = stdout, value = value, module = mod, vm = vm }
end

function M.run_twig(source, use_jit)
    local result = M.run_twig_detailed(source, use_jit)
    return result.stdout, result.value
end

function M.emit_twig(source, target)
    return codegen.BackendRegistry.default():compile(M.compile_twig(source), target)
end

return M
