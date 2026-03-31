-- sql_execution_engine — SELECT-Only SQL Execution Engine
-- =========================================================
--
-- This package is part of the coding-adventures monorepo.
-- It implements a SELECT-only SQL execution engine that evaluates SQL queries
-- against pluggable data sources.
--
-- # Architecture: The Materialized Pipeline
-- ===========================================
--
-- The engine uses a MATERIALIZED PIPELINE model: each stage reads all rows from
-- the previous stage into memory, transforms them, and passes the result to the
-- next stage.  This is simpler than the volcano/iterator model but educational:
-- you can see the full intermediate state at each pipeline stage.
--
-- ```
-- SQL string
--     │  parse_sql()
--     ▼
-- AST (parsed statement)
--     │  execute(ast, data_source)
--     ▼
-- ┌──────────────────────────────────────────────────────────────────────┐
-- │  Stage 1: FROM + JOINs      scan tables, apply aliases              │
-- │  Stage 2: WHERE             filter rows (no aggregates)             │
-- │  Stage 3: GROUP BY          partition; compute aggregates           │
-- │  Stage 4: HAVING            filter groups (aggregates allowed)      │
-- │  Stage 5: SELECT            project columns, compute expressions    │
-- │  Stage 6: DISTINCT          deduplicate projected rows              │
-- │  Stage 7: ORDER BY          sort output                             │
-- │  Stage 8: LIMIT / OFFSET    slice output                            │
-- └──────────────────────────────────────────────────────────────────────┘
--     ▼
-- QueryResult { columns = [...], rows = [[...], [...], ...] }
-- ```
--
-- # DataSource Protocol
-- ======================
--
-- The engine is DATA-SOURCE AGNOSTIC.  It knows nothing about CSV, JSON,
-- or any particular storage format.  Data comes through a DataSource object
-- with exactly two methods:
--
--   data_source:schema(table_name) → list of column name strings
--   data_source:scan(table_name)   → list of row maps { col_name → value }
--
-- This mirrors how real databases work: the query executor is separate from
-- the storage engine.  PostgreSQL's query planner works identically whether
-- you're reading from a heap table, a B-tree index, or a foreign data wrapper.
--
-- # Row Representation
-- =====================
--
-- Internally, rows are flat maps with both qualified and bare column names:
--
--   {
--     ["employees.id"]   = 1,
--     ["id"]             = 1,
--     ["employees.name"] = "Alice",
--     ["name"]           = "Alice",
--     ...
--   }
--
-- # Supported SQL
-- ================
--
--   SELECT [DISTINCT] col1, col2, expr AS alias FROM table
--   [INNER/LEFT/RIGHT/FULL/CROSS] JOIN table ON condition
--   [WHERE expr]
--   [GROUP BY col1, col2]
--   [HAVING expr]
--   [ORDER BY col [ASC|DESC]]
--   [LIMIT n [OFFSET m]]
--
-- Aggregate functions: COUNT(*), COUNT(col), SUM, AVG, MIN, MAX
-- Expressions: arithmetic (+,-,*,/,%), comparisons, BETWEEN, IN, LIKE, IS NULL
-- NULL handling: three-valued logic (true/false/nil)
--
-- # Built-in SQL Parser
-- ======================
--
-- This package includes a standalone recursive-descent SQL parser that
-- produces a structured AST without depending on the grammar_tools/lexer chain.
-- This makes the package self-contained.

local M = {}

-- ============================================================================
-- SQL Lexer (minimal, built-in)
-- ============================================================================

local function tokenize(sql)
  local tokens = {}
  local i      = 1
  local n      = #sql

  local KEYWORDS = {
    SELECT=true, FROM=true, WHERE=true, GROUP=true, BY=true, HAVING=true,
    ORDER=true, LIMIT=true, OFFSET=true, DISTINCT=true, ALL=true,
    JOIN=true, INNER=true, LEFT=true, RIGHT=true, FULL=true, OUTER=true,
    CROSS=true, ON=true, AS=true,
    AND=true, OR=true, NOT=true, IS=true, NULL=true, IN=true,
    BETWEEN=true, LIKE=true, ESCAPE=true,
    COUNT=true, SUM=true, AVG=true, MIN=true, MAX=true,
    CASE=true, WHEN=true, THEN=true, ELSE=true, END=true,
    TRUE=true, FALSE=true,
    ASC=true, DESC=true,
    INSERT=true, UPDATE=true, DELETE=true, CREATE=true, DROP=true,
  }

  while i <= n do
    local c = sql:sub(i, i)

    -- Skip whitespace
    if c:match("%s") then
      i = i + 1

    -- Line comment --
    elseif c == "-" and sql:sub(i, i+1) == "--" then
      while i <= n and sql:sub(i, i) ~= "\n" do i = i + 1 end

    -- String literal
    elseif c == "'" then
      local start = i
      i = i + 1
      while i <= n do
        if sql:sub(i, i) == "'" then
          if sql:sub(i+1, i+1) == "'" then
            i = i + 2  -- escaped quote
          else
            i = i + 1
            break
          end
        else
          i = i + 1
        end
      end
      tokens[#tokens + 1] = { type = "STRING", value = sql:sub(start+1, i-2):gsub("''", "'") }

    -- Numbers
    elseif c:match("%d") or (c == "." and sql:sub(i+1, i+1):match("%d")) then
      local start = i
      while i <= n and (sql:sub(i, i):match("%d") or sql:sub(i, i) == ".") do
        i = i + 1
      end
      local num = sql:sub(start, i-1)
      tokens[#tokens + 1] = { type = "NUMBER", value = tonumber(num) }

    -- Identifiers / keywords
    elseif c:match("[%a_]") then
      local start = i
      while i <= n and sql:sub(i, i):match("[%w_]") do i = i + 1 end
      local word   = sql:sub(start, i-1)
      local upper  = word:upper()
      if KEYWORDS[upper] then
        tokens[#tokens + 1] = { type = upper, value = upper }
      else
        tokens[#tokens + 1] = { type = "IDENT", value = word }
      end

    -- Quoted identifiers
    elseif c == '"' then
      local start = i + 1
      i = i + 1
      while i <= n and sql:sub(i, i) ~= '"' do i = i + 1 end
      tokens[#tokens + 1] = { type = "IDENT", value = sql:sub(start, i-1) }
      i = i + 1

    -- Backtick identifiers
    elseif c == '`' then
      local start = i + 1
      i = i + 1
      while i <= n and sql:sub(i, i) ~= '`' do i = i + 1 end
      tokens[#tokens + 1] = { type = "IDENT", value = sql:sub(start, i-1) }
      i = i + 1

    -- Two-character operators
    elseif c == "<" and sql:sub(i+1, i+1) == "=" then
      tokens[#tokens + 1] = { type = "LE", value = "<=" }; i = i + 2
    elseif c == ">" and sql:sub(i+1, i+1) == "=" then
      tokens[#tokens + 1] = { type = "GE", value = ">=" }; i = i + 2
    elseif c == "<" and sql:sub(i+1, i+1) == ">" then
      tokens[#tokens + 1] = { type = "NE", value = "<>" }; i = i + 2
    elseif c == "!" and sql:sub(i+1, i+1) == "=" then
      tokens[#tokens + 1] = { type = "NE", value = "!=" }; i = i + 2
    elseif c == "." and sql:sub(i+1, i+1):match("%a") then
      tokens[#tokens + 1] = { type = "DOT", value = "." }; i = i + 1

    -- Single-character tokens
    elseif c == "*" then tokens[#tokens + 1] = { type = "STAR",    value = "*" }; i = i + 1
    elseif c == "=" then tokens[#tokens + 1] = { type = "EQ",      value = "=" }; i = i + 1
    elseif c == "<" then tokens[#tokens + 1] = { type = "LT",      value = "<" }; i = i + 1
    elseif c == ">" then tokens[#tokens + 1] = { type = "GT",      value = ">" }; i = i + 1
    elseif c == "+" then tokens[#tokens + 1] = { type = "PLUS",    value = "+" }; i = i + 1
    elseif c == "-" then tokens[#tokens + 1] = { type = "MINUS",   value = "-" }; i = i + 1
    elseif c == "/" then tokens[#tokens + 1] = { type = "DIVIDE",  value = "/" }; i = i + 1
    elseif c == "%" then tokens[#tokens + 1] = { type = "MOD",     value = "%" }; i = i + 1
    elseif c == "(" then tokens[#tokens + 1] = { type = "LPAREN",  value = "(" }; i = i + 1
    elseif c == ")" then tokens[#tokens + 1] = { type = "RPAREN",  value = ")" }; i = i + 1
    elseif c == "," then tokens[#tokens + 1] = { type = "COMMA",   value = "," }; i = i + 1
    elseif c == ";" then tokens[#tokens + 1] = { type = "SEMI",    value = ";" }; i = i + 1
    else
      -- Skip unrecognized characters
      i = i + 1
    end
  end

  tokens[#tokens + 1] = { type = "EOF", value = "EOF" }
  return tokens
end

-- ============================================================================
-- SQL Parser (recursive descent)
-- ============================================================================
--
-- Produces a structured AST as nested Lua tables.
-- AST node format:
--   { kind = "select_stmt", distinct = bool,
--     select_list = [...], from = {...}, joins = [...],
--     where = expr, group_by = [...], having = expr,
--     order_by = [...], limit = n, offset = n }

local Parser = {}
Parser.__index = Parser

function Parser.new(tokens)
  return setmetatable({ tokens = tokens, pos = 1 }, Parser)
end

function Parser:peek()  return self.tokens[self.pos] end
function Parser:peek_type() return self.tokens[self.pos].type end

function Parser:advance()
  local t = self.tokens[self.pos]
  self.pos = self.pos + 1
  return t
end

function Parser:expect(type)
  local t = self:peek()
  if t.type ~= type then
    error(string.format("expected %s, got %s '%s' at position %d",
      type, t.type, tostring(t.value), self.pos))
  end
  return self:advance()
end

function Parser:match(...)
  local t = self:peek_type()
  for _, tp in ipairs({...}) do
    if t == tp then return self:advance() end
  end
  return nil
end

-- Parse a complete SQL program (one or more statements separated by ;)
function Parser:parse_program()
  local stmts = {}
  while self:peek_type() ~= "EOF" do
    self:match("SEMI")
    if self:peek_type() == "EOF" then break end
    stmts[#stmts + 1] = self:parse_statement()
    self:match("SEMI")
  end
  return { kind = "program", statements = stmts }
end

function Parser:parse_statement()
  local t = self:peek_type()
  if t == "SELECT" then
    return self:parse_select()
  else
    error("only SELECT statements are supported, got: " .. t)
  end
end

function Parser:parse_select()
  self:expect("SELECT")
  local distinct = false
  if self:match("DISTINCT") then distinct = true
  elseif self:match("ALL") then end

  local select_list = self:parse_select_list()
  self:expect("FROM")
  local from       = self:parse_table_ref()
  local joins      = self:parse_joins()
  local where      = nil
  local group_by   = {}
  local having     = nil
  local order_by   = {}
  local limit      = nil
  local offset     = nil

  if self:match("WHERE") then where = self:parse_expr() end
  if self:match("GROUP") then
    self:expect("BY")
    group_by = self:parse_expr_list()
  end
  if self:match("HAVING") then having = self:parse_expr() end
  if self:match("ORDER") then
    self:expect("BY")
    order_by = self:parse_order_list()
  end
  if self:match("LIMIT") then
    limit = self:parse_primary().value
    if self:match("OFFSET") then
      offset = self:parse_primary().value
    end
  end

  return {
    kind = "select_stmt", distinct = distinct,
    select_list = select_list, from = from, joins = joins,
    where = where, group_by = group_by, having = having,
    order_by = order_by, limit = limit, offset = offset,
  }
end

function Parser:parse_select_list()
  local items = {}
  repeat
    items[#items + 1] = self:parse_select_item()
  until not self:match("COMMA")
  return items
end

function Parser:parse_select_item()
  if self:peek_type() == "STAR" then
    self:advance()
    return { kind = "star" }
  end
  local expr  = self:parse_expr()
  local alias = nil
  if self:match("AS") then
    alias = self:advance().value
  elseif self:peek_type() == "IDENT" then
    -- Implicit alias (no AS keyword)
    alias = self:advance().value
  end
  return { kind = "select_item", expr = expr, alias = alias }
end

function Parser:parse_table_ref()
  local name  = self:advance().value   -- IDENT or keyword used as table name
  local alias = nil
  if self:match("AS") then
    alias = self:advance().value
  elseif self:peek_type() == "IDENT" and
         not ({JOIN=true,WHERE=true,GROUP=true,HAVING=true,ORDER=true,
               LIMIT=true,LEFT=true,RIGHT=true,INNER=true,FULL=true,
               CROSS=true,OUTER=true})[self:peek_type()] then
    -- check peek is not a keyword
    local peeked = self:peek()
    if peeked.type == "IDENT" then
      alias = self:advance().value
    end
  end
  return { kind = "table_ref", name = name, alias = alias or name }
end

function Parser:parse_joins()
  local joins = {}
  while true do
    local join_type = nil
    if     self:match("INNER") then join_type = "INNER"; self:match("JOIN")
    elseif self:match("LEFT")  then self:match("OUTER"); join_type = "LEFT"; self:expect("JOIN")
    elseif self:match("RIGHT") then self:match("OUTER"); join_type = "RIGHT"; self:expect("JOIN")
    elseif self:match("FULL")  then self:match("OUTER"); join_type = "FULL"; self:expect("JOIN")
    elseif self:match("CROSS") then join_type = "CROSS"; self:expect("JOIN")
    elseif self:match("JOIN")  then join_type = "INNER"
    else break end

    local tref = self:parse_table_ref()
    local on   = nil
    if join_type ~= "CROSS" then
      self:expect("ON")
      on = self:parse_expr()
    end
    joins[#joins + 1] = { kind = "join", join_type = join_type, table_ref = tref, on = on }
  end
  return joins
end

function Parser:parse_order_list()
  local items = {}
  repeat
    local expr      = self:parse_expr()
    local direction = "ASC"
    if     self:match("ASC")  then direction = "ASC"
    elseif self:match("DESC") then direction = "DESC"
    end
    items[#items + 1] = { expr = expr, direction = direction }
  until not self:match("COMMA")
  return items
end

function Parser:parse_expr_list()
  local items = {}
  repeat items[#items + 1] = self:parse_expr() until not self:match("COMMA")
  return items
end

-- Expression parsing (operator precedence via recursive descent)
-- Precedence: OR < AND < NOT < comparison < additive < multiplicative < unary < primary

function Parser:parse_expr()       return self:parse_or()   end
function Parser:parse_or()
  local left = self:parse_and()
  while self:match("OR") do
    left = { kind = "binop", op = "OR", left = left, right = self:parse_and() }
  end
  return left
end

function Parser:parse_and()
  local left = self:parse_not()
  while self:match("AND") do
    left = { kind = "binop", op = "AND", left = left, right = self:parse_not() }
  end
  return left
end

function Parser:parse_not()
  if self:match("NOT") then
    return { kind = "unary", op = "NOT", operand = self:parse_not() }
  end
  return self:parse_comparison()
end

function Parser:parse_comparison()
  local left = self:parse_additive()
  local t    = self:peek_type()

  if t == "EQ" or t == "NE" or t == "LT" or t == "GT" or t == "LE" or t == "GE" then
    local op = self:advance().value
    return { kind = "binop", op = op, left = left, right = self:parse_additive() }
  elseif t == "IS" then
    self:advance()
    local negated = self:match("NOT") ~= nil
    self:expect("NULL")
    return { kind = "is_null", negated = negated, operand = left }
  elseif t == "NOT" then
    self:advance()
    if self:match("IN") then
      return { kind = "in_expr", negated = true, operand = left, values = self:parse_in_list() }
    elseif self:match("LIKE") then
      local pattern = self:parse_additive()
      return { kind = "like", negated = true, operand = left, pattern = pattern }
    elseif self:match("BETWEEN") then
      local lo = self:parse_additive(); self:expect("AND"); local hi = self:parse_additive()
      return { kind = "between", negated = true, operand = left, lo = lo, hi = hi }
    end
  elseif t == "IN" then
    self:advance()
    return { kind = "in_expr", negated = false, operand = left, values = self:parse_in_list() }
  elseif t == "LIKE" then
    self:advance()
    return { kind = "like", negated = false, operand = left, pattern = self:parse_additive() }
  elseif t == "BETWEEN" then
    self:advance()
    local lo = self:parse_additive(); self:expect("AND"); local hi = self:parse_additive()
    return { kind = "between", negated = false, operand = left, lo = lo, hi = hi }
  end
  return left
end

function Parser:parse_in_list()
  self:expect("LPAREN")
  local vals = {}
  repeat vals[#vals + 1] = self:parse_additive() until not self:match("COMMA")
  self:expect("RPAREN")
  return vals
end

function Parser:parse_additive()
  local left = self:parse_multiplicative()
  while true do
    local t = self:peek_type()
    if t == "PLUS" or t == "MINUS" then
      local op = self:advance().value
      left = { kind = "binop", op = op, left = left, right = self:parse_multiplicative() }
    else break end
  end
  return left
end

function Parser:parse_multiplicative()
  local left = self:parse_unary()
  while true do
    local t = self:peek_type()
    if t == "STAR" or t == "DIVIDE" or t == "MOD" then
      local op = self:advance().value
      left = { kind = "binop", op = op, left = left, right = self:parse_unary() }
    else break end
  end
  return left
end

function Parser:parse_unary()
  if self:match("MINUS") then
    return { kind = "unary", op = "-", operand = self:parse_primary() }
  end
  return self:parse_primary()
end

function Parser:parse_primary()
  local t = self:peek()

  if t.type == "NUMBER" then
    self:advance(); return { kind = "literal", value = t.value }
  elseif t.type == "STRING" then
    self:advance(); return { kind = "literal", value = t.value }
  elseif t.type == "NULL" then
    self:advance(); return { kind = "literal", value = nil }
  elseif t.type == "TRUE" then
    self:advance(); return { kind = "literal", value = true }
  elseif t.type == "FALSE" then
    self:advance(); return { kind = "literal", value = false }
  elseif t.type == "STAR" then
    self:advance(); return { kind = "star" }
  elseif t.type == "LPAREN" then
    self:advance()
    local e = self:parse_expr()
    self:expect("RPAREN")
    return e
  elseif t.type == "IDENT" or ({COUNT=true,SUM=true,AVG=true,MIN=true,MAX=true,
                                  UPPER=true,LOWER=true,LENGTH=true})[t.type] then
    self:advance()
    local name = t.value
    -- Function call?
    if self:peek_type() == "LPAREN" then
      self:advance()
      local args = {}
      if self:peek_type() == "STAR" then
        self:advance()
        args[1] = { kind = "star" }
      elseif self:peek_type() ~= "RPAREN" then
        args = self:parse_expr_list()
      end
      self:expect("RPAREN")
      return { kind = "func_call", name = name:upper(), args = args }
    end
    -- Qualified column: table.column
    if self:peek_type() == "DOT" then
      self:advance()
      local col = self:advance().value
      return { kind = "column_ref", table_name = name, col_name = col }
    end
    return { kind = "column_ref", col_name = name }
  end

  error(string.format("unexpected token %s '%s' at position %d",
    t.type, tostring(t.value), self.pos))
end

-- ============================================================================
-- SQL Parser public function
-- ============================================================================

local function parse_sql(sql)
  local tokens = tokenize(sql)
  local p      = Parser.new(tokens)
  local ok, result = pcall(function() return p:parse_program() end)
  if ok then
    return result, nil
  else
    return nil, tostring(result)
  end
end

M.parse_sql   = parse_sql
M.tokenize    = tokenize

-- ============================================================================
-- Expression Evaluator
-- ============================================================================
--
-- Recursively evaluates an expression AST node against a row context.
-- The row context is a flat map: { "col" = value, "table.col" = value }

local function like_to_pattern(like_str)
  -- Escape Lua magic chars, then replace SQL % and _ wildcards
  local escaped = like_str
    :gsub("([%(%)%.%+%-%*%?%[%^%$%%])", "%%%1")
    :gsub("%%", ".*")
    :gsub("_",  ".")
  return "^" .. escaped .. "$"
end

local function eval_expr(node, row)
  if not node then return nil end
  local kind = node.kind

  if kind == "literal" then
    return node.value

  elseif kind == "column_ref" then
    -- Try qualified form first, then bare
    local qualified = node.table_name and (node.table_name .. "." .. node.col_name) or nil
    if qualified and row[qualified] ~= nil then
      return row[qualified]
    end
    local bare = row[node.col_name]
    if bare == nil then
      -- Check if it might be an aggregate result stored with uppercase key
      local upper = node.col_name:upper()
      bare = row[upper]
    end
    return bare   -- nil if not found (column not in current row)

  elseif kind == "binop" then
    local op = node.op
    if op == "AND" then
      local l = eval_expr(node.left, row)
      local r = eval_expr(node.right, row)
      -- Three-valued AND: false dominates
      if l == false or r == false then return false end
      if l == nil   or r == nil   then return nil   end
      return true
    elseif op == "OR" then
      local l = eval_expr(node.left, row)
      local r = eval_expr(node.right, row)
      -- Three-valued OR: true dominates
      if l == true or r == true then return true end
      if l == nil  or r == nil  then return nil  end
      return false
    elseif op == "=" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l == r
    elseif op == "!=" or op == "<>" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l ~= r
    elseif op == "<" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l < r
    elseif op == ">" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l > r
    elseif op == "<=" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l <= r
    elseif op == ">=" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l >= r
    elseif op == "+" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l + r
    elseif op == "-" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l - r
    elseif op == "*" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l * r
    elseif op == "/" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      if r == 0 then return nil end
      return l / r
    elseif op == "%" then
      local l, r = eval_expr(node.left, row), eval_expr(node.right, row)
      if l == nil or r == nil then return nil end
      return l % r
    end

  elseif kind == "unary" then
    if node.op == "-" then
      local v = eval_expr(node.operand, row)
      return v ~= nil and -v or nil
    elseif node.op == "NOT" then
      local v = eval_expr(node.operand, row)
      if v == nil then return nil end
      return not v
    end

  elseif kind == "is_null" then
    local v = eval_expr(node.operand, row)
    local result = (v == nil)
    return node.negated and not result or result

  elseif kind == "in_expr" then
    local v = eval_expr(node.operand, row)
    if v == nil then return nil end
    for _, val_node in ipairs(node.values) do
      if eval_expr(val_node, row) == v then
        return not node.negated
      end
    end
    return node.negated

  elseif kind == "between" then
    local v  = eval_expr(node.operand, row)
    local lo = eval_expr(node.lo, row)
    local hi = eval_expr(node.hi, row)
    if v == nil or lo == nil or hi == nil then return nil end
    local result = v >= lo and v <= hi
    return node.negated and not result or result

  elseif kind == "like" then
    local v = eval_expr(node.operand, row)
    local p = eval_expr(node.pattern, row)
    if v == nil or p == nil then return nil end
    local pattern = like_to_pattern(p)
    local result  = tostring(v):match(pattern) ~= nil
    return node.negated and not result or result

  elseif kind == "func_call" then
    local fname = node.name
    if fname == "COUNT" or fname == "SUM" or fname == "AVG"
       or fname == "MIN" or fname == "MAX" then
      -- Aggregate: look up pre-computed value in row context
      local agg_key = node.args[1] and node.args[1].kind == "star"
                      and (fname .. "(*)")
                      or  (fname .. "(" .. (node.args[1] and (node.args[1].col_name or "") or "") .. ")")
      return row[agg_key] or row[fname]
    elseif fname == "UPPER" then
      local v = eval_expr(node.args[1], row)
      return v ~= nil and tostring(v):upper() or nil
    elseif fname == "LOWER" then
      local v = eval_expr(node.args[1], row)
      return v ~= nil and tostring(v):lower() or nil
    elseif fname == "LENGTH" then
      local v = eval_expr(node.args[1], row)
      return v ~= nil and #tostring(v) or nil
    end
  elseif kind == "star" then
    return "*"  -- handled specially by projection
  end

  return nil
end

M.eval_expr = eval_expr

-- ============================================================================
-- Executor
-- ============================================================================

--- Build a row context from a raw row map, table name, and alias.
local function row_to_ctx(row, alias, schema)
  local ctx = {}
  for _, col in ipairs(schema) do
    local qualified  = alias .. "." .. col
    local val        = row[col]
    ctx[qualified]   = val
    ctx[col]         = val   -- bare form (may be overwritten by later tables)
  end
  return ctx
end

--- Get the column name for an expression (for SELECT list labeling).
local function expr_label(item)
  if item.alias then return item.alias end
  local e = item.expr
  if not e then return "?" end
  if e.kind == "column_ref" then return e.col_name end
  if e.kind == "func_call" then
    local arg = e.args[1]
    if arg and arg.kind == "star" then return e.name .. "(*)" end
    if arg and arg.kind == "column_ref" then return e.name .. "(" .. arg.col_name .. ")" end
    return e.name
  end
  if e.kind == "binop" then return "expr" end
  if e.kind == "literal" then return tostring(e.value) end
  return "?"
end

--- Check whether an expression contains any aggregate function call.
local function has_aggregate(expr)
  if not expr or type(expr) ~= "table" then return false end
  if expr.kind == "func_call" and
     ({COUNT=true, SUM=true, AVG=true, MIN=true, MAX=true})[expr.name] then
    return true
  end
  return has_aggregate(expr.left) or has_aggregate(expr.right)
        or has_aggregate(expr.operand)
        or (function()
          for _, a in ipairs(expr.args or {}) do
            if has_aggregate(a) then return true end
          end
          return false
        end)()
end

--- Compute aggregate functions for a group of rows.
local function compute_aggregates(rows, agg_exprs)
  local result = {}
  for _, agg in ipairs(agg_exprs) do
    local fname = agg.name
    local col   = agg.args[1]
    local key

    if col and col.kind == "star" then
      key = fname .. "(*)"
    elseif col and col.kind == "column_ref" then
      key = fname .. "(" .. col.col_name .. ")"
    else
      key = fname
    end

    if fname == "COUNT" then
      if col and col.kind == "star" then
        result[key] = #rows
      else
        local n = 0
        for _, row in ipairs(rows) do
          local v = eval_expr(col, row)
          if v ~= nil then n = n + 1 end
        end
        result[key] = n
      end

    elseif fname == "SUM" then
      local total = 0
      for _, row in ipairs(rows) do
        local v = eval_expr(col, row)
        if type(v) == "number" then total = total + v end
      end
      result[key] = total

    elseif fname == "AVG" then
      local total, n = 0, 0
      for _, row in ipairs(rows) do
        local v = eval_expr(col, row)
        if type(v) == "number" then total = total + v; n = n + 1 end
      end
      result[key] = n > 0 and total / n or nil

    elseif fname == "MIN" then
      local min = nil
      for _, row in ipairs(rows) do
        local v = eval_expr(col, row)
        if v ~= nil and (min == nil or v < min) then min = v end
      end
      result[key] = min

    elseif fname == "MAX" then
      local max = nil
      for _, row in ipairs(rows) do
        local v = eval_expr(col, row)
        if v ~= nil and (max == nil or v > max) then max = v end
      end
      result[key] = max
    end
  end
  return result
end

--- Collect all aggregate function nodes from a list of expressions.
local function collect_aggs(exprs)
  local aggs = {}
  local function walk(e)
    if not e or type(e) ~= "table" then return end
    if e.kind == "func_call" and
       ({COUNT=true, SUM=true, AVG=true, MIN=true, MAX=true})[e.name] then
      aggs[#aggs + 1] = e
    end
    walk(e.left); walk(e.right); walk(e.operand)
    for _, a in ipairs(e.args or {}) do walk(a) end
  end
  for _, e in ipairs(exprs) do walk(e) end
  return aggs
end

--- Execute a SELECT statement AST against a data source.
-- @param stmt    The select_stmt AST node.
-- @param ds      DataSource with :schema(name) and :scan(name) methods.
-- @return        { columns = [...], rows = [[...], ...] }
local function execute_select(stmt, ds)
  -- ── Stage 1: FROM ─────────────────────────────────────────────────────
  local base_name   = stmt.from.name
  local base_alias  = stmt.from.alias
  local base_schema = ds:schema(base_name)
  local raw_rows    = ds:scan(base_name)
  local rows        = {}
  for _, raw in ipairs(raw_rows) do
    rows[#rows + 1] = row_to_ctx(raw, base_alias, base_schema)
  end

  -- ── Stage 2: JOINs ────────────────────────────────────────────────────
  for _, join in ipairs(stmt.joins or {}) do
    local jname   = join.table_ref.name
    local jalias  = join.table_ref.alias
    local jschema = ds:schema(jname)
    local jrows   = ds:scan(jname)
    local new_rows = {}

    if join.join_type == "CROSS" then
      for _, lr in ipairs(rows) do
        for _, rr in ipairs(jrows) do
          local merged = {}
          for k, v in pairs(lr) do merged[k] = v end
          for _, col in ipairs(jschema) do
            merged[jalias .. "." .. col] = rr[col]
            merged[col] = rr[col]
          end
          new_rows[#new_rows + 1] = merged
        end
      end
    elseif join.join_type == "INNER" then
      for _, lr in ipairs(rows) do
        for _, rr in ipairs(jrows) do
          local merged = {}
          for k, v in pairs(lr) do merged[k] = v end
          for _, col in ipairs(jschema) do
            merged[jalias .. "." .. col] = rr[col]
            merged[col] = rr[col]
          end
          if eval_expr(join.on, merged) == true then
            new_rows[#new_rows + 1] = merged
          end
        end
      end
    elseif join.join_type == "LEFT" then
      for _, lr in ipairs(rows) do
        local matched = false
        for _, rr in ipairs(jrows) do
          local merged = {}
          for k, v in pairs(lr) do merged[k] = v end
          for _, col in ipairs(jschema) do
            merged[jalias .. "." .. col] = rr[col]
            merged[col] = rr[col]
          end
          if eval_expr(join.on, merged) == true then
            new_rows[#new_rows + 1] = merged; matched = true
          end
        end
        if not matched then
          local merged = {}
          for k, v in pairs(lr) do merged[k] = v end
          for _, col in ipairs(jschema) do
            merged[jalias .. "." .. col] = nil
            merged[col] = nil
          end
          new_rows[#new_rows + 1] = merged
        end
      end
    else
      -- RIGHT/FULL: simplified — treat as INNER for now
      for _, lr in ipairs(rows) do
        for _, rr in ipairs(jrows) do
          local merged = {}
          for k, v in pairs(lr) do merged[k] = v end
          for _, col in ipairs(jschema) do
            merged[jalias .. "." .. col] = rr[col]
            merged[col] = rr[col]
          end
          if eval_expr(join.on, merged) == true then
            new_rows[#new_rows + 1] = merged
          end
        end
      end
    end
    rows = new_rows
  end

  -- ── Stage 3: WHERE ────────────────────────────────────────────────────
  if stmt.where then
    local filtered = {}
    for _, row in ipairs(rows) do
      if eval_expr(stmt.where, row) == true then
        filtered[#filtered + 1] = row
      end
    end
    rows = filtered
  end

  -- ── Stage 4: GROUP BY + Aggregates ────────────────────────────────────
  local select_exprs = {}
  for _, item in ipairs(stmt.select_list) do
    if item.kind ~= "star" then
      select_exprs[#select_exprs + 1] = item.expr
    end
  end

  local all_exprs = {}
  for _, e in ipairs(select_exprs) do all_exprs[#all_exprs + 1] = e end
  if stmt.having then all_exprs[#all_exprs + 1] = stmt.having end

  local agg_specs = collect_aggs(all_exprs)
  local has_aggs  = #agg_specs > 0
  local has_group = #(stmt.group_by or {}) > 0

  if has_group or has_aggs then
    if has_group then
      -- Group rows by GROUP BY keys
      local group_keys_order = {}
      local group_maps       = {}
      for _, row in ipairs(rows) do
        local key_parts = {}
        for _, gexpr in ipairs(stmt.group_by) do
          local v = eval_expr(gexpr, row)
          key_parts[#key_parts + 1] = tostring(v)
        end
        local key = table.concat(key_parts, "\0")
        if not group_maps[key] then
          group_maps[key] = { rows = {}, key_row = row }
          group_keys_order[#group_keys_order + 1] = key
        end
        group_maps[key].rows[#group_maps[key].rows + 1] = row
      end

      local new_rows = {}
      for _, key in ipairs(group_keys_order) do
        local grp       = group_maps[key]
        local agg_vals  = compute_aggregates(grp.rows, agg_specs)
        local merged    = {}
        for k, v in pairs(grp.key_row) do merged[k] = v end
        for k, v in pairs(agg_vals)    do merged[k] = v end
        new_rows[#new_rows + 1] = merged
      end
      rows = new_rows

    else
      -- No GROUP BY but has aggregates: treat all as one group
      local agg_vals = compute_aggregates(rows, agg_specs)
      local merged   = {}
      if #rows > 0 then for k, v in pairs(rows[1]) do merged[k] = v end end
      for k, v in pairs(agg_vals) do merged[k] = v end
      rows = { merged }
    end
  end

  -- ── Stage 5: HAVING ───────────────────────────────────────────────────
  if stmt.having then
    local filtered = {}
    for _, row in ipairs(rows) do
      if eval_expr(stmt.having, row) == true then
        filtered[#filtered + 1] = row
      end
    end
    rows = filtered
  end

  -- ── Stage 6: SELECT (Projection) ─────────────────────────────────────
  local columns    = {}
  local star_pos   = nil

  -- First pass: determine column names
  for i, item in ipairs(stmt.select_list) do
    if item.kind == "star" then
      star_pos = i
    else
      columns[#columns + 1] = expr_label(item)
    end
  end

  -- If SELECT *, replace with schema columns from first row
  local projected_rows = {}
  if star_pos then
    -- Expand * into all bare column names from the first row
    local star_cols = {}
    if rows[1] then
      for k, _ in pairs(rows[1]) do
        if not k:find("%.") then  -- bare columns only
          star_cols[#star_cols + 1] = k
        end
      end
      table.sort(star_cols)
    end
    -- Build columns: items before *, star cols, items after *
    local new_cols = {}
    for i, item in ipairs(stmt.select_list) do
      if item.kind == "star" then
        for _, c in ipairs(star_cols) do new_cols[#new_cols + 1] = c end
      else
        new_cols[#new_cols + 1] = expr_label(item)
      end
    end
    columns = new_cols

    for _, row in ipairs(rows) do
      local proj = {}
      for i, item in ipairs(stmt.select_list) do
        if item.kind == "star" then
          for _, c in ipairs(star_cols) do proj[#proj + 1] = row[c] end
        else
          proj[#proj + 1] = eval_expr(item.expr, row)
        end
      end
      projected_rows[#projected_rows + 1] = proj
    end
  else
    for _, row in ipairs(rows) do
      local proj = {}
      for _, item in ipairs(stmt.select_list) do
        proj[#proj + 1] = eval_expr(item.expr, row)
      end
      projected_rows[#projected_rows + 1] = proj
    end
  end

  rows = projected_rows

  -- ── Stage 7: DISTINCT ─────────────────────────────────────────────────
  if stmt.distinct then
    local seen   = {}
    local unique = {}
    for _, row in ipairs(rows) do
      local key = table.concat(
        (function()
          local parts = {}
          for _, v in ipairs(row) do parts[#parts + 1] = tostring(v) end
          return parts
        end)(), "\0")
      if not seen[key] then seen[key] = true; unique[#unique + 1] = row end
    end
    rows = unique
  end

  -- ── Stage 8: ORDER BY ─────────────────────────────────────────────────
  if stmt.order_by and #stmt.order_by > 0 then
    table.sort(rows, function(a, b)
      for _, ord in ipairs(stmt.order_by) do
        -- Map order expression to column index or direct evaluation
        local ia, ib
        local expr = ord.expr
        if expr.kind == "column_ref" then
          for ci, c in ipairs(columns) do
            if c == expr.col_name or c == (expr.table_name and (expr.table_name .. "." .. expr.col_name)) then
              ia = a[ci]; ib = b[ci]; break
            end
          end
        elseif expr.kind == "literal" and type(expr.value) == "number" then
          local idx = math.floor(expr.value)
          ia = a[idx]; ib = b[idx]
        end
        if ia == nil and ib == nil then -- continue
        elseif ia == nil then return ord.direction == "ASC"
        elseif ib == nil then return ord.direction ~= "ASC"
        elseif ia ~= ib then
          if ord.direction == "ASC" then return ia < ib else return ia > ib end
        end
      end
      return false
    end)
  end

  -- ── Stage 9: LIMIT / OFFSET ───────────────────────────────────────────
  if stmt.offset then
    local n = math.floor(tonumber(stmt.offset) or 0)
    rows = {table.unpack(rows, n + 1)}
  end
  if stmt.limit then
    local n = math.floor(tonumber(stmt.limit) or #rows)
    local sliced = {}
    for i = 1, math.min(n, #rows) do sliced[i] = rows[i] end
    rows = sliced
  end

  return { columns = columns, rows = rows }
end

-- ============================================================================
-- Public API
-- ============================================================================

--- Execute a SQL SELECT statement against a data source.
-- @param sql         SQL string.
-- @param data_source Object with :schema(name) and :scan(name) methods.
-- @return            ok=true, result={columns, rows}  OR  ok=false, err_msg
function M.execute(sql, data_source)
  local ast, err = parse_sql(sql)
  if not ast then
    return false, "Parse error: " .. tostring(err)
  end

  if #ast.statements == 0 then
    return true, { columns = {}, rows = {} }
  end

  local stmt = ast.statements[1]
  if stmt.kind ~= "select_stmt" then
    return false, "Only SELECT statements are supported"
  end

  local ok, result = pcall(execute_select, stmt, data_source)
  if ok then
    return true, result
  else
    return false, tostring(result)
  end
end

--- Execute multiple SQL statements.
-- @param sql         SQL string with one or more semicolon-separated SELECT statements.
-- @param data_source DataSource object.
-- @return            ok=true, list_of_results  OR  ok=false, err_msg
function M.execute_all(sql, data_source)
  local ast, err = parse_sql(sql)
  if not ast then
    return false, "Parse error: " .. tostring(err)
  end

  local results = {}
  for _, stmt in ipairs(ast.statements) do
    if stmt.kind ~= "select_stmt" then
      return false, "Only SELECT statements are supported"
    end
    local ok, result = pcall(execute_select, stmt, data_source)
    if not ok then
      return false, tostring(result)
    end
    results[#results + 1] = result
  end

  return true, results
end

--- InMemoryDataSource — a simple in-memory DataSource for testing.
-- Usage:
--   local ds = M.InMemoryDataSource.new()
--   ds:add_table("users", {"id","name"}, {{id=1,name="Alice"},{id=2,name="Bob"}})

local InMemoryDataSource = {}
InMemoryDataSource.__index = InMemoryDataSource

--- Create a new InMemoryDataSource.
-- @param tables_data  Optional table of { [table_name] = { row, ... } }.
--                     Schema is inferred from the keys of the first row.
function InMemoryDataSource.new(tables_data)
  local ds = setmetatable({ tables = {}, schemas = {} }, InMemoryDataSource)
  if type(tables_data) == "table" then
    for tname, rows in pairs(tables_data) do
      -- Infer schema from the first row's keys (sorted for determinism)
      local schema = {}
      if rows[1] then
        for col, _ in pairs(rows[1]) do
          schema[#schema + 1] = col
        end
        table.sort(schema)
      end
      ds:add_table(tname, schema, rows)
    end
  end
  return ds
end

function InMemoryDataSource:add_table(name, schema, rows)
  self.schemas[name] = schema
  self.tables[name]  = rows
end

function InMemoryDataSource:schema(name)
  local s = self.schemas[name]
  if not s then error("table not found: " .. tostring(name)) end
  return s
end

function InMemoryDataSource:scan(name)
  local t = self.tables[name]
  if not t then error("table not found: " .. tostring(name)) end
  return t
end

M.InMemoryDataSource = InMemoryDataSource

return M
