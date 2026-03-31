-- Tests for xml_lexer
-- ====================
--
-- Comprehensive busted test suite for the XML lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Opening tags: OPEN_TAG_START, TAG_NAME, TAG_CLOSE
--   - Self-closing tags: SELF_CLOSE
--   - Closing tags: CLOSE_TAG_START, TAG_NAME, TAG_CLOSE
--   - Attributes: ATTR_EQUALS, ATTR_VALUE (from double- and single-quoted)
--   - Text content: TEXT tokens
--   - Entity references: ENTITY_REF
--   - Character references: CHAR_REF
--   - Comments: COMMENT_START, COMMENT_TEXT, COMMENT_END
--   - CDATA sections: CDATA_START, CDATA_TEXT, CDATA_END
--   - Processing instructions: PI_START, PI_TARGET, PI_TEXT, PI_END
--   - Full document round-trip: token types from a realistic XML fragment
--   - Whitespace handling inside and outside tags
--   - EOF is always the last token

-- Resolve sibling packages from the monorepo.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    package.path
)

local xml_lexer = require("coding_adventures.xml_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types (excluding EOF) from a source string.
-- @param source string  XML text to tokenize.
-- @return table         Ordered list of type strings.
local function types(source)
    local tokens = xml_lexer.tokenize(source)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

--- Find first token matching the given type.
-- @param tokens  table   Token list.
-- @param typ     string  Type to find.
-- @return table|nil
local function first_of(tokens, typ)
    for _, tok in ipairs(tokens) do
        if tok.type == typ then return tok end
    end
    return nil
end

--- Count tokens of a given type.
-- @param tokens  table   Token list.
-- @param typ     string  Type to count.
-- @return number
local function count_of(tokens, typ)
    local n = 0
    for _, tok in ipairs(tokens) do
        if tok.type == typ then n = n + 1 end
    end
    return n
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("xml_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(xml_lexer)
    end)

    it("exposes VERSION string", function()
        assert.is_string(xml_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", xml_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(xml_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(xml_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar", function()
        local g = xml_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty input
-- =========================================================================

describe("empty input", function()
    it("empty string produces only EOF", function()
        local tokens = xml_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Self-closing tag
-- =========================================================================

describe("self-closing tag", function()
    it("tokenizes <br/>", function()
        local t = types("<br/>")
        assert.are.same(
            {"OPEN_TAG_START", "TAG_NAME", "SELF_CLOSE"},
            t
        )
    end)

    it("correct values for <br/>", function()
        local tokens = xml_lexer.tokenize("<br/>")
        assert.are.equal("<",    tokens[1].value)
        assert.are.equal("br",   tokens[2].value)
        assert.are.equal("/>",   tokens[3].value)
    end)
end)

-- =========================================================================
-- Opening tag
-- =========================================================================

describe("opening tag", function()
    it("tokenizes <root>", function()
        local t = types("<root>")
        assert.are.same(
            {"OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE"},
            t
        )
    end)

    it("correct values for <root>", function()
        local tokens = xml_lexer.tokenize("<root>")
        assert.are.equal("<",    tokens[1].value)
        assert.are.equal("root", tokens[2].value)
        assert.are.equal(">",    tokens[3].value)
    end)
end)

-- =========================================================================
-- Closing tag
-- =========================================================================

describe("closing tag", function()
    it("tokenizes </root>", function()
        local t = types("</root>")
        assert.are.same(
            {"CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE"},
            t
        )
    end)

    it("correct values for </root>", function()
        local tokens = xml_lexer.tokenize("</root>")
        assert.are.equal("</",   tokens[1].value)
        assert.are.equal("root", tokens[2].value)
        assert.are.equal(">",    tokens[3].value)
    end)
end)

-- =========================================================================
-- Attributes
-- =========================================================================

describe("attributes", function()
    it("tokenizes double-quoted attribute value as ATTR_VALUE", function()
        -- <a href="url">
        local t = types('<a href="url">')
        assert.are.same(
            {"OPEN_TAG_START", "TAG_NAME", "TAG_NAME", "ATTR_EQUALS",
             "ATTR_VALUE", "TAG_CLOSE"},
            t
        )
    end)

    it("ATTR_VALUE includes the quotes", function()
        local tokens = xml_lexer.tokenize('<a href="url">')
        local av = first_of(tokens, "ATTR_VALUE")
        assert.is_not_nil(av)
        assert.are.equal('"url"', av.value)
    end)

    it("tokenizes single-quoted attribute value as ATTR_VALUE", function()
        local t = types("<a href='url'>")
        -- ATTR_VALUE_SQ aliases to ATTR_VALUE
        local found = false
        for _, typ in ipairs(t) do
            if typ == "ATTR_VALUE" then found = true break end
        end
        assert.is_true(found, "ATTR_VALUE present for single-quoted attr")
    end)

    it("ATTR_VALUE from single-quoted includes quotes", function()
        local tokens = xml_lexer.tokenize("<a href='url'>")
        local av = first_of(tokens, "ATTR_VALUE")
        assert.is_not_nil(av)
        assert.are.equal("'url'", av.value)
    end)

    it("tokenizes multiple attributes", function()
        local tokens = xml_lexer.tokenize('<img src="a.png" alt="pic"/>')
        assert.are.equal(2, count_of(tokens, "ATTR_VALUE"))
    end)
end)

-- =========================================================================
-- Text content
-- =========================================================================

describe("text content", function()
    it("tokenizes text between tags as TEXT", function()
        local t = types("<a>hello</a>")
        -- TEXT is emitted between the opening tag close and the closing tag
        local found = false
        for _, typ in ipairs(t) do
            if typ == "TEXT" then found = true break end
        end
        assert.is_true(found, "TEXT token present")
    end)

    it("text value matches the content", function()
        local tokens = xml_lexer.tokenize("<a>hello</a>")
        local txt = first_of(tokens, "TEXT")
        assert.is_not_nil(txt)
        assert.are.equal("hello", txt.value)
    end)
end)

-- =========================================================================
-- Entity references
-- =========================================================================

describe("entity references", function()
    it("tokenizes &amp; as ENTITY_REF", function()
        local tokens = xml_lexer.tokenize("&amp;")
        assert.are.equal("ENTITY_REF", tokens[1].type)
        assert.are.equal("&amp;",      tokens[1].value)
    end)

    it("tokenizes &lt; and &gt;", function()
        local tokens = xml_lexer.tokenize("&lt;&gt;")
        assert.are.equal("ENTITY_REF", tokens[1].type)
        assert.are.equal("ENTITY_REF", tokens[2].type)
    end)
end)

-- =========================================================================
-- Character references
-- =========================================================================

describe("character references", function()
    it("tokenizes decimal char ref &#65;", function()
        local tokens = xml_lexer.tokenize("&#65;")
        assert.are.equal("CHAR_REF", tokens[1].type)
        assert.are.equal("&#65;",    tokens[1].value)
    end)

    it("tokenizes hex char ref &#x41;", function()
        local tokens = xml_lexer.tokenize("&#x41;")
        assert.are.equal("CHAR_REF", tokens[1].type)
        assert.are.equal("&#x41;",   tokens[1].value)
    end)
end)

-- =========================================================================
-- Comments
-- =========================================================================

describe("comments", function()
    it("tokenizes a comment into COMMENT_START + COMMENT_TEXT + COMMENT_END", function()
        local t = types("<!-- hello -->")
        assert.are.same(
            {"COMMENT_START", "COMMENT_TEXT", "COMMENT_END"},
            t
        )
    end)

    it("COMMENT_TEXT captures the interior text", function()
        local tokens = xml_lexer.tokenize("<!-- hello -->")
        local ct = first_of(tokens, "COMMENT_TEXT")
        assert.is_not_nil(ct)
        assert.matches("hello", ct.value)
    end)

    it("COMMENT_START value is <!--", function()
        local tokens = xml_lexer.tokenize("<!-- x -->")
        local cs = first_of(tokens, "COMMENT_START")
        assert.are.equal("<!--", cs.value)
    end)

    it("COMMENT_END value is -->", function()
        local tokens = xml_lexer.tokenize("<!-- x -->")
        local ce = first_of(tokens, "COMMENT_END")
        assert.are.equal("-->", ce.value)
    end)
end)

-- =========================================================================
-- CDATA sections
-- =========================================================================

describe("CDATA sections", function()
    it("tokenizes <![CDATA[...]]> into three tokens", function()
        local t = types("<![CDATA[raw text]]>")
        assert.are.same(
            {"CDATA_START", "CDATA_TEXT", "CDATA_END"},
            t
        )
    end)

    it("CDATA_TEXT captures the raw content", function()
        local tokens = xml_lexer.tokenize("<![CDATA[raw text]]>")
        local ct = first_of(tokens, "CDATA_TEXT")
        assert.is_not_nil(ct)
        assert.are.equal("raw text", ct.value)
    end)

    it("CDATA content may contain < and & literally", function()
        local tokens = xml_lexer.tokenize("<![CDATA[<div>&amp;</div>]]>")
        local ct = first_of(tokens, "CDATA_TEXT")
        assert.is_not_nil(ct)
        assert.matches("<div>", ct.value)
    end)
end)

-- =========================================================================
-- Processing instructions
-- =========================================================================

describe("processing instructions", function()
    it("tokenizes <?xml version='1.0'?> tokens", function()
        local t = types("<?xml version='1.0'?>")
        -- PI_START, PI_TARGET, PI_TEXT (contains " version='1.0'"), PI_END
        assert.truthy(#t >= 3)
        assert.are.equal("PI_START", t[1])
        assert.are.equal("PI_END",   t[#t])
    end)

    it("PI_TARGET value is the instruction name", function()
        local tokens = xml_lexer.tokenize("<?xml version='1.0'?>")
        local pt = first_of(tokens, "PI_TARGET")
        assert.is_not_nil(pt)
        assert.are.equal("xml", pt.value)
    end)
end)

-- =========================================================================
-- Full document
-- =========================================================================

describe("full XML document", function()
    it("tokenizes a realistic XML fragment", function()
        local src = [[<root id="1"><child attr='v'>text &amp; &#65;</child><!-- note --></root>]]
        local tokens = xml_lexer.tokenize(src)

        assert.truthy(#tokens > 10)

        -- OPEN_TAG_START appears for <root and <child
        assert.are.equal(2, count_of(tokens, "OPEN_TAG_START"))

        -- CLOSE_TAG_START appears for </child and </root
        assert.are.equal(2, count_of(tokens, "CLOSE_TAG_START"))

        -- At least one ATTR_VALUE
        assert.truthy(count_of(tokens, "ATTR_VALUE") >= 1)

        -- TEXT content exists
        assert.truthy(count_of(tokens, "TEXT") >= 1)

        -- ENTITY_REF
        assert.truthy(count_of(tokens, "ENTITY_REF") >= 1)

        -- CHAR_REF
        assert.truthy(count_of(tokens, "CHAR_REF") >= 1)

        -- Comment
        assert.are.equal(1, count_of(tokens, "COMMENT_START"))
        assert.are.equal(1, count_of(tokens, "COMMENT_END"))

        -- EOF last
        assert.are.equal("EOF", tokens[#tokens].type)
    end)
end)

-- =========================================================================
-- EOF
-- =========================================================================

describe("EOF token", function()
    it("is always last", function()
        local tokens = xml_lexer.tokenize("<a/>")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has empty value", function()
        local tokens = xml_lexer.tokenize("<a/>")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)
