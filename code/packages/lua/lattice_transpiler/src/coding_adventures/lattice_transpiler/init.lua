-- coding_adventures.lattice_transpiler — End-to-end Lattice → CSS pipeline
-- =========================================================================
--
-- This module wires together two packages into a single `transpile` function:
--
--   1. `coding_adventures.lattice_parser`   — Lattice source text → AST
--   2. `coding_adventures.lattice_ast_to_css` — Lattice AST → CSS text
--
-- ## Pipeline Diagram
--
--   Lattice Source
--        │
--        ▼
--   ┌────────────────┐
--   │ lattice_parser │  ← tokenize + grammar-driven parse
--   └───────┬────────┘
--           │ AST (ASTNode tree)
--           ▼
--   ┌──────────────────────┐
--   │ lattice_ast_to_css   │  ← variable expansion, mixin expansion,
--   └───────┬──────────────┘    control flow, nesting flattening
--           │
--           ▼
--       CSS text
--
-- ## Usage
--
--   local transpiler = require("coding_adventures.lattice_transpiler")
--
--   -- Transpile a source string
--   local css, err = transpiler.transpile("$c: red; h1 { color: $c; }")
--   -- css == "h1 {\n  color: red;\n}\n"
--
--   -- Transpile a file
--   local css, err = transpiler.transpile_file("/path/to/style.lattice")
--
-- ## Error Handling
--
-- Both functions return `(css, nil)` on success or `(nil, error_message)` on
-- failure.  Errors can arise from:
--   - Lexer errors (unknown characters in the source)
--   - Parser errors (syntax errors)
--   - Compiler errors (undefined variable reference, etc.)

local lattice_parser     = require("coding_adventures.lattice_parser")
local lattice_ast_to_css = require("coding_adventures.lattice_ast_to_css")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Public API
-- =========================================================================

--- Transpile a Lattice source string to CSS text.
--
-- This is the main entry point.  Pass in a Lattice source string and
-- receive compiled CSS text.
--
-- Internally:
--   1. `lattice_parser.parse(source)` tokenizes and parses the source into
--      an AST, raising an error on lexer/parser failure.
--   2. `lattice_ast_to_css.compile(ast)` walks the AST and emits CSS.
--
-- @param source string  The Lattice source text to transpile.
-- @return string|nil    The compiled CSS text on success, nil on failure.
-- @return nil|string    nil on success, error message string on failure.
--
-- Example:
--
--   local css, err = transpiler.transpile([[
--     $primary: #4a90d9;
--
--     @mixin button($bg, $fg: white) {
--       background: $bg;
--       color: $fg;
--       padding: 8px 16px;
--     }
--
--     .btn {
--       @include button($primary);
--     }
--   ]])
--   -- css:
--   -- .btn {
--   --   background: #4a90d9;
--   --   color: white;
--   --   padding: 8px 16px;
--   -- }
function M.transpile(source)
    -- Step 1: Parse
    -- lattice_parser.parse() raises a Lua error on failure.
    -- We catch it and return (nil, error_message).
    local ok, ast_or_err = pcall(lattice_parser.parse, source)
    if not ok then
        return nil, tostring(ast_or_err)
    end
    local ast = ast_or_err

    -- Step 2: Compile AST → CSS
    local ok2, css_or_err = pcall(lattice_ast_to_css.compile, ast)
    if not ok2 then
        return nil, tostring(css_or_err)
    end

    return css_or_err, nil
end

--- Transpile a Lattice file to CSS text.
--
-- Reads the file at `path`, then calls `transpile(source)`.
--
-- @param path string  Absolute or relative path to the .lattice file.
-- @return string|nil  The compiled CSS text on success, nil on failure.
-- @return nil|string  nil on success, error message string on failure.
--
-- Example:
--
--   local css, err = transpiler.transpile_file("styles/theme.lattice")
--   if err then
--     io.stderr:write("transpile error: " .. err .. "\n")
--   else
--     io.write(css)
--   end
function M.transpile_file(path)
    -- Open and read the file
    local f, open_err = io.open(path, "r")
    if not f then
        return nil, "lattice_transpiler: cannot open file: " .. path ..
                    " (" .. (open_err or "unknown error") .. ")"
    end
    local source = f:read("*all")
    f:close()

    -- Delegate to transpile()
    return M.transpile(source)
end

return M
