local M = {}

local NOT_HANDLED = {}

local function normalize_kind(kind)
    local normalized = {}
    local last_underscore = false

    for i = 1, #kind do
        local ch = kind:sub(i, i)
        if ch:match("[%w]") then
            table.insert(normalized, ch)
            last_underscore = false
        elseif not last_underscore then
            table.insert(normalized, "_")
            last_underscore = true
        end
    end

    local joined = table.concat(normalized)
    joined = joined:gsub("^_+", "")
    joined = joined:gsub("_+$", "")
    return joined
end

function M.new_type_error_diagnostic(message, line, column)
    return {
        message = message,
        line = line or 1,
        column = column or 1,
    }
end

function M.new_type_check_result(typed_ast, errors)
    local list = errors or {}
    return {
        typed_ast = typed_ast,
        errors = list,
        ok = #list == 0,
    }
end

local GenericTypeChecker = {}
GenericTypeChecker.__index = GenericTypeChecker

function M.new_generic_type_checker(node_kind, locate)
    return setmetatable({
        hooks = {},
        errors = {},
        node_kind = node_kind,
        locate = locate or function()
            return 1, 1
        end,
    }, GenericTypeChecker)
end

function GenericTypeChecker:reset()
    self.errors = {}
    return self
end

function GenericTypeChecker:register_hook(phase, kind, hook)
    local key_kind = kind == "*" and "*" or normalize_kind(kind)
    local key = phase .. ":" .. key_kind
    self.hooks[key] = self.hooks[key] or {}
    table.insert(self.hooks[key], hook)
    return self
end

function GenericTypeChecker:dispatch(phase, node, ...)
    local kind = ""
    if self.node_kind then
        kind = normalize_kind(self.node_kind(node) or "")
    end

    for _, key in ipairs({phase .. ":" .. kind, phase .. ":*"}) do
        local hooks = self.hooks[key] or {}
        for _, hook in ipairs(hooks) do
            local result = hook(node, ...)
            if result ~= NOT_HANDLED then
                return result
            end
        end
    end

    return nil
end

function GenericTypeChecker:not_handled()
    return NOT_HANDLED
end

function GenericTypeChecker:error(message, subject)
    local line, column = self.locate(subject)
    table.insert(self.errors, M.new_type_error_diagnostic(message, line, column))
    return nil
end

function GenericTypeChecker:check(ast)
    self:reset()
    if self.run then
        self:run(ast)
    end
    return M.new_type_check_result(ast, self.errors)
end

M.GenericTypeChecker = GenericTypeChecker

return M
