-- test_capabilities.lua — Capabilities advertisement tests
-- ========================================================
--
-- The server advertises capabilities based on which functions the bridge
-- provides. A minimal bridge (only tokenize + parse) should only advertise
-- textDocumentSync. A full bridge should advertise all optional capabilities.
--
-- This tests the build_capabilities() function which inspects the bridge
-- table at runtime.

local ls00 = require("coding_adventures.ls00")

-- A minimal bridge with only the required functions.
local function make_minimal_bridge()
    return {
        tokenize = function(source) return {}, nil end,
        parse = function(source) return source, {}, nil end,
    }
end

-- A bridge with hover and document_symbols (like the Go MockBridge).
local function make_mock_bridge()
    return {
        tokenize = function(source) return {}, nil end,
        parse = function(source) return source, {}, nil end,
        hover = function(ast, pos) return nil end,
        document_symbols = function(ast) return {} end,
    }
end

-- A full bridge with ALL optional capabilities.
local function make_full_bridge()
    return {
        tokenize = function(source) return {}, nil end,
        parse = function(source) return source, {}, nil end,
        hover = function(ast, pos) return nil end,
        definition = function(ast, pos, uri) return nil end,
        references = function(ast, pos, uri, incl) return {} end,
        completion = function(ast, pos) return {} end,
        rename = function(ast, pos, name) return nil end,
        semantic_tokens = function(source, tokens) return {} end,
        document_symbols = function(ast) return {} end,
        folding_ranges = function(ast) return {} end,
        signature_help = function(ast, pos) return nil end,
        format = function(source) return {} end,
    }
end

describe("build_capabilities", function()
    it("always includes textDocumentSync", function()
        local bridge = make_minimal_bridge()
        local caps = ls00.build_capabilities(bridge)
        assert.are.equal(2, caps.textDocumentSync)
    end)

    it("omits optional capabilities for minimal bridge", function()
        local bridge = make_minimal_bridge()
        local caps = ls00.build_capabilities(bridge)

        local optional = {
            "hoverProvider", "definitionProvider", "referencesProvider",
            "completionProvider", "renameProvider", "documentSymbolProvider",
            "foldingRangeProvider", "signatureHelpProvider",
            "documentFormattingProvider", "semanticTokensProvider",
        }
        for _, cap in ipairs(optional) do
            assert.is_nil(caps[cap], "minimal bridge should not advertise " .. cap)
        end
    end)

    it("advertises hoverProvider when bridge has hover", function()
        local bridge = make_mock_bridge()
        local caps = ls00.build_capabilities(bridge)
        assert.is_true(caps.hoverProvider)
    end)

    it("advertises documentSymbolProvider when bridge has document_symbols", function()
        local bridge = make_mock_bridge()
        local caps = ls00.build_capabilities(bridge)
        assert.is_true(caps.documentSymbolProvider)
    end)

    it("advertises all capabilities for full bridge", function()
        local bridge = make_full_bridge()
        local caps = ls00.build_capabilities(bridge)

        assert.are.equal(2, caps.textDocumentSync)
        assert.is_true(caps.hoverProvider)
        assert.is_true(caps.definitionProvider)
        assert.is_true(caps.referencesProvider)
        assert.is_not_nil(caps.completionProvider)
        assert.is_true(caps.renameProvider)
        assert.is_true(caps.documentSymbolProvider)
        assert.is_true(caps.foldingRangeProvider)
        assert.is_not_nil(caps.signatureHelpProvider)
        assert.is_true(caps.documentFormattingProvider)
        assert.is_not_nil(caps.semanticTokensProvider)
    end)

    it("includes semanticTokensProvider with legend and full=true", function()
        local bridge = make_full_bridge()
        local caps = ls00.build_capabilities(bridge)

        local stp = caps.semanticTokensProvider
        assert.is_not_nil(stp)
        assert.is_true(stp.full)
        assert.is_not_nil(stp.legend)
        assert.is_not_nil(stp.legend.token_types)
        assert.is_not_nil(stp.legend.token_modifiers)
    end)

    it("includes completionProvider with trigger characters", function()
        local bridge = make_full_bridge()
        local caps = ls00.build_capabilities(bridge)

        local cp = caps.completionProvider
        assert.is_not_nil(cp)
        assert.is_not_nil(cp.triggerCharacters)
        assert.are.equal(2, #cp.triggerCharacters)
    end)

    it("includes signatureHelpProvider with trigger characters", function()
        local bridge = make_full_bridge()
        local caps = ls00.build_capabilities(bridge)

        local shp = caps.signatureHelpProvider
        assert.is_not_nil(shp)
        assert.is_not_nil(shp.triggerCharacters)
        assert.are.equal(2, #shp.triggerCharacters)
    end)
end)
