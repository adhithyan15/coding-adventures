-- coding_adventures.ls00 — Generic LSP Server Framework
-- ======================================================
--
-- The Language Server Protocol (LSP) is a standardized protocol between code
-- editors (VS Code, Neovim, Emacs) and language servers. When you see red
-- squiggles under syntax errors, autocomplete suggestions, or "Go to
-- Definition" — none of that is built into the editor. It comes from a
-- *language server*: a separate process that speaks LSP.
--
-- LSP solves the M x N problem:
--
--   M editors x N languages = M*N integrations to write
--
-- With LSP, each language writes ONE server, and every LSP-aware editor gets
-- all features automatically. This module is the *generic* half — it handles
-- all protocol boilerplate. A language author only writes the "bridge": a Lua
-- table with function fields that connects their lexer/parser to this framework.
--
-- # Architecture
--
--   Lexer -> Parser -> [Bridge Table] -> [LspServer] -> VS Code / Neovim / Emacs
--
-- # Bridge Design
--
-- In Go, the bridge uses interface type assertions for capability detection.
-- In Lua, the bridge is a plain table with function fields. Capability detection
-- is simply:  if bridge.hover ~= nil then ... end
--
-- Required bridge fields:
--   bridge.tokenize(source) -> tokens, err
--   bridge.parse(source)    -> ast, diagnostics, err
--
-- Optional bridge fields (each enables one LSP feature):
--   bridge.hover(ast, pos)                     -> hover_result or nil
--   bridge.definition(ast, pos, uri)           -> location or nil
--   bridge.references(ast, pos, uri, incl_decl) -> locations
--   bridge.completion(ast, pos)                -> items
--   bridge.rename(ast, pos, new_name)          -> workspace_edit
--   bridge.semantic_tokens(source, tokens)     -> semantic_tokens
--   bridge.document_symbols(ast)               -> symbols
--   bridge.folding_ranges(ast)                 -> ranges
--   bridge.signature_help(ast, pos)            -> result or nil
--   bridge.format(source)                      -> text_edits
--
-- # JSON-RPC
--
-- LSP speaks JSON-RPC 2.0 over stdin/stdout. This module uses the
-- coding-adventures json_rpc package for the transport layer.

local json_rpc = require("coding_adventures.json_rpc")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- LSP Error Codes
-- =========================================================================
--
-- The JSON-RPC 2.0 spec reserves error codes in the range [-32768, -32000].
-- LSP further reserves [-32899, -32800] for protocol-level errors.

M.errors = {
    -- Server has received a request before the initialize handshake.
    SERVER_NOT_INITIALIZED = -32002,
    -- Generic unknown error.
    UNKNOWN_ERROR_CODE     = -32001,
    -- A request failed but not due to a protocol problem.
    REQUEST_FAILED         = -32803,
    -- The server cancelled the request.
    SERVER_CANCELLED       = -32802,
    -- Document content was modified before request completed.
    CONTENT_MODIFIED       = -32801,
    -- The client cancelled the request.
    REQUEST_CANCELLED      = -32800,
}

-- =========================================================================
-- LSP Types — Constructor Functions
-- =========================================================================
--
-- LSP uses a coordinate system based on 0-based lines and UTF-16 code units.
-- These constructor functions create plain Lua tables that mirror the LSP
-- specification's TypeScript type definitions.
--
-- # Coordinate System
--
-- Line 0, character 0 is the very first character of the file. This differs
-- from most editors (which display 1-based line numbers).
--
-- # UTF-16 Code Units
--
-- LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
-- Unicode codepoints. This is a historical artifact from VS Code using
-- TypeScript (which uses UTF-16 strings internally).

--- Create a Position (cursor location in a document).
-- Both line and character are 0-based. Character is in UTF-16 code units.
--
-- Example: in "hello 🎸 world", the guitar emoji (U+1F3B8) occupies UTF-16
-- characters 6 and 7 (it requires two surrogates). "world" starts at char 8.
--
-- @param line      number  0-based line number
-- @param character number  0-based character offset in UTF-16 code units
-- @return          table   {line=N, character=N}
function M.Position(line, character)
    return { line = line, character = character }
end

--- Create a Range (span of text from start inclusive to end exclusive).
-- Think of it like a text selection: start is where the cursor lands when
-- you click, end is where you drag to.
--
-- @param start_pos table  Position (start, inclusive)
-- @param end_pos   table  Position (end, exclusive)
-- @return          table  {start={...}, ["end"]={...}}
function M.Range(start_pos, end_pos)
    return { start = start_pos, ["end"] = end_pos }
end

--- Create a Location (position in a specific file).
-- URI uses the "file://" scheme, e.g., "file:///home/user/main.lua".
--
-- @param uri    string  Document URI
-- @param range  table   Range within the document
-- @return       table   {uri=S, range={...}}
function M.Location(uri, range)
    return { uri = uri, range = range }
end

-- Diagnostic severity constants (match LSP integer codes).
--
-- | Value | Name        | Meaning                                    |
-- |-------|-------------|------------------------------------------  |
-- | 1     | Error       | A hard error; the code cannot run           |
-- | 2     | Warning     | Potentially problematic, but not blocking   |
-- | 3     | Information | Informational message                      |
-- | 4     | Hint        | A suggestion (e.g., "consider using const") |
M.SEVERITY_ERROR       = 1
M.SEVERITY_WARNING     = 2
M.SEVERITY_INFORMATION = 3
M.SEVERITY_HINT        = 4

--- Create a Diagnostic (error, warning, or hint to display in the editor).
-- The editor renders diagnostics as underlined squiggles. Red = Error,
-- yellow = Warning, blue = Info.
--
-- @param range    table   Range where the diagnostic applies
-- @param severity number  One of M.SEVERITY_* constants
-- @param message  string  Human-readable message
-- @param code     string  Optional error code (e.g. "E001")
-- @return         table
function M.Diagnostic(range, severity, message, code)
    local d = { range = range, severity = severity, message = message }
    if code then d.code = code end
    return d
end

--- Create a Token (single lexical token from the language's lexer).
-- Line and Column are 1-based (matching most lexers). The bridge must
-- convert to 0-based when building SemanticToken values.
--
-- @param token_type string  e.g. "KEYWORD", "IDENTIFIER", "STRING_LIT"
-- @param value      string  The actual source text, e.g. "let" or "myVar"
-- @param line       number  1-based line number
-- @param column     number  1-based column number
-- @return           table
function M.Token(token_type, value, line, column)
    return { type = token_type, value = value, line = line, column = column }
end

--- Create a TextEdit (single text replacement in a document).
-- Used by formatting (replace the whole file) and rename (replace occurrences).
-- If new_text is empty, the range is deleted.
--
-- @param range    table   Range to replace
-- @param new_text string  Replacement text
-- @return         table
function M.TextEdit(range, new_text)
    return { range = range, newText = new_text }
end

--- Create a WorkspaceEdit (text edits grouped across multiple files).
-- For rename operations, changes maps URIs to arrays of TextEdits.
--
-- @param changes table  {[uri] = {TextEdit, ...}, ...}
-- @return        table
function M.WorkspaceEdit(changes)
    return { changes = changes }
end

--- Create a HoverResult (content to show in the hover popup).
-- Contents is Markdown text. Range is optional.
--
-- @param contents string  Markdown text
-- @param range    table   Optional: range to highlight while hover is shown
-- @return         table
function M.HoverResult(contents, range)
    local h = { contents = contents }
    if range then h.range = range end
    return h
end

-- CompletionItemKind constants — classify autocomplete suggestions so the
-- editor shows the right icon (function icon, variable icon, etc.).
M.COMPLETION_TEXT            = 1
M.COMPLETION_METHOD          = 2
M.COMPLETION_FUNCTION        = 3
M.COMPLETION_CONSTRUCTOR     = 4
M.COMPLETION_FIELD           = 5
M.COMPLETION_VARIABLE        = 6
M.COMPLETION_CLASS           = 7
M.COMPLETION_INTERFACE       = 8
M.COMPLETION_MODULE          = 9
M.COMPLETION_PROPERTY        = 10
M.COMPLETION_UNIT            = 11
M.COMPLETION_VALUE           = 12
M.COMPLETION_ENUM            = 13
M.COMPLETION_KEYWORD         = 14
M.COMPLETION_SNIPPET         = 15
M.COMPLETION_COLOR           = 16
M.COMPLETION_FILE            = 17
M.COMPLETION_REFERENCE       = 18
M.COMPLETION_FOLDER          = 19
M.COMPLETION_ENUM_MEMBER     = 20
M.COMPLETION_CONSTANT        = 21
M.COMPLETION_STRUCT          = 22
M.COMPLETION_EVENT           = 23
M.COMPLETION_OPERATOR        = 24
M.COMPLETION_TYPE_PARAMETER  = 25

--- Create a CompletionItem (single autocomplete suggestion).
--
-- @param label      string  Display text in the dropdown
-- @param kind       number  One of M.COMPLETION_* constants
-- @param detail     string  Optional type/detail string
-- @param doc        string  Optional documentation
-- @param insert     string  Optional text to insert (defaults to label)
-- @return           table
function M.CompletionItem(label, kind, detail, doc, insert)
    local item = { label = label }
    if kind then item.kind = kind end
    if detail then item.detail = detail end
    if doc then item.documentation = doc end
    if insert then item.insertText = insert end
    return item
end

--- Create a SemanticToken (one token's contribution to semantic highlighting).
-- Line and Character are 0-based. TokenType and Modifiers reference entries
-- in the legend returned by semantic_token_legend().
--
-- @param line       number    0-based line
-- @param character  number    0-based, UTF-16 code units
-- @param length     number    In UTF-16 code units
-- @param token_type string    Must match an entry in the legend's token_types
-- @param modifiers  table     Array of modifier strings (subset of legend)
-- @return           table
function M.SemanticToken(line, character, length, token_type, modifiers)
    return {
        line = line,
        character = character,
        length = length,
        token_type = token_type,
        modifiers = modifiers or {},
    }
end

-- SymbolKind constants — classify document symbols for the outline panel.
M.SYMBOL_FILE            = 1
M.SYMBOL_MODULE          = 2
M.SYMBOL_NAMESPACE       = 3
M.SYMBOL_PACKAGE         = 4
M.SYMBOL_CLASS           = 5
M.SYMBOL_METHOD          = 6
M.SYMBOL_PROPERTY        = 7
M.SYMBOL_FIELD           = 8
M.SYMBOL_CONSTRUCTOR     = 9
M.SYMBOL_ENUM            = 10
M.SYMBOL_INTERFACE       = 11
M.SYMBOL_FUNCTION        = 12
M.SYMBOL_VARIABLE        = 13
M.SYMBOL_CONSTANT        = 14
M.SYMBOL_STRING          = 15
M.SYMBOL_NUMBER          = 16
M.SYMBOL_BOOLEAN         = 17
M.SYMBOL_ARRAY           = 18
M.SYMBOL_OBJECT          = 19
M.SYMBOL_KEY             = 20
M.SYMBOL_NULL            = 21
M.SYMBOL_ENUM_MEMBER     = 22
M.SYMBOL_STRUCT          = 23
M.SYMBOL_EVENT           = 24
M.SYMBOL_OPERATOR        = 25
M.SYMBOL_TYPE_PARAMETER  = 26

--- Create a DocumentSymbol (one entry in the document outline panel).
-- Range covers the entire symbol. SelectionRange covers just the name.
-- Children allows nesting: a class can contain method symbols.
--
-- @param name            string  Symbol name
-- @param kind            number  One of M.SYMBOL_* constants
-- @param range           table   Range covering the entire symbol
-- @param selection_range table   Range of just the symbol's name
-- @param children        table   Optional array of child DocumentSymbols
-- @return                table
function M.DocumentSymbol(name, kind, range, selection_range, children)
    local s = {
        name = name,
        kind = kind,
        range = range,
        selectionRange = selection_range,
    }
    if children and #children > 0 then s.children = children end
    return s
end

--- Create a FoldingRange (collapsible region in the document).
-- The editor shows a collapse arrow in the gutter next to start_line.
--
-- @param start_line number  0-based start line
-- @param end_line   number  0-based end line
-- @param kind       string  Optional: "region", "imports", or "comment"
-- @return           table
function M.FoldingRange(start_line, end_line, kind)
    local r = { startLine = start_line, endLine = end_line }
    if kind then r.kind = kind end
    return r
end

--- Create a ParameterInformation (one parameter in a function signature).
--
-- @param label string  Parameter label text
-- @param doc   string  Optional documentation
-- @return      table
function M.ParameterInformation(label, doc)
    local p = { label = label }
    if doc then p.documentation = doc end
    return p
end

--- Create a SignatureInformation (one function overload's signature).
--
-- @param label      string  Full signature string
-- @param doc        string  Optional documentation
-- @param parameters table   Optional array of ParameterInformation
-- @return           table
function M.SignatureInformation(label, doc, parameters)
    local s = { label = label }
    if doc then s.documentation = doc end
    if parameters then s.parameters = parameters end
    return s
end

--- Create a SignatureHelpResult (tooltip for function call arguments).
--
-- @param signatures       table   Array of SignatureInformation
-- @param active_signature number  Index into signatures
-- @param active_parameter number  Index into the active signature's parameters
-- @return                 table
function M.SignatureHelpResult(signatures, active_signature, active_parameter)
    return {
        signatures = signatures,
        activeSignature = active_signature,
        activeParameter = active_parameter,
    }
end

-- =========================================================================
-- UTF-16 Conversion
-- =========================================================================
--
-- # Why UTF-16?
--
-- LSP character offsets are measured in UTF-16 code units because VS Code's
-- internal string representation is UTF-16 (as is JavaScript's String type).
-- Lua strings are byte strings (effectively UTF-8). We must convert between
-- the two.
--
-- # How UTF-8 encoding works
--
-- UTF-8 encodes Unicode codepoints into 1-4 bytes:
--
--   Codepoint range     | UTF-8 bytes | Leading byte pattern
--   --------------------|-------------|---------------------
--   U+0000 - U+007F    | 1 byte      | 0xxxxxxx
--   U+0080 - U+07FF    | 2 bytes     | 110xxxxx 10xxxxxx
--   U+0800 - U+FFFF    | 3 bytes     | 1110xxxx 10xxxxxx 10xxxxxx
--   U+10000 - U+10FFFF | 4 bytes     | 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
--
-- # How UTF-16 encoding works
--
-- UTF-16 encodes codepoints into 1 or 2 "code units" (each 16 bits):
--   - Basic Multilingual Plane (U+0000 - U+FFFF): 1 code unit
--   - Above U+FFFF (emoji, rare CJK): 2 code units (a "surrogate pair")
--
-- The guitar emoji 🎸 (U+1F3B8) is above U+FFFF:
--   UTF-8:  4 bytes  (0xF0 0x9F 0x8E 0xB8)
--   UTF-16: 2 code units (surrogate pair)
--
-- So we cannot simply equate "UTF-16 character N" with "byte N" in the Lua
-- string. We must walk the UTF-8 bytes, counting UTF-16 code units.

--- Determine the number of bytes in a UTF-8 sequence from its leading byte.
-- The leading byte's high bits tell us the sequence length:
--   0xxxxxxx -> 1 byte  (ASCII)
--   110xxxxx -> 2 bytes
--   1110xxxx -> 3 bytes
--   11110xxx -> 4 bytes
--   10xxxxxx -> continuation byte (should not be a leading byte)
--
-- @param byte_val number  The byte value (0-255)
-- @return         number  Sequence length (1-4), or 1 for invalid bytes
local function utf8_sequence_length(byte_val)
    if byte_val < 0x80 then return 1       -- ASCII: 0xxxxxxx
    elseif byte_val < 0xC0 then return 1   -- continuation byte (invalid as leader)
    elseif byte_val < 0xE0 then return 2   -- 110xxxxx
    elseif byte_val < 0xF0 then return 3   -- 1110xxxx
    else return 4                           -- 11110xxx
    end
end

--- Determine the number of UTF-16 code units for a UTF-8 sequence.
-- 4-byte UTF-8 sequences encode codepoints above U+FFFF, which need
-- 2 UTF-16 code units (a surrogate pair). All others need 1.
--
-- @param seq_len number  UTF-8 sequence length (1-4)
-- @return        number  UTF-16 code units (1 or 2)
local function utf16_units_for_sequence(seq_len)
    if seq_len == 4 then return 2 end
    return 1
end

--- Convert a (line, UTF-16 character) position to a byte offset in a UTF-8 string.
--
-- Algorithm:
--  1. Walk the string byte-by-byte to find the start of the target line
--     (counting newline characters).
--  2. From the line start, walk UTF-8 codepoints, converting each to its
--     UTF-16 length, until we reach the target character offset.
--
-- @param text string  UTF-8 encoded source text
-- @param line number  0-based line number
-- @param char number  0-based UTF-16 character offset within the line
-- @return     number  1-based byte offset in the Lua string
function M.convert_utf16_offset_to_byte_offset(text, line, char)
    local len = #text
    local pos = 1  -- 1-based byte position (Lua convention)

    -- Phase 1: skip to the target line by counting newlines.
    local current_line = 0
    while current_line < line do
        if pos > len then
            -- Line number exceeds file length; clamp to end.
            return len + 1
        end
        local b = string.byte(text, pos)
        if b == 10 then  -- '\n'
            current_line = current_line + 1
        end
        pos = pos + 1
    end

    -- Phase 2: from the line start, advance char UTF-16 code units.
    local utf16_count = 0
    while utf16_count < char and pos <= len do
        local b = string.byte(text, pos)
        -- Stop at newline — don't advance past the end of the line.
        if b == 10 then
            break
        end
        local seq_len = utf8_sequence_length(b)
        local utf16_len = utf16_units_for_sequence(seq_len)

        if utf16_count + utf16_len > char then
            -- This codepoint would overshoot. Stop here.
            -- Can happen in the middle of a surrogate pair.
            break
        end

        pos = pos + seq_len
        utf16_count = utf16_count + utf16_len
    end

    return pos
end

-- =========================================================================
-- DocumentManager
-- =========================================================================
--
-- When the user opens a file in VS Code, the editor sends didOpen with the
-- full file content. After that, it sends incremental changes (what changed,
-- and where). The DocumentManager applies these changes to maintain the
-- current text of each open file.
--
--   Editor opens file:  didOpen   -> store text at version 1
--   User types "X":     didChange -> apply delta -> version 2
--   User saves:         didSave   -> (optional: trigger format)
--   User closes:        didClose  -> remove entry

--- Document represents an open file.
-- @field uri     string  Document URI
-- @field text    string  Current content (UTF-8)
-- @field version number  Monotonically increasing version number

local DocumentManager = {}
DocumentManager.__index = DocumentManager

--- Create a new, empty DocumentManager.
-- @return DocumentManager
function DocumentManager:new()
    return setmetatable({ docs = {} }, self)
end

--- Record a newly opened file.
-- @param uri     string  Document URI
-- @param text    string  Initial file content
-- @param version number  Initial version number (typically 1)
function DocumentManager:open(uri, text, version)
    self.docs[uri] = { uri = uri, text = text, version = version }
end

--- Get the document for a URI.
-- @param uri string  Document URI
-- @return    table   Document table, or nil if not open
function DocumentManager:get(uri)
    return self.docs[uri]
end

--- Remove a document from the manager.
-- @param uri string  Document URI
function DocumentManager:close(uri)
    self.docs[uri] = nil
end

--- Apply incremental changes to an open document.
-- Changes are applied in order. Each change has:
--   - range: optional table {start={line,character}, ["end"]={line,character}}
--            nil means full replacement
--   - text:  the new text to splice in (or the full replacement text)
--
-- @param uri     string  Document URI
-- @param changes table   Array of {range=..., text=...}
-- @param version number  New version number after changes
-- @return        string  nil on success, error message on failure
function DocumentManager:apply_changes(uri, changes, version)
    local doc = self.docs[uri]
    if not doc then
        return "document not open: " .. uri
    end

    for _, change in ipairs(changes) do
        if change.range == nil then
            -- Full document replacement — simplest case.
            doc.text = change.text
        else
            -- Incremental update: splice new text at the specified range.
            local new_text, err = apply_range_change(doc.text, change.range, change.text)
            if err then
                return "applying change to " .. uri .. ": " .. err
            end
            doc.text = new_text
        end
    end

    doc.version = version
    return nil
end

M.DocumentManager = DocumentManager

--- Apply a single range change to a text string.
-- Converts LSP (line, UTF-16 character) coordinates to byte offsets, then
-- splices the new text in.
--
-- @param text     string  Current document text
-- @param range    table   LSP range {start={line,character}, ["end"]={line,character}}
-- @param new_text string  Replacement text
-- @return         string  New text after the splice
-- @return         string  Error message, or nil
function apply_range_change(text, range, new_text)
    local start_byte = M.convert_utf16_offset_to_byte_offset(
        text, range.start.line, range.start.character)
    local end_byte = M.convert_utf16_offset_to_byte_offset(
        text, range["end"].line, range["end"].character)

    if start_byte > end_byte then
        return nil, string.format("start offset %d > end offset %d", start_byte, end_byte)
    end
    if end_byte > #text + 1 then
        end_byte = #text + 1
    end

    -- Lua strings are 1-based. text:sub(1, start_byte - 1) gets bytes before
    -- the start. text:sub(end_byte) gets bytes from end onward.
    return text:sub(1, start_byte - 1) .. new_text .. text:sub(end_byte), nil
end

-- =========================================================================
-- ParseCache
-- =========================================================================
--
-- Parsing is the most expensive operation in a language server. For a large
-- file, parsing on every keystroke would lag the editor noticeably.
--
-- The cache key is (uri, version). Version is a monotonically increasing
-- integer that the editor increments with each change. Same (uri, version)
-- means the document has not changed => cache hit.
--
-- The old entry is evicted when a new version is cached for the same URI.
-- This keeps memory bounded at O(open_documents) entries.

local ParseCache = {}
ParseCache.__index = ParseCache

--- Create a new, empty ParseCache.
-- @return ParseCache
function ParseCache:new()
    return setmetatable({ cache = {} }, self)
end

--- Build a cache key string from uri and version.
-- Using string concatenation as the key avoids needing a two-level lookup.
--
-- @param uri     string
-- @param version number
-- @return        string
local function cache_key(uri, version)
    return uri .. "\0" .. tostring(version)
end

--- Get or compute the parse result for a document.
-- If the result is already cached, it is returned immediately. Otherwise,
-- bridge.parse(source) is called and the result is stored.
--
-- @param uri     string           Document URI
-- @param version number           Document version
-- @param source  string           Document text
-- @param bridge  table            Language bridge table
-- @return        table            {ast=..., diagnostics={...}, err=...}
function ParseCache:get_or_parse(uri, version, source, bridge)
    local key = cache_key(uri, version)

    -- Cache hit: document has not changed since last parse.
    if self.cache[key] then
        return self.cache[key]
    end

    -- Cache miss: evict old entries for this URI, then parse.
    self:evict(uri)

    local ast, diags, err = bridge.parse(source)
    if diags == nil then
        diags = {}  -- normalize nil to empty table for consistent iteration
    end

    local result = {
        ast = ast,
        diagnostics = diags,
        err = err,
    }
    self.cache[key] = result
    return result
end

--- Remove all cached entries for a given URI.
-- Called when a document is closed, or internally before adding a new entry.
--
-- @param uri string  Document URI
function ParseCache:evict(uri)
    -- Walk all keys and remove any that start with this URI.
    local prefix = uri .. "\0"
    local to_remove = {}
    for k in pairs(self.cache) do
        if k:sub(1, #prefix) == prefix then
            to_remove[#to_remove + 1] = k
        end
    end
    for _, k in ipairs(to_remove) do
        self.cache[k] = nil
    end
end

M.ParseCache = ParseCache

-- =========================================================================
-- Semantic Token Legend & Encoding
-- =========================================================================
--
-- Semantic tokens are the "second pass" of syntax highlighting. The editor's
-- grammar-based highlighter (TextMate) does a fast regex pass first. Semantic
-- tokens layer on top with accurate, context-aware type information.
--
-- Instead of sending {"type":"keyword"} per token, LSP uses a compact integer
-- encoding with a legend. The legend maps integer indices to type/modifier names.

--- Return the standard semantic token legend.
-- The legend is sent once in the capabilities response. Afterwards, each
-- token is encoded as an integer index into this legend.
--
-- @return table  {token_types={...}, token_modifiers={...}}
function M.semantic_token_legend()
    return {
        -- Standard LSP token types (in the order VS Code expects).
        token_types = {
            "namespace",     -- 0 (indices are 0-based in LSP)
            "type",          -- 1
            "class",         -- 2
            "enum",          -- 3
            "interface",     -- 4
            "struct",        -- 5
            "typeParameter", -- 6
            "parameter",     -- 7
            "variable",      -- 8
            "property",      -- 9
            "enumMember",    -- 10
            "event",         -- 11
            "function",      -- 12
            "method",        -- 13
            "macro",         -- 14
            "keyword",       -- 15
            "modifier",      -- 16
            "comment",       -- 17
            "string",        -- 18
            "number",        -- 19
            "regexp",        -- 20
            "operator",      -- 21
            "decorator",     -- 22
        },
        -- Standard LSP token modifiers (bitmask flags).
        -- tokenModifier[0] = "declaration" -> bit 0 (value 1)
        -- tokenModifier[1] = "definition"  -> bit 1 (value 2)
        token_modifiers = {
            "declaration",   -- bit 0
            "definition",    -- bit 1
            "readonly",      -- bit 2
            "static",        -- bit 3
            "deprecated",    -- bit 4
            "abstract",      -- bit 5
            "async",         -- bit 6
            "modification",  -- bit 7
            "documentation", -- bit 8
            "defaultLibrary", -- bit 9
        },
    }
end

--- Find the 0-based index of a token type in the legend.
-- Returns -1 if the type is not found (caller should skip such tokens).
--
-- @param token_type string  Token type name
-- @return           number  0-based index, or -1
local function token_type_index(token_type)
    local legend = M.semantic_token_legend()
    for i, t in ipairs(legend.token_types) do
        if t == token_type then
            return i - 1  -- convert Lua 1-based to LSP 0-based
        end
    end
    return -1
end

--- Compute the bitmask for a list of modifier strings.
-- Each modifier maps to a bit position based on its index in the legend.
-- Multiple modifiers are combined with bitwise OR.
--
-- Example: {"declaration", "readonly"} -> bit 0 | bit 2 = 5
--
-- @param modifiers table  Array of modifier strings
-- @return          number Bitmask
local function token_modifier_mask(modifiers)
    if not modifiers then return 0 end
    local legend = M.semantic_token_legend()
    local mask = 0
    for _, mod in ipairs(modifiers) do
        for i, m in ipairs(legend.token_modifiers) do
            if m == mod then
                -- Lua's bit operations: use math for portability with Lua 5.1.
                -- Bit i-1 (0-based) has value 2^(i-1).
                mask = mask + (2 ^ (i - 1))
                break
            end
        end
    end
    -- Ensure the mask is an integer.
    return math.floor(mask)
end

--- Encode semantic tokens into the LSP compact integer format.
--
-- LSP encodes semantic tokens as a flat array of integers, grouped in 5-tuples:
--
--   [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]
--
-- Where "delta" means the difference from the PREVIOUS token's position.
-- This delta encoding makes most values small (often 0 or 1).
--
-- When deltaLine > 0, deltaStartChar is absolute for the new line (relative
-- to column 0). When deltaLine == 0, deltaStartChar is relative to the
-- previous token's start character.
--
-- Example with three tokens:
--
--   Token A: line=0, char=0, len=3, type="keyword",  modifiers={}
--   Token B: line=0, char=4, len=5, type="function", modifiers={"declaration"}
--   Token C: line=1, char=0, len=8, type="variable", modifiers={}
--
-- Encoded as:
--   {0, 0, 3, 15, 0,   -- A: first token, absolute position
--    0, 4, 5, 12, 1,   -- B: same line, 4 chars after A
--    1, 0, 8,  8, 0}   -- C: next line, absolute char on new line
--
-- @param tokens table  Array of SemanticToken tables
-- @return       table  Flat array of integers
function M.encode_semantic_tokens(tokens)
    if not tokens or #tokens == 0 then
        return {}
    end

    -- Sort by (line, character) ascending. Delta encoding requires document order.
    local sorted = {}
    for i, tok in ipairs(tokens) do
        sorted[i] = tok
    end
    table.sort(sorted, function(a, b)
        if a.line ~= b.line then
            return a.line < b.line
        end
        return a.character < b.character
    end)

    local data = {}
    local prev_line = 0
    local prev_char = 0

    for _, tok in ipairs(sorted) do
        local type_idx = token_type_index(tok.token_type)
        if type_idx == -1 then
            -- Unknown token type — skip it. The client wouldn't know what to
            -- do with an index outside the legend anyway.
            goto continue
        end

        local delta_line = tok.line - prev_line
        local delta_char
        if delta_line == 0 then
            -- Same line: character offset is relative to previous token.
            delta_char = tok.character - prev_char
        else
            -- Different line: character offset is absolute (relative to line start).
            delta_char = tok.character
        end

        local mod_mask = token_modifier_mask(tok.modifiers)

        data[#data + 1] = delta_line
        data[#data + 1] = delta_char
        data[#data + 1] = tok.length
        data[#data + 1] = type_idx
        data[#data + 1] = mod_mask

        prev_line = tok.line
        prev_char = tok.character

        ::continue::
    end

    return data
end

-- =========================================================================
-- Capabilities
-- =========================================================================
--
-- During the initialize handshake, the server sends a "capabilities" object
-- telling the editor which features it supports. If a capability is absent,
-- the editor won't send the corresponding requests.
--
-- Building capabilities dynamically (based on the bridge's function fields)
-- means the server is always honest about what it can do.

--- Build the LSP capabilities object based on which functions the bridge provides.
-- Uses Lua's nil check: if bridge.hover ~= nil, advertise hoverProvider.
--
-- @param bridge table  Language bridge table
-- @return       table  LSP capabilities object
function M.build_capabilities(bridge)
    -- textDocumentSync=2 means "incremental": the editor sends only changed
    -- ranges, not the full file, on every keystroke.
    local caps = {
        textDocumentSync = 2,
    }

    if bridge.hover ~= nil then
        caps.hoverProvider = true
    end

    if bridge.definition ~= nil then
        caps.definitionProvider = true
    end

    if bridge.references ~= nil then
        caps.referencesProvider = true
    end

    if bridge.completion ~= nil then
        -- completionProvider is an object with trigger characters.
        caps.completionProvider = {
            triggerCharacters = { " ", "." },
        }
    end

    if bridge.rename ~= nil then
        caps.renameProvider = true
    end

    if bridge.document_symbols ~= nil then
        caps.documentSymbolProvider = true
    end

    if bridge.folding_ranges ~= nil then
        caps.foldingRangeProvider = true
    end

    if bridge.signature_help ~= nil then
        -- signatureHelpProvider includes trigger characters.
        caps.signatureHelpProvider = {
            triggerCharacters = { "(", "," },
        }
    end

    if bridge.format ~= nil then
        caps.documentFormattingProvider = true
    end

    if bridge.semantic_tokens ~= nil then
        caps.semanticTokensProvider = {
            legend = M.semantic_token_legend(),
            full = true,
        }
    end

    return caps
end

-- =========================================================================
-- LspServer
-- =========================================================================
--
-- The LspServer wires together:
--   - The bridge (language-specific logic)
--   - The DocumentManager (tracks open file contents)
--   - The ParseCache (avoids redundant parses)
--   - The JSON-RPC Server (protocol layer)
--
-- It registers all LSP handlers with the JSON-RPC server, then calls
-- serve() to start the blocking read-dispatch-write loop.

local LspServer = {}
LspServer.__index = LspServer

--- Create a new LspServer.
--
-- @param bridge     table  Language bridge table (must have tokenize and parse)
-- @param in_stream  table  Readable stream (must have :read() method)
-- @param out_stream table  Writable stream (must have :write() method)
-- @return           LspServer
function LspServer:new(bridge, in_stream, out_stream)
    local server = setmetatable({
        bridge      = bridge,
        doc_manager = DocumentManager:new(),
        parse_cache = ParseCache:new(),
        rpc_server  = json_rpc.Server:new(in_stream, out_stream),
        writer      = json_rpc.MessageWriter:new(out_stream),
        shutdown    = false,
        initialized = false,
    }, self)

    server:register_handlers()
    return server
end

--- Start the blocking JSON-RPC read-dispatch-write loop.
-- This call blocks until the editor closes the connection (EOF on stdin).
function LspServer:serve()
    self.rpc_server:serve()
end

--- Send a server-initiated notification to the editor.
-- Used for pushing diagnostics, etc. Notifications have no "id" and
-- the editor sends no response.
--
-- @param method string  Notification method name
-- @param params table   Notification parameters
function LspServer:send_notification(method, params)
    local notif = json_rpc.Notification(method, params)
    self.writer:write_message(notif)
end

--- Get the current parse result for a document.
-- This is the hot path for all feature handlers. It gets the document from
-- the DocumentManager and returns the cached ParseResult (or re-parses).
--
-- @param uri string  Document URI
-- @return    table   Document table
-- @return    table   ParseResult {ast, diagnostics, err}
-- @return    table   Error response table, or nil
function LspServer:get_parse_result(uri)
    local doc = self.doc_manager:get(uri)
    if not doc then
        return nil, nil, {
            code = M.errors.REQUEST_FAILED,
            message = "document not open: " .. uri,
        }
    end

    local result = self.parse_cache:get_or_parse(uri, doc.version, doc.text, self.bridge)
    return doc, result, nil
end

--- Publish diagnostics (squiggles) to the editor.
-- Called after every didOpen and didChange to update the editor's display.
--
-- @param uri         string  Document URI
-- @param version     number  Document version
-- @param diagnostics table   Array of Diagnostic tables
function LspServer:publish_diagnostics(uri, version, diagnostics)
    local lsp_diags = {}
    for i, d in ipairs(diagnostics) do
        local diag = {
            range = range_to_lsp(d.range),
            severity = d.severity,
            message = d.message,
        }
        if d.code then
            diag.code = d.code
        end
        lsp_diags[i] = diag
    end

    local params = {
        uri = uri,
        diagnostics = lsp_diags,
    }
    if version > 0 then
        params.version = version
    end

    self:send_notification("textDocument/publishDiagnostics", params)
end

-- ─── LSP type conversion helpers ────────────────────────────────────────────

--- Convert a Position to LSP format.
local function position_to_lsp(p)
    return { line = p.line, character = p.character }
end

--- Convert a Range to LSP format.
function range_to_lsp(r)
    return {
        start = position_to_lsp(r.start),
        ["end"] = position_to_lsp(r["end"]),
    }
end

--- Convert a Location to LSP format.
local function location_to_lsp(l)
    return {
        uri = l.uri,
        range = range_to_lsp(l.range),
    }
end

--- Extract a Position from JSON-RPC params.
local function parse_position(params)
    local pos = params.position or {}
    return M.Position(pos.line or 0, pos.character or 0)
end

--- Extract the document URI from params with a textDocument field.
local function parse_uri(params)
    local td = params.textDocument or {}
    return td.uri or ""
end

--- Parse a raw LSP range object from the protocol.
local function parse_lsp_range(raw)
    if type(raw) ~= "table" then return M.Range(M.Position(0, 0), M.Position(0, 0)) end
    local s = raw.start or {}
    local e = raw["end"] or {}
    return M.Range(
        M.Position(s.line or 0, s.character or 0),
        M.Position(e.line or 0, e.character or 0)
    )
end

-- ─── Handler Registration ───────────────────────────────────────────────────

--- Wire all LSP method names to their handler functions.
function LspServer:register_handlers()
    -- Lifecycle
    self.rpc_server:on_request("initialize", function(id, params) return self:handle_initialize(id, params) end)
    self.rpc_server:on_notification("initialized", function(params) self:handle_initialized(params) end)
    self.rpc_server:on_request("shutdown", function(id, params) return self:handle_shutdown(id, params) end)
    self.rpc_server:on_notification("exit", function(params) self:handle_exit(params) end)

    -- Text document synchronization
    self.rpc_server:on_notification("textDocument/didOpen", function(params) self:handle_did_open(params) end)
    self.rpc_server:on_notification("textDocument/didChange", function(params) self:handle_did_change(params) end)
    self.rpc_server:on_notification("textDocument/didClose", function(params) self:handle_did_close(params) end)
    self.rpc_server:on_notification("textDocument/didSave", function(params) self:handle_did_save(params) end)

    -- Feature requests
    self.rpc_server:on_request("textDocument/hover", function(id, params) return self:handle_hover(id, params) end)
    self.rpc_server:on_request("textDocument/definition", function(id, params) return self:handle_definition(id, params) end)
    self.rpc_server:on_request("textDocument/references", function(id, params) return self:handle_references(id, params) end)
    self.rpc_server:on_request("textDocument/completion", function(id, params) return self:handle_completion(id, params) end)
    self.rpc_server:on_request("textDocument/rename", function(id, params) return self:handle_rename(id, params) end)
    self.rpc_server:on_request("textDocument/documentSymbol", function(id, params) return self:handle_document_symbol(id, params) end)
    self.rpc_server:on_request("textDocument/semanticTokens/full", function(id, params) return self:handle_semantic_tokens_full(id, params) end)
    self.rpc_server:on_request("textDocument/foldingRange", function(id, params) return self:handle_folding_range(id, params) end)
    self.rpc_server:on_request("textDocument/signatureHelp", function(id, params) return self:handle_signature_help(id, params) end)
    self.rpc_server:on_request("textDocument/formatting", function(id, params) return self:handle_formatting(id, params) end)
end

-- ─── Lifecycle Handlers ─────────────────────────────────────────────────────

--- Handle the initialize request.
-- The editor sends this as the very first message. We return our capabilities.
function LspServer:handle_initialize(id, params)
    self.initialized = true
    local caps = M.build_capabilities(self.bridge)

    return {
        capabilities = caps,
        serverInfo = {
            name = "ls00-generic-lsp-server",
            version = "0.1.0",
        },
    }
end

--- Handle the initialized notification (handshake complete, no-op).
function LspServer:handle_initialized(params)
    -- No-op. Normal operation begins now.
end

--- Handle the shutdown request. Set the shutdown flag and return null.
function LspServer:handle_shutdown(id, params)
    self.shutdown = true
    return json_rpc.null
end

--- Handle the exit notification. Terminate the process.
function LspServer:handle_exit(params)
    if self.shutdown then
        os.exit(0)
    else
        os.exit(1)
    end
end

-- ─── Text Document Handlers ─────────────────────────────────────────────────

--- Handle textDocument/didOpen.
-- Store the document and push initial diagnostics.
function LspServer:handle_did_open(params)
    if type(params) ~= "table" then return end
    local td = params.textDocument
    if type(td) ~= "table" then return end

    local uri = td.uri or ""
    local text = td.text or ""
    local version = td.version or 1

    if uri == "" then return end

    self.doc_manager:open(uri, text, version)

    local result = self.parse_cache:get_or_parse(uri, version, text, self.bridge)
    self:publish_diagnostics(uri, version, result.diagnostics)
end

--- Handle textDocument/didChange.
-- Apply incremental changes and push updated diagnostics.
function LspServer:handle_did_change(params)
    if type(params) ~= "table" then return end

    local uri = parse_uri(params)
    if uri == "" then return end

    local version = 0
    local td = params.textDocument
    if type(td) == "table" and td.version then
        version = td.version
    end

    local content_changes = params.contentChanges or {}
    local changes = {}
    for _, change_raw in ipairs(content_changes) do
        if type(change_raw) == "table" then
            local change = { text = change_raw.text or "" }
            if change_raw.range ~= nil then
                change.range = parse_lsp_range(change_raw.range)
            end
            changes[#changes + 1] = change
        end
    end

    local err = self.doc_manager:apply_changes(uri, changes, version)
    if err then return end

    local doc = self.doc_manager:get(uri)
    if not doc then return end

    local result = self.parse_cache:get_or_parse(uri, doc.version, doc.text, self.bridge)
    self:publish_diagnostics(uri, version, result.diagnostics)
end

--- Handle textDocument/didClose.
-- Remove the document and clear diagnostics.
function LspServer:handle_did_close(params)
    if type(params) ~= "table" then return end
    local uri = parse_uri(params)
    if uri == "" then return end

    self.doc_manager:close(uri)
    self.parse_cache:evict(uri)

    -- Clear diagnostics by publishing an empty list.
    self:publish_diagnostics(uri, 0, {})
end

--- Handle textDocument/didSave.
-- Re-parse if the client sends full text in didSave.
function LspServer:handle_did_save(params)
    if type(params) ~= "table" then return end
    local uri = parse_uri(params)
    if uri == "" then return end

    if params.text and params.text ~= "" then
        local doc = self.doc_manager:get(uri)
        if doc then
            self.doc_manager:close(uri)
            self.doc_manager:open(uri, params.text, doc.version)
            local result = self.parse_cache:get_or_parse(uri, doc.version, params.text, self.bridge)
            self:publish_diagnostics(uri, doc.version, result.diagnostics)
        end
    end
end

-- ─── Feature Handlers ───────────────────────────────────────────────────────

--- Handle textDocument/hover.
function LspServer:handle_hover(id, params)
    if type(params) ~= "table" then return json_rpc.null end
    local uri = parse_uri(params)
    local pos = parse_position(params)

    if not self.bridge.hover then return json_rpc.null end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return err end
    if not parse_result or not parse_result.ast then return json_rpc.null end

    local ok, hover_result = pcall(self.bridge.hover, parse_result.ast, pos)
    if not ok or not hover_result then return json_rpc.null end

    local result = {
        contents = {
            kind = "markdown",
            value = hover_result.contents,
        },
    }
    if hover_result.range then
        result.range = range_to_lsp(hover_result.range)
    end

    return result
end

--- Handle textDocument/definition.
function LspServer:handle_definition(id, params)
    if type(params) ~= "table" then return json_rpc.null end
    local uri = parse_uri(params)
    local pos = parse_position(params)

    if not self.bridge.definition then return json_rpc.null end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return err end
    if not parse_result or not parse_result.ast then return json_rpc.null end

    local ok, loc = pcall(self.bridge.definition, parse_result.ast, pos, uri)
    if not ok or not loc then return json_rpc.null end

    return location_to_lsp(loc)
end

--- Handle textDocument/references.
function LspServer:handle_references(id, params)
    if type(params) ~= "table" then return {} end
    local uri = parse_uri(params)
    local pos = parse_position(params)
    local ctx = params.context or {}
    local include_decl = ctx.includeDeclaration or false

    if not self.bridge.references then return {} end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return {} end
    if not parse_result or not parse_result.ast then return {} end

    local ok, locs = pcall(self.bridge.references, parse_result.ast, pos, uri, include_decl)
    if not ok or not locs then return {} end

    local result = {}
    for i, loc in ipairs(locs) do
        result[i] = location_to_lsp(loc)
    end
    return result
end

--- Handle textDocument/completion.
function LspServer:handle_completion(id, params)
    if type(params) ~= "table" then return {} end
    local uri = parse_uri(params)
    local pos = parse_position(params)

    if not self.bridge.completion then return {} end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return {} end
    if not parse_result or not parse_result.ast then return {} end

    local ok, items = pcall(self.bridge.completion, parse_result.ast, pos)
    if not ok or not items then return {} end

    return items
end

--- Handle textDocument/rename.
function LspServer:handle_rename(id, params)
    if type(params) ~= "table" then return json_rpc.null end
    local uri = parse_uri(params)
    local pos = parse_position(params)
    local new_name = params.newName or ""

    if not self.bridge.rename then return json_rpc.null end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return json_rpc.null end
    if not parse_result or not parse_result.ast then return json_rpc.null end

    local ok, edit = pcall(self.bridge.rename, parse_result.ast, pos, new_name)
    if not ok or not edit then return json_rpc.null end

    -- Convert workspace edit to LSP format.
    local lsp_changes = {}
    if edit.changes then
        for file_uri, edits in pairs(edit.changes) do
            local lsp_edits = {}
            for i, te in ipairs(edits) do
                lsp_edits[i] = {
                    range = range_to_lsp(te.range),
                    newText = te.newText,
                }
            end
            lsp_changes[file_uri] = lsp_edits
        end
    end

    return { changes = lsp_changes }
end

--- Handle textDocument/documentSymbol.
function LspServer:handle_document_symbol(id, params)
    if type(params) ~= "table" then return {} end
    local uri = parse_uri(params)

    if not self.bridge.document_symbols then return {} end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return {} end
    if not parse_result or not parse_result.ast then return {} end

    local ok, symbols = pcall(self.bridge.document_symbols, parse_result.ast)
    if not ok or not symbols then return {} end

    -- Convert symbols to LSP format (recursive).
    local function convert_symbols(syms)
        local result = {}
        for i, s in ipairs(syms) do
            local lsp_sym = {
                name = s.name,
                kind = s.kind,
                range = range_to_lsp(s.range),
                selectionRange = range_to_lsp(s.selectionRange),
            }
            if s.children and #s.children > 0 then
                lsp_sym.children = convert_symbols(s.children)
            end
            result[i] = lsp_sym
        end
        return result
    end

    return convert_symbols(symbols)
end

--- Handle textDocument/semanticTokens/full.
function LspServer:handle_semantic_tokens_full(id, params)
    if type(params) ~= "table" then return { data = {} } end
    local uri = parse_uri(params)

    if not self.bridge.semantic_tokens then return { data = {} } end

    local doc = self.doc_manager:get(uri)
    if not doc then return { data = {} } end

    local ok_tok, tokens = pcall(self.bridge.tokenize, doc.text)
    if not ok_tok or not tokens then return { data = {} } end

    local ok_sem, sem_tokens = pcall(self.bridge.semantic_tokens, doc.text, tokens)
    if not ok_sem or not sem_tokens then return { data = {} } end

    local data = M.encode_semantic_tokens(sem_tokens)
    return { data = data }
end

--- Handle textDocument/foldingRange.
function LspServer:handle_folding_range(id, params)
    if type(params) ~= "table" then return {} end
    local uri = parse_uri(params)

    if not self.bridge.folding_ranges then return {} end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return {} end
    if not parse_result or not parse_result.ast then return {} end

    local ok, ranges = pcall(self.bridge.folding_ranges, parse_result.ast)
    if not ok or not ranges then return {} end

    return ranges
end

--- Handle textDocument/signatureHelp.
function LspServer:handle_signature_help(id, params)
    if type(params) ~= "table" then return json_rpc.null end
    local uri = parse_uri(params)
    local pos = parse_position(params)

    if not self.bridge.signature_help then return json_rpc.null end

    local _, parse_result, err = self:get_parse_result(uri)
    if err then return json_rpc.null end
    if not parse_result or not parse_result.ast then return json_rpc.null end

    local ok, result = pcall(self.bridge.signature_help, parse_result.ast, pos)
    if not ok or not result then return json_rpc.null end

    return result
end

--- Handle textDocument/formatting.
function LspServer:handle_formatting(id, params)
    if type(params) ~= "table" then return {} end
    local uri = parse_uri(params)

    if not self.bridge.format then return {} end

    local doc = self.doc_manager:get(uri)
    if not doc then return {} end

    local ok, edits = pcall(self.bridge.format, doc.text)
    if not ok or not edits then return {} end

    -- Convert text edits to LSP format.
    local result = {}
    for i, te in ipairs(edits) do
        result[i] = {
            range = range_to_lsp(te.range),
            newText = te.newText,
        }
    end
    return result
end

M.LspServer = LspServer

-- =========================================================================
-- Module exports
-- =========================================================================

return M
