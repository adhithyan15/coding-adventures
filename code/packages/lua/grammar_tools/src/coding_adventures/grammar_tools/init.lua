-- grammar_tools — Grammar definition and manipulation for lexers and parsers
-- ============================================================================
--
-- This package parses and validates two kinds of grammar files:
--
--   1. Token grammars (.tokens files) — define the lexical tokens a lexer
--      recognizes: identifiers, numbers, operators, string literals, etc.
--
--   2. Parser grammars (.grammar files) — define the syntactic structure of a
--      language using EBNF-like rules that reference the tokens from (1).
--
-- It also cross-validates token and parser grammars to ensure consistency:
-- every token the parser references must be defined, and every token defined
-- should ideally be used somewhere.
--
-- # Why two separate files?
--
-- Lexing (tokenization) and parsing are fundamentally different tasks:
--
--   - Lexing is about CHARACTER patterns: "what sequence of characters makes
--     a NUMBER?" Answer: /[0-9]+/. This is regular-language territory.
--
--   - Parsing is about TOKEN patterns: "what sequence of tokens makes an
--     expression?" Answer: term { PLUS term }. This is context-free grammar
--     territory.
--
-- Keeping them separate mirrors the classic compiler pipeline (lexer -> parser)
-- and makes each file simpler and more focused.
--
-- # Token grammar format
--
-- A .tokens file contains token definitions, one per line:
--
--     NAME = /[a-zA-Z_]+/       -- regex pattern (delimited by /)
--     EQUALS = "="              -- literal pattern (delimited by ")
--     STRING_DQ = /"[^"]*"/ -> STRING   -- alias (lexer emits STRING)
--
-- Special sections (indented lines after a header):
--
--     keywords:       -- keywords recognized from NAME tokens
--       if
--       else
--
--     reserved:       -- keywords that cause lex errors
--       class
--
--     skip:           -- patterns consumed without producing tokens
--       WHITESPACE = /[ \t]+/
--
--     errors:         -- error recovery patterns (last resort)
--       BAD_STRING = /"[^"\n]*/
--
--     group NAME:     -- context-sensitive lexing groups
--       TAG_NAME = /[a-zA-Z]+/
--
-- Directives:
--
--     mode: indentation    -- enables INDENT/DEDENT token generation
--     escapes: none        -- disables escape sequence processing
--
-- # Parser grammar format
--
-- A .grammar file contains EBNF-like production rules:
--
--     expression = term { ( PLUS | MINUS ) term } ;
--     term = NUMBER ;
--
-- Elements:
--   - UPPERCASE names  -> token references (e.g., NUMBER, PLUS)
--   - lowercase names  -> rule references (e.g., expression, term)
--   - "literal"        -> literal strings
--   - { ... }          -> repetition (zero or more)
--   - [ ... ]          -> optional (zero or one)
--   - ( ... )          -> grouping
--   - |                -> alternation (choice)
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
-- This gives us method dispatch through metatables, constructor functions,
-- and instanceof checks via getmetatable().
--
-- # Port lineage
--
-- This is a Lua 5.4 port of the Go implementation at:
--   code/packages/go/grammar-tools/
--
-- The Go version is the reference implementation. This port preserves the
-- same data structures, parsing logic, validation rules, and error messages.

local grammar_tools = {}
grammar_tools.VERSION = "0.1.0"

-- ============================================================================
-- TokenDefinition — a single token rule from a .tokens file
-- ============================================================================
--
-- Each token definition maps a name to a pattern. The pattern is either a
-- regex (delimited by /.../) or a literal string (delimited by "...").
--
-- Fields:
--   name        (string)  Token name, e.g. "NUMBER" or "PLUS"
--   pattern     (string)  The pattern text (without delimiters)
--   is_regex    (boolean) true for /regex/, false for "literal"
--   line_number (integer) Source line where this definition appears
--   alias       (string)  Optional type alias, e.g. "STRING" for STRING_DQ

local TokenDefinition = {}
TokenDefinition.__index = TokenDefinition

--- Create a new TokenDefinition.
-- @param fields Table with name, pattern, is_regex, line_number, alias
-- @return TokenDefinition instance
function TokenDefinition.new(fields)
    local self = setmetatable({}, TokenDefinition)
    self.name = fields.name or ""
    self.pattern = fields.pattern or ""
    self.is_regex = fields.is_regex or false
    self.line_number = fields.line_number or 0
    self.alias = fields.alias or ""
    return self
end

grammar_tools.TokenDefinition = TokenDefinition

-- ============================================================================
-- PatternGroup — a named set of token definitions for context-sensitive lexing
-- ============================================================================
--
-- When a pattern group is at the top of the lexer's group stack, only its
-- patterns are tried during token matching. Skip patterns are global and
-- always tried regardless of the active group.
--
-- Example: an XML lexer defines a "tag" group with patterns for attribute
-- names, equals signs, and attribute values. These patterns are only active
-- inside tags.
--
-- Fields:
--   name        (string)  Group name, e.g. "tag" or "cdata"
--   definitions (table)   Ordered list of TokenDefinition instances

local PatternGroup = {}
PatternGroup.__index = PatternGroup

--- Create a new PatternGroup.
-- @param name  Group name (lowercase identifier)
-- @param definitions  Optional list of TokenDefinition instances
-- @return PatternGroup instance
function PatternGroup.new(name, definitions)
    local self = setmetatable({}, PatternGroup)
    self.name = name
    self.definitions = definitions or {}
    return self
end

grammar_tools.PatternGroup = PatternGroup

-- ============================================================================
-- TokenGrammar — the complete contents of a parsed .tokens file
-- ============================================================================
--
-- This is the top-level data structure returned by parse_token_grammar().
-- It holds all token definitions, keywords, skip patterns, error patterns,
-- mode directives, and named pattern groups.
--
-- Fields:
--   definitions       (table)  Ordered list of TokenDefinition instances
--   keywords          (table)  List of keyword strings
--   mode              (string) Lexer mode, e.g. "indentation" or "layout"
--   escape_mode       (string) Escape processing mode, e.g. "none"
--   skip_definitions  (table)  Skip pattern TokenDefinition instances
--   error_definitions (table)  Error recovery TokenDefinition instances
--   reserved_keywords (table)  Keywords that cause lex errors
--   groups            (table)  Map of group name -> PatternGroup

local TokenGrammar = {}
TokenGrammar.__index = TokenGrammar

--- Create a new, empty TokenGrammar.
-- @return TokenGrammar instance
function TokenGrammar.new()
    local self = setmetatable({}, TokenGrammar)
    self.definitions = {}
    self.keywords = {}
    self.context_keywords = {}
    self.layout_keywords = {}
    self.soft_keywords = {}
    self.mode = ""
    self.escape_mode = ""
    self.skip_definitions = {}
    self.error_definitions = {}
    self.reserved_keywords = {}
    self.groups = {}
    return self
end

--- Return the set of all defined token names (including aliases).
--
-- When a definition has an alias, both the original name and the alias are
-- included. This includes names from all pattern groups, since group tokens
-- can also appear in parser grammars.
--
-- @return table  Set of token names (name -> true)
function TokenGrammar:token_names()
    local names = {}

    -- Collect all definitions: top-level plus all group definitions
    local all_defs = {}
    for _, d in ipairs(self.definitions) do
        all_defs[#all_defs + 1] = d
    end
    for _, group in pairs(self.groups) do
        for _, d in ipairs(group.definitions) do
            all_defs[#all_defs + 1] = d
        end
    end

    for _, d in ipairs(all_defs) do
        names[d.name] = true
        if d.alias ~= "" then
            names[d.alias] = true
        end
    end
    return names
end

--- Return the set of token names as the parser will see them.
--
-- For definitions with aliases, this returns the alias (not the definition
-- name), because that is what the lexer emits. For definitions without
-- aliases, this returns the definition name.
--
-- @return table  Set of effective token names (name -> true)
function TokenGrammar:effective_token_names()
    local names = {}

    local all_defs = {}
    for _, d in ipairs(self.definitions) do
        all_defs[#all_defs + 1] = d
    end
    for _, group in pairs(self.groups) do
        for _, d in ipairs(group.definitions) do
            all_defs[#all_defs + 1] = d
        end
    end

    for _, d in ipairs(all_defs) do
        if d.alias ~= "" then
            names[d.alias] = true
        else
            names[d.name] = true
        end
    end
    return names
end

grammar_tools.TokenGrammar = TokenGrammar

-- ============================================================================
-- parse_definition — parse a single pattern with optional -> ALIAS suffix
-- ============================================================================
--
-- Patterns come in two forms:
--
--   /regex/          — regex pattern, delimited by forward slashes
--   "literal"        — literal string, delimited by double quotes
--
-- Either form can have an optional alias suffix:
--
--   /regex/ -> ALIAS
--   "literal" -> ALIAS
--
-- The alias tells the lexer to emit a different token type than the
-- definition name. For example, STRING_DQ = /"[^"]*"/ -> STRING means
-- "match double-quoted strings and emit them as STRING tokens."
--
-- @param pattern_part  The pattern portion of a definition line
-- @param name_part     The token name
-- @param line_number   Source line number (for error messages)
-- @return TokenDefinition, nil on success; nil, error_string on failure

local function parse_definition(pattern_part, name_part, line_number)
    local defn = TokenDefinition.new({
        name = name_part,
        line_number = line_number,
    })

    if pattern_part:sub(1, 1) == "/" then
        -- Regex pattern — find the closing /
        -- We search for the LAST / to handle patterns containing /
        local last_slash = nil
        for i = #pattern_part, 2, -1 do
            if pattern_part:sub(i, i) == "/" then
                last_slash = i
                break
            end
        end

        if not last_slash or last_slash == 1 then
            return nil, string.format(
                "Line %d: Unclosed regex pattern for token %q",
                line_number, name_part
            )
        end

        defn.pattern = pattern_part:sub(2, last_slash - 1)
        defn.is_regex = true
        local remainder = pattern_part:sub(last_slash + 1):match("^%s*(.-)%s*$")

        if defn.pattern == "" then
            return nil, string.format(
                "Line %d: Empty regex pattern for token %q",
                line_number, name_part
            )
        end

        if remainder:sub(1, 2) == "->" then
            local alias = remainder:sub(3):match("^%s*(.-)%s*$")
            if alias == "" then
                return nil, string.format(
                    "Line %d: Missing alias after '->' for token %q",
                    line_number, name_part
                )
            end
            defn.alias = alias
        elseif remainder ~= "" then
            return nil, string.format(
                "Line %d: Unexpected text after pattern for token %q: %q",
                line_number, name_part, remainder
            )
        end

    elseif pattern_part:sub(1, 1) == '"' then
        -- Literal pattern — find the closing "
        local close_quote = pattern_part:find('"', 2, true)
        if not close_quote then
            return nil, string.format(
                "Line %d: Unclosed literal pattern for token %q",
                line_number, name_part
            )
        end

        defn.pattern = pattern_part:sub(2, close_quote - 1)
        defn.is_regex = false
        local remainder = pattern_part:sub(close_quote + 1):match("^%s*(.-)%s*$")

        if defn.pattern == "" then
            return nil, string.format(
                "Line %d: Empty literal pattern for token %q",
                line_number, name_part
            )
        end

        if remainder:sub(1, 2) == "->" then
            local alias = remainder:sub(3):match("^%s*(.-)%s*$")
            if alias == "" then
                return nil, string.format(
                    "Line %d: Missing alias after '->' for token %q",
                    line_number, name_part
                )
            end
            defn.alias = alias
        elseif remainder ~= "" then
            return nil, string.format(
                "Line %d: Unexpected text after pattern for token %q: %q",
                line_number, name_part, remainder
            )
        end

    else
        return nil, string.format(
            "Line %d: Pattern must be /regex/ or \"literal\"",
            line_number
        )
    end

    return defn, nil
end

-- ============================================================================
-- Group name validation
-- ============================================================================
--
-- Group names must be lowercase identifiers: they start with a lowercase
-- letter or underscore, followed by lowercase letters, digits, or underscores.
-- This matches the pattern [a-z_][a-z0-9_]*.
--
-- Certain names are reserved because they have special meaning in the
-- .tokens format: "default", "skip", "keywords", "reserved", "errors".

--- Check if a string is a valid group name (lowercase identifier).
-- @param name  String to check
-- @return boolean
local function is_valid_group_name(name)
    return name:match("^[a-z_][a-z0-9_]*$") ~= nil
end

--- Set of reserved group names that cannot be used.
local RESERVED_GROUP_NAMES = {
    default = true,
    skip = true,
    keywords = true,
    reserved = true,
    errors = true,
    context_keywords = true,
    layout_keywords = true,
    soft_keywords = true,
}

-- ============================================================================
-- parse_token_grammar — parse a .tokens file into a TokenGrammar
-- ============================================================================
--
-- This is the main entry point for token grammar parsing. It handles:
--   - mode: and escapes: directives
--   - keywords:, reserved:, skip:, errors: sections
--   - group NAME: sections for context-sensitive lexing
--   - Top-level token definitions (NAME = pattern)
--   - Comments (lines starting with #) and blank lines
--   - -> ALIAS syntax on definitions
--
-- @param source  String contents of a .tokens file
-- @return TokenGrammar, nil on success; nil, error_string on failure

function grammar_tools.parse_token_grammar(source)
    local grammar = TokenGrammar.new()
    local lines = {}

    -- Split source into lines. We handle both \n and \r\n line endings.
    for line in (source .. "\n"):gmatch("([^\n]*)\n") do
        lines[#lines + 1] = line
    end

    local current_section = "" -- "keywords", "reserved", "skip", "errors", or "group:NAME"

    for i, raw_line in ipairs(lines) do
        local line_number = i
        -- Strip trailing whitespace (spaces, tabs, carriage returns)
        local line = raw_line:gsub("[%s]+$", "")
        local stripped = line:match("^%s*(.-)%s*$")

        -- Skip blank lines and comments
        if stripped == "" or stripped:sub(1, 1) == "#" then
            goto continue
        end

        -- mode: directive
        if stripped:sub(1, 5) == "mode:" then
            local mode_value = stripped:sub(6):match("^%s*(.-)%s*$")
            if mode_value == "" then
                return nil, string.format(
                    "Line %d: Missing value after 'mode:'", line_number
                )
            end
            grammar.mode = mode_value
            current_section = ""
            goto continue
        end

        -- escapes: directive
        if stripped:sub(1, 8) == "escapes:" then
            local escape_value = stripped:sub(9):match("^%s*(.-)%s*$")
            if escape_value == "" then
                return nil, string.format(
                    "Line %d: Missing value after 'escapes:'", line_number
                )
            end
            grammar.escape_mode = escape_value
            current_section = ""
            goto continue
        end

        -- case_insensitive: directive (e.g., "case_insensitive: true")
        -- Marks the grammar as case-insensitive; the lexer will fold
        -- keyword lookups to lowercase so "SELECT" matches "select".
        if stripped:sub(1, 17) == "case_insensitive:" then
            local ci_value = stripped:sub(18):match("^%s*(.-)%s*$")
            if ci_value == "true" then
                grammar.case_insensitive = true
            end
            current_section = ""
            goto continue
        end

        -- case_sensitive: directive (e.g., "case_sensitive: false")
        -- "case_sensitive: false" is equivalent to "case_insensitive: true".
        -- "case_sensitive:" is 15 characters; value starts at position 16.
        if stripped:sub(1, 15) == "case_sensitive:" then
            local cs_value = stripped:sub(16):match("^%s*(.-)%s*$")
            if cs_value == "false" then
                grammar.case_insensitive = true
            end
            current_section = ""
            goto continue
        end

        -- Group headers — "group NAME:" declares a named pattern group
        if stripped:sub(1, 6) == "group " and stripped:sub(-1) == ":" then
            local group_name = stripped:sub(7, -2):match("^%s*(.-)%s*$")
            if group_name == "" then
                return nil, string.format(
                    "Line %d: Missing group name after 'group'", line_number
                )
            end
            if not is_valid_group_name(group_name) then
                return nil, string.format(
                    "Line %d: Invalid group name: %q (must be a lowercase identifier like 'tag' or 'cdata')",
                    line_number, group_name
                )
            end
            if RESERVED_GROUP_NAMES[group_name] then
                return nil, string.format(
                    "Line %d: Reserved group name: %q (cannot use default, errors, keywords, reserved, skip)",
                    line_number, group_name
                )
            end
            if grammar.groups[group_name] then
                return nil, string.format(
                    "Line %d: Duplicate group name: %q", line_number, group_name
                )
            end
            grammar.groups[group_name] = PatternGroup.new(group_name)
            current_section = "group:" .. group_name
            goto continue
        end

        -- Section headers
        if stripped == "keywords:" or stripped == "keywords :" then
            current_section = "keywords"
            goto continue
        end
        if stripped == "reserved:" or stripped == "reserved :" then
            current_section = "reserved"
            goto continue
        end
        if stripped == "skip:" or stripped == "skip :" then
            current_section = "skip"
            goto continue
        end
        if stripped == "errors:" or stripped == "errors :" then
            current_section = "errors"
            goto continue
        end
        if stripped == "context_keywords:" or stripped == "context_keywords :" then
            current_section = "context_keywords"
            goto continue
        end
        if stripped == "layout_keywords:" or stripped == "layout_keywords :" then
            current_section = "layout_keywords"
            goto continue
        end
        if stripped == "soft_keywords:" or stripped == "soft_keywords :" then
            current_section = "soft_keywords"
            goto continue
        end

        -- Inside a section: lines must be indented (start with space or tab)
        if current_section ~= "" then
            local first_char = line:sub(1, 1)
            if first_char == " " or first_char == "\t" then
                -- Dispatch based on the current section
                if current_section == "keywords" then
                    if stripped ~= "" then
                        grammar.keywords[#grammar.keywords + 1] = stripped
                    end

                elseif current_section == "context_keywords" then
                    if stripped ~= "" then
                        grammar.context_keywords[#grammar.context_keywords + 1] = stripped
                    end

                elseif current_section == "layout_keywords" then
                    if stripped ~= "" then
                        grammar.layout_keywords[#grammar.layout_keywords + 1] = stripped
                    end

                elseif current_section == "soft_keywords" then
                    if stripped ~= "" then
                        grammar.soft_keywords[#grammar.soft_keywords + 1] = stripped
                    end

                elseif current_section == "reserved" then
                    if stripped ~= "" then
                        grammar.reserved_keywords[#grammar.reserved_keywords + 1] = stripped
                    end

                elseif current_section == "skip" then
                    local eq_pos = stripped:find("=", 1, true)
                    if not eq_pos then
                        return nil, string.format(
                            "Line %d: Expected skip pattern (NAME = pattern), got: %q",
                            line_number, stripped
                        )
                    end
                    local skip_name = stripped:sub(1, eq_pos - 1):match("^%s*(.-)%s*$")
                    local skip_pattern = stripped:sub(eq_pos + 1):match("^%s*(.-)%s*$")
                    if skip_name == "" or skip_pattern == "" then
                        return nil, string.format(
                            "Line %d: Incomplete skip pattern definition: %q",
                            line_number, stripped
                        )
                    end
                    local defn, err = parse_definition(skip_pattern, skip_name, line_number)
                    if err then return nil, err end
                    grammar.skip_definitions[#grammar.skip_definitions + 1] = defn

                elseif current_section == "errors" then
                    local eq_pos = stripped:find("=", 1, true)
                    if not eq_pos then
                        return nil, string.format(
                            "Line %d: Expected error pattern (NAME = pattern), got: %q",
                            line_number, stripped
                        )
                    end
                    local err_name = stripped:sub(1, eq_pos - 1):match("^%s*(.-)%s*$")
                    local err_pattern = stripped:sub(eq_pos + 1):match("^%s*(.-)%s*$")
                    if err_name == "" or err_pattern == "" then
                        return nil, string.format(
                            "Line %d: Incomplete error pattern definition: %q",
                            line_number, stripped
                        )
                    end
                    local defn, parse_err = parse_definition(err_pattern, err_name, line_number)
                    if parse_err then return nil, parse_err end
                    grammar.error_definitions[#grammar.error_definitions + 1] = defn

                elseif current_section:sub(1, 6) == "group:" then
                    local group_name = current_section:sub(7)
                    local eq_pos = stripped:find("=", 1, true)
                    if not eq_pos then
                        return nil, string.format(
                            "Line %d: Expected token definition in group '%s' (NAME = pattern), got: %q",
                            line_number, group_name, stripped
                        )
                    end
                    local g_name = stripped:sub(1, eq_pos - 1):match("^%s*(.-)%s*$")
                    local g_pattern = stripped:sub(eq_pos + 1):match("^%s*(.-)%s*$")
                    if g_name == "" or g_pattern == "" then
                        return nil, string.format(
                            "Line %d: Incomplete definition in group '%s': %q",
                            line_number, group_name, stripped
                        )
                    end
                    local defn, parse_err = parse_definition(g_pattern, g_name, line_number)
                    if parse_err then return nil, parse_err end
                    local group = grammar.groups[group_name]
                    group.definitions[#group.definitions + 1] = defn
                end

                goto continue
            end
            -- Non-indented line — exit section
            current_section = ""
        end

        -- Top-level token definition: NAME = pattern
        local eq_pos = line:find("=", 1, true)
        if not eq_pos then
            return nil, string.format(
                "Line %d: Expected token definition (NAME = pattern)", line_number
            )
        end

        local name_part = line:sub(1, eq_pos - 1):match("^%s*(.-)%s*$")
        local pattern_part = line:sub(eq_pos + 1):match("^%s*(.-)%s*$")

        if name_part == "" then
            return nil, string.format(
                "Line %d: Missing token name", line_number
            )
        end

        if pattern_part == "" then
            return nil, string.format(
                "Line %d: Missing pattern after '='", line_number
            )
        end

        local defn, err = parse_definition(pattern_part, name_part, line_number)
        if err then return nil, err end
        grammar.definitions[#grammar.definitions + 1] = defn

        ::continue::
    end

    return grammar, nil
end

-- ============================================================================
-- validate_definitions — check a list of token definitions for problems
-- ============================================================================
--
-- This is a helper used by validate_token_grammar to check definitions from
-- the top-level list, skip section, error section, and pattern groups.
--
-- Checks performed:
--   - Duplicate token names within the list
--   - Empty patterns (should be caught during parsing, but double-checked)
--   - Non-UPPER_CASE token names (convention violation)
--   - Non-UPPER_CASE alias names (convention violation)
--
-- Note: We do NOT validate regex patterns in Lua because Lua uses its own
-- pattern system (not PCRE/RE2). The patterns are meant for the lexer which
-- uses a regex engine. We trust that the Go or Python validator checks regex
-- validity.
--
-- @param definitions  List of TokenDefinition instances
-- @param label        Context label for error messages (e.g. "token", "skip pattern")
-- @return table  List of issue strings

local function validate_definitions(definitions, label)
    local issues = {}
    local seen_names = {} -- name -> first line number

    for _, defn in ipairs(definitions) do
        -- Duplicate check
        if seen_names[defn.name] then
            issues[#issues + 1] = string.format(
                "Line %d: Duplicate %s name '%s' (first defined on line %d)",
                defn.line_number, label, defn.name, seen_names[defn.name]
            )
        else
            seen_names[defn.name] = defn.line_number
        end

        -- Empty pattern check
        if defn.pattern == "" then
            issues[#issues + 1] = string.format(
                "Line %d: Empty pattern for %s '%s'",
                defn.line_number, label, defn.name
            )
        end

        -- Naming convention check: token names should be UPPER_CASE
        if defn.name ~= defn.name:upper() then
            issues[#issues + 1] = string.format(
                "Line %d: Token name '%s' should be UPPER_CASE",
                defn.line_number, defn.name
            )
        end

        -- Alias convention check
        if defn.alias ~= "" and defn.alias ~= defn.alias:upper() then
            issues[#issues + 1] = string.format(
                "Line %d: Alias '%s' for token '%s' should be UPPER_CASE",
                defn.line_number, defn.alias, defn.name
            )
        end
    end

    return issues
end

-- ============================================================================
-- validate_token_grammar — lint pass on a parsed TokenGrammar
-- ============================================================================
--
-- This runs after parsing succeeds. It looks for semantic issues that would
-- cause problems downstream:
--
--   - Duplicate token names within each definition list
--   - Empty patterns
--   - Non-UPPER_CASE token names and aliases
--   - Invalid lexer mode (only "indentation" is supported)
--   - Invalid escape mode (only "none" is supported)
--   - Invalid group names
--   - Empty pattern groups (no definitions)
--   - Definition issues within groups
--
-- @param grammar  TokenGrammar instance
-- @return table  List of issue strings (empty = all clear)

function grammar_tools.validate_token_grammar(grammar)
    local issues = {}

    -- Validate regular definitions
    for _, issue in ipairs(validate_definitions(grammar.definitions, "token")) do
        issues[#issues + 1] = issue
    end

    -- Validate skip definitions
    for _, issue in ipairs(validate_definitions(grammar.skip_definitions, "skip pattern")) do
        issues[#issues + 1] = issue
    end

    -- Validate error definitions
    for _, issue in ipairs(validate_definitions(grammar.error_definitions, "error pattern")) do
        issues[#issues + 1] = issue
    end

    -- Validate mode
    if grammar.mode ~= "" and grammar.mode ~= "indentation" and grammar.mode ~= "layout" then
        issues[#issues + 1] = string.format(
            "Unknown lexer mode '%s' (only 'indentation' and 'layout' are supported)",
            grammar.mode
        )
    end

    if grammar.mode == "layout" and #grammar.layout_keywords == 0 then
        issues[#issues + 1] = "Layout mode requires a non-empty layout_keywords section"
    end

    -- Validate escape mode
    if grammar.escape_mode ~= "" and grammar.escape_mode ~= "none" then
        issues[#issues + 1] = string.format(
            "Unknown escape mode '%s' (only 'none' is supported)",
            grammar.escape_mode
        )
    end

    -- Validate pattern groups
    -- Sort group names for deterministic output
    local group_names = {}
    for name, _ in pairs(grammar.groups) do
        group_names[#group_names + 1] = name
    end
    table.sort(group_names)

    for _, group_name in ipairs(group_names) do
        local group = grammar.groups[group_name]

        -- Group name format
        if not is_valid_group_name(group_name) then
            issues[#issues + 1] = string.format(
                "Invalid group name '%s' (must be a lowercase identifier)",
                group_name
            )
        end

        -- Empty group warning
        if #group.definitions == 0 then
            issues[#issues + 1] = string.format(
                "Empty pattern group '%s' (has no token definitions)",
                group_name
            )
        end

        -- Validate definitions within the group
        local group_label = string.format("group '%s' token", group_name)
        for _, issue in ipairs(validate_definitions(group.definitions, group_label)) do
            issues[#issues + 1] = issue
        end
    end

    return issues
end

-- ============================================================================
-- Parser grammar element types
-- ============================================================================
--
-- The parser grammar is represented as an AST (abstract syntax tree). Each
-- node is one of these types, identified by a "type" field. This is the Lua
-- equivalent of Go's interface + concrete types pattern.
--
-- Element types:
--
--   RuleReference  — a reference to another rule (lowercase) or token (UPPERCASE)
--                    { type="rule_reference", name="expr", is_token=false }
--
--   Literal        — a quoted string literal in the grammar
--                    { type="literal", value="+" }
--
--   Sequence       — multiple elements that must appear in order
--                    { type="sequence", elements={...} }
--
--   Alternation    — a choice between alternatives (separated by |)
--                    { type="alternation", choices={...} }
--
--   Repetition     — zero or more occurrences (delimited by { ... })
--                    { type="repetition", element=... }
--
--   Optional       — zero or one occurrence (delimited by [ ... ])
--                    { type="optional", element=... }
--
--   Group          — parenthesized grouping (delimited by ( ... ))
--                    { type="group", element=... }

--- Create a RuleReference element.
-- @param name     Reference name
-- @param is_token Whether this references a token (UPPERCASE) vs a rule
-- @return table   Grammar element
local function make_rule_reference(name, is_token)
    return { type = "rule_reference", name = name, is_token = is_token }
end

--- Create a Literal element.
-- @param value  The literal string value
-- @return table Grammar element
local function make_literal(value)
    return { type = "literal", value = value }
end

--- Create a Sequence element.
-- @param elements  List of child elements
-- @return table    Grammar element
local function make_sequence(elements)
    return { type = "sequence", elements = elements }
end

--- Create an Alternation element.
-- @param choices  List of alternative elements
-- @return table   Grammar element
local function make_alternation(choices)
    return { type = "alternation", choices = choices }
end

--- Create a Repetition element (zero or more).
-- @param element  The repeated element
-- @return table   Grammar element
local function make_repetition(element)
    return { type = "repetition", element = element }
end

--- Create an Optional element (zero or one).
-- @param element  The optional element
-- @return table   Grammar element
local function make_optional(element)
    return { type = "optional", element = element }
end

--- Create a Group element (parenthesized).
-- @param element  The grouped element
-- @return table   Grammar element
local function make_group(element)
    return { type = "group", element = element }
end

--- Create a PositiveLookahead element (written as &element in the grammar).
-- Succeeds if element matches at the current position, consuming no input.
-- @param element  The element to check
-- @return table   Grammar element
local function make_positive_lookahead(element)
    return { type = "positive_lookahead", element = element }
end

--- Create a NegativeLookahead element (written as !element in the grammar).
-- Succeeds if element does NOT match at the current position, consuming no input.
-- @param element  The element to check
-- @return table   Grammar element
local function make_negative_lookahead(element)
    return { type = "negative_lookahead", element = element }
end

--- Create a OneOrMore element (written as { element }+ in the grammar).
-- Like zero-or-more but requires at least one match.
-- @param element  The repeated element
-- @return table   Grammar element
local function make_one_or_more(element)
    return { type = "one_or_more", element = element }
end

--- Create a SeparatedRepetition element (written as { element // separator }).
-- Matches zero or more occurrences of element separated by separator.
-- @param element    The repeated element
-- @param separator  The separator element
-- @param at_least_one boolean  Whether at least one element is required
-- @return table     Grammar element
local function make_separated_repetition(element, separator, at_least_one)
    return { type = "separated_repetition", element = element, separator = separator, at_least_one = at_least_one }
end

-- Export element constructors for testing
grammar_tools.make_rule_reference = make_rule_reference
grammar_tools.make_literal = make_literal
grammar_tools.make_sequence = make_sequence
grammar_tools.make_alternation = make_alternation
grammar_tools.make_repetition = make_repetition
grammar_tools.make_optional = make_optional
grammar_tools.make_group = make_group
grammar_tools.make_positive_lookahead = make_positive_lookahead
grammar_tools.make_negative_lookahead = make_negative_lookahead
grammar_tools.make_one_or_more = make_one_or_more
grammar_tools.make_separated_repetition = make_separated_repetition

-- ============================================================================
-- GrammarRule — a single production rule in a parser grammar
-- ============================================================================
--
-- Fields:
--   name        (string)   Rule name, e.g. "expression"
--   body        (table)    Grammar element (the right-hand side)
--   line_number (integer)  Source line number

local GrammarRule = {}
GrammarRule.__index = GrammarRule

function GrammarRule.new(name, body, line_number)
    local self = setmetatable({}, GrammarRule)
    self.name = name
    self.body = body
    self.line_number = line_number
    return self
end

grammar_tools.GrammarRule = GrammarRule

-- ============================================================================
-- ParserGrammar — the complete contents of a parsed .grammar file
-- ============================================================================

local ParserGrammar = {}
ParserGrammar.__index = ParserGrammar

function ParserGrammar.new(rules)
    local self = setmetatable({}, ParserGrammar)
    self.rules = rules or {}
    return self
end

--- Return the set of all defined rule names.
-- @return table  Set of rule names (name -> true)
function ParserGrammar:rule_names()
    local names = {}
    for _, rule in ipairs(self.rules) do
        names[rule.name] = true
    end
    return names
end

--- Return all lowercase rule names referenced in rule bodies.
-- These are the non-token references that should correspond to other
-- rules in this grammar.
-- @return table  Set of referenced rule names (name -> true)
function ParserGrammar:rule_references()
    local refs = {}
    for _, rule in ipairs(self.rules) do
        collect_rule_refs(rule.body, refs)
    end
    return refs
end

--- Return all UPPERCASE names referenced in rule bodies.
-- These are token references that should correspond to tokens defined
-- in a .tokens file.
-- @return table  Set of referenced token names (name -> true)
function ParserGrammar:token_references()
    local refs = {}
    for _, rule in ipairs(self.rules) do
        collect_token_refs(rule.body, refs)
    end
    return refs
end

grammar_tools.ParserGrammar = ParserGrammar

-- ============================================================================
-- collect_rule_refs / collect_token_refs — walk the AST collecting references
-- ============================================================================
--
-- These recursive functions walk the grammar element tree and collect
-- lowercase (rule) or UPPERCASE (token) references into a set.

--- Collect lowercase rule references from a grammar element tree.
-- @param element  Grammar element to walk
-- @param refs     Set to add references to (name -> true)
function collect_rule_refs(element, refs)
    if not element then return end

    if element.type == "rule_reference" then
        if not element.is_token then
            refs[element.name] = true
        end
    elseif element.type == "sequence" then
        for _, sub in ipairs(element.elements) do
            collect_rule_refs(sub, refs)
        end
    elseif element.type == "alternation" then
        for _, choice in ipairs(element.choices) do
            collect_rule_refs(choice, refs)
        end
    elseif element.type == "repetition"
        or element.type == "optional"
        or element.type == "group"
        or element.type == "positive_lookahead"
        or element.type == "negative_lookahead"
        or element.type == "one_or_more" then
        collect_rule_refs(element.element, refs)
    elseif element.type == "separated_repetition" then
        collect_rule_refs(element.element, refs)
        collect_rule_refs(element.separator, refs)
    end
end

--- Collect UPPERCASE token references from a grammar element tree.
-- @param element  Grammar element to walk
-- @param refs     Set to add references to (name -> true)
function collect_token_refs(element, refs)
    if not element then return end

    if element.type == "rule_reference" then
        if element.is_token then
            refs[element.name] = true
        end
    elseif element.type == "sequence" then
        for _, sub in ipairs(element.elements) do
            collect_token_refs(sub, refs)
        end
    elseif element.type == "alternation" then
        for _, choice in ipairs(element.choices) do
            collect_token_refs(choice, refs)
        end
    elseif element.type == "repetition"
        or element.type == "optional"
        or element.type == "group"
        or element.type == "positive_lookahead"
        or element.type == "negative_lookahead"
        or element.type == "one_or_more" then
        collect_token_refs(element.element, refs)
    elseif element.type == "separated_repetition" then
        collect_token_refs(element.element, refs)
        collect_token_refs(element.separator, refs)
    end
end

-- ============================================================================
-- tokenize_grammar — tokenize a .grammar file source string
-- ============================================================================
--
-- The grammar tokenizer breaks source text into a flat list of tokens:
--
--   IDENT    — identifiers (rule names and token names)
--   STRING   — quoted string literals
--   EQUALS   — the = sign (separates rule name from body)
--   SEMI     — the ; sign (terminates a rule)
--   PIPE     — the | sign (alternation)
--   LBRACE / RBRACE   — { } (repetition)
--   LBRACKET / RBRACKET — [ ] (optional)
--   LPAREN / RPAREN   — ( ) (grouping)
--   EOF      — end of input
--
-- Comments (# to end of line) and whitespace are skipped.
--
-- @param source  String contents of a .grammar file
-- @return table of tokens, nil on success; nil, error_string on failure

local function tokenize_grammar(source)
    local tokens = {}
    local lines = {}

    for line in (source .. "\n"):gmatch("([^\n]*)\n") do
        lines[#lines + 1] = line
    end

    for i, raw_line in ipairs(lines) do
        local line_num = i
        local line = raw_line:gsub("[%s]+$", "")
        local stripped = line:match("^%s*(.-)%s*$")

        if stripped == "" or stripped:sub(1, 1) == "#" then
            goto next_line
        end

        local j = 1
        while j <= #line do
            local ch = line:sub(j, j)

            -- Skip whitespace
            if ch == " " or ch == "\t" then
                j = j + 1
                goto next_char
            end

            -- Comment — skip rest of line
            if ch == "#" then
                break
            end

            -- Single-character tokens
            if ch == "=" then
                tokens[#tokens + 1] = { kind = "EQUALS", value = "=", line = line_num }
                j = j + 1
            elseif ch == ";" then
                tokens[#tokens + 1] = { kind = "SEMI", value = ";", line = line_num }
                j = j + 1
            elseif ch == "|" then
                tokens[#tokens + 1] = { kind = "PIPE", value = "|", line = line_num }
                j = j + 1
            elseif ch == "{" then
                tokens[#tokens + 1] = { kind = "LBRACE", value = "{", line = line_num }
                j = j + 1
            elseif ch == "}" then
                tokens[#tokens + 1] = { kind = "RBRACE", value = "}", line = line_num }
                j = j + 1
            elseif ch == "[" then
                tokens[#tokens + 1] = { kind = "LBRACKET", value = "[", line = line_num }
                j = j + 1
            elseif ch == "]" then
                tokens[#tokens + 1] = { kind = "RBRACKET", value = "]", line = line_num }
                j = j + 1
            elseif ch == "(" then
                tokens[#tokens + 1] = { kind = "LPAREN", value = "(", line = line_num }
                j = j + 1
            elseif ch == ")" then
                tokens[#tokens + 1] = { kind = "RPAREN", value = ")", line = line_num }
                j = j + 1

            -- Lookahead predicates
            elseif ch == "&" then
                tokens[#tokens + 1] = { kind = "AMPERSAND", value = "&", line = line_num }
                j = j + 1
            elseif ch == "!" then
                tokens[#tokens + 1] = { kind = "BANG", value = "!", line = line_num }
                j = j + 1

            -- One-or-more suffix
            elseif ch == "+" then
                tokens[#tokens + 1] = { kind = "PLUS", value = "+", line = line_num }
                j = j + 1

            -- Separator operator (// inside repetition braces)
            elseif ch == "/" and j + 1 <= #line and line:sub(j+1, j+1) == "/" then
                tokens[#tokens + 1] = { kind = "DOUBLE_SLASH", value = "//", line = line_num }
                j = j + 2

            -- String literal
            elseif ch == '"' then
                local k = j + 1
                while k <= #line and line:sub(k, k) ~= '"' do
                    if line:sub(k, k) == "\\" then
                        k = k + 1
                    end
                    k = k + 1
                end
                if k > #line then
                    return nil, string.format(
                        "Line %d: Unterminated string literal", line_num
                    )
                end
                tokens[#tokens + 1] = {
                    kind = "STRING",
                    value = line:sub(j + 1, k - 1),
                    line = line_num,
                }
                j = k + 1

            -- Identifier
            elseif ch:match("[%a_]") then
                local k = j
                while k <= #line and line:sub(k, k):match("[%w_]") do
                    k = k + 1
                end
                tokens[#tokens + 1] = {
                    kind = "IDENT",
                    value = line:sub(j, k - 1),
                    line = line_num,
                }
                j = k

            else
                return nil, string.format(
                    "Line %d: Unexpected character %q", line_num, ch
                )
            end

            ::next_char::
        end

        ::next_line::
    end

    tokens[#tokens + 1] = { kind = "EOF", value = "", line = #lines }
    return tokens, nil
end

-- ============================================================================
-- Parser — recursive descent parser for .grammar files
-- ============================================================================
--
-- The parser consumes the token stream produced by tokenize_grammar() and
-- builds an AST of grammar elements. It uses a simple recursive descent
-- approach with these grammar rules:
--
--   grammar   = { rule }
--   rule      = IDENT "=" body ";"
--   body      = sequence { "|" sequence }
--   sequence  = element { element }
--   element   = IDENT | STRING | "{" body "}" | "[" body "]" | "(" body ")"

local Parser = {}
Parser.__index = Parser

function Parser.new(tokens)
    local self = setmetatable({}, Parser)
    self.tokens = tokens
    self.pos = 1
    return self
end

--- Return the current token without consuming it.
function Parser:peek()
    return self.tokens[self.pos]
end

--- Consume and return the current token.
function Parser:advance()
    local tok = self.tokens[self.pos]
    self.pos = self.pos + 1
    return tok
end

--- Consume the current token, returning an error if it's not the expected kind.
-- @param kind  Expected token kind string
-- @return token, nil on success; nil, error_string on failure
function Parser:expect(kind)
    local tok = self:advance()
    if tok.kind ~= kind then
        return nil, string.format(
            "Line %d: Expected %s, got %s", tok.line, kind, tok.kind
        )
    end
    return tok, nil
end

--- Parse all rules until EOF.
-- @return list of GrammarRule, nil on success; nil, error on failure
function Parser:parse()
    local rules = {}
    while self:peek().kind ~= "EOF" do
        local rule, err = self:parse_rule()
        if err then return nil, err end
        rules[#rules + 1] = rule
    end
    return rules, nil
end

--- Parse a single rule: IDENT = body ;
function Parser:parse_rule()
    local name_tok, err = self:expect("IDENT")
    if err then return nil, err end

    _, err = self:expect("EQUALS")
    if err then return nil, err end

    local body
    body, err = self:parse_body()
    if err then return nil, err end

    _, err = self:expect("SEMI")
    if err then return nil, err end

    return GrammarRule.new(name_tok.value, body, name_tok.line), nil
end

--- Parse a body: sequence { "|" sequence }
-- If there's only one alternative, returns it directly (no Alternation wrapper).
function Parser:parse_body()
    local first, err = self:parse_sequence()
    if err then return nil, err end

    local alternatives = { first }

    while self:peek().kind == "PIPE" do
        self:advance()
        local seq
        seq, err = self:parse_sequence()
        if err then return nil, err end
        alternatives[#alternatives + 1] = seq
    end

    if #alternatives == 1 then
        return alternatives[1], nil
    end
    return make_alternation(alternatives), nil
end

--- Parse a sequence: element { element }
-- Stops at |, ;, }, ], ), or EOF. Returns a single element if only one is
-- found (no Sequence wrapper for a single element).
function Parser:parse_sequence()
    local elements = {}

    while true do
        local kind = self:peek().kind
        if kind == "PIPE" or kind == "SEMI" or kind == "RBRACE"
           or kind == "RBRACKET" or kind == "RPAREN" or kind == "EOF"
           or kind == "DOUBLE_SLASH" then
            break
        end
        local elem, err = self:parse_element()
        if err then return nil, err end
        elements[#elements + 1] = elem
    end

    if #elements == 0 then
        return nil, string.format(
            "Line %d: Expected at least one element in sequence",
            self:peek().line
        )
    end
    if #elements == 1 then
        return elements[1], nil
    end
    return make_sequence(elements), nil
end

--- Parse a single element: IDENT | STRING | & elem | ! elem | { body } | [ body ] | ( body )
function Parser:parse_element()
    local tok = self:peek()

    -- Lookahead predicates: & (positive) and ! (negative)
    if tok.kind == "AMPERSAND" then
        self:advance()
        local inner, err = self:parse_element()
        if err then return nil, err end
        return make_positive_lookahead(inner), nil
    end

    if tok.kind == "BANG" then
        self:advance()
        local inner, err = self:parse_element()
        if err then return nil, err end
        return make_negative_lookahead(inner), nil
    end

    if tok.kind == "IDENT" then
        self:advance()
        -- Determine if this is a token reference (starts with uppercase)
        -- or a rule reference (starts with lowercase)
        local first_char = tok.value:sub(1, 1)
        local is_token = (first_char >= "A" and first_char <= "Z")
        return make_rule_reference(tok.value, is_token), nil

    elseif tok.kind == "STRING" then
        self:advance()
        return make_literal(tok.value), nil

    elseif tok.kind == "LBRACE" then
        self:advance()
        local body, err = self:parse_body()
        if err then return nil, err end

        -- Check for separator syntax: { element // separator }
        if self:peek().kind == "DOUBLE_SLASH" then
            self:advance()  -- consume //
            local separator
            separator, err = self:parse_body()
            if err then return nil, err end
            _, err = self:expect("RBRACE")
            if err then return nil, err end
            -- Check for + suffix (one-or-more)
            local at_least_one = (self:peek().kind == "PLUS")
            if at_least_one then self:advance() end
            return make_separated_repetition(body, separator, at_least_one), nil
        end

        _, err = self:expect("RBRACE")
        if err then return nil, err end
        -- Check for + suffix: { element }+
        if self:peek().kind == "PLUS" then
            self:advance()
            return make_one_or_more(body), nil
        end
        return make_repetition(body), nil

    elseif tok.kind == "LBRACKET" then
        self:advance()
        local body, err = self:parse_body()
        if err then return nil, err end
        _, err = self:expect("RBRACKET")
        if err then return nil, err end
        return make_optional(body), nil

    elseif tok.kind == "LPAREN" then
        self:advance()
        local body, err = self:parse_body()
        if err then return nil, err end
        _, err = self:expect("RPAREN")
        if err then return nil, err end
        return make_group(body), nil
    end

    return nil, string.format(
        "Line %d: Unexpected token %s", tok.line, tok.kind
    )
end

-- ============================================================================
-- parse_parser_grammar — parse a .grammar file into a ParserGrammar
-- ============================================================================
--
-- @param source  String contents of a .grammar file
-- @return ParserGrammar, nil on success; nil, error_string on failure

function grammar_tools.parse_parser_grammar(source)
    local tokens, err = tokenize_grammar(source)
    if err then return nil, err end

    local p = Parser.new(tokens)
    local rules
    rules, err = p:parse()
    if err then return nil, err end

    return ParserGrammar.new(rules), nil
end

-- ============================================================================
-- sorted_keys — return the keys of a table in sorted order
-- ============================================================================
--
-- Used to ensure deterministic output for validation messages.
--
-- @param t  Table with string keys
-- @return table  Sorted list of keys

local function sorted_keys(t)
    local keys = {}
    for k, _ in pairs(t) do
        keys[#keys + 1] = k
    end
    table.sort(keys)
    return keys
end

-- ============================================================================
-- validate_parser_grammar — check a parsed ParserGrammar for problems
-- ============================================================================
--
-- Validation checks:
--   - Duplicate rule names
--   - Non-lowercase rule names (convention violation)
--   - Undefined rule references (lowercase name not defined as a rule)
--   - Undefined token references (UPPERCASE name not in token_names, if provided)
--   - Unreachable rules (defined but never referenced; start rule exempt)
--
-- Synthetic tokens (NEWLINE, INDENT, DEDENT, EOF) are always valid.
--
-- @param grammar      ParserGrammar instance
-- @param token_names  Optional set of valid token names (name -> true).
--                     When non-nil, UPPERCASE references are checked against it.
-- @return table  List of issue strings (empty = all clear)

function grammar_tools.validate_parser_grammar(grammar, token_names)
    local issues = {}

    local defined = grammar:rule_names()
    local referenced_rules = grammar:rule_references()
    local referenced_tokens = grammar:token_references()

    -- Duplicate rule names
    local seen = {} -- name -> first line number
    for _, rule in ipairs(grammar.rules) do
        if seen[rule.name] then
            issues[#issues + 1] = string.format(
                "Line %d: Duplicate rule name '%s' (first defined on line %d)",
                rule.line_number, rule.name, seen[rule.name]
            )
        else
            seen[rule.name] = rule.line_number
        end
    end

    -- Non-lowercase rule names
    for _, rule in ipairs(grammar.rules) do
        if rule.name ~= rule.name:lower() then
            issues[#issues + 1] = string.format(
                "Line %d: Rule name '%s' should be lowercase",
                rule.line_number, rule.name
            )
        end
    end

    -- Undefined rule references
    local sorted_rule_refs = sorted_keys(referenced_rules)
    for _, ref in ipairs(sorted_rule_refs) do
        if not defined[ref] then
            issues[#issues + 1] = string.format(
                "Undefined rule reference: '%s'", ref
            )
        end
    end

    -- Undefined token references
    -- Synthetic tokens are always valid:
    --   NEWLINE — emitted at bare '\n' when skip pattern excludes newlines
    --   INDENT/DEDENT — emitted in indentation mode
    --   EOF — always emitted at end of input
    local synthetic_tokens = {
        NEWLINE = true,
        INDENT = true,
        DEDENT = true,
        EOF = true,
    }
    if token_names then
        local sorted_tok_refs = sorted_keys(referenced_tokens)
        for _, ref in ipairs(sorted_tok_refs) do
            if not token_names[ref] and not synthetic_tokens[ref] then
                issues[#issues + 1] = string.format(
                    "Undefined token reference: '%s'", ref
                )
            end
        end
    end

    -- Unreachable rules: defined but never referenced.
    -- The first rule is the start symbol and is always reachable.
    if #grammar.rules > 0 then
        local start_rule = grammar.rules[1].name
        for _, rule in ipairs(grammar.rules) do
            if rule.name ~= start_rule and not referenced_rules[rule.name] then
                issues[#issues + 1] = string.format(
                    "Line %d: Rule '%s' is defined but never referenced (unreachable)",
                    rule.line_number, rule.name
                )
            end
        end
    end

    return issues
end

-- ============================================================================
-- cross_validate — check that a TokenGrammar and ParserGrammar are consistent
-- ============================================================================
--
-- The whole point of having two separate grammar files is that they reference
-- each other: the .grammar file uses UPPERCASE names to refer to tokens
-- defined in the .tokens file. This function checks that the two files are
-- consistent.
--
-- Checks:
--
--   1. Missing token references (errors): Every UPPERCASE name in the grammar
--      must correspond to a token definition. If not, the generated parser
--      will try to match a token type that the lexer never produces.
--
--   2. Unused tokens (warnings): Every token defined in the .tokens file
--      should ideally be referenced somewhere in the grammar. Unused tokens
--      suggest either a typo or leftover cruft.
--
-- Synthetic tokens (NEWLINE, INDENT, DEDENT, EOF) are always valid.
--
-- @param token_grammar   TokenGrammar instance
-- @param parser_grammar  ParserGrammar instance
-- @return table  List of error/warning strings (empty = fully consistent)

function grammar_tools.cross_validate(token_grammar, parser_grammar)
    local issues = {}

    -- Build the set of all token names the parser can reference.
    local defined_tokens = token_grammar:token_names()

    -- Synthetic tokens are always valid
    defined_tokens["NEWLINE"] = true
    defined_tokens["EOF"] = true
    if token_grammar.mode == "indentation" then
        defined_tokens["INDENT"] = true
        defined_tokens["DEDENT"] = true
    end

    local referenced_tokens = parser_grammar:token_references()

    -- Missing token references (errors)
    local sorted_refs = sorted_keys(referenced_tokens)
    for _, ref in ipairs(sorted_refs) do
        if not defined_tokens[ref] then
            issues[#issues + 1] = string.format(
                "Error: Grammar references token '%s' which is not defined in the tokens file",
                ref
            )
        end
    end

    -- Unused tokens (warnings)
    -- A definition is "used" if its name OR alias is referenced anywhere
    for _, defn in ipairs(token_grammar.definitions) do
        local is_used = referenced_tokens[defn.name]
        if defn.alias ~= "" and referenced_tokens[defn.alias] then
            is_used = true
        end

        if not is_used then
            issues[#issues + 1] = string.format(
                "Warning: Token '%s' (line %d) is defined but never used in the grammar",
                defn.name, defn.line_number
            )
        end
    end

    return issues
end

-- ===========================================================================
-- Compiler delegations
-- ===========================================================================

local Compiler = require("coding_adventures.grammar_tools.compiler")
grammar_tools.compile_token_grammar  = Compiler.compile_token_grammar
grammar_tools.compile_parser_grammar = Compiler.compile_parser_grammar

return grammar_tools
