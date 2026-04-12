-- parser -- Recursive descent parser building Abstract Syntax Trees from token streams
-- ======================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- Layer 3 in the computing stack.
--
-- # What is a parser?
--
-- A parser takes a flat sequence of tokens (produced by a lexer) and builds
-- a tree structure that captures the *meaning* of the program. This tree is
-- called an Abstract Syntax Tree (AST).
--
-- For example, the tokens:  NUMBER("1") PLUS("+") NUMBER("2") STAR("*") NUMBER("3")
-- become the tree:
--
--       (+)
--      /   \
--    1     (*)
--         /   \
--        2     3
--
-- Notice how multiplication binds tighter than addition -- the parser encodes
-- operator precedence directly in the tree structure.
--
-- # Two parsing modes
--
-- This package provides two parsers:
--
-- 1. **Hand-written recursive descent parser** (Parser)
--    A classic recursive descent parser with explicit precedence climbing.
--    It recognises a small language of expressions, assignments, and statements.
--    Useful for understanding how parsing works "by hand".
--
-- 2. **Grammar-driven parser** (GrammarParser)
--    An interpreter that reads grammar rules (produced by grammar-tools) and
--    parses any token stream according to those rules. Uses packrat memoization
--    for efficient backtracking. This is the "general purpose" parser.
--
-- # AST node types (hand-written parser)
--
-- The hand-written parser produces these node types:
--
--   NumberLiteral  -- a numeric value like 42
--   StringLiteral  -- a string value like "hello"
--   NameNode       -- an identifier like x or foo
--   BinaryOp       -- two expressions joined by an operator: left op right
--   Assignment     -- name = expression
--   ExpressionStmt -- a bare expression used as a statement
--   Program        -- the top-level node containing a list of statements
--
-- # Grammar-driven AST
--
-- The grammar parser produces ASTNode objects. Each ASTNode has:
--   - rule_name: which grammar rule produced this node
--   - children: a list of child ASTNodes and/or lexer tokens
--
-- A leaf ASTNode wraps a single token. The is_leaf() and token() methods
-- help distinguish leaf nodes from internal nodes.
--
-- # OOP pattern
--
-- We use the standard Lua metatable OOP pattern:
--
--     local MyClass = {}
--     MyClass.__index = MyClass
--
--     function MyClass.new(...)
--         return setmetatable({...}, MyClass)
--     end
--
-- Each AST node type and each parser type follows this pattern.

local parser = {}

parser.VERSION = "0.1.0"

-- =========================================================================
-- Token type constants
-- =========================================================================
--
-- These mirror the Go lexer.TokenType enum. In the Lua lexer, tokens use
-- string type names, but we define numeric constants for backward compat
-- with the hand-written parser that checks token.type (a number).

parser.TOKEN_NAME          = 0
parser.TOKEN_NUMBER        = 1
parser.TOKEN_STRING        = 2
parser.TOKEN_KEYWORD       = 3
parser.TOKEN_PLUS          = 4
parser.TOKEN_MINUS         = 5
parser.TOKEN_STAR          = 6
parser.TOKEN_SLASH         = 7
parser.TOKEN_EQUALS        = 8
parser.TOKEN_EQUALS_EQUALS = 9
parser.TOKEN_LPAREN        = 10
parser.TOKEN_RPAREN        = 11
parser.TOKEN_COMMA         = 12
parser.TOKEN_COLON         = 13
parser.TOKEN_SEMICOLON     = 14
parser.TOKEN_LBRACE        = 15
parser.TOKEN_RBRACE        = 16
parser.TOKEN_LBRACKET      = 17
parser.TOKEN_RBRACKET      = 18
parser.TOKEN_DOT           = 19
parser.TOKEN_BANG          = 20
parser.TOKEN_NEWLINE       = 21
parser.TOKEN_EOF           = 22

-- =========================================================================
-- token_type_name(token) -> string
-- =========================================================================
--
-- Given a token table, return the effective type name as an uppercase string.
-- If the token has a type_name field (grammar-driven lexer), use that.
-- Otherwise, fall back to the numeric type field mapped through a lookup table.

local type_name_map = {
    [parser.TOKEN_NAME]          = "NAME",
    [parser.TOKEN_NUMBER]        = "NUMBER",
    [parser.TOKEN_STRING]        = "STRING",
    [parser.TOKEN_KEYWORD]       = "KEYWORD",
    [parser.TOKEN_PLUS]          = "PLUS",
    [parser.TOKEN_MINUS]         = "MINUS",
    [parser.TOKEN_STAR]          = "STAR",
    [parser.TOKEN_SLASH]         = "SLASH",
    [parser.TOKEN_EQUALS]        = "EQUALS",
    [parser.TOKEN_EQUALS_EQUALS] = "EQUALS_EQUALS",
    [parser.TOKEN_LPAREN]        = "LPAREN",
    [parser.TOKEN_RPAREN]        = "RPAREN",
    [parser.TOKEN_COMMA]         = "COMMA",
    [parser.TOKEN_COLON]         = "COLON",
    [parser.TOKEN_SEMICOLON]     = "SEMICOLON",
    [parser.TOKEN_LBRACE]        = "LBRACE",
    [parser.TOKEN_RBRACE]        = "RBRACE",
    [parser.TOKEN_LBRACKET]      = "LBRACKET",
    [parser.TOKEN_RBRACKET]      = "RBRACKET",
    [parser.TOKEN_DOT]           = "DOT",
    [parser.TOKEN_BANG]          = "BANG",
    [parser.TOKEN_NEWLINE]       = "NEWLINE",
    [parser.TOKEN_EOF]           = "EOF",
}

--- Map a grammar token name string back to a numeric TokenType constant.
-- Used for backward compatibility with the enum-based token matching.
local string_to_token_type = {
    NAME          = parser.TOKEN_NAME,
    NUMBER        = parser.TOKEN_NUMBER,
    STRING        = parser.TOKEN_STRING,
    KEYWORD       = parser.TOKEN_KEYWORD,
    PLUS          = parser.TOKEN_PLUS,
    MINUS         = parser.TOKEN_MINUS,
    STAR          = parser.TOKEN_STAR,
    SLASH         = parser.TOKEN_SLASH,
    EQUALS        = parser.TOKEN_EQUALS,
    EQUALS_EQUALS = parser.TOKEN_EQUALS_EQUALS,
    LPAREN        = parser.TOKEN_LPAREN,
    RPAREN        = parser.TOKEN_RPAREN,
    COMMA         = parser.TOKEN_COMMA,
    COLON         = parser.TOKEN_COLON,
    SEMICOLON     = parser.TOKEN_SEMICOLON,
    LBRACE        = parser.TOKEN_LBRACE,
    RBRACE        = parser.TOKEN_RBRACE,
    LBRACKET      = parser.TOKEN_LBRACKET,
    RBRACKET      = parser.TOKEN_RBRACKET,
    DOT           = parser.TOKEN_DOT,
    BANG          = parser.TOKEN_BANG,
    NEWLINE       = parser.TOKEN_NEWLINE,
    EOF           = parser.TOKEN_EOF,
}

function parser.token_type_name(token)
    if token.type_name and token.type_name ~= "" then
        return token.type_name
    end
    -- Grammar-driven lexers return tokens with a string 'type' field
    -- (not a numeric enum). Handle that case here so GrammarParser works
    -- with both the hand-written parser's numeric token types and the
    -- grammar-driven lexers' string token types.
    if type(token.type) == "string" and token.type ~= "" then
        return token.type
    end
    return type_name_map[token.type] or "UNKNOWN"
end

-- =========================================================================
-- AST node types for the hand-written parser
-- =========================================================================
--
-- Each node type is a small class with a `kind` field that acts as a
-- discriminator (like Go's interface method markers isNode/isExpression).
--
-- Truth table for node classification:
--
--   Node Type       | is_node | is_expression | is_statement
--   ----------------+---------+---------------+-------------
--   NumberLiteral   |   yes   |     yes       |     no
--   StringLiteral   |   yes   |     yes       |     no
--   NameNode        |   yes   |     yes       |     no
--   BinaryOp        |   yes   |     yes       |     no
--   Assignment      |   yes   |     no        |     yes
--   ExpressionStmt  |   yes   |     no        |     yes
--   Program         |   yes   |     no        |     no

-- NumberLiteral -- wraps an integer value
--
-- Example: the token NUMBER("42") becomes NumberLiteral{ value = 42 }
local NumberLiteral = {}
NumberLiteral.__index = NumberLiteral

function NumberLiteral.new(value)
    return setmetatable({ kind = "NumberLiteral", value = value }, NumberLiteral)
end

parser.NumberLiteral = NumberLiteral

-- StringLiteral -- wraps a string value
--
-- Example: the token STRING("hello") becomes StringLiteral{ value = "hello" }
local StringLiteral = {}
StringLiteral.__index = StringLiteral

function StringLiteral.new(value)
    return setmetatable({ kind = "StringLiteral", value = value }, StringLiteral)
end

parser.StringLiteral = StringLiteral

-- NameNode -- wraps an identifier name
--
-- Example: the token NAME("foo") becomes NameNode{ name = "foo" }
local NameNode = {}
NameNode.__index = NameNode

function NameNode.new(name)
    return setmetatable({ kind = "NameNode", name = name }, NameNode)
end

parser.NameNode = NameNode

-- BinaryOp -- two expressions joined by an operator
--
-- Example: 1 + 2 becomes BinaryOp{ left=NumberLiteral(1), op="+", right=NumberLiteral(2) }
--
-- The parser builds these according to precedence rules:
--   - Multiplication and division bind tighter than addition and subtraction
--   - All operators are left-associative
--
-- So "1 + 2 * 3" parses as:
--
--       (+)            not as         (*)
--      /   \                         /   \
--    1     (*)                     (+)    3
--         /   \                   /   \
--        2     3                 1     2
local BinaryOp = {}
BinaryOp.__index = BinaryOp

function BinaryOp.new(left, op, right)
    return setmetatable({ kind = "BinaryOp", left = left, op = op, right = right }, BinaryOp)
end

parser.BinaryOp = BinaryOp

-- Assignment -- name = expression
--
-- Example: x = 42 becomes Assignment{ target=NameNode("x"), value=NumberLiteral(42) }
local Assignment = {}
Assignment.__index = Assignment

function Assignment.new(target, value)
    return setmetatable({ kind = "Assignment", target = target, value = value }, Assignment)
end

parser.Assignment = Assignment

-- ExpressionStmt -- a bare expression used as a statement
--
-- When an expression appears on its own line (not as part of an assignment),
-- it is wrapped in an ExpressionStmt.
-- Example: 42 on its own line becomes ExpressionStmt{ expression=NumberLiteral(42) }
local ExpressionStmt = {}
ExpressionStmt.__index = ExpressionStmt

function ExpressionStmt.new(expression)
    return setmetatable({ kind = "ExpressionStmt", expression = expression }, ExpressionStmt)
end

parser.ExpressionStmt = ExpressionStmt

-- Program -- the top-level AST node, containing a list of statements
local Program = {}
Program.__index = Program

function Program.new(statements)
    return setmetatable({ kind = "Program", statements = statements }, Program)
end

parser.Program = Program

-- =========================================================================
-- ParseError
-- =========================================================================
--
-- Raised when the hand-written parser encounters an unexpected token.
-- Includes the error message and the offending token for precise error
-- reporting (line and column).

local ParseError = {}
ParseError.__index = ParseError

function ParseError.new(message, token)
    local self = setmetatable({}, ParseError)
    self.message = message
    self.token = token
    return self
end

function ParseError:error_string()
    return string.format("%s at line %d, column %d",
        self.message, self.token.line, self.token.column)
end

-- Make ParseError work with error() by providing __tostring
ParseError.__tostring = ParseError.error_string

parser.ParseError = ParseError

-- =========================================================================
-- Parser (hand-written recursive descent)
-- =========================================================================
--
-- # How recursive descent parsing works
--
-- The parser has one function per grammar rule. Each function:
--   1. Looks at the current token (peek)
--   2. Decides which rule to apply
--   3. Consumes tokens (advance) as it builds AST nodes
--   4. Returns the constructed AST node
--
-- # Precedence climbing
--
-- Operator precedence is encoded in the call hierarchy:
--
--   parseExpression  ->  handles + and - (lowest precedence)
--       calls parseTerm
--   parseTerm        ->  handles * and / (higher precedence)
--       calls parseFactor
--   parseFactor      ->  handles atoms: numbers, strings, names, parens
--
-- Because parseTerm is called from parseExpression, multiplication
-- groups tighter than addition. This is the classic technique for
-- encoding precedence without a precedence table.

local Parser = {}
Parser.__index = Parser

--- Create a new hand-written parser.
-- @param tokens  array of token tables, each with {type, value, line, column}
-- @return Parser instance
function Parser.new(tokens)
    return setmetatable({
        tokens = tokens,
        pos = 1,  -- Lua arrays are 1-indexed
    }, Parser)
end

--- Look at the current token without consuming it.
function Parser:peek()
    if self.pos <= #self.tokens then
        return self.tokens[self.pos]
    end
    return self.tokens[#self.tokens]
end

--- Consume and return the current token.
function Parser:advance()
    local token = self:peek()
    self.pos = self.pos + 1
    return token
end

--- Consume the current token, asserting its type matches expected_type.
-- Panics (via error()) if the type doesn't match.
function Parser:expect(expected_type)
    local token = self:peek()
    if token.type ~= expected_type then
        error(ParseError.new(
            string.format("Expected %s, got %s (%q)",
                type_name_map[expected_type] or tostring(expected_type),
                type_name_map[token.type] or tostring(token.type),
                token.value),
            token
        ))
    end
    return self:advance()
end

--- Try to match the current token against one or more types.
-- Returns the token if matched (and advances), or nil if no match.
function Parser:match(...)
    local token = self:peek()
    for _, t in ipairs({...}) do
        if token.type == t then
            return self:advance()
        end
    end
    return nil
end

--- Check whether we've reached the end of input.
function Parser:at_end()
    return self:peek().type == parser.TOKEN_EOF
end

--- Skip over consecutive newline tokens.
-- Newlines act as statement terminators in the hand-written parser,
-- but leading/trailing newlines are insignificant.
function Parser:skip_newlines()
    while self:peek().type == parser.TOKEN_NEWLINE do
        self:advance()
    end
end

--- Parse the entire token stream into a Program AST.
function Parser:parse()
    return self:parse_program()
end

--- Parse a program: a sequence of statements separated by newlines.
function Parser:parse_program()
    local statements = {}
    self:skip_newlines()
    while not self:at_end() do
        local stmt = self:parse_statement()
        statements[#statements + 1] = stmt
        self:skip_newlines()
    end
    return Program.new(statements)
end

--- Parse a single statement.
-- A statement is either an assignment (name = expr) or a bare expression.
--
-- We look ahead two tokens to decide: if the current token is a NAME
-- and the next token is EQUALS, it's an assignment. Otherwise, it's
-- an expression statement.
function Parser:parse_statement()
    if self:peek().type == parser.TOKEN_NAME
       and self.pos + 1 <= #self.tokens
       and self.tokens[self.pos + 1].type == parser.TOKEN_EQUALS then
        return self:parse_assignment()
    end
    return self:parse_expression_stmt()
end

--- Parse an assignment: NAME EQUALS expression [NEWLINE]
function Parser:parse_assignment()
    local name_token = self:expect(parser.TOKEN_NAME)
    local target = NameNode.new(name_token.value)
    self:expect(parser.TOKEN_EQUALS)
    local value = self:parse_expression()
    if not self:at_end() then
        self:expect(parser.TOKEN_NEWLINE)
    end
    return Assignment.new(target, value)
end

--- Parse an expression statement: expression [NEWLINE]
function Parser:parse_expression_stmt()
    local expr = self:parse_expression()
    if not self:at_end() then
        self:expect(parser.TOKEN_NEWLINE)
    end
    return ExpressionStmt.new(expr)
end

--- Parse an expression: term ((PLUS | MINUS) term)*
-- This is the lowest-precedence level, handling addition and subtraction.
function Parser:parse_expression()
    local left = self:parse_term()
    while true do
        local op_tok = self:match(parser.TOKEN_PLUS, parser.TOKEN_MINUS)
        if not op_tok then break end
        local right = self:parse_term()
        left = BinaryOp.new(left, op_tok.value, right)
    end
    return left
end

--- Parse a term: factor ((STAR | SLASH) factor)*
-- This is the higher-precedence level, handling multiplication and division.
function Parser:parse_term()
    local left = self:parse_factor()
    while true do
        local op_tok = self:match(parser.TOKEN_STAR, parser.TOKEN_SLASH)
        if not op_tok then break end
        local right = self:parse_factor()
        left = BinaryOp.new(left, op_tok.value, right)
    end
    return left
end

--- Parse a factor: the highest-precedence level.
-- Handles atomic expressions: numbers, strings, names, and parenthesized groups.
--
-- Grammar:
--   factor = NUMBER | STRING | NAME | '(' expression ')'
function Parser:parse_factor()
    local token = self:peek()

    if token.type == parser.TOKEN_NUMBER then
        self:advance()
        return NumberLiteral.new(tonumber(token.value))
    end

    if token.type == parser.TOKEN_STRING then
        self:advance()
        return StringLiteral.new(token.value)
    end

    if token.type == parser.TOKEN_NAME then
        self:advance()
        return NameNode.new(token.value)
    end

    if token.type == parser.TOKEN_LPAREN then
        self:advance()
        local expr = self:parse_expression()
        self:expect(parser.TOKEN_RPAREN)
        return expr
    end

    error(ParseError.new(
        string.format("Unexpected token %s (%q)",
            type_name_map[token.type] or tostring(token.type),
            token.value),
        token
    ))
end

parser.Parser = Parser

-- =========================================================================
-- ASTNode (grammar-driven parser)
-- =========================================================================
--
-- A generic AST node produced by the grammar-driven parser. Each node
-- stores the name of the grammar rule that produced it and a list of
-- children, which can be other ASTNode instances or lexer Token tables.
--
-- Leaf nodes wrap a single token. The is_leaf() and token() methods
-- make it easy to distinguish leaves from internal nodes when walking
-- the tree.

local ASTNode = {}
ASTNode.__index = ASTNode

function ASTNode.new(rule_name, children, start_line, start_column, end_line, end_column)
    return setmetatable({
        rule_name    = rule_name,
        children     = children or {},
        start_line   = start_line,
        start_column = start_column,
        end_line     = end_line,
        end_column   = end_column,
    }, ASTNode)
end

--- Check if this node is a leaf (wraps exactly one token).
-- A leaf node has exactly one child, and that child is a token table
-- (not another ASTNode). We detect tokens by checking for the absence
-- of the rule_name field.
function ASTNode:is_leaf()
    if #self.children == 1 then
        local child = self.children[1]
        -- Tokens are plain tables without rule_name; ASTNodes have rule_name
        return getmetatable(child) ~= ASTNode
    end
    return false
end

--- Return the wrapped token if this is a leaf node, nil otherwise.
function ASTNode:token()
    if self:is_leaf() then
        return self.children[1]
    end
    return nil
end

parser.ASTNode = ASTNode

-- =========================================================================
-- GrammarParseError
-- =========================================================================
--
-- Raised when the grammar-driven parser fails. Includes the position
-- (line:column) from the token where the failure occurred.

local GrammarParseError = {}
GrammarParseError.__index = GrammarParseError

function GrammarParseError.new(message, token)
    local self = setmetatable({}, GrammarParseError)
    self.message = message
    self.tok = token
    return self
end

function GrammarParseError:error_string()
    if self.tok and self.tok.line then
        -- Tokens may use either 'column' or 'col' depending on the lexer.
        local col = self.tok.column or self.tok.col or 0
        return string.format("Parse error at %d:%d: %s",
            self.tok.line, col, self.message)
    else
        return string.format("Parse error: %s", self.message)
    end
end

GrammarParseError.__tostring = GrammarParseError.error_string

parser.GrammarParseError = GrammarParseError

-- =========================================================================
-- GrammarParser
-- =========================================================================
--
-- # How grammar-driven parsing works
--
-- Instead of writing parsing code by hand, the GrammarParser reads grammar
-- rules (BNF-like specifications produced by grammar-tools) and interprets
-- them at runtime.
--
-- A grammar looks like:
--
--   program = { statement } ;
--   statement = assignment | expression_stmt ;
--   assignment = NAME EQUALS expression ;
--
-- The parser walks the grammar rules, trying to match them against the
-- token stream. When a rule matches, it produces an ASTNode. When it
-- doesn't, the parser backtracks and tries alternatives.
--
-- # Packrat memoization
--
-- Naive recursive descent with backtracking can be exponentially slow.
-- Packrat memoization fixes this: every time we try a rule at a position,
-- we cache the result. If we try the same rule at the same position again
-- (due to backtracking), we return the cached result immediately.
--
-- The memo table uses a composite key: (rule_index, position).
-- This guarantees O(n * g) time where n = token count, g = grammar size.
--
-- # Newline significance
--
-- Some grammars use NEWLINE as a meaningful token (e.g. Python-like
-- languages where newlines terminate statements). Others don't mention
-- NEWLINE at all (e.g. C-like languages with semicolons).
--
-- The parser auto-detects this: if any grammar rule references NEWLINE,
-- newlines are "significant" and preserved. Otherwise, the parser
-- automatically skips newline tokens when matching.
--
-- # Trace mode
--
-- When trace=true, the parser prints each rule attempt to stderr:
--
--   [TRACE] rule 'expr' at token 3 (NUMBER "42") -> match
--   [TRACE] rule 'stmt' at token 3 (NUMBER "42") -> fail
--
-- This is invaluable for debugging grammar problems.

local GrammarParser = {}
GrammarParser.__index = GrammarParser

--- Create a new grammar-driven parser.
-- @param tokens   array of token tables
-- @param grammar  a ParserGrammar table with a .rules array
-- @return GrammarParser instance
function GrammarParser.new(tokens, grammar)
    return GrammarParser.new_with_trace(tokens, grammar, false)
end

--- Create a grammar-driven parser with optional trace output.
-- @param tokens   array of token tables
-- @param grammar  a ParserGrammar table with a .rules array
-- @param trace    boolean, when true prints rule attempts to stderr
-- @return GrammarParser instance
function GrammarParser.new_with_trace(tokens, grammar, trace)
    -- Build lookup tables for rules by name and by index
    local rules = {}
    local rule_index = {}
    for i, rule in ipairs(grammar.rules) do
        rules[rule.name] = rule
        rule_index[rule.name] = i
    end

    local p = setmetatable({
        tokens = tokens,
        grammar = grammar,
        pos = 1,  -- 1-indexed
        rules = rules,
        rule_index = rule_index,
        newlines_significant = false,
        memo = {},            -- key: "ruleIdx:pos" -> memoEntry
        furthest_pos = 0,
        furthest_expected = {},
        trace = trace or false,
    }, GrammarParser)

    p.newlines_significant = p:_grammar_references_newline()
    return p
end

--- Returns whether newlines are significant in this grammar.
function GrammarParser:newlines_are_significant()
    return self.newlines_significant
end

--- Get the token at the current position.
function GrammarParser:current()
    if self.pos <= #self.tokens then
        return self.tokens[self.pos]
    end
    return self.tokens[#self.tokens]
end

--- Record a failure at the current position for error reporting.
-- The parser tracks the "furthest" position reached and what was expected
-- there. This gives much better error messages than just "parse failed".
function GrammarParser:_record_failure(expected)
    if self.pos > self.furthest_pos then
        self.furthest_pos = self.pos
        self.furthest_expected = { expected }
    elseif self.pos == self.furthest_pos then
        -- Avoid duplicates
        for _, e in ipairs(self.furthest_expected) do
            if e == expected then return end
        end
        self.furthest_expected[#self.furthest_expected + 1] = expected
    end
end

--- Check if any grammar rule references NEWLINE.
-- Walks all rule bodies recursively looking for NEWLINE token references.
function GrammarParser:_grammar_references_newline()
    for _, rule in ipairs(self.grammar.rules) do
        if self:_element_references_newline(rule.body) then
            return true
        end
    end
    return false
end

--- Recursively check if a grammar element references NEWLINE.
-- Grammar elements come in several kinds (from grammar-tools):
--
--   type="rule_reference"  -- references a rule or token by name
--   type="literal"         -- matches a literal string value
--   type="sequence"        -- all elements must match in order
--   type="alternation"     -- one of several choices must match
--   type="repetition"      -- zero or more matches of the element
--   type="optional"        -- zero or one match of the element
--   type="group"           -- parenthesized grouping
--
-- Note: grammar-tools (grammar_tools package) uses the 'type' field for
-- grammar element discrimination, NOT 'kind'. The 'kind' field is used by
-- AST nodes (ASTNode). Do not confuse the two.
function GrammarParser:_element_references_newline(element)
    if element.type == "rule_reference" then
        return element.is_token and element.name == "NEWLINE"
    elseif element.type == "sequence" then
        for _, sub in ipairs(element.elements) do
            if self:_element_references_newline(sub) then
                return true
            end
        end
    elseif element.type == "alternation" then
        for _, choice in ipairs(element.choices) do
            if self:_element_references_newline(choice) then
                return true
            end
        end
    elseif element.type == "repetition"
        or element.type == "optional"
        or element.type == "group"
        or element.type == "positive_lookahead"
        or element.type == "negative_lookahead"
        or element.type == "one_or_more" then
        return self:_element_references_newline(element.element)
    elseif element.type == "separated_repetition" then
        return self:_element_references_newline(element.element)
            or self:_element_references_newline(element.separator)
    end
    return false
end

--- Parse the token stream using the first grammar rule as entry point.
-- @return ASTNode, nil on success
-- @return nil, error_string on failure
function GrammarParser:parse()
    if #self.grammar.rules == 0 then
        return nil, "Grammar has no rules"
    end

    local entry_rule = self.grammar.rules[1]
    local result = self:_parse_rule(entry_rule.name)

    if result == nil then
        local tok = self:current()
        if #self.furthest_expected > 0 then
            local expected = table.concat(self.furthest_expected, " or ")
            return nil, GrammarParseError.new(
                string.format("Expected %s, got %q", expected, tok.value),
                tok
            ):error_string()
        end
        return nil, GrammarParseError.new("Failed to parse", tok):error_string()
    end

    -- Skip trailing newlines
    while self.pos <= #self.tokens
          and parser.token_type_name(self:current()) == "NEWLINE" do
        self.pos = self.pos + 1
    end

    -- Check for unconsumed tokens
    if self.pos <= #self.tokens
       and parser.token_type_name(self:current()) ~= "EOF" then
        local tok = self:current()
        if #self.furthest_expected > 0 and self.furthest_pos > self.pos then
            local expected = table.concat(self.furthest_expected, " or ")
            local furthest_tok = tok
            if self.furthest_pos <= #self.tokens then
                furthest_tok = self.tokens[self.furthest_pos]
            end
            return nil, GrammarParseError.new(
                string.format("Expected %s, got %q", expected, furthest_tok.value),
                furthest_tok
            ):error_string()
        end
        return nil, GrammarParseError.new(
            string.format("Unexpected token: %q", tok.value),
            tok
        ):error_string()
    end

    return result, nil
end

--- Attempt to parse a named rule at the current position.
-- Uses packrat memoization with the seed-and-grow left-recursion technique.
function GrammarParser:_parse_rule(rule_name)
    local rule = self.rules[rule_name]
    if not rule then return nil end

    -- Check memo cache
    local idx = self.rule_index[rule_name]
    if idx then
        local key = idx .. ":" .. self.pos
        local entry = self.memo[key]
        if entry then
            self.pos = entry.end_pos
            if not entry.ok then
                return nil
            end
            return ASTNode.new(rule_name, entry.children)
        end
    end

    -- Trace output
    if self.trace then
        local tok = self:current()
        io.stderr:write(string.format("[TRACE] rule '%s' at token %d (%s %q)",
            rule_name, self.pos, parser.token_type_name(tok), tok.value))
    end

    local start_pos = self.pos
    local key

    -- Seed the memo table before parsing to break left-recursive cycles.
    -- If this rule references itself at the same input position, the memo
    -- lookup above will see this failure entry and terminate that branch.
    if idx then
        key = idx .. ":" .. start_pos
        self.memo[key] = {
            children = nil,
            end_pos = start_pos,
            ok = false,
        }
    end

    local children, ok = self:_match_element(rule.body)

    -- Cache result
    if idx then
        self.memo[key] = {
            children = children,
            end_pos = self.pos,
            ok = ok,
        }

        -- If the initial parse succeeded, repeatedly re-parse with the best
        -- result cached so left-recursive alternatives can consume more input.
        if ok then
            while true do
                local prev_end = self.pos
                self.pos = start_pos
                self.memo[key] = {
                    children = children,
                    end_pos = prev_end,
                    ok = true,
                }

                local new_children, new_ok = self:_match_element(rule.body)
                if not new_ok or self.pos <= prev_end then
                    self.pos = prev_end
                    self.memo[key] = {
                        children = children,
                        end_pos = prev_end,
                        ok = true,
                    }
                    break
                end

                children = new_children
            end
        end
    end

    if not ok then
        if self.trace then
            io.stderr:write(" -> fail\n")
        end
        self.pos = start_pos
        self:_record_failure(rule_name)
        return nil
    end

    if self.trace then
        io.stderr:write(" -> match\n")
    end

    -- Normalize nil children to empty table
    if children == nil then
        children = {}
    end

    -- Compute position info from child tokens
    local first_tok = parser._find_first_token(children)
    local last_tok = parser._find_last_token(children)
    if first_tok and last_tok then
        return ASTNode.new(rule_name, children,
            first_tok.line, first_tok.column or first_tok.col,
            last_tok.line, last_tok.column or last_tok.col)
    end
    return ASTNode.new(rule_name, children)
end

--- Match a grammar element against the token stream.
-- This is the core dispatch function for the grammar interpreter.
-- Returns (children_array, true) on match, (nil, false) on failure.
--
-- The element kinds and their matching logic:
--
--   sequence    -> all sub-elements must match in order
--   alternation -> try each choice; first match wins
--   repetition  -> match zero or more times (always succeeds)
--   optional    -> match zero or one time (always succeeds)
--   group       -> match the inner element (just grouping)
--   rule_reference -> if is_token: match a token; else: recurse into rule
--   literal     -> match a token by its value string
function GrammarParser:_match_element(element)
    local save_pos = self.pos

    if element.type == "sequence" then
        local children = {}
        for _, sub in ipairs(element.elements) do
            local res, ok = self:_match_element(sub)
            if not ok then
                self.pos = save_pos
                return nil, false
            end
            for _, child in ipairs(res) do
                children[#children + 1] = child
            end
        end
        return children, true

    elseif element.type == "alternation" then
        for _, choice in ipairs(element.choices) do
            self.pos = save_pos
            local res, ok = self:_match_element(choice)
            if ok then
                return res, true
            end
        end
        self.pos = save_pos
        return nil, false

    elseif element.type == "repetition" then
        -- Zero or more matches. Always succeeds (with zero matches minimum).
        local children = {}
        while true do
            local save_rep = self.pos
            local res, ok = self:_match_element(element.element)
            if not ok then
                self.pos = save_rep
                break
            end
            for _, child in ipairs(res) do
                children[#children + 1] = child
            end
        end
        return children, true

    elseif element.type == "optional" then
        -- Zero or one match. Always succeeds.
        local res, ok = self:_match_element(element.element)
        if not ok then
            return {}, true
        end
        return res, true

    elseif element.type == "group" then
        return self:_match_element(element.element)

    elseif element.type == "rule_reference" then
        if element.is_token then
            return self:_match_token_reference(element)
        end
        local node = self:_parse_rule(element.name)
        if node then
            return { node }, true
        end
        self.pos = save_pos
        return nil, false

    elseif element.type == "literal" then
        local token = self:current()
        -- Skip insignificant newlines before literal matching
        if not self.newlines_significant then
            while parser.token_type_name(token) == "NEWLINE" do
                self.pos = self.pos + 1
                token = self:current()
            end
        end
        if token.value == element.value then
            self.pos = self.pos + 1
            return { token }, true
        end
        self:_record_failure(string.format("%q", element.value))
        return nil, false

    -- ---------------------------------------------------------------
    -- Extension: Syntactic predicates (lookahead without consuming)
    -- ---------------------------------------------------------------

    elseif element.type == "positive_lookahead" then
        -- Succeed if inner element matches, but consume no input.
        local res, ok = self:_match_element(element.element)
        self.pos = save_pos
        if ok then
            return {}, true
        end
        return nil, false

    elseif element.type == "negative_lookahead" then
        -- Succeed if inner element does NOT match, consume no input.
        local res, ok = self:_match_element(element.element)
        self.pos = save_pos
        if ok then
            return nil, false
        end
        return {}, true

    -- ---------------------------------------------------------------
    -- Extension: One-or-more repetition
    -- ---------------------------------------------------------------

    elseif element.type == "one_or_more" then
        -- Match one required, then zero or more additional.
        local first, first_ok = self:_match_element(element.element)
        if not first_ok then
            self.pos = save_pos
            return nil, false
        end
        local children = {}
        for _, child in ipairs(first) do
            children[#children + 1] = child
        end
        while true do
            local save_rep = self.pos
            local res, ok = self:_match_element(element.element)
            if not ok then
                self.pos = save_rep
                break
            end
            for _, child in ipairs(res) do
                children[#children + 1] = child
            end
        end
        return children, true

    -- ---------------------------------------------------------------
    -- Extension: Separated repetition
    -- ---------------------------------------------------------------

    elseif element.type == "separated_repetition" then
        -- Match: element { separator element }
        local first, first_ok = self:_match_element(element.element)
        if not first_ok then
            self.pos = save_pos
            if element.at_least_one then return nil, false end
            return {}, true  -- zero occurrences is valid
        end
        local children = {}
        for _, child in ipairs(first) do
            children[#children + 1] = child
        end
        while true do
            local save_sep = self.pos
            local sep, sep_ok = self:_match_element(element.separator)
            if not sep_ok then
                self.pos = save_sep
                break
            end
            local nxt, nxt_ok = self:_match_element(element.element)
            if not nxt_ok then
                self.pos = save_sep
                break
            end
            for _, child in ipairs(sep) do
                children[#children + 1] = child
            end
            for _, child in ipairs(nxt) do
                children[#children + 1] = child
            end
        end
        return children, true
    end

    return nil, false
end

--- Match a token reference (UPPERCASE name in the grammar).
-- Handles newline skipping for insignificant newlines.
-- Uses both string-based type_name matching and backward-compatible
-- numeric type matching.
function GrammarParser:_match_token_reference(element)
    local token = self:current()

    -- Skip newlines when matching non-NEWLINE tokens and newlines are insignificant
    if not self.newlines_significant and element.name ~= "NEWLINE" then
        while parser.token_type_name(token) == "NEWLINE" do
            self.pos = self.pos + 1
            token = self:current()
        end
    end

    local type_name = parser.token_type_name(token)

    -- Direct string comparison (works for both string and enum types)
    if type_name == element.name then
        self.pos = self.pos + 1
        return { token }, true
    end

    -- Grammar-driven lexers may promote keywords to their specific token
    -- names ("var" -> VAR, "puts" -> PUTS) while parser grammars still
    -- reference the broader KEYWORD token class.
    if element.name == "KEYWORD"
       and type_name ~= "KEYWORD"
       and type(token.value) == "string"
       and token.value:match("^[%a_][%w_]*$")
       and type_name == token.value:upper() then
        self.pos = self.pos + 1
        return { token }, true
    end

    -- Backward compatibility: try enum-based matching
    local expected_type = string_to_token_type[element.name]
    if expected_type and token.type == expected_type
       and expected_type ~= parser.TOKEN_NAME then
        self.pos = self.pos + 1
        return { token }, true
    end

    -- Keyword category matching: grammar-driven lexers promote keyword
    -- names to specific string types (e.g. "var" → type "VAR", "let" →
    -- "LET").  When the grammar references the generic KEYWORD token, we
    -- must still match these promoted types.  A promoted keyword has a
    -- string `type` field that is NOT one of the standard token names
    -- (NAME, NUMBER, PLUS, …).  If the token's string type is absent
    -- from string_to_token_type it was produced by keyword promotion.
    if element.name == "KEYWORD"
       and type(token.type) == "string"
       and not string_to_token_type[token.type] then
        self.pos = self.pos + 1
        return { token }, true
    end

    self:_record_failure(element.name)
    return nil, false
end

parser.GrammarParser = GrammarParser

-- =========================================================================
-- AST Position Computation Helpers
-- =========================================================================
--
-- These find the first and last leaf tokens in a children array,
-- walking into ASTNode children recursively.

--- Find the first token in a children array (depth-first).
-- @param children array  List of ASTNode/Token children.
-- @return Token|nil
function parser._find_first_token(children)
    for _, child in ipairs(children) do
        if getmetatable(child) == ASTNode then
            local tok = parser._find_first_token(child.children)
            if tok then return tok end
        else
            return child
        end
    end
    return nil
end

--- Find the last token in a children array (depth-first, reverse).
-- @param children array  List of ASTNode/Token children.
-- @return Token|nil
function parser._find_last_token(children)
    for i = #children, 1, -1 do
        local child = children[i]
        if getmetatable(child) == ASTNode then
            local tok = parser._find_last_token(child.children)
            if tok then return tok end
        else
            return child
        end
    end
    return nil
end

-- =========================================================================
-- AST Walking Utilities
-- =========================================================================
--
-- Generic tree traversal functions for grammar-driven ASTs.

--- Check if a child element is an ASTNode (not a Token).
-- @param child ASTNode|Token
-- @return boolean
function parser.is_ast_node(child)
    return getmetatable(child) == ASTNode
end

--- Depth-first walk of an AST tree with enter/leave visitor callbacks.
--
-- The visitor table may have:
--   enter(node, parent) -> ASTNode|nil  (replacement or nil to keep)
--   leave(node, parent) -> ASTNode|nil  (replacement or nil to keep)
--
-- Token children are not visited -- only ASTNode children are walked.
--
-- @param node ASTNode  The root node to walk.
-- @param visitor table  Visitor with optional enter/leave functions.
-- @return ASTNode  The (possibly replaced) root node.
function parser.walk_ast(node, visitor)
    return parser._walk_node(node, nil, visitor)
end

function parser._walk_node(node, parent, visitor)
    -- Enter phase
    local current = node
    if visitor.enter then
        local replacement = visitor.enter(current, parent)
        if replacement ~= nil then
            current = replacement
        end
    end

    -- Walk children recursively
    local children_changed = false
    local new_children = {}
    for _, child in ipairs(current.children) do
        if parser.is_ast_node(child) then
            local walked = parser._walk_node(child, current, visitor)
            if walked ~= child then children_changed = true end
            new_children[#new_children + 1] = walked
        else
            new_children[#new_children + 1] = child
        end
    end

    -- If children changed, create a new node
    if children_changed then
        current = ASTNode.new(current.rule_name, new_children,
            current.start_line, current.start_column,
            current.end_line, current.end_column)
    end

    -- Leave phase
    if visitor.leave then
        local replacement = visitor.leave(current, parent)
        if replacement ~= nil then
            current = replacement
        end
    end

    return current
end

--- Find all nodes matching a rule name (depth-first order).
-- @param node ASTNode  The root node to search.
-- @param rule_name string  The rule name to match.
-- @return table  Array of matching ASTNode instances.
function parser.find_nodes(node, rule_name)
    local results = {}
    parser.walk_ast(node, {
        enter = function(n)
            if n.rule_name == rule_name then
                results[#results + 1] = n
            end
        end,
    })
    return results
end

--- Collect all tokens in depth-first order, optionally filtered by type.
-- @param node ASTNode  The root node.
-- @param token_type string|nil  Optional type filter.
-- @return table  Array of Token tables.
function parser.collect_tokens(node, token_type)
    local results = {}
    local function walk(n)
        for _, child in ipairs(n.children) do
            if parser.is_ast_node(child) then
                walk(child)
            else
                if token_type == nil or parser.token_type_name(child) == token_type then
                    results[#results + 1] = child
                end
            end
        end
    end
    walk(node)
    return results
end

return parser
