local codegen = require("coding_adventures.codegen_core")
local ir = require("coding_adventures.interpreter_ir")
local jit = require("coding_adventures.jit_core")
local vm_core = require("coding_adventures.vm_core")

local M = {}
M.VERSION = "0.1.0"

local keywords = { fn = true, ["let"] = true, ["return"] = true }
local ops = { ["+"] = "add", ["-"] = "sub", ["*"] = "mul", ["/"] = "div", ["%"] = "mod" }

function M.tokenize_tetrad(source)
    local tokens = {}
    local i = 1
    while i <= #source do
        local c = source:sub(i, i)
        if c:match("%s") then
            i = i + 1
        elseif c == "#" then
            while i <= #source and source:sub(i, i) ~= "\n" do i = i + 1 end
        elseif c == ":" and source:sub(i + 1, i + 1) == "=" then
            tokens[#tokens + 1] = { type = "symbol", value = ":=" }
            i = i + 2
        elseif ("+-*/%%(),{}=;"):find(c, 1, true) then
            tokens[#tokens + 1] = { type = "symbol", value = c }
            i = i + 1
        elseif c:match("%d") then
            local s = i
            while i <= #source and source:sub(i, i):match("%d") do i = i + 1 end
            tokens[#tokens + 1] = { type = "number", value = source:sub(s, i - 1) }
        elseif c:match("[%a_]") then
            local s = i
            while i <= #source and source:sub(i, i):match("[%w_]") do i = i + 1 end
            local value = source:sub(s, i - 1)
            tokens[#tokens + 1] = { type = keywords[value] and "keyword" or "name", value = value }
        else
            error("unexpected Tetrad character: " .. c)
        end
    end
    tokens[#tokens + 1] = { type = "eof", value = "" }
    return tokens
end

local Parser = {}
Parser.__index = Parser

function Parser.new(tokens)
    return setmetatable({ tokens = tokens, p = 1 }, Parser)
end

function Parser:peek(offset)
    offset = offset or 0
    return self.tokens[self.p + offset] or { type = "eof", value = "" }
end

function Parser:consume(type_name, value)
    local t = self:peek()
    if t.type ~= type_name or (value ~= nil and t.value ~= value) then
        error("expected " .. tostring(value or type_name) .. ", got " .. tostring(t.value))
    end
    self.p = self.p + 1
    return t
end

function Parser:match(type_name, value)
    local t = self:peek()
    if t.type == type_name and t.value == value then
        self.p = self.p + 1
        return true
    end
    return false
end

function Parser:semis()
    while self:match("symbol", ";") do end
end

function Parser:parse_program()
    local forms = {}
    while self:peek().type ~= "eof" do
        if self:peek().value == "fn" then forms[#forms + 1] = self:parse_function() else forms[#forms + 1] = self:parse_statement() end
        self:semis()
    end
    return { forms = forms }
end

function Parser:parse_function()
    self:consume("keyword", "fn")
    local name = self:consume("name").value
    self:consume("symbol", "(")
    local params = {}
    if not self:match("symbol", ")") then
        repeat params[#params + 1] = self:consume("name").value until not self:match("symbol", ",")
        self:consume("symbol", ")")
    end
    self:consume("symbol", "{")
    local body = {}
    while not self:match("symbol", "}") do
        body[#body + 1] = self:parse_statement()
        self:semis()
    end
    return { kind = "function", name = name, params = params, body = body }
end

function Parser:parse_statement()
    if self:match("keyword", "let") then
        local name = self:consume("name").value
        self:consume("symbol", "=")
        return { kind = "let", name = name, expr = self:expr() }
    end
    if self:match("keyword", "return") then
        return { kind = "return", expr = self:expr() }
    end
    if self:peek().type == "name" and (self:peek(1).value == "=" or self:peek(1).value == ":=") then
        local name = self:consume("name").value
        self.p = self.p + 1
        return { kind = "assign", name = name, expr = self:expr() }
    end
    return { kind = "expr", expr = self:expr() }
end

function Parser:expr() return self:add() end

function Parser:add()
    local e = self:mul()
    while self:peek().value == "+" or self:peek().value == "-" do
        local op = self:consume("symbol").value
        e = { kind = "binary", left = e, op = op, right = self:mul() }
    end
    return e
end

function Parser:mul()
    local e = self:primary()
    while self:peek().value == "*" or self:peek().value == "/" or self:peek().value == "%" do
        local op = self:consume("symbol").value
        e = { kind = "binary", left = e, op = op, right = self:primary() }
    end
    return e
end

function Parser:primary()
    if self:peek().type == "number" then
        return { kind = "number", value = tonumber(self:consume("number").value) }
    end
    if self:peek().type == "name" then
        local name = self:consume("name").value
        if self:match("symbol", "(") then
            local args = {}
            if not self:match("symbol", ")") then
                repeat args[#args + 1] = self:expr() until not self:match("symbol", ",")
                self:consume("symbol", ")")
            end
            return { kind = "call", name = name, args = args }
        end
        return { kind = "var", name = name }
    end
    if self:match("symbol", "(") then
        local e = self:expr()
        self:consume("symbol", ")")
        return e
    end
    error("expected expression, got " .. tostring(self:peek().value))
end

function M.parse_tetrad(source)
    return Parser.new(M.tokenize_tetrad(source)):parse_program()
end

local Ctx = {}
Ctx.__index = Ctx

function Ctx.new()
    return setmetatable({ instructions = {}, n = 0 }, Ctx)
end

function Ctx:temp()
    local name = "t" .. tostring(self.n)
    self.n = self.n + 1
    return name
end

function Ctx:emit(op, options)
    self.instructions[#self.instructions + 1] = ir.IirInstr.of(op, options or {})
end

function Ctx:register_count(params)
    local total = self.n + (params or 0)
    return total > 32 and total or 32
end

local compile_expr

local function compile_stmt(stmt, c)
    if stmt.kind == "let" or stmt.kind == "assign" then
        c:emit("tetrad.move", { dest = stmt.name, srcs = { compile_expr(stmt.expr, c) }, type_hint = ir.Types.U8 })
        return false
    elseif stmt.kind == "return" then
        c:emit("ret", { srcs = { compile_expr(stmt.expr, c) } })
        return true
    end
    compile_expr(stmt.expr, c)
    return false
end

compile_expr = function(expr, c)
    if expr.kind == "number" then
        local d = c:temp()
        c:emit("const", { dest = d, srcs = { expr.value % 256 }, type_hint = ir.Types.U8 })
        return d
    elseif expr.kind == "var" then
        return expr.name
    elseif expr.kind == "binary" then
        local d = c:temp()
        c:emit(ops[expr.op] or "add", { dest = d, srcs = { compile_expr(expr.left, c), compile_expr(expr.right, c) }, type_hint = ir.Types.U8 })
        return d
    end
    local d = c:temp()
    local srcs = { expr.name }
    for _, arg in ipairs(expr.args) do srcs[#srcs + 1] = compile_expr(arg, c) end
    c:emit("call", { dest = d, srcs = srcs, type_hint = ir.Types.U8 })
    return d
end

local function compile_function(def)
    local c = Ctx.new()
    local terminated = false
    for _, stmt in ipairs(def.body) do
        if not terminated then terminated = compile_stmt(stmt, c) end
    end
    if not terminated then c:emit("ret_void") end
    local params = {}
    for i, name in ipairs(def.params) do params[i] = { name = name, type = ir.Types.U8 } end
    return ir.IirFunction.new({
        name = def.name,
        params = params,
        return_type = terminated and ir.Types.U8 or ir.Types.Void,
        instructions = c.instructions,
        register_count = c:register_count(#params),
        type_status = ir.FunctionTypeStatus.FullyTyped,
    })
end

function M.compile_tetrad(source, module_name)
    module_name = module_name or "tetrad"
    local program = M.parse_tetrad(source)
    local functions, top = {}, {}
    for _, form in ipairs(program.forms) do
        if form.kind == "function" then functions[#functions + 1] = compile_function(form) else top[#top + 1] = form end
    end
    local has_main = false
    for _, fn in ipairs(functions) do if fn.name == "main" then has_main = true end end
    if not has_main then
        functions[#functions + 1] = compile_function({ kind = "function", name = "main", params = {}, body = top })
    end
    local mod = ir.IirModule.new({ name = module_name, functions = functions, entry_point = "main", language = "tetrad" })
    mod:validate()
    return mod
end

function M.run_tetrad(source, use_jit)
    local mod = M.compile_tetrad(source)
    local vm = vm_core.VMCore.new({ u8_wrap = true })
    if use_jit then return jit.JITCore.new(vm):execute_with_jit(mod) end
    return vm:execute(mod)
end

function M.emit_tetrad(source, target)
    return codegen.BackendRegistry.default():compile(M.compile_tetrad(source), target)
end

return M
