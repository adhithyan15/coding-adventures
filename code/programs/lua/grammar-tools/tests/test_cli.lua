-- Tests for the grammar-tools Lua CLI program.
--
-- We load main.lua as a module (not executed as a script) and call the
-- exported public functions directly, just as all other language
-- implementations do.
--
-- Grammar fixture files are located by walking up from the test file to
-- find the monorepo root (the directory that contains code/grammars/).

-- ---------------------------------------------------------------------------
-- Module path setup — make grammar_tools loadable
-- ---------------------------------------------------------------------------

-- The test runner (busted) is invoked from the tests/ directory.
-- We need both:
--   1. main.lua's own directory (parent of tests/) to `require("main")`
--   2. The grammar_tools library source tree

local sep = package.config:sub(1, 1) -- "/" on Unix, "\" on Windows

-- Walk up from the test file's directory to find the monorepo root.
local function find_root()
    local current = "."
    for _ = 1, 20 do
        local f = io.open(current .. sep .. "code" .. sep .. "specs"
                          .. sep .. "grammar-tools.json", "r")
        if f then
            f:close()
            return current
        end
        current = ".." .. sep .. current
    end
    return "."
end

local ROOT = find_root()
local GRAMMARS = ROOT .. sep .. "code" .. sep .. "grammars"

-- Add parent directory (where main.lua lives) to the path.
package.path = ".." .. sep .. "?.lua;"
             .. ".." .. sep .. "?" .. sep .. "init.lua;"
             .. ROOT .. sep .. "code" .. sep .. "packages" .. sep .. "lua"
                     .. sep .. "grammar_tools" .. sep .. "src" .. sep .. "?.lua;"
             .. ROOT .. sep .. "code" .. sep .. "packages" .. sep .. "lua"
                     .. sep .. "grammar_tools" .. sep .. "src" .. sep .. "?" .. sep .. "init.lua;"
             .. package.path

local cli = require("main")

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

local function grammar_path(name)
    return GRAMMARS .. sep .. name
end

local function exists(name)
    local f = io.open(grammar_path(name), "r")
    if f then f:close(); return true end
    return false
end

-- ---------------------------------------------------------------------------
-- validate_command
-- ---------------------------------------------------------------------------

describe("validate_command", function()
    it("succeeds on json pair", function()
        if not exists("json.tokens") or not exists("json.grammar") then return end
        assert.equals(0, cli.validate_command(grammar_path("json.tokens"), grammar_path("json.grammar")))
    end)

    it("succeeds on lisp pair", function()
        if not exists("lisp.tokens") or not exists("lisp.grammar") then return end
        assert.equals(0, cli.validate_command(grammar_path("lisp.tokens"), grammar_path("lisp.grammar")))
    end)

    it("returns 1 on missing tokens file", function()
        assert.equals(1, cli.validate_command("/nonexistent/x.tokens", "any.grammar"))
    end)

    it("returns 1 on missing grammar file", function()
        if not exists("json.tokens") then return end
        assert.equals(1, cli.validate_command(grammar_path("json.tokens"), "/nonexistent/x.grammar"))
    end)
end)

-- ---------------------------------------------------------------------------
-- validate_tokens_only
-- ---------------------------------------------------------------------------

describe("validate_tokens_only", function()
    it("succeeds on json.tokens", function()
        if not exists("json.tokens") then return end
        assert.equals(0, cli.validate_tokens_only(grammar_path("json.tokens")))
    end)

    it("returns 1 on missing file", function()
        assert.equals(1, cli.validate_tokens_only("/nonexistent/x.tokens"))
    end)
end)

-- ---------------------------------------------------------------------------
-- validate_grammar_only
-- ---------------------------------------------------------------------------

describe("validate_grammar_only", function()
    it("succeeds on json.grammar", function()
        if not exists("json.grammar") then return end
        assert.equals(0, cli.validate_grammar_only(grammar_path("json.grammar")))
    end)

    it("returns 1 on missing file", function()
        assert.equals(1, cli.validate_grammar_only("/nonexistent/x.grammar"))
    end)
end)

-- ---------------------------------------------------------------------------
-- dispatch
-- ---------------------------------------------------------------------------

describe("dispatch", function()
    it("returns 2 for unknown command", function()
        assert.equals(2, cli.dispatch("unknown", {}))
    end)

    it("returns 2 for validate with wrong file count", function()
        assert.equals(2, cli.dispatch("validate", { "one.tokens" }))
    end)

    it("returns 2 for validate-tokens with no files", function()
        assert.equals(2, cli.dispatch("validate-tokens", {}))
    end)

    it("returns 2 for validate-grammar with no files", function()
        assert.equals(2, cli.dispatch("validate-grammar", {}))
    end)

    it("dispatches validate correctly", function()
        if not exists("json.tokens") or not exists("json.grammar") then return end
        assert.equals(0, cli.dispatch("validate", {
            grammar_path("json.tokens"),
            grammar_path("json.grammar"),
        }))
    end)

    it("dispatches validate-tokens correctly", function()
        if not exists("json.tokens") then return end
        assert.equals(0, cli.dispatch("validate-tokens", { grammar_path("json.tokens") }))
    end)

    it("dispatches validate-grammar correctly", function()
        if not exists("json.grammar") then return end
        assert.equals(0, cli.dispatch("validate-grammar", { grammar_path("json.grammar") }))
    end)
end)
