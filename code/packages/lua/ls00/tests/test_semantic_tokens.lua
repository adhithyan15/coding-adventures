-- test_semantic_tokens.lua — Semantic token encoding tests
-- ========================================================
--
-- Semantic tokens use a compact binary encoding. Instead of sending
-- {"type":"keyword"} per token, LSP sends a flat integer array where each
-- token is represented by 5 integers:
--
--   [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask]
--
-- "Delta" means the difference from the PREVIOUS token's position.
-- When deltaLine > 0, deltaStartChar is absolute for the new line.
-- When deltaLine == 0, deltaStartChar is relative to the previous token.
--
-- These tests verify the encoder handles:
--   1. Empty input
--   2. Single token
--   3. Multiple tokens on the same line
--   4. Multiple tokens on different lines
--   5. Unsorted input (encoder must sort)
--   6. Unknown token types (must be skipped)
--   7. Modifier bitmasks

local ls00 = require("coding_adventures.ls00")

describe("encode_semantic_tokens", function()
    it("returns empty array for nil input", function()
        local data = ls00.encode_semantic_tokens(nil)
        assert.are.equal(0, #data)
    end)

    it("returns empty array for empty input", function()
        local data = ls00.encode_semantic_tokens({})
        assert.are.equal(0, #data)
    end)

    it("encodes a single token", function()
        local tokens = {
            ls00.SemanticToken(0, 0, 5, "keyword", {}),
        }
        local data = ls00.encode_semantic_tokens(tokens)

        -- Expected: [deltaLine=0, deltaChar=0, length=5, typeIndex=15 (keyword), modifiers=0]
        assert.are.equal(5, #data)
        assert.are.equal(0, data[1])   -- deltaLine
        assert.are.equal(0, data[2])   -- deltaChar
        assert.are.equal(5, data[3])   -- length
        assert.are.equal(15, data[4])  -- keyword is at index 15
        assert.are.equal(0, data[5])   -- no modifiers
    end)

    it("encodes multiple tokens on the same line", function()
        local tokens = {
            ls00.SemanticToken(0, 0, 3, "keyword", {}),
            ls00.SemanticToken(0, 4, 4, "function", { "declaration" }),
        }
        local data = ls00.encode_semantic_tokens(tokens)

        assert.are.equal(10, #data)

        -- Token A: deltaLine=0, deltaChar=0, length=3, keyword(15), mods=0
        assert.are.equal(0, data[1])
        assert.are.equal(0, data[2])
        assert.are.equal(3, data[3])
        assert.are.equal(15, data[4])
        assert.are.equal(0, data[5])

        -- Token B: deltaLine=0, deltaChar=4, length=4, function(12), mods=1 (declaration=bit0)
        assert.are.equal(0, data[6])
        assert.are.equal(4, data[7])
        assert.are.equal(4, data[8])
        assert.are.equal(12, data[9])
        assert.are.equal(1, data[10])
    end)

    it("encodes multiple tokens on different lines", function()
        local tokens = {
            ls00.SemanticToken(0, 0, 3, "keyword", {}),
            ls00.SemanticToken(2, 4, 5, "number", {}),
        }
        local data = ls00.encode_semantic_tokens(tokens)

        assert.are.equal(10, #data)

        -- Token B: deltaLine=2, deltaChar=4 (absolute on new line), number=19
        assert.are.equal(2, data[6])
        assert.are.equal(4, data[7])
        assert.are.equal(19, data[9])
    end)

    it("sorts unsorted input", function()
        -- Tokens in reverse order — the encoder should sort them.
        local tokens = {
            ls00.SemanticToken(1, 0, 2, "number", {}),
            ls00.SemanticToken(0, 0, 3, "keyword", {}),
        }
        local data = ls00.encode_semantic_tokens(tokens)

        assert.are.equal(10, #data)
        -- After sorting: keyword on line 0 first, number on line 1 second.
        assert.are.equal(15, data[4])  -- first token = keyword (15)
        assert.are.equal(19, data[9])  -- second token = number (19)
    end)

    it("skips unknown token types", function()
        local tokens = {
            ls00.SemanticToken(0, 0, 3, "unknownType", {}),
            ls00.SemanticToken(0, 4, 2, "keyword", {}),
        }
        local data = ls00.encode_semantic_tokens(tokens)

        -- unknownType should be skipped, leaving only one 5-tuple.
        assert.are.equal(5, #data)
    end)

    it("encodes modifier bitmask correctly", function()
        -- "readonly" is at index 2 in the modifier list (0-based).
        -- Bit 2 = value 4.
        local tokens = {
            ls00.SemanticToken(0, 0, 3, "variable", { "readonly" }),
        }
        local data = ls00.encode_semantic_tokens(tokens)

        assert.are.equal(4, data[5])  -- readonly = bit 2 = value 4
    end)

    it("combines multiple modifiers with bitwise OR", function()
        -- "declaration" = bit 0 (value 1), "readonly" = bit 2 (value 4).
        -- Combined: 1 | 4 = 5.
        local tokens = {
            ls00.SemanticToken(0, 0, 3, "variable", { "declaration", "readonly" }),
        }
        local data = ls00.encode_semantic_tokens(tokens)

        assert.are.equal(5, data[5])
    end)
end)

describe("semantic_token_legend", function()
    it("returns non-empty token types", function()
        local legend = ls00.semantic_token_legend()
        assert.is_true(#legend.token_types > 0)
    end)

    it("returns non-empty token modifiers", function()
        local legend = ls00.semantic_token_legend()
        assert.is_true(#legend.token_modifiers > 0)
    end)

    it("contains required token types", function()
        local legend = ls00.semantic_token_legend()
        local required = { "keyword", "string", "number", "variable", "function" }

        for _, req in ipairs(required) do
            local found = false
            for _, t in ipairs(legend.token_types) do
                if t == req then
                    found = true
                    break
                end
            end
            assert.is_true(found, "legend missing required type: " .. req)
        end
    end)
end)
