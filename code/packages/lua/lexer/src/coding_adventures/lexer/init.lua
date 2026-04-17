-- lexer -- Character-by-character analysis translating source bytes to typed tokens
-- ================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- Layer 2 in the computing stack.
--
-- # What is a lexer?
--
-- A lexer (also called a tokenizer or scanner) is the first phase of any
-- language implementation. It reads raw source text character by character
-- and groups those characters into *tokens* -- the smallest meaningful units
-- of the language.
--
-- For example, the source text "x = 42" becomes three tokens:
--
--     Token(Name, "x", 1:1)
--     Token(Equals, "=", 1:3)
--     Token(Number, "42", 1:5)
--
-- # Architecture
--
-- This package provides two lexers:
--
--   1. **Lexer** (hand-written) -- A character-by-character tokenizer with
--      a dispatch DFA. The DFA classifies each character and dispatches to
--      the appropriate sub-routine (readNumber, readName, readString, etc.).
--
--   2. **GrammarLexer** (grammar-driven) -- A regex-based tokenizer driven
--      by a TokenGrammar object (parsed from a .tokens file). It compiles
--      token definitions into regexes and tries them in priority order.
--
-- Both produce the same Token objects.
--
-- # OOP Pattern
--
-- We use the standard Lua metatable OOP pattern:
--
--     local Lexer = {}
--     Lexer.__index = Lexer
--     function Lexer.new(...) ... end
--
-- This gives us method dispatch via the : operator:
--
--     local lex = Lexer.new("x = 42", config)
--     local tokens = lex:tokenize()
--
-- # Dependencies
--
-- - state_machine: DFA for the tokenizer dispatch logic
-- - grammar_tools: TokenGrammar for the grammar-driven lexer (when available)

local state_machine = require("coding_adventures.state_machine")
local DFA = state_machine.DFA

-- =========================================================================
-- Token Types
-- =========================================================================
--
-- Each token has a numeric type that classifies it. These correspond to the
-- Go implementation's TokenType enum.
--
-- Token type table:
--
--   Value  Name           Example
--   -----  ----           -------
--   0      Name           x, foo, myVar
--   1      Number         42, 0, 999
--   2      String         "hello"
--   3      Keyword        if, while, def
--   4      Plus           +
--   5      Minus          -
--   6      Star           *
--   7      Slash          /
--   8      Equals         =
--   9      EqualsEquals   ==
--   10     LParen         (
--   11     RParen         )
--   12     Comma          ,
--   13     Colon          :
--   14     Semicolon      ;
--   15     LBrace         {
--   16     RBrace         }
--   17     LBracket       [
--   18     RBracket       ]
--   19     Dot            .
--   20     Bang           !
--   21     Newline        \n
--   22     EOF            (end of input)

local TokenType = {
    Name         = 0,
    Number       = 1,
    String       = 2,
    Keyword      = 3,
    Plus         = 4,
    Minus        = 5,
    Star         = 6,
    Slash        = 7,
    Equals       = 8,
    EqualsEquals = 9,
    LParen       = 10,
    RParen       = 11,
    Comma        = 12,
    Colon        = 13,
    Semicolon    = 14,
    LBrace       = 15,
    RBrace       = 16,
    LBracket     = 17,
    RBracket     = 18,
    Dot          = 19,
    Bang         = 20,
    Newline      = 21,
    EOF          = 22,
}

--- Map from token type number to human-readable name.
-- Used by Token:__tostring() for debugging output.
local token_type_names = {
    [0]  = "Name",
    [1]  = "Number",
    [2]  = "String",
    [3]  = "Keyword",
    [4]  = "Plus",
    [5]  = "Minus",
    [6]  = "Star",
    [7]  = "Slash",
    [8]  = "Equals",
    [9]  = "EqualsEquals",
    [10] = "LParen",
    [11] = "RParen",
    [12] = "Comma",
    [13] = "Colon",
    [14] = "Semicolon",
    [15] = "LBrace",
    [16] = "RBrace",
    [17] = "LBracket",
    [18] = "RBracket",
    [19] = "Dot",
    [20] = "Bang",
    [21] = "Newline",
    [22] = "EOF",
}

--- Convert a token type number to its name string.
-- @param t number The token type number.
-- @return string The name, or "Unknown" if not found.
local function token_type_to_string(t)
    return token_type_names[t] or "Unknown"
end

-- =========================================================================
-- Token
-- =========================================================================
--
-- A Token is the output unit of the lexer. It carries:
--   - type:      numeric TokenType (for fast comparisons)
--   - value:     the matched text (or processed text for strings)
--   - line:      1-based line number where the token starts
--   - column:    1-based column number where the token starts
--   - type_name: grammar-driven token name (e.g. "INT", "FLOAT"), empty
--                for hand-written lexer tokens

-- =========================================================================
-- Token Flag Constants
-- =========================================================================
--
-- Bitmask flags for token metadata. Flags carry information that is neither
-- type nor value but affects how downstream consumers interpret a token.
--
-- Flags are optional — when nil or 0, all flags are off.
-- Use bitwise AND to test: (token.flags or 0) & TOKEN_PRECEDED_BY_NEWLINE

--- Set when a line break appeared between this token and the previous one.
-- Languages with automatic semicolon insertion (JavaScript, Go) use this.
local TOKEN_PRECEDED_BY_NEWLINE = 1

--- Set for context-sensitive keywords — words that are keywords in some
-- syntactic positions but identifiers in others.
-- Example: JavaScript's async, yield, await, get, set.
local TOKEN_CONTEXT_KEYWORD = 2

local Token = {}
Token.__index = Token

--- Create a new Token.
-- @param ttype number Token type (from TokenType).
-- @param value string The token's text value.
-- @param line number Line number (1-based).
-- @param column number Column number (1-based).
-- @param type_name string|nil Grammar-driven type name (optional).
-- @param flags number|nil Bitmask of TOKEN_* flags (optional).
-- @return Token
function Token.new(ttype, value, line, column, type_name, flags)
    return setmetatable({
        type      = ttype,
        value     = value,
        line      = line,
        column    = column,
        type_name = type_name or "",
        flags     = flags or 0,
    }, Token)
end

--- Human-readable string representation of the token.
-- Format matches the Go implementation: Token(TypeName, "value", line:col)
function Token:__tostring()
    return string.format(
        'Token(%s, %q, %d:%d)',
        token_type_to_string(self.type),
        self.value,
        self.line,
        self.column
    )
end

-- =========================================================================
-- Simple Token Map
-- =========================================================================
--
-- Maps single-character operators and delimiters to their token types.
-- These characters always produce a single token with no lookahead needed
-- (unlike '=' which requires checking for '==').

local simple_tokens = {
    ["+"] = TokenType.Plus,
    ["-"] = TokenType.Minus,
    ["*"] = TokenType.Star,
    ["/"] = TokenType.Slash,
    ["("] = TokenType.LParen,
    [")"] = TokenType.RParen,
    [","] = TokenType.Comma,
    [":"] = TokenType.Colon,
    [";"] = TokenType.Semicolon,
    ["{"] = TokenType.LBrace,
    ["}"] = TokenType.RBrace,
    ["["] = TokenType.LBracket,
    ["]"] = TokenType.RBracket,
    ["."] = TokenType.Dot,
    ["!"] = TokenType.Bang,
}

-- =========================================================================
-- Character Classification for the Tokenizer DFA
-- =========================================================================
--
-- The tokenizer DFA does NOT replace the tokenizer's logic. The sub-routines
-- like read_number() and read_string() still do the actual work. What the
-- DFA provides is a formal, verifiable model of the dispatch decision.
--
-- Character class table:
--
--   Class           Characters       Triggers
--   "eof"           end of input     EOF token
--   "whitespace"    space/tab/CR     skip whitespace
--   "newline"       \n               NEWLINE token
--   "digit"         0-9              read number
--   "alpha"         a-zA-Z           read name/keyword
--   "underscore"    _                read name/keyword
--   "quote"         "                read string literal
--   "equals"        =                lookahead for = vs ==
--   "operator"      +-*/             simple operator token
--   "open_paren"    (                LPAREN
--   "close_paren"   )                RPAREN
--   "comma"         ,                COMMA
--   "colon"         :                COLON
--   "semicolon"     ;                SEMICOLON
--   "open_brace"    {                LBRACE
--   "close_brace"   }                RBRACE
--   "open_bracket"  [                LBRACKET
--   "close_bracket" ]                RBRACKET
--   "dot"           .                DOT
--   "bang"          !                BANG
--   "other"         everything else  error

--- Classify a character into its character class for the tokenizer DFA.
-- @param ch string|nil The character (single byte string), or nil for EOF.
-- @return string The character class name.
local function classify_char(ch)
    if ch == nil then
        return "eof"
    end

    -- Whitespace: space, tab, carriage return
    if ch == " " or ch == "\t" or ch == "\r" then
        return "whitespace"
    end

    -- Newline
    if ch == "\n" then
        return "newline"
    end

    -- Digit: 0-9
    local byte = string.byte(ch)
    if byte >= 48 and byte <= 57 then  -- '0' = 48, '9' = 57
        return "digit"
    end

    -- Alpha: a-z, A-Z
    if (byte >= 65 and byte <= 90) or (byte >= 97 and byte <= 122) then
        return "alpha"
    end

    -- Underscore
    if ch == "_" then
        return "underscore"
    end

    -- Quote (double quote only for the hand-written lexer)
    if ch == '"' then
        return "quote"
    end

    -- Equals (needs lookahead for ==)
    if ch == "=" then
        return "equals"
    end

    -- Arithmetic operators
    if ch == "+" or ch == "-" or ch == "*" or ch == "/" then
        return "operator"
    end

    -- Delimiters and punctuation
    if ch == "(" then return "open_paren" end
    if ch == ")" then return "close_paren" end
    if ch == "," then return "comma" end
    if ch == ":" then return "colon" end
    if ch == ";" then return "semicolon" end
    if ch == "{" then return "open_brace" end
    if ch == "}" then return "close_brace" end
    if ch == "[" then return "open_bracket" end
    if ch == "]" then return "close_bracket" end
    if ch == "." then return "dot" end
    if ch == "!" then return "bang" end

    return "other"
end

-- =========================================================================
-- Tokenizer DFA Construction
-- =========================================================================
--
-- # States
--
--   - "start"         -- idle, examining the next character
--   - "in_number"     -- reading a sequence of digits
--   - "in_name"       -- reading an identifier
--   - "in_string"     -- reading a string literal
--   - "in_operator"   -- emitting a single-character operator/delimiter
--   - "in_equals"     -- handling = with lookahead for ==
--   - "at_newline"    -- emitting a NEWLINE token
--   - "at_whitespace" -- skipping whitespace
--   - "done"          -- end of input
--   - "error"         -- unexpected character

local tokenizer_dfa_states = {
    "start", "in_number", "in_name", "in_string",
    "in_operator", "in_equals", "at_newline", "at_whitespace",
    "done", "error",
}

local tokenizer_dfa_alphabet = {
    "digit", "alpha", "underscore", "quote", "newline", "whitespace",
    "operator", "equals", "open_paren", "close_paren", "comma", "colon",
    "semicolon", "open_brace", "close_brace", "open_bracket",
    "close_bracket", "dot", "bang", "eof", "other",
}

--- Maps each character class to its target state from "start".
local start_dispatch = {
    digit         = "in_number",
    alpha         = "in_name",
    underscore    = "in_name",
    quote         = "in_string",
    newline       = "at_newline",
    whitespace    = "at_whitespace",
    operator      = "in_operator",
    equals        = "in_equals",
    open_paren    = "in_operator",
    close_paren   = "in_operator",
    comma         = "in_operator",
    colon         = "in_operator",
    semicolon     = "in_operator",
    open_brace    = "in_operator",
    close_brace   = "in_operator",
    open_bracket  = "in_operator",
    close_bracket = "in_operator",
    dot           = "in_operator",
    bang          = "in_operator",
    eof           = "done",
    other         = "error",
}

--- Build the full transition map for the tokenizer DFA.
--
-- The transition map has the structure:
--   transitions["state\0event"] = target_state
--
-- From "start", dispatch based on character class. All handler states
-- return to "start" on every symbol. "done" and "error" loop on themselves.
--
-- @return table Transition map compatible with state_machine.DFA.
local function build_tokenizer_dfa_transitions()
    local transitions = {}

    -- From "start", dispatch based on character class.
    for char_class, target in pairs(start_dispatch) do
        transitions[{ "start", char_class }] = target
    end

    -- All handler states return to "start" on every symbol.
    local handlers = {
        "in_number", "in_name", "in_string", "in_operator",
        "in_equals", "at_newline", "at_whitespace",
    }
    for _, handler in ipairs(handlers) do
        for _, symbol in ipairs(tokenizer_dfa_alphabet) do
            transitions[{ handler, symbol }] = "start"
        end
    end

    -- "done" loops on itself for every symbol.
    for _, symbol in ipairs(tokenizer_dfa_alphabet) do
        transitions[{ "done", symbol }] = "done"
    end

    -- "error" loops on itself for every symbol.
    for _, symbol in ipairs(tokenizer_dfa_alphabet) do
        transitions[{ "error", symbol }] = "error"
    end

    return transitions
end

--- Create a new instance of the tokenizer dispatch DFA.
--
-- Each call returns a fresh DFA so callers can process independently.
-- The DFA models the top-level character classification dispatch of the
-- hand-written tokenizer.
--
-- @return DFA A state_machine.DFA instance.
local function new_tokenizer_dfa()
    return DFA.new(
        tokenizer_dfa_states,
        tokenizer_dfa_alphabet,
        build_tokenizer_dfa_transitions(),
        "start",
        { "done" },
        nil
    )
end

-- =========================================================================
-- Hand-Written Lexer
-- =========================================================================
--
-- The hand-written lexer processes source text character by character,
-- using the tokenizer DFA to dispatch to the appropriate sub-routine
-- for each token type. It supports:
--
--   - Identifiers (names) and keywords
--   - Integer numbers
--   - Double-quoted string literals with escape sequences
--   - Single-character operators and delimiters
--   - The = / == distinction via one-character lookahead
--   - Whitespace skipping and newline tokens

local Lexer = {}
Lexer.__index = Lexer

--- Create a new hand-written Lexer.
--
-- @param source string The source text to tokenize.
-- @param config table|nil Configuration with optional `keywords` array.
-- @return Lexer
function Lexer.new(source, config)
    config = config or {}
    local keywords_set = {}
    if config.keywords then
        for _, kw in ipairs(config.keywords) do
            keywords_set[kw] = true
        end
    end
    return setmetatable({
        _source       = source,
        _config       = config,
        _pos          = 1,       -- 1-based index into source
        _line         = 1,
        _column       = 1,
        _tokens       = {},
        _keywords_set = keywords_set,
    }, Lexer)
end

--- Return the current character, or nil if at end of input.
-- @return string|nil
function Lexer:_current_char()
    if self._pos <= #self._source then
        return self._source:sub(self._pos, self._pos)
    end
    return nil
end

--- Return the next character (one ahead of current), or nil if past end.
-- @return string|nil
function Lexer:_peek()
    local next_pos = self._pos + 1
    if next_pos <= #self._source then
        return self._source:sub(next_pos, next_pos)
    end
    return nil
end

--- Advance past the current character and return it.
-- Updates line/column tracking: newlines reset column to 1 and
-- increment line; all other characters increment column.
-- @return string The consumed character.
function Lexer:_advance()
    local ch = self._source:sub(self._pos, self._pos)
    self._pos = self._pos + 1
    if ch == "\n" then
        self._line = self._line + 1
        self._column = 1
    else
        self._column = self._column + 1
    end
    return ch
end

--- Skip whitespace (spaces, tabs, carriage returns).
-- Does NOT skip newlines -- those become NEWLINE tokens.
function Lexer:_skip_whitespace()
    while true do
        local ch = self:_current_char()
        if ch == nil or (ch ~= " " and ch ~= "\t" and ch ~= "\r") then
            break
        end
        self:_advance()
    end
end

--- Read a contiguous sequence of digits and return a Number token.
-- @return Token
function Lexer:_read_number()
    local start_line = self._line
    local start_col = self._column
    local chars = {}
    while true do
        local ch = self:_current_char()
        if ch == nil then break end
        local b = string.byte(ch)
        if b < 48 or b > 57 then break end  -- not a digit
        chars[#chars + 1] = self:_advance()
    end
    return Token.new(TokenType.Number, table.concat(chars), start_line, start_col)
end

--- Read an identifier (letters, digits, underscores) and return a Name
-- or Keyword token.
--
-- After reading the full identifier, we check whether it appears in the
-- keyword set. If so, we emit a Keyword token instead of a Name token.
--
-- @return Token
function Lexer:_read_name()
    local start_line = self._line
    local start_col = self._column
    local chars = {}
    while true do
        local ch = self:_current_char()
        if ch == nil then break end
        local b = string.byte(ch)
        -- Letter, digit, or underscore
        local is_letter = (b >= 65 and b <= 90) or (b >= 97 and b <= 122)
        local is_digit = (b >= 48 and b <= 57)
        local is_under = (ch == "_")
        if not (is_letter or is_digit or is_under) then break end
        chars[#chars + 1] = self:_advance()
    end
    local value = table.concat(chars)
    local ttype = TokenType.Name
    if self._keywords_set[value] then
        ttype = TokenType.Keyword
    end
    return Token.new(ttype, value, start_line, start_col)
end

--- Read a double-quoted string literal with escape sequence support.
--
-- Supported escape sequences:
--   \\n  -> newline
--   \\t  -> tab
--   \\\\  -> backslash
--   \\"  -> double quote
--   \\x  -> x (any other character is passed through)
--
-- Raises an error for unterminated strings.
-- @return Token
function Lexer:_read_string()
    local start_line = self._line
    local start_col = self._column
    local chars = {}
    self:_advance()  -- consume opening quote
    while true do
        local ch = self:_current_char()
        if ch == nil then
            error(string.format(
                "LexerError at %d:%d: Unterminated string literal",
                start_line, start_col
            ))
        end
        if ch == '"' then
            self:_advance()  -- consume closing quote
            break
        end
        if ch == '\\' then
            self:_advance()  -- consume backslash
            local escaped = self:_current_char()
            if escaped == nil then
                error(string.format(
                    "LexerError at %d:%d: Unterminated string literal (ends with backslash)",
                    start_line, start_col
                ))
            end
            if escaped == "n" then
                chars[#chars + 1] = "\n"
            elseif escaped == "t" then
                chars[#chars + 1] = "\t"
            elseif escaped == "\\" then
                chars[#chars + 1] = "\\"
            elseif escaped == '"' then
                chars[#chars + 1] = '"'
            else
                chars[#chars + 1] = escaped
            end
            self:_advance()
        else
            chars[#chars + 1] = ch
            self:_advance()
        end
    end
    return Token.new(TokenType.String, table.concat(chars), start_line, start_col)
end

--- Tokenize the source text and return an array of Token objects.
--
-- The main loop uses the tokenizer DFA to classify each character and
-- dispatch to the appropriate handler. After each handler completes,
-- the DFA is reset to "start" for the next character.
--
-- @return table Array of Token objects, ending with an EOF token.
function Lexer:tokenize()
    self._tokens = {}
    local dfa = new_tokenizer_dfa()

    while true do
        local ch = self:_current_char()
        local char_class = classify_char(ch)
        local next_state = dfa:process(char_class)

        if next_state == "at_whitespace" then
            self:_skip_whitespace()
        elseif next_state == "at_newline" then
            local t = Token.new(TokenType.Newline, "\\n", self._line, self._column)
            self:_advance()
            self._tokens[#self._tokens + 1] = t
        elseif next_state == "in_number" then
            self._tokens[#self._tokens + 1] = self:_read_number()
        elseif next_state == "in_name" then
            self._tokens[#self._tokens + 1] = self:_read_name()
        elseif next_state == "in_string" then
            self._tokens[#self._tokens + 1] = self:_read_string()
        elseif next_state == "in_equals" then
            local start_line = self._line
            local start_col = self._column
            self:_advance()
            local next_ch = self:_current_char()
            if next_ch == "=" then
                self:_advance()
                self._tokens[#self._tokens + 1] = Token.new(
                    TokenType.EqualsEquals, "==", start_line, start_col
                )
            else
                self._tokens[#self._tokens + 1] = Token.new(
                    TokenType.Equals, "=", start_line, start_col
                )
            end
        elseif next_state == "in_operator" then
            local ttype = simple_tokens[ch]
            local t = Token.new(ttype, ch, self._line, self._column)
            self:_advance()
            self._tokens[#self._tokens + 1] = t
        elseif next_state == "done" then
            break
        elseif next_state == "error" then
            error(string.format(
                "LexerError at %d:%d: Unexpected character %q",
                self._line, self._column, ch
            ))
        end

        -- Reset the DFA back to "start" for the next character.
        dfa:reset()
    end

    self._tokens[#self._tokens + 1] = Token.new(
        TokenType.EOF, "", self._line, self._column
    )
    return self._tokens
end

-- =========================================================================
-- Escape Processing (shared by GrammarLexer)
-- =========================================================================
--
-- Handles escape sequences in string literals: \n, \t, \\, \", and
-- pass-through for any other escaped character.

--- Process escape sequences in a string.
-- @param s string The raw string (without surrounding quotes).
-- @return string The string with escape sequences resolved.
local function process_escapes(s)
    local result = {}
    local i = 1
    while i <= #s do
        if s:sub(i, i) == '\\' and i + 1 <= #s then
            local next_ch = s:sub(i + 1, i + 1)
            if next_ch == 'n' then
                result[#result + 1] = '\n'
            elseif next_ch == 't' then
                result[#result + 1] = '\t'
            elseif next_ch == '\\' then
                result[#result + 1] = '\\'
            elseif next_ch == '"' then
                result[#result + 1] = '"'
            else
                result[#result + 1] = next_ch
            end
            i = i + 2
        else
            result[#result + 1] = s:sub(i, i)
            i = i + 1
        end
    end
    return table.concat(result)
end

-- =========================================================================
-- Lexer Context -- Callback Interface for Group Transitions
-- =========================================================================
--
-- LexerContext is the interface that on-token callbacks use to control the
-- grammar-driven lexer. When a callback is registered via
-- GrammarLexer:set_on_token(), it receives a LexerContext on every token
-- match.
--
-- Methods that modify state (push_group/pop_group/emit/suppress) take
-- effect after the callback returns -- they do not interrupt the current
-- match.

local LexerContext = {}
LexerContext.__index = LexerContext

--- Create a new LexerContext (internal -- called by GrammarLexer).
-- @param lexer GrammarLexer The lexer that created this context.
-- @param source string The complete source text.
-- @param pos_after number Position after the current token.
-- @param previous_token Token|nil The most recently emitted token.
-- @param current_token_line number Line number of the current token.
-- @return LexerContext
function LexerContext._new(lexer, source, pos_after, previous_token, current_token_line)
    return setmetatable({
        _lexer              = lexer,
        _source             = source,
        _pos_after          = pos_after,
        _suppressed         = false,
        _emitted            = {},
        _group_actions      = {},
        _skip_enabled       = nil,  -- nil = no change
        _previous_token     = previous_token,
        _current_token_line = current_token_line or 0,
    }, LexerContext)
end

--- Push a pattern group onto the group stack.
--
-- The pushed group becomes active for the next token match. Errors if the
-- group name is not defined in the grammar.
-- @param group_name string The name of the group to push.
function LexerContext:push_group(group_name)
    if not self._lexer._group_patterns[group_name] then
        local available = {}
        for name in pairs(self._lexer._group_patterns) do
            available[#available + 1] = name
        end
        error(string.format(
            'Unknown pattern group: %q. Available groups: %s',
            group_name,
            table.concat(available, ", ")
        ))
    end
    self._group_actions[#self._group_actions + 1] = { action = "push", group_name = group_name }
end

--- Pop the current group from the stack.
--
-- If only the default group remains, this is a no-op. The default group is
-- the floor of the stack and cannot be popped.
function LexerContext:pop_group()
    self._group_actions[#self._group_actions + 1] = { action = "pop", group_name = "" }
end

--- Return the name of the currently active group.
-- @return string The active group name (always at least "default").
function LexerContext:active_group()
    local stack = self._lexer._group_stack
    return stack[#stack]
end

--- Return the depth of the group stack (always >= 1).
-- @return number
function LexerContext:group_stack_depth()
    return #self._lexer._group_stack
end

--- Inject a synthetic token after the current one.
--
-- Emitted tokens do NOT trigger the callback (prevents infinite loops).
-- Multiple emit() calls produce tokens in call order.
-- @param token Token The token to inject.
function LexerContext:emit(token)
    self._emitted[#self._emitted + 1] = token
end

--- Suppress the current token -- it will not appear in the output.
--
-- Emitted tokens (from emit()) are still included even when the current
-- token is suppressed. This enables token replacement.
function LexerContext:suppress()
    self._suppressed = true
end

--- Read a source character past the current token.
--
-- offset=1 means the character immediately after the token. Returns an
-- empty string if the position is past EOF.
-- @param offset number Number of characters past the token end.
-- @return string Single character or empty string.
function LexerContext:peek(offset)
    local idx = self._pos_after + offset - 1
    if idx >= 1 and idx <= #self._source then
        return self._source:sub(idx, idx)
    end
    return ""
end

--- Read the next `length` characters past the current token.
-- @param length number How many characters to read.
-- @return string The substring (may be shorter if near EOF).
function LexerContext:peek_str(length)
    local start = self._pos_after
    local stop = start + length - 1
    if stop > #self._source then
        stop = #self._source
    end
    if start > #self._source then
        return ""
    end
    return self._source:sub(start, stop)
end

--- Toggle skip pattern processing.
--
-- When disabled, skip patterns (whitespace, comments) are not tried.
-- Useful for groups where whitespace is significant.
-- @param enabled boolean Whether to enable skip processing.
function LexerContext:set_skip_enabled(enabled)
    self._skip_enabled = enabled
end

-- -----------------------------------------------------------------------
-- Extension: Token Lookbehind
-- -----------------------------------------------------------------------

--- Return the most recently emitted token, or nil at the start of input.
--
-- "Emitted" means the token actually made it into the output list --
-- suppressed tokens are not counted. This provides lookbehind capability
-- for context-sensitive decisions.
--
-- For example, in JavaScript `/` is a regex literal after `=`, `(` or `,`
-- but a division operator after `)`, `]`, identifiers, or numbers.
-- @return Token|nil
function LexerContext:previous_token()
    return self._previous_token
end

-- -----------------------------------------------------------------------
-- Extension: Bracket Depth Tracking
-- -----------------------------------------------------------------------

--- Return the current nesting depth for a specific bracket type,
-- or the total depth across all types if no argument is given.
--
-- Depth starts at 0 and increments on each opener, decrements on each
-- closer. The count never goes below 0.
--
-- @param kind string|nil "paren", "bracket", or "brace"; nil for total.
-- @return number
function LexerContext:bracket_depth(kind)
    return self._lexer:bracket_depth(kind)
end

-- -----------------------------------------------------------------------
-- Extension: Newline Detection
-- -----------------------------------------------------------------------

--- Return true if a newline appeared between the previous token and the
-- current token (i.e., they are on different lines).
--
-- Used by languages with automatic semicolon insertion (JavaScript, Go).
-- Returns false if there is no previous token (start of input).
-- @return boolean
function LexerContext:preceded_by_newline()
    if self._previous_token == nil then return false end
    return self._previous_token.line < self._current_token_line
end

-- =========================================================================
-- Grammar-Driven Lexer
-- =========================================================================
--
-- Instead of hardcoded character-matching logic, this lexer:
--
--   1. Compiles each token definition's pattern into a Lua pattern or regex
--   2. At each position, tries each pattern in definition order (first match
--      wins)
--   3. Emits a Token with the matched type and value
--
-- Supports skip patterns, type aliases, reserved keywords, indentation
-- mode, pattern groups with stackable transitions, and on-token callbacks.
--
-- # TokenGrammar format
--
-- The grammar is a table with these fields:
--   - definitions:       array of {name, pattern, is_regex, alias}
--   - keywords:          array of keyword strings
--   - mode:              "indentation" or nil
--   - escape_mode:       "none" or nil
--   - skip_definitions:  array of {name, pattern, is_regex}
--   - reserved_keywords: array of reserved keyword strings
--   - groups:            table mapping group name to {definitions = {...}}

local GrammarLexer = {}
GrammarLexer.__index = GrammarLexer

--- Convert a PCRE-style regex pattern string to a Lua pattern string.
--
-- Grammar files use PCRE-style regex syntax (e.g. \s, \d, *?, {n,m}).
-- Lua's string.find uses its own pattern syntax that differs in several
-- important ways. This function bridges the gap by converting the most
-- common PCRE constructs found in the .tokens grammar files.
--
-- Conversions performed:
--   \t \r \n            → actual tab, CR, LF bytes
--   \s \S \d \D         → %s %S %d %D  (Lua character classes)
--   \w                  → [%w_]  (alphanumeric + underscore)
--   \W                  → [^%w_]
--   \/ \* \+ \. \?      → / %* %+ %. %?  (unescape PCRE-escaped punctuation)
--   \( \) \[ \] \^      → %( %) %[ %] %^
--   \$ \{ \} \-         → %$ %{ %} %-
--   \xNN                → character with hex code NN
--   *?                  → -   (non-greedy in Lua)
--   - (outside [...])   → %-  (literal hyphen; in Lua bare - is a quantifier)
--   (A|B)*              → [%s%S]-  (any-char non-greedy, approximation)
--   (A|B)+              → [%s%S]+
--   (A|B)?              → [%s%S]?
--   (?!...) (?=...)     → (dropped — no Lua equivalent)
--   {n}  {n,m}          → expanded repetition of the preceding item
--
-- This is intentionally NOT a complete PCRE-to-Lua translator. Constructs
-- that cannot be approximated (backreferences, lookaheads, top-level |)
-- are left as-is or silently dropped. The goal is "good enough for the
-- grammar files used in this monorepo", not theoretical completeness.
--
-- @param s string  PCRE pattern string (without surrounding //).
-- @return string   Equivalent Lua pattern string.
local function pcre_to_lua(s)
    local result = {}
    local i = 1
    local len = #s

    -- Collect and convert a [...] character class starting at position `start`.
    -- Returns (converted_class_string, position_after_closing_bracket).
    local function collect_class(start)
        local buf = {"["}
        local j = start + 1
        if j <= len and s:sub(j,j) == "^" then
            buf[#buf+1] = "^"; j = j+1
        end
        -- ] as first char = literal ] (not end of class)
        if j <= len and s:sub(j,j) == "]" then
            buf[#buf+1] = "]"; j = j+1
        end
        while j <= len and s:sub(j,j) ~= "]" do
            local bc = s:sub(j,j)
            if bc == "\\" and j < len then
                local bnc = s:sub(j+1,j+1)
                if     bnc == "t"  then buf[#buf+1] = "\t";  j = j+2
                elseif bnc == "r"  then buf[#buf+1] = "\r";  j = j+2
                elseif bnc == "n"  then buf[#buf+1] = "\n";  j = j+2
                elseif bnc == "v"  then buf[#buf+1] = "\11"; j = j+2  -- vertical tab (U+000B)
                elseif bnc == "f"  then buf[#buf+1] = "\12"; j = j+2  -- form feed (U+000C)
                elseif bnc == "s"  then buf[#buf+1] = "%s";  j = j+2
                elseif bnc == "S"  then buf[#buf+1] = "%S";  j = j+2
                elseif bnc == "d"  then buf[#buf+1] = "%d";  j = j+2
                elseif bnc == "D"  then buf[#buf+1] = "%D";  j = j+2
                elseif bnc == "w"  then buf[#buf+1] = "%w";  j = j+2
                elseif bnc == "/"  then buf[#buf+1] = "/";   j = j+2
                elseif bnc == "'"  then buf[#buf+1] = "'";   j = j+2
                elseif bnc == '"'  then buf[#buf+1] = '"';   j = j+2
                elseif bnc == "\\" then buf[#buf+1] = "\\";  j = j+2
                elseif bnc == "-"  then buf[#buf+1] = "%-";  j = j+2
                elseif bnc == "["  then buf[#buf+1] = "%[";  j = j+2
                elseif bnc == "]"  then buf[#buf+1] = "%]";  j = j+2
                elseif bnc == "x" and j+3 <= len
                       and s:sub(j+2,j+3):match("^%x%x$") then
                    buf[#buf+1] = string.char(tonumber(s:sub(j+2,j+3),16))
                    j = j+4
                else
                    -- Unknown escape inside class: strip backslash, keep char
                    buf[#buf+1] = bnc; j = j+2
                end
            else
                buf[#buf+1] = bc; j = j+1
            end
        end
        buf[#buf+1] = "]"
        return table.concat(buf), j+1  -- j+1 skips the closing ]
    end

    while i <= len do
        local c = s:sub(i, i)

        -- ---- Backslash escape sequences ----
        if c == "\\" and i < len then
            local nc = s:sub(i+1, i+1)
            if     nc == "t"  then result[#result+1] = "\t";     i = i+2
            elseif nc == "r"  then result[#result+1] = "\r";     i = i+2
            elseif nc == "n"  then result[#result+1] = "\n";     i = i+2
            elseif nc == "v"  then result[#result+1] = "\11";    i = i+2  -- vertical tab (U+000B)
            elseif nc == "f"  then result[#result+1] = "\12";    i = i+2  -- form feed (U+000C)
            elseif nc == "s"  then result[#result+1] = "%s";     i = i+2
            elseif nc == "S"  then result[#result+1] = "%S";     i = i+2
            elseif nc == "d"  then result[#result+1] = "%d";     i = i+2
            elseif nc == "D"  then result[#result+1] = "%D";     i = i+2
            elseif nc == "w"  then result[#result+1] = "[%w_]";  i = i+2
            elseif nc == "W"  then result[#result+1] = "[^%w_]"; i = i+2
            elseif nc == "/"  then result[#result+1] = "/";      i = i+2
            elseif nc == "*"  then result[#result+1] = "%*";     i = i+2
            elseif nc == "+"  then result[#result+1] = "%+";     i = i+2
            elseif nc == "."  then result[#result+1] = "%.";     i = i+2
            elseif nc == "?"  then result[#result+1] = "%?";     i = i+2
            elseif nc == "("  then result[#result+1] = "%(";     i = i+2
            elseif nc == ")"  then result[#result+1] = "%)";     i = i+2
            elseif nc == "["  then result[#result+1] = "%[";     i = i+2
            elseif nc == "]"  then result[#result+1] = "%]";     i = i+2
            elseif nc == "^"  then result[#result+1] = "%^";     i = i+2
            elseif nc == "$"  then result[#result+1] = "%$";     i = i+2
            elseif nc == "{"  then result[#result+1] = "%{";     i = i+2
            elseif nc == "}"  then result[#result+1] = "%}";     i = i+2
            elseif nc == "-"  then result[#result+1] = "%-";     i = i+2
            elseif nc == "x" and i+3 <= len
                   and s:sub(i+2,i+3):match("^%x%x$") then
                result[#result+1] = string.char(tonumber(s:sub(i+2,i+3),16))
                i = i+4
            else
                -- Unknown escape: emit the backslash and advance by 1
                -- (the next char will be processed on the next iteration)
                result[#result+1] = c; i = i+1
            end

        -- ---- Character class [...] ----
        elseif c == "[" then
            local class_str, new_i = collect_class(i)
            result[#result+1] = class_str
            i = new_i

        -- ---- *? non-greedy → Lua's - quantifier ----
        elseif c == "*" and i < len and s:sub(i+1,i+1) == "?" then
            result[#result+1] = "-"; i = i+2

        -- ---- {n} / {n,m} quantifier expansion ----
        elseif c == "{" then
            local j = i+1
            while j <= len and s:sub(j,j) ~= "}" do j = j+1 end
            if j <= len then
                local qs = s:sub(i+1, j-1)
                local n, m
                local a, b = qs:match("^(%d+),(%d+)$")
                if a then
                    n, m = tonumber(a), tonumber(b)
                else
                    local a2 = qs:match("^(%d+)$")
                    if a2 then n, m = tonumber(a2), tonumber(a2) end
                end
                if n then
                    local prev = table.remove(result)  -- pop preceding item
                    if prev then
                        for _ = 1, n       do result[#result+1] = prev end
                        for _ = 1, (m - n) do result[#result+1] = prev .. "?" end
                    end
                    i = j+1
                else
                    result[#result+1] = "%{"; i = i+1
                end
            else
                result[#result+1] = "%{"; i = i+1
            end

        -- ---- ( for groups ----
        elseif c == "(" then
            -- Check for (?...) — lookahead or non-capturing group
            if i+1 <= len and s:sub(i+1,i+1) == "?" then
                local sp = (i+2 <= len) and s:sub(i+2,i+2) or ""

                if sp == ":" then
                    -- Non-capturing group (?:...) — process like a regular
                    -- capturing group: scan for alternation, emit approximation.
                    local start_inner = i+3  -- position after '(?:'
                    local depth = 1; local j = start_inner; local has_alt = false
                    while j <= len and depth > 0 do
                        local jc = s:sub(j,j)
                        if jc == "(" then
                            depth = depth+1
                        elseif jc == ")" then
                            depth = depth-1
                        elseif jc == "|" and depth == 1 then
                            has_alt = true
                        elseif jc == "[" then
                            j = j+1
                            if j <= len and s:sub(j,j) == "^" then j = j+1 end
                            if j <= len and s:sub(j,j) == "]" then j = j+1 end
                            while j <= len and s:sub(j,j) ~= "]" do
                                if s:sub(j,j) == "\\" then j = j+1 end
                                j = j+1
                            end
                        elseif jc == "\\" then
                            j = j+1
                        end
                        j = j+1
                    end
                    local group_end = j  -- position after closing ')'

                    if has_alt then
                        -- Alternation inside (?:...) — approximate with any-char.
                        -- Adjust for quantifier that follows the closing ')'.
                        local qc = s:sub(group_end, group_end)
                        local qs2 = ""
                        local adv = 0
                        if qc == "*" then
                            if group_end < len and s:sub(group_end+1,group_end+1) == "?" then
                                qs2 = "-"; adv = 1
                            else
                                qs2 = "-"
                            end
                        elseif qc == "+" then
                            qs2 = "+"; adv = 0
                        elseif qc == "?" then
                            qs2 = "?"; adv = 0
                        elseif qc == "-" then
                            qs2 = "-"; adv = 0
                        end
                        if qs2 ~= "" then
                            result[#result+1] = "[%s%S]" .. qs2
                            i = group_end + 1 + adv
                        else
                            -- No quantifier: non-greedy to handle multi-char alternatives.
                            result[#result+1] = "[%s%S]-"
                            i = group_end
                        end
                    else
                        -- No alternation: inline the content via recursive conversion.
                        local inner = s:sub(start_inner, group_end - 2)
                        local qc2 = s:sub(group_end, group_end)
                        if qc2 == "?" then
                            result[#result+1] = "[%s%S]?"; i = group_end + 1
                        elseif qc2 == "*" then
                            if group_end < len and s:sub(group_end+1,group_end+1) == "?" then
                                result[#result+1] = "[%s%S]-"; i = group_end + 2
                            else
                                result[#result+1] = "[%s%S]-"; i = group_end + 1
                            end
                        elseif qc2 == "+" then
                            result[#result+1] = "[%s%S]+"; i = group_end + 1
                        else
                            -- No quantifier: inline the inner content directly.
                            result[#result+1] = pcre_to_lua(inner)
                            i = group_end
                        end
                    end

                else
                    -- Lookahead (?=...) (?!...) (?<...) — drop entirely
                    local depth = 1; local j = i+2
                    if j <= len then
                        if sp == "!" or sp == "=" then
                            j = j+1
                        elseif sp == "<" and j+1 <= len then
                            j = j+2
                        end
                    end
                    while j <= len and depth > 0 do
                        if     s:sub(j,j) == "(" then depth = depth+1
                        elseif s:sub(j,j) == ")" then depth = depth-1
                        end
                        j = j+1
                    end
                    -- Emit nothing — lookaheads have no Lua equivalent
                    i = j
                end

            else
                -- Regular capturing group — scan for alternation
                local depth = 1; local j = i+1; local has_alt = false
                while j <= len and depth > 0 do
                    local jc = s:sub(j,j)
                    if jc == "(" then
                        depth = depth+1
                    elseif jc == ")" then
                        depth = depth-1
                    elseif jc == "|" and depth == 1 then
                        has_alt = true
                    elseif jc == "[" then
                        -- Skip character class (avoid spurious | detection)
                        j = j+1
                        if j <= len and s:sub(j,j) == "^" then j = j+1 end
                        if j <= len and s:sub(j,j) == "]" then j = j+1 end
                        while j <= len and s:sub(j,j) ~= "]" do
                            if s:sub(j,j) == "\\" then j = j+1 end
                            j = j+1
                        end
                    elseif jc == "\\" then
                        j = j+1  -- skip escaped char
                    end
                    j = j+1
                end
                local group_end = j  -- position after )

                if has_alt then
                    -- Replace (A|B)QUANT with any-character approximation.
                    -- Non-greedy (-) is used by default to avoid over-matching.
                    local qc = s:sub(group_end, group_end)
                    local qs2 = ""
                    local adv = 0
                    if qc == "*" then
                        if group_end < len and s:sub(group_end+1,group_end+1) == "?" then
                            qs2 = "-"; adv = 1
                        else
                            qs2 = "-"  -- default to non-greedy for token patterns
                        end
                    elseif qc == "+" then
                        qs2 = "+"; adv = 0
                    elseif qc == "?" then
                        qs2 = "?"; adv = 0
                    elseif qc == "-" then
                        qs2 = "-"; adv = 0
                    end
                    if qs2 ~= "" then
                        result[#result+1] = "[%s%S]" .. qs2
                        i = group_end + 1 + adv
                    else
                        result[#result+1] = "[%s%S]"
                        i = group_end
                    end
                else
                    -- Non-alternation group: check the quantifier that follows
                    -- the closing paren to decide how to emit.
                    --
                    -- In PCRE, `(A)?` means "optionally match A".
                    -- In Lua, `(A)?` is INVALID: `?` after `)` is a literal `?`.
                    -- So we must handle each quantifier specially.
                    --
                    -- Strategy: emit a "match-anything" approximation for the
                    -- quantified group (we can't faithfully replicate PCRE group
                    -- semantics in Lua patterns, but for our grammar use-case an
                    -- approximation is fine because the token ordering in the
                    -- grammar file ensures the right rule wins first).
                    local qc2 = s:sub(group_end, group_end)
                    if qc2 == "?" then
                        -- (A)? — optional group, skip it entirely (emit nothing).
                        -- The content can appear zero times, so we just allow
                        -- the rest of the pattern to match without it.
                        i = group_end + 1
                    elseif qc2 == "*" then
                        if group_end < len and s:sub(group_end+1,group_end+1) == "?" then
                            result[#result+1] = "[%s%S]-"; i = group_end + 2
                        else
                            result[#result+1] = "[%s%S]-"; i = group_end + 1
                        end
                    elseif qc2 == "+" then
                        result[#result+1] = "[%s%S]+"; i = group_end + 1
                    else
                        -- No quantifier after the group.
                        -- Keep `(` as a Lua capture start; the body will be
                        -- re-processed as we advance i by 1.
                        result[#result+1] = "("; i = i+1
                    end
                end
            end

        -- ---- ) closing paren ----
        elseif c == ")" then
            result[#result+1] = ")"; i = i+1

        -- ---- | top-level alternation (outside parens) ----
        -- Lua has no alternation operator. Emit | as a literal character;
        -- this will produce incorrect matches for patterns that rely on it,
        -- but at least won't crash. Most test inputs don't contain |.
        elseif c == "|" then
            result[#result+1] = "|"; i = i+1

        -- ---- - outside a character class ----
        -- In PCRE, bare - is a literal hyphen.
        -- In Lua patterns, bare - is the non-greedy quantifier.
        -- Escape it to %-.
        elseif c == "-" then
            result[#result+1] = "%-"; i = i+1

        -- ---- Lua magic character: % must be doubled to %% ----
        -- In PCRE, % is not special. In Lua patterns, % is the escape
        -- prefix for magic characters. A literal % in the pattern string
        -- (e.g. from CSS PERCENTAGE = /...%/) would leave a bare % at the
        -- end of a character class or pattern and cause "malformed pattern".
        elseif c == "%" then
            result[#result+1] = "%%"; i = i+1

        -- ---- All other characters pass through unchanged ----
        else
            result[#result+1] = c; i = i+1
        end
    end

    return table.concat(result)
end

--- Compile a single token definition into a Lua pattern string.
--
-- If is_regex is true, the PCRE pattern is converted to a Lua pattern
-- via pcre_to_lua() and anchored to the start of the remaining source (^).
-- If is_regex is false, the pattern is escaped for literal matching.
--- Split a PCRE pattern string on top-level | (alternation outside groups/classes).
-- Returns an array of PCRE pattern strings (alternatives).
-- If there is no top-level |, returns a single-element array.
--
-- @param s string  PCRE pattern string.
-- @return table    Array of PCRE alternative strings.
local function split_top_level_alt(s)
    local parts = {}
    local depth = 0
    local in_class = false
    local start = 1
    local i = 1
    while i <= #s do
        local c = s:sub(i, i)
        if c == "\\" then
            i = i + 2
        elseif c == "[" and not in_class then
            in_class = true; i = i + 1
            if i <= #s and s:sub(i, i) == "^" then i = i + 1 end
            if i <= #s and s:sub(i, i) == "]" then i = i + 1 end
        elseif c == "]" and in_class then
            in_class = false; i = i + 1
        elseif not in_class and c == "(" then
            depth = depth + 1; i = i + 1
        elseif not in_class and c == ")" then
            depth = depth - 1; i = i + 1
        elseif not in_class and c == "|" and depth == 0 then
            parts[#parts + 1] = s:sub(start, i - 1)
            start = i + 1
            i = i + 1
        else
            i = i + 1
        end
    end
    parts[#parts + 1] = s:sub(start)
    return parts
end

--- Expand a PCRE pattern ending with a trailing (A)? optional group into two
-- alternatives: one where A is required (more specific, tried first) and one
-- where A is absent.  This handles the Lua limitation that (A)? with a
-- multi-character A cannot be expressed in Lua patterns.
--
-- Only expands the LAST optional group.  If the pattern does not end with
-- )?, returns {s} unchanged.
--
-- @param s string  PCRE pattern string (a single alternative, no top-level |).
-- @return table    Array of 1 or 2 PCRE pattern strings.
local function expand_trailing_optional(s)
    if #s < 3 then return {s} end
    if s:sub(#s) ~= "?" or s:sub(#s - 1, #s - 1) ~= ")" then return {s} end

    -- Walk forward to find the ( matching the ) at position #s-1.
    local close_pos = #s - 1
    local depth = 0
    local in_cl = false
    local open_pos = nil
    local i = 1
    while i <= close_pos do
        local c = s:sub(i, i)
        if c == "\\" then
            i = i + 2
        elseif c == "[" and not in_cl then
            in_cl = true; i = i + 1
            if i <= #s and s:sub(i, i) == "^" then i = i + 1 end
            if i <= #s and s:sub(i, i) == "]" then i = i + 1 end
        elseif c == "]" and in_cl then
            in_cl = false; i = i + 1
        elseif not in_cl and c == "(" then
            depth = depth + 1
            open_pos = i  -- track latest open paren at depth 1+
            i = i + 1
        elseif not in_cl and c == ")" then
            if i == close_pos and depth == 1 then
                -- open_pos is the matching (
                break
            end
            depth = depth - 1
            i = i + 1
        else
            i = i + 1
        end
    end

    if not open_pos then return {s} end

    local before = s:sub(1, open_pos - 1)
    local inner  = s:sub(open_pos + 1, close_pos - 1)
    -- Return [with A required, without A] — longer match tried first.
    return { before .. inner, before }
end

--
-- @param defn table Token definition with name, pattern, is_regex, alias.
-- @return table Array of compiled patterns (each with name, pattern, alias).
--              Usually one entry; multiple when PCRE alternation is expanded.
local function compile_pattern(defn)
    if defn.is_regex then
        -- Step 1: split the PCRE on top-level | into separate alternatives.
        local alts = split_top_level_alt(defn.pattern)
        -- Step 2: for each alternative, expand any trailing (A)? optional group.
        local expanded = {}
        for _, alt in ipairs(alts) do
            local sub = expand_trailing_optional(alt)
            for _, sa in ipairs(sub) do
                expanded[#expanded + 1] = sa
            end
        end
        -- Step 3: convert each expanded alternative to a Lua pattern.
        local result = {}
        for _, alt in ipairs(expanded) do
            result[#result + 1] = {
                name    = defn.name,
                pattern = "^" .. pcre_to_lua(alt),
                alias   = defn.alias or "",
            }
        end
        return result
    else
        -- Literal token: escape Lua magic characters.
        local pat_str = "^" .. defn.pattern:gsub("([%(%)%.%%%+%-%*%?%[%]%^%$])", "%%%1")
        return {{
            name    = defn.name,
            pattern = pat_str,
            alias   = defn.alias or "",
        }}
    end
end

--- Map from effective token name to TokenType + type_name.
--
-- This maps grammar token names (like "NAME", "NUMBER", "PLUS") to
-- their corresponding TokenType enum values. Unknown names default
-- to TokenType.Name with the grammar name as type_name.
--
-- @param token_name string The definition name.
-- @param value string The matched text.
-- @param alias string The alias name (or "").
-- @param keyword_set table Set of keywords.
-- @param reserved_set table Set of reserved keywords.
-- @param line number Current line for error messages.
-- @param column number Current column for error messages.
-- @return number, string  TokenType and type_name.
local function resolve_token_type(token_name, value, alias, keyword_set, reserved_set, line, column)
    -- Reserved keyword check
    if token_name == "NAME" then
        if reserved_set[value] then
            error(string.format(
                "LexerError at %d:%d: Reserved keyword %q cannot be used as an identifier",
                line, column, value
            ))
        end
    end

    -- Regular keyword check: return the uppercased keyword value as type_name
    -- so that "var" → type_name = "VAR", "if" → "IF", etc.
    -- For case-insensitive grammars the keyword_set also contains lowercase
    -- versions of all keywords (inserted by GrammarLexer.new), so looking up
    -- value:lower() finds a match regardless of input capitalisation.
    if token_name == "NAME" then
        if keyword_set[value] or keyword_set[value:lower()] then
            return TokenType.Keyword, value:upper()
        end
    end

    -- Determine effective name (alias takes precedence)
    local effective = token_name
    if alias ~= "" then
        effective = alias
    end

    -- Map known token types
    local type_map = {
        NAME          = { TokenType.Name,         "NAME" },
        NUMBER        = { TokenType.Number,       "NUMBER" },
        STRING        = { TokenType.String,       "STRING" },
        PLUS          = { TokenType.Plus,         "PLUS" },
        MINUS         = { TokenType.Minus,        "MINUS" },
        STAR          = { TokenType.Star,         "STAR" },
        SLASH         = { TokenType.Slash,        "SLASH" },
        EQUALS        = { TokenType.Equals,       "EQUALS" },
        EQUALS_EQUALS = { TokenType.EqualsEquals, "EQUALS_EQUALS" },
        LPAREN        = { TokenType.LParen,       "LPAREN" },
        RPAREN        = { TokenType.RParen,       "RPAREN" },
        COMMA         = { TokenType.Comma,        "COMMA" },
        COLON         = { TokenType.Colon,        "COLON" },
        SEMICOLON     = { TokenType.Semicolon,    "SEMICOLON" },
        LBRACE        = { TokenType.LBrace,       "LBRACE" },
        RBRACE        = { TokenType.RBrace,       "RBRACE" },
        LBRACKET      = { TokenType.LBracket,     "LBRACKET" },
        RBRACKET      = { TokenType.RBracket,     "RBRACKET" },
        DOT           = { TokenType.Dot,          "DOT" },
        BANG          = { TokenType.Bang,         "BANG" },
    }

    local mapping = type_map[effective]
    if mapping then
        return mapping[1], mapping[2]
    end

    -- Unknown type -- use Name as base with the grammar type name
    return TokenType.Name, effective
end

--- Create a new grammar-driven lexer.
--
-- @param source string The source text to tokenize.
-- @param grammar table A TokenGrammar table.
-- @return GrammarLexer
function GrammarLexer.new(source, grammar)
    -- Build keyword set.
    -- For case-insensitive grammars (grammar.case_insensitive == true) we
    -- also insert a lowercase version of each keyword so that the resolver
    -- can look up value:lower() and still find a match regardless of how
    -- the keyword was written in the grammar file (e.g. "SELECT" vs "select").
    local keyword_set = {}
    for _, kw in ipairs(grammar.keywords or {}) do
        keyword_set[kw] = true
        if grammar.case_insensitive then
            keyword_set[kw:lower()] = true
        end
    end

    -- Build reserved set
    local reserved_set = {}
    for _, rk in ipairs(grammar.reserved_keywords or {}) do
        reserved_set[rk] = true
    end

    -- Build alias map
    local alias_map = {}
    for _, defn in ipairs(grammar.definitions or {}) do
        if defn.alias and defn.alias ~= "" then
            alias_map[defn.name] = defn.alias
        end
    end

    -- Compile top-level token patterns.
    -- compile_pattern() returns an array (may be >1 entry when a PCRE pattern
    -- contains top-level | or trailing (A)? optional groups that were expanded
    -- into separate alternatives for Lua compatibility).
    local patterns = {}
    for _, defn in ipairs(grammar.definitions or {}) do
        local compiled = compile_pattern(defn)
        for _, p in ipairs(compiled) do
            patterns[#patterns + 1] = p
        end
    end

    -- Compile skip patterns (also apply PCRE-to-Lua conversion)
    local skip_patterns = {}
    for _, defn in ipairs(grammar.skip_definitions or {}) do
        local pat_str
        if defn.is_regex then
            pat_str = "^" .. pcre_to_lua(defn.pattern)
        else
            pat_str = "^" .. defn.pattern:gsub("([%(%)%.%%%+%-%*%?%[%]%^%$])", "%%%1")
        end
        skip_patterns[#skip_patterns + 1] = pat_str
    end

    -- Compile pattern groups
    -- The "default" group uses the top-level definitions.
    local group_patterns = {
        default = {},
    }
    for i, p in ipairs(patterns) do
        group_patterns["default"][i] = { name = p.name, pattern = p.pattern, alias = p.alias }
    end

    for group_name, group in pairs(grammar.groups or {}) do
        local compiled = {}
        for _, defn in ipairs(group.definitions or {}) do
            local defn_patterns = compile_pattern(defn)
            for _, p in ipairs(defn_patterns) do
                compiled[#compiled + 1] = p
            end
            if defn.alias and defn.alias ~= "" then
                alias_map[defn.name] = defn.alias
            end
        end
        group_patterns[group_name] = compiled
    end

    -- Build context keyword set
    local context_keyword_set = {}
    for _, ck in ipairs(grammar.context_keywords or {}) do
        context_keyword_set[ck] = true
    end

    -- Build layout keyword set
    local layout_keyword_set = {}
    for _, lk in ipairs(grammar.layout_keywords or {}) do
        layout_keyword_set[lk] = true
    end

    return setmetatable({
        _source              = source,
        _grammar             = grammar,
        _pos                 = 1,
        _line                = 1,
        _column              = 1,
        _keyword_set         = keyword_set,
        _reserved_set        = reserved_set,
        _context_keyword_set = context_keyword_set,
        _patterns            = patterns,
        _skip_patterns       = skip_patterns,
        _has_skip_patterns   = #skip_patterns > 0,
        _indent_mode         = (grammar.mode == "indentation"),
        _layout_mode         = (grammar.mode == "layout"),
        _indent_stack        = { 0 },
        _bracket_depth       = 0,
        _bracket_depths      = { paren = 0, bracket = 0, brace = 0 },
        _last_emitted_token  = nil,
        _group_patterns      = group_patterns,
        _group_stack         = { "default" },
        _on_token            = nil,
        _skip_enabled        = true,
        _alias_map           = alias_map,
        _escape_mode         = grammar.escape_mode or "",
        _case_insensitive    = grammar.case_insensitive or false,
        _layout_keyword_set  = layout_keyword_set,
    }, GrammarLexer)
end

--- Register a callback that fires on every token match.
--
-- The callback receives (token, context) where context is a LexerContext.
-- Pass nil to clear the callback.
-- @param callback function|nil The callback function.
function GrammarLexer:set_on_token(callback)
    self._on_token = callback
end

--- Return the current nesting depth for a specific bracket type,
-- or the total depth across all types if no argument is given.
--
-- @param kind string|nil "paren", "bracket", or "brace"; nil for total.
-- @return number
function GrammarLexer:bracket_depth(kind)
    if kind == nil then
        return self._bracket_depths.paren
             + self._bracket_depths.bracket
             + self._bracket_depths.brace
    end
    return self._bracket_depths[kind] or 0
end

--- Update bracket depth tracking based on the token value.
-- Called after each token match.
-- @param value string The token's text value.
function GrammarLexer:_update_bracket_depth(value)
    if value == "(" then
        self._bracket_depths.paren = self._bracket_depths.paren + 1
    elseif value == ")" then
        self._bracket_depths.paren = math.max(0, self._bracket_depths.paren - 1)
    elseif value == "[" then
        self._bracket_depths.bracket = self._bracket_depths.bracket + 1
    elseif value == "]" then
        self._bracket_depths.bracket = math.max(0, self._bracket_depths.bracket - 1)
    elseif value == "{" then
        self._bracket_depths.brace = self._bracket_depths.brace + 1
    elseif value == "}" then
        self._bracket_depths.brace = math.max(0, self._bracket_depths.brace - 1)
    end
end

--- Advance past the current character (internal).
function GrammarLexer:_advance()
    if self._pos <= #self._source then
        local ch = self._source:sub(self._pos, self._pos)
        if ch == "\n" then
            self._line = self._line + 1
            self._column = 1
        else
            self._column = self._column + 1
        end
        self._pos = self._pos + 1
    end
end

--- Try to match and consume a skip pattern at the current position.
-- @return boolean True if a skip pattern matched (and was consumed).
function GrammarLexer:_try_skip()
    local remaining = self._source:sub(self._pos)
    for _, pat in ipairs(self._skip_patterns) do
        local s, e = remaining:find(pat)
        if s == 1 then
            for _ = 1, e do
                self:_advance()
            end
            return true
        end
    end
    return false
end

--- Try to match a token using a specific group's patterns.
--
-- Tries each compiled pattern in the named group in priority order
-- (first match wins). Handles keyword detection, reserved word checking,
-- aliases, and string escape processing.
--
-- @param group_name string The group to match against.
-- @return Token|nil The matched token, or nil if nothing matched.
function GrammarLexer:_try_match_token_in_group(group_name)
    local remaining = self._source:sub(self._pos)

    local pats = self._group_patterns[group_name]
    if not pats then
        pats = self._patterns
    end

    for _, p in ipairs(pats) do
        local s, e = remaining:find(p.pattern)
        if s == 1 then
            local value = remaining:sub(1, e)
            local start_line = self._line
            local start_col = self._column

            local ttype, type_name = resolve_token_type(
                p.name, value, p.alias,
                self._keyword_set, self._reserved_set,
                self._line, self._column
            )

            -- Handle STRING tokens: strip quotes and optionally process
            -- escapes. When escape_mode is "none", we strip quotes but
            -- leave escape sequences as raw text.
            if p.name:find("STRING") or (p.alias ~= "" and p.alias:find("STRING")) then
                if #value >= 2 then
                    local quote = value:sub(1, 1)
                    if quote == '"' or quote == "'" then
                        -- Check for triple-quoted strings (don't apply escape-scanner)
                        if #value >= 6 and value:sub(1, 3) == quote:rep(3) then
                            local inner = value:sub(4, #value - 3)
                            if self._escape_mode ~= "none" then
                                inner = process_escapes(inner)
                            end
                            value = inner
                        else
                            -- For single-quoted strings, the regex approximation for
                            -- patterns like /"([^"\\]|\\.)*"/ (converted to ".."-style
                            -- non-greedy) may stop prematurely at an escaped quote like
                            -- \" inside the string. Scan forward to find the true end.
                            local true_end = e
                            local k = 2  -- skip opening quote
                            while k <= #remaining do
                                local c = remaining:sub(k, k)
                                if c == "\\" then
                                    k = k + 2  -- skip backslash and escaped char
                                elseif c == quote then
                                    true_end = k  -- found the real closing quote
                                    break
                                else
                                    k = k + 1
                                end
                            end
                            if true_end > e then
                                e = true_end
                                value = remaining:sub(1, e)
                            end
                            local inner = value:sub(2, #value - 1)
                            if self._escape_mode ~= "none" then
                                inner = process_escapes(inner)
                            end
                            value = inner
                        end
                    end
                end
            end

            local tok = Token.new(ttype, value, start_line, start_col, type_name)

            for _ = 1, e do
                self:_advance()
            end

            return tok
        end
    end
    return nil
end

--- Try to match a token using the default group's patterns.
-- @return Token|nil
function GrammarLexer:_try_match_token()
    return self:_try_match_token_in_group("default")
end

--- Tokenize the source using the grammar's token definitions.
--
-- Dispatches to the appropriate tokenization method based on whether
-- indentation mode is active.
-- @return table Array of Token objects.
function GrammarLexer:tokenize()
    if self._indent_mode then
        return self:_tokenize_indentation()
    elseif self._layout_mode then
        return self:_tokenize_layout()
    end
    return self:_tokenize_standard()
end

--- Standard (non-indentation) tokenization.
--
-- Algorithm:
--   1. While there are characters left:
--      a. If skip patterns exist and skip is enabled, try them first.
--      b. If no skip patterns, use default whitespace skip.
--      c. If the current character is a newline, emit NEWLINE.
--      d. Try the active group's token patterns (first match wins).
--      e. If a callback is registered, invoke it and process actions.
--      f. If nothing matches, raise a LexerError.
--   2. Append EOF.
--   3. Reset group stack and skip state for reuse.
-- @return table Array of Token objects.
function GrammarLexer:_tokenize_standard()
    local tokens = {}

    -- Reset extension state for reuse
    self._last_emitted_token = nil
    self._bracket_depths = { paren = 0, bracket = 0, brace = 0 }

    while self._pos <= #self._source do
        local ch = self._source:sub(self._pos, self._pos)

        -- Skip patterns (grammar-defined)
        if self._has_skip_patterns then
            if self._skip_enabled and self:_try_skip() then
                goto continue
            end
        else
            -- Default whitespace skip
            if ch == " " or ch == "\t" or ch == "\r" then
                self:_advance()
                goto continue
            end
        end

        -- Newlines become NEWLINE tokens
        if ch == "\n" then
            local newline_tok = Token.new(
                TokenType.Newline, "\\n",
                self._line, self._column, "NEWLINE"
            )
            tokens[#tokens + 1] = newline_tok
            self._last_emitted_token = newline_tok
            self:_advance()
            goto continue
        end

        -- Try active group's token patterns
        do
            local active_group = self._group_stack[#self._group_stack]
            local tok = self:_try_match_token_in_group(active_group)
            if tok then
                -- Update bracket depth tracking
                self:_update_bracket_depth(tok.value)

                -- Set context keyword flag if applicable
                if self._context_keyword_set[tok.value]
                   and (tok.type_name == "NAME" or tok.type == TokenType.Name) then
                    tok.flags = (tok.flags or 0) | TOKEN_CONTEXT_KEYWORD
                end

                -- Invoke on-token callback
                if self._on_token then
                    local ctx = LexerContext._new(
                        self, self._source, self._pos,
                        self._last_emitted_token, tok.line
                    )
                    self._on_token(tok, ctx)

                    -- Apply suppression
                    if not ctx._suppressed then
                        tokens[#tokens + 1] = tok
                        self._last_emitted_token = tok
                    end

                    -- Append emitted tokens
                    for _, emitted in ipairs(ctx._emitted) do
                        tokens[#tokens + 1] = emitted
                        self._last_emitted_token = emitted
                    end

                    -- Apply group stack actions
                    for _, ga in ipairs(ctx._group_actions) do
                        if ga.action == "push" then
                            self._group_stack[#self._group_stack + 1] = ga.group_name
                        elseif ga.action == "pop" and #self._group_stack > 1 then
                            self._group_stack[#self._group_stack] = nil
                        end
                    end

                    -- Apply skip toggle
                    if ctx._skip_enabled ~= nil then
                        self._skip_enabled = ctx._skip_enabled
                    end
                else
                    tokens[#tokens + 1] = tok
                    self._last_emitted_token = tok
                end
                goto continue
            end
        end

        error(string.format(
            "LexerError at %d:%d: Unexpected character %q",
            self._line, self._column, ch
        ))

        ::continue::
    end

    tokens[#tokens + 1] = Token.new(
        TokenType.EOF, "", self._line, self._column, "EOF"
    )

    -- Reset group stack and skip state for reuse
    self._group_stack = { "default" }
    self._skip_enabled = true

    return tokens
end

function GrammarLexer:_tokenize_layout()
    return self:_apply_layout(self:_tokenize_standard())
end

function GrammarLexer:_apply_layout(tokens)
    local result = {}
    local layout_stack = {}
    local pending_layouts = 0
    local suppress_depth = 0

    for index, token in ipairs(tokens) do
        local type_name = token.type_name or token_type_to_string(token.type):upper()

        if type_name == "NEWLINE" then
            result[#result + 1] = token
            local next_token = self:_next_layout_token(tokens, index + 1)
            if suppress_depth == 0 and next_token ~= nil then
                while #layout_stack > 0 and next_token.column < layout_stack[#layout_stack] do
                    result[#result + 1] = self:_virtual_layout_token("VIRTUAL_RBRACE", "}", next_token)
                    layout_stack[#layout_stack] = nil
                end

                local next_type = next_token.type_name or token_type_to_string(next_token.type):upper()
                if #layout_stack > 0
                    and next_type ~= "EOF"
                    and next_token.value ~= "}"
                    and next_token.column == layout_stack[#layout_stack] then
                    result[#result + 1] = self:_virtual_layout_token("VIRTUAL_SEMICOLON", ";", next_token)
                end
            end
            goto continue
        end

        if type_name == "EOF" then
            while #layout_stack > 0 do
                result[#result + 1] = self:_virtual_layout_token("VIRTUAL_RBRACE", "}", token)
                layout_stack[#layout_stack] = nil
            end
            result[#result + 1] = token
            goto continue
        end

        if pending_layouts > 0 then
            if token.value == "{" then
                pending_layouts = pending_layouts - 1
            else
                for _ = 1, pending_layouts do
                    layout_stack[#layout_stack + 1] = token.column
                    result[#result + 1] = self:_virtual_layout_token("VIRTUAL_LBRACE", "{", token)
                end
                pending_layouts = 0
            end
        end

        result[#result + 1] = token

        if not self:_is_virtual_layout_token(token) then
            if token.value == "(" or token.value == "[" or token.value == "{" then
                suppress_depth = suppress_depth + 1
            elseif (token.value == ")" or token.value == "]" or token.value == "}") and suppress_depth > 0 then
                suppress_depth = suppress_depth - 1
            end
        end

        if self:_is_layout_keyword(token) then
            pending_layouts = pending_layouts + 1
        end

        ::continue::
    end

    return result
end

function GrammarLexer:_next_layout_token(tokens, start_index)
    for i = start_index, #tokens do
        local token = tokens[i]
        local type_name = token.type_name or token_type_to_string(token.type):upper()
        if type_name ~= "NEWLINE" then
            return token
        end
    end
    return nil
end

function GrammarLexer:_virtual_layout_token(type_name, value, anchor)
    return Token.new(TokenType.Name, value, anchor.line, anchor.column, type_name)
end

function GrammarLexer:_is_virtual_layout_token(token)
    local type_name = token.type_name or ""
    return type_name:sub(1, 8) == "VIRTUAL_"
end

function GrammarLexer:_is_layout_keyword(token)
    if not next(self._layout_keyword_set) then
        return false
    end
    local value = token.value or ""
    return self._layout_keyword_set[value] or self._layout_keyword_set[value:lower()] or false
end

--- Process indentation at the start of a logical line.
--
-- Counts leading spaces, then compares against the indent stack to emit
-- INDENT/DEDENT tokens. Blank lines and comment-only lines are skipped.
--
-- @return table|nil Array of indent/dedent tokens (or nil for skip).
-- @return boolean True if the line should be skipped entirely.
function GrammarLexer:_process_line_start()
    local indent = 0
    while self._pos <= #self._source do
        local ch = self._source:sub(self._pos, self._pos)
        if ch == " " then
            indent = indent + 1
            self:_advance()
        elseif ch == "\t" then
            error(string.format(
                "LexerError at %d:%d: Tab character in indentation (use spaces only)",
                self._line, self._column
            ))
        else
            break
        end
    end

    -- Blank line or EOF
    if self._pos > #self._source then
        return nil, true
    end
    if self._source:sub(self._pos, self._pos) == "\n" then
        self:_advance()
        return nil, true
    end

    -- Comment-only line
    local remaining = self._source:sub(self._pos)
    for _, pat in ipairs(self._skip_patterns) do
        local s, e = remaining:find(pat)
        if s == 1 then
            local peek_pos = self._pos + e
            if peek_pos > #self._source or self._source:sub(peek_pos, peek_pos) == "\n" then
                for _ = 1, e do
                    self:_advance()
                end
                if self._pos <= #self._source and self._source:sub(self._pos, self._pos) == "\n" then
                    self:_advance()
                end
                return nil, true
            end
        end
    end

    -- Compare indent to current level
    local current_indent = self._indent_stack[#self._indent_stack]
    local indent_tokens = {}

    if indent > current_indent then
        self._indent_stack[#self._indent_stack + 1] = indent
        indent_tokens[#indent_tokens + 1] = Token.new(
            TokenType.Name, "", self._line, 1, "INDENT"
        )
    elseif indent < current_indent then
        while #self._indent_stack > 1 and self._indent_stack[#self._indent_stack] > indent do
            self._indent_stack[#self._indent_stack] = nil
            indent_tokens[#indent_tokens + 1] = Token.new(
                TokenType.Name, "", self._line, 1, "DEDENT"
            )
        end
        if self._indent_stack[#self._indent_stack] ~= indent then
            error(string.format(
                "LexerError at %d:%d: Inconsistent dedent",
                self._line, self._column
            ))
        end
    end

    return indent_tokens, false
end

--- Indentation-mode tokenization.
--
-- Tracks indentation levels and emits INDENT/DEDENT tokens. Brackets
-- suppress indentation processing (implicit line continuation).
-- @return table Array of Token objects.
function GrammarLexer:_tokenize_indentation()
    local tokens = {}
    local at_line_start = true

    while self._pos <= #self._source do
        -- Process line start
        if at_line_start and self._bracket_depth == 0 then
            local indent_tokens, skip_line = self:_process_line_start()
            if skip_line then
                goto continue
            end
            if indent_tokens then
                for _, t in ipairs(indent_tokens) do
                    tokens[#tokens + 1] = t
                end
            end
            at_line_start = false
            if self._pos > #self._source then
                break
            end
        end

        local ch = self._source:sub(self._pos, self._pos)

        -- Newline handling
        if ch == "\n" then
            if self._bracket_depth == 0 then
                tokens[#tokens + 1] = Token.new(
                    TokenType.Newline, "\\n",
                    self._line, self._column, "NEWLINE"
                )
            end
            self:_advance()
            at_line_start = true
            goto continue
        end

        -- Inside brackets: skip whitespace
        if self._bracket_depth > 0 and (ch == " " or ch == "\t" or ch == "\r") then
            self:_advance()
            goto continue
        end

        -- Try skip patterns
        if self:_try_skip() then
            goto continue
        end

        -- Try token patterns
        do
            local tok = self:_try_match_token()
            if tok then
                -- Track bracket depth
                if tok.value == "(" or tok.value == "[" or tok.value == "{" then
                    self._bracket_depth = self._bracket_depth + 1
                elseif tok.value == ")" or tok.value == "]" or tok.value == "}" then
                    self._bracket_depth = self._bracket_depth - 1
                end
                tokens[#tokens + 1] = tok
                goto continue
            end
        end

        error(string.format(
            "LexerError at %d:%d: Unexpected character %q",
            self._line, self._column, ch
        ))

        ::continue::
    end

    -- EOF: emit remaining DEDENTs
    while #self._indent_stack > 1 do
        self._indent_stack[#self._indent_stack] = nil
        tokens[#tokens + 1] = Token.new(
            TokenType.Name, "", self._line, self._column, "DEDENT"
        )
    end

    -- Final NEWLINE if needed
    if #tokens == 0 or tokens[#tokens].type ~= TokenType.Newline then
        tokens[#tokens + 1] = Token.new(
            TokenType.Newline, "\\n", self._line, self._column, "NEWLINE"
        )
    end

    tokens[#tokens + 1] = Token.new(
        TokenType.EOF, "", self._line, self._column, "EOF"
    )
    return tokens
end

-- =========================================================================
-- Module Exports
-- =========================================================================

local lexer = {}

lexer.VERSION = "0.1.0"

-- Classes
lexer.Token        = Token
lexer.Lexer        = Lexer
lexer.GrammarLexer = GrammarLexer
lexer.LexerContext = LexerContext

-- Constants
lexer.TokenType = TokenType
lexer.TOKEN_PRECEDED_BY_NEWLINE = TOKEN_PRECEDED_BY_NEWLINE
lexer.TOKEN_CONTEXT_KEYWORD     = TOKEN_CONTEXT_KEYWORD

-- Functions (exported for testing)
lexer.classify_char        = classify_char
lexer.new_tokenizer_dfa    = new_tokenizer_dfa
lexer.token_type_to_string = token_type_to_string
lexer.process_escapes      = process_escapes

return lexer
