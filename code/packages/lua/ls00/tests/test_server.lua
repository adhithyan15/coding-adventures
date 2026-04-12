-- test_server.lua — LspServer integration tests
-- ===============================================
--
-- These tests verify the LspServer's integration with the JSON-RPC layer
-- and the bridge. They test:
--
--   1. Server creation with a bridge
--   2. Capabilities advertisement matches the bridge
--   3. The full lifecycle: initialize -> didOpen -> hover -> shutdown
--   4. Diagnostics are published on didOpen (via parse cache)
--   5. Handler routing for all feature requests
--
-- We use mock streams (tables with read/write methods) to simulate
-- stdin/stdout without needing real I/O.

local ls00 = require("coding_adventures.ls00")
local json_rpc = require("coding_adventures.json_rpc")

-- ─── Mock Bridge ────────────────────────────────────────────────────────────

-- A mock bridge that implements all optional providers for thorough testing.
local function make_full_mock_bridge()
    return {
        tokenize = function(source)
            -- Split by whitespace into WORD tokens.
            local tokens = {}
            local col = 1
            for word in source:gmatch("%S+") do
                tokens[#tokens + 1] = ls00.Token("WORD", word, 1, col)
                col = col + #word + 1
            end
            return tokens, nil
        end,

        parse = function(source)
            local diags = {}
            if source:find("ERROR") then
                diags[1] = ls00.Diagnostic(
                    ls00.Range(ls00.Position(0, 0), ls00.Position(0, 5)),
                    ls00.SEVERITY_ERROR,
                    "syntax error: unexpected ERROR token"
                )
            end
            return source, diags, nil
        end,

        hover = function(ast, pos)
            return ls00.HoverResult("**test** hover content")
        end,

        definition = function(ast, pos, uri)
            return ls00.Location(uri, ls00.Range(pos, pos))
        end,

        references = function(ast, pos, uri, include_decl)
            return { ls00.Location(uri, ls00.Range(pos, pos)) }
        end,

        completion = function(ast, pos)
            return { ls00.CompletionItem("foo", ls00.COMPLETION_FUNCTION, "() void") }
        end,

        rename = function(ast, pos, new_name)
            return ls00.WorkspaceEdit({
                ["file:///test.txt"] = {
                    ls00.TextEdit(ls00.Range(pos, pos), new_name),
                },
            })
        end,

        semantic_tokens = function(source, tokens)
            local result = {}
            for _, tok in ipairs(tokens) do
                result[#result + 1] = ls00.SemanticToken(
                    tok.line - 1, tok.column - 1, #tok.value, "variable", {}
                )
            end
            return result, nil
        end,

        document_symbols = function(ast)
            return {
                ls00.DocumentSymbol(
                    "main",
                    ls00.SYMBOL_FUNCTION,
                    ls00.Range(ls00.Position(0, 0), ls00.Position(10, 1)),
                    ls00.Range(ls00.Position(0, 9), ls00.Position(0, 13)),
                    {
                        ls00.DocumentSymbol(
                            "x",
                            ls00.SYMBOL_VARIABLE,
                            ls00.Range(ls00.Position(1, 4), ls00.Position(1, 12)),
                            ls00.Range(ls00.Position(1, 8), ls00.Position(1, 9))
                        ),
                    }
                ),
            }
        end,

        folding_ranges = function(ast)
            return { ls00.FoldingRange(0, 5, "region") }
        end,

        signature_help = function(ast, pos)
            return ls00.SignatureHelpResult(
                {
                    ls00.SignatureInformation(
                        "foo(a int, b string)",
                        nil,
                        {
                            ls00.ParameterInformation("a int"),
                            ls00.ParameterInformation("b string"),
                        }
                    ),
                },
                0, 0
            )
        end,

        format = function(source)
            return {
                ls00.TextEdit(
                    ls00.Range(ls00.Position(0, 0), ls00.Position(999, 0)),
                    source  -- no-op formatter
                ),
            }
        end,
    }
end

-- A minimal bridge with only required functions.
local function make_minimal_bridge()
    return {
        tokenize = function(source) return {}, nil end,
        parse = function(source) return source, {}, nil end,
    }
end

-- ─── Mock Stream ────────────────────────────────────────────────────────────

-- Create an in-memory readable stream from a string.
local function make_read_stream(content)
    local pos = 1
    return {
        read = function(self, what)
            if pos > #content then return nil end

            if what == "l" then
                -- Read one line.
                local nl = content:find("\n", pos)
                if nl then
                    local line = content:sub(pos, nl - 1)
                    pos = nl + 1
                    return line
                else
                    local line = content:sub(pos)
                    pos = #content + 1
                    return line
                end
            elseif type(what) == "number" then
                -- Read N bytes.
                local data = content:sub(pos, pos + what - 1)
                pos = pos + what
                if #data == 0 then return nil end
                return data
            end
            return nil
        end,
    }
end

-- Create an in-memory writable stream.
local function make_write_stream()
    local buf = {}
    return {
        write = function(self, data)
            buf[#buf + 1] = data
        end,
        flush = function(self) end,
        get_content = function()
            return table.concat(buf)
        end,
    }
end

-- Build a Content-Length-framed message string.
local function make_message(obj)
    local payload = json_rpc.json_encode(obj)
    return string.format("Content-Length: %d\r\n\r\n%s", #payload, payload)
end

-- ─── Tests ──────────────────────────────────────────────────────────────────

describe("LspServer", function()
    it("can be created with a bridge", function()
        local bridge = make_minimal_bridge()
        local in_stream = make_read_stream("")
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        assert.is_not_nil(server)
    end)

    it("responds to initialize with capabilities", function()
        local bridge = make_full_mock_bridge()

        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = {
                processId = 12345,
                capabilities = {},
            },
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)

        -- Serve will process the message then hit EOF and stop.
        server:serve()

        local content = out_stream.get_content()
        assert.is_truthy(content:find("capabilities"))
        assert.is_truthy(content:find("hoverProvider"))
        assert.is_truthy(content:find("documentSymbolProvider"))
    end)

    it("publishes diagnostics on didOpen with error source", function()
        local bridge = make_full_mock_bridge()

        -- Send initialize + initialized + didOpen with ERROR source.
        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = { processId = 1, capabilities = {} },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "initialized",
            params = {},
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didOpen",
            params = {
                textDocument = {
                    uri = "file:///test.txt",
                    languageId = "test",
                    version = 1,
                    text = "hello ERROR world",
                },
            },
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        server:serve()

        local content = out_stream.get_content()
        assert.is_truthy(content:find("publishDiagnostics"))
        assert.is_truthy(content:find("syntax error"))
    end)

    it("handles hover requests", function()
        local bridge = make_full_mock_bridge()

        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = { processId = 1, capabilities = {} },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didOpen",
            params = {
                textDocument = {
                    uri = "file:///test.txt",
                    languageId = "test",
                    version = 1,
                    text = "hello world",
                },
            },
        }) .. make_message({
            jsonrpc = "2.0",
            id = 2,
            method = "textDocument/hover",
            params = {
                textDocument = { uri = "file:///test.txt" },
                position = { line = 0, character = 0 },
            },
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        server:serve()

        local content = out_stream.get_content()
        assert.is_truthy(content:find("hover content"))
        assert.is_truthy(content:find("markdown"))
    end)

    it("handles shutdown request", function()
        local bridge = make_minimal_bridge()

        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = { processId = 1, capabilities = {} },
        }) .. make_message({
            jsonrpc = "2.0",
            id = 2,
            method = "shutdown",
            params = {},
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        server:serve()

        local content = out_stream.get_content()
        -- Shutdown returns null result.
        assert.is_truthy(content:find('"result"'))
    end)

    it("handles didChange with incremental update", function()
        local bridge = make_full_mock_bridge()

        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = { processId = 1, capabilities = {} },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didOpen",
            params = {
                textDocument = {
                    uri = "file:///test.txt",
                    languageId = "test",
                    version = 1,
                    text = "hello world",
                },
            },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didChange",
            params = {
                textDocument = { uri = "file:///test.txt", version = 2 },
                contentChanges = {
                    {
                        range = {
                            start = { line = 0, character = 6 },
                            ["end"] = { line = 0, character = 11 },
                        },
                        text = "ERROR",
                    },
                },
            },
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        server:serve()

        local content = out_stream.get_content()
        -- After changing "world" to "ERROR", diagnostics should fire.
        assert.is_truthy(content:find("publishDiagnostics"))
    end)

    it("handles didClose and clears diagnostics", function()
        local bridge = make_full_mock_bridge()

        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = { processId = 1, capabilities = {} },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didOpen",
            params = {
                textDocument = {
                    uri = "file:///test.txt",
                    languageId = "test",
                    version = 1,
                    text = "hello ERROR",
                },
            },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didClose",
            params = {
                textDocument = { uri = "file:///test.txt" },
            },
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        server:serve()

        -- Should have received publishDiagnostics at least twice:
        -- once for didOpen (with errors), once for didClose (empty).
        local content = out_stream.get_content()
        -- Count occurrences of publishDiagnostics.
        local count = 0
        for _ in content:gmatch("publishDiagnostics") do
            count = count + 1
        end
        assert.is_true(count >= 2)
    end)

    it("handles semantic tokens request", function()
        local bridge = make_full_mock_bridge()

        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = { processId = 1, capabilities = {} },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didOpen",
            params = {
                textDocument = {
                    uri = "file:///test.txt",
                    languageId = "test",
                    version = 1,
                    text = "hello world",
                },
            },
        }) .. make_message({
            jsonrpc = "2.0",
            id = 3,
            method = "textDocument/semanticTokens/full",
            params = {
                textDocument = { uri = "file:///test.txt" },
            },
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        server:serve()

        local content = out_stream.get_content()
        assert.is_truthy(content:find('"data"'))
    end)

    it("handles completion request", function()
        local bridge = make_full_mock_bridge()

        local input = make_message({
            jsonrpc = "2.0",
            id = 1,
            method = "initialize",
            params = { processId = 1, capabilities = {} },
        }) .. make_message({
            jsonrpc = "2.0",
            method = "textDocument/didOpen",
            params = {
                textDocument = {
                    uri = "file:///test.txt",
                    languageId = "test",
                    version = 1,
                    text = "hello",
                },
            },
        }) .. make_message({
            jsonrpc = "2.0",
            id = 2,
            method = "textDocument/completion",
            params = {
                textDocument = { uri = "file:///test.txt" },
                position = { line = 0, character = 0 },
            },
        })

        local in_stream = make_read_stream(input)
        local out_stream = make_write_stream()
        local server = ls00.LspServer:new(bridge, in_stream, out_stream)
        server:serve()

        local content = out_stream.get_content()
        assert.is_truthy(content:find("foo"))
    end)
end)
