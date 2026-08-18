-- xml_lexer -- Tokenizes XML text using the grammar-driven infrastructure
-- =========================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `xml.tokens` grammar file to configure the tokenizer.
--
-- # What is XML tokenization?
--
-- XML is *context-sensitive* at the lexical level: the same character
-- can mean different things depending on where it appears in the document.
-- For example, `<` starts a tag opener in content context, but inside a
-- CDATA section `<![CDATA[ ... ]]>` it is plain text.
--
-- The `xml.tokens` grammar handles this via **pattern groups**:
--
--   default (implicit) — content between tags: TEXT, ENTITY_REF, CHAR_REF,
--                        COMMENT_START, CDATA_START, PI_START,
--                        CLOSE_TAG_START, OPEN_TAG_START
--
--   tag    — inside a tag opener/closer: TAG_NAME, ATTR_EQUALS,
--            ATTR_VALUE (aliased from ATTR_VALUE_DQ / ATTR_VALUE_SQ),
--            TAG_CLOSE, SELF_CLOSE, SLASH
--
--   comment — inside <!-- ... -->: COMMENT_TEXT, COMMENT_END
--   cdata   — inside <![CDATA[ ... ]]>: CDATA_TEXT, CDATA_END
--   pi      — inside <? ... ?>: PI_TARGET, PI_TEXT, PI_END
--
-- The GrammarLexer switches between groups when specific tokens are seen.
-- This module registers an `on_token` callback to drive those transitions.
--
-- # Callback-driven group switching
--
-- The `xml.tokens` grammar comment describes the rules:
--
--   OPEN_TAG_START  or CLOSE_TAG_START → push("tag")
--   TAG_CLOSE       or SELF_CLOSE      → pop()
--   COMMENT_START                      → push("comment")
--   COMMENT_END                        → pop()
--   CDATA_START                        → push("cdata")
--   CDATA_END                          → pop()
--   PI_START                           → push("pi")
--   PI_END                             → pop()
--
-- # Grammar loading
--
-- The grammar is compiled ahead of time from `xml.tokens` via
-- `grammar-tools compile-tokens` and required as native Lua data — no
-- disk I/O at runtime.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.1"

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for XML.
-- @return TokenGrammar  The compiled XML token grammar.
local function get_grammar()
    if not _grammar_cache then
        _grammar_cache = require("coding_adventures.xml_lexer._grammar").token_grammar()
    end
    return _grammar_cache
end

-- =========================================================================
-- Group-switching callback
-- =========================================================================
--
-- XML tokenization is context-sensitive: the active pattern group depends
-- on which tokens have been seen.  We register an `on_token` callback on
-- the GrammarLexer so it can push/pop groups as tokens are emitted.
--
-- The callback receives a `LexerContext` with `push_group` and `pop_group`.
-- We key on the *effective* token type (after alias resolution).

--- Build and register the XML group-switch callback on a GrammarLexer.
-- @param gl  GrammarLexer  The lexer to configure.
local function attach_xml_callbacks(gl)
    gl:set_on_token(function(token, ctx)
        local t = token.type_name

        -- Entering a tag: switch to "tag" group so we recognise
        -- TAG_NAME, ATTR_VALUE, TAG_CLOSE, etc.
        if t == "OPEN_TAG_START" or t == "CLOSE_TAG_START" then
            ctx:push_group("tag")

        -- Leaving a tag: return to the group we were in before.
        elseif t == "TAG_CLOSE" or t == "SELF_CLOSE" then
            ctx:pop_group()

        -- XML comments: push "comment" group.
        elseif t == "COMMENT_START" then
            ctx:push_group("comment")
        elseif t == "COMMENT_END" then
            ctx:pop_group()

        -- CDATA sections: push "cdata" group.
        elseif t == "CDATA_START" then
            ctx:push_group("cdata")
        elseif t == "CDATA_END" then
            ctx:pop_group()

        -- Processing instructions: push "pi" group.
        elseif t == "PI_START" then
            ctx:push_group("pi")

        -- PI_TARGET is only ever the first token in a PI. Swap (not push)
        -- from "pi" to "pi_body": once matched, the rest of the body must
        -- never be re-offered PI_TARGET's pattern (see xml.tokens' pi/
        -- pi_body groups). PI_END's single pop_group() below still
        -- returns straight past this swap.
        elseif t == "PI_TARGET" then
            ctx:pop_group()
            ctx:push_group("pi_body")

        elseif t == "PI_END" then
            ctx:pop_group()
        end
    end)
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize an XML source string.
--
-- Loads the `xml.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer` with a group-switching callback that handles
-- XML's context-sensitive lexical rules.
--
-- Returns the complete flat token list, including a terminal `EOF` token.
--
-- Token types emitted:
--   TEXT, ENTITY_REF, CHAR_REF, COMMENT_START, CDATA_START, PI_START,
--   CLOSE_TAG_START, OPEN_TAG_START, TAG_NAME, ATTR_EQUALS, ATTR_VALUE,
--   TAG_CLOSE, SELF_CLOSE, SLASH, COMMENT_TEXT, COMMENT_END, CDATA_TEXT,
--   CDATA_END, PI_TARGET, PI_TEXT, PI_END, EOF.
--
-- @param source string  The XML text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local xml_lexer = require("coding_adventures.xml_lexer")
--   local tokens = xml_lexer.tokenize('<root attr="v">text</root>')
--   -- tokens[1].type  → "OPEN_TAG_START"
--   -- tokens[1].value → "<"
--   -- tokens[2].type  → "TAG_NAME"
--   -- tokens[2].value → "root"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    attach_xml_callbacks(gl)
    local raw     = gl:tokenize()
    local tokens  = {}
    for _, tok in ipairs(raw) do
        tokens[#tokens + 1] = {
            type  = tok.type_name,
            value = tok.value,
            line  = tok.line,
            col   = tok.column,
        }
    end
    return tokens
end

--- Return the cached (or freshly loaded) TokenGrammar for XML.
-- @return TokenGrammar  The parsed XML token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
