-- resolver.lua -- Dependency Resolution from Package Metadata
-- ============================================================
--
-- This module reads package metadata files (pyproject.toml, .gemspec, go.mod,
-- package.json, Cargo.toml, mix.exs, .rockspec) and extracts internal
-- dependencies. It builds a directed graph where edges represent build
-- ordering: an edge from A to B means "A must be built before B".
--
-- Dependency mapping conventions
-- ------------------------------
--
-- Each language ecosystem uses a different naming convention:
--
--   Python:     "coding-adventures-logic-gates"      (hyphens, pyproject.toml)
--   Ruby:       "coding_adventures_logic_gates"      (underscores, .gemspec)
--   Go:         full module path                     (go.mod)
--   TypeScript: "@coding-adventures/logic-gates"     (npm scope, package.json)
--   Rust:       "logic-gates"                        (crate name, Cargo.toml)
--   Elixir:     "coding_adventures_logic_gates"      (underscores, mix.exs)
--   Lua:        "coding-adventures-logic-gates"      (hyphens, .rockspec)
--
-- External dependencies are silently skipped.

local DirectedGraph = require("build_tool.directed_graph")

local Resolver = {}

local MetadataEncodingError = {}
MetadataEncodingError.__index = MetadataEncodingError

function MetadataEncodingError.new(pkg, manifest)
    return setmetatable({
        code = "METADATA_INVALID_UTF8",
        package = pkg.name,
        manifest = manifest,
        encoding = "UTF-8",
    }, MetadataEncodingError)
end

function MetadataEncodingError:__tostring()
    return string.format(
        "%s: package=%s manifest=%s encoding=%s",
        self.code,
        self.package,
        self.manifest,
        self.encoding
    )
end

--- Return whether a caught error is the stable metadata encoding error.
---
--- @param value any A value returned by pcall.
--- @return boolean True when value is a MetadataEncodingError.
function Resolver.is_metadata_encoding_error(value)
    return getmetatable(value) == MetadataEncodingError
end

-- A package whose own manifest resolves the package itself as a dependency.
-- The conformance contract rejects this as DEPENDENCY_SELF_EDGE: a self-edge
-- is neither dropped nor drawn (a drawn self-loop would surface later as a
-- cycle, far from its cause). A Cargo manifest can do it with nothing more
-- than a path entry named after its own directory or [package] name. The
-- manifest is repository-relative, so the message never discloses the
-- checkout root.
local DependencySelfEdgeError = {}
DependencySelfEdgeError.__index = DependencySelfEdgeError

function DependencySelfEdgeError.new(pkg, manifest)
    return setmetatable({
        code = "DEPENDENCY_SELF_EDGE",
        package = pkg.name,
        manifest = manifest,
        dependency = pkg.name,
    }, DependencySelfEdgeError)
end

function DependencySelfEdgeError:__tostring()
    return string.format(
        "%s: package=%s manifest=%s dependency=%s",
        self.code,
        self.package,
        self.manifest,
        self.dependency
    )
end

--- Return whether a caught error is the stable dependency self-edge error.
---
--- @param value any A value returned by pcall.
--- @return boolean True when value is a DependencySelfEdgeError.
function Resolver.is_dependency_self_edge_error(value)
    return getmetatable(value) == DependencySelfEdgeError
end

-- =========================================================================
-- Helper: read a file's contents as a string.
-- =========================================================================

local function read_file(path)
    local f = io.open(path, "r")
    if not f then return nil end
    local content = f:read("*a")
    f:close()
    return content
end

local function normalize_path(path)
    local normalized = path:gsub("\\", "/")
    local prefix = ""
    local segments = {}

    if normalized:match("^[A-Za-z]:/") then
        prefix = normalized:sub(1, 2):lower()
        normalized = normalized:sub(3)
    elseif normalized:sub(1, 1) == "/" then
        prefix = "/"
        normalized = normalized:sub(2)
    end

    for segment in normalized:gmatch("[^/]+") do
        if segment == ".." then
            if #segments > 0 and segments[#segments] ~= ".." then
                table.remove(segments)
            elseif prefix == "" then
                segments[#segments + 1] = segment
            end
        elseif segment ~= "." and segment ~= "" then
            segments[#segments + 1] = segment
        end
    end

    local joined = table.concat(segments, "/")
    if prefix == "/" then
        return joined == "" and "/" or "/" .. joined
    elseif prefix ~= "" then
        return joined == "" and (prefix .. "/") or (prefix .. "/" .. joined)
    end
    return joined
end

local function repository_relative_manifest(path)
    local normalized = normalize_path(path)
    if normalized:sub(1, 5) == "code/" then
        return normalized
    end

    local marker = normalized:find("/code/", 1, true)
    if marker then
        return normalized:sub(marker + 1)
    end

    return normalized:match("([^/]+)$") or normalized
end

local function is_continuation(byte)
    return byte and byte >= 0x80 and byte <= 0xbf
end

--- Validate one byte string as strict UTF-8.
---
--- This rejects truncated sequences, continuation bytes in leading position,
--- overlong forms, UTF-16 surrogate code points, and values above U+10FFFF.
--- A literal U+FFFD remains valid because it is ordinary well-formed UTF-8.
---
--- @param text string Raw file bytes.
--- @return boolean True when every byte belongs to a valid UTF-8 sequence.
local function is_valid_utf8(text)
    local index = 1
    while index <= #text do
        local first = text:byte(index)

        if first <= 0x7f then
            index = index + 1
        elseif first >= 0xc2 and first <= 0xdf then
            if not is_continuation(text:byte(index + 1)) then return false end
            index = index + 2
        elseif first >= 0xe0 and first <= 0xef then
            local second = text:byte(index + 1)
            local third = text:byte(index + 2)
            if first == 0xe0 then
                if not second or second < 0xa0 or second > 0xbf then return false end
            elseif first == 0xed then
                if not second or second < 0x80 or second > 0x9f then return false end
            elseif not is_continuation(second) then
                return false
            end
            if not is_continuation(third) then return false end
            index = index + 3
        elseif first >= 0xf0 and first <= 0xf4 then
            local second = text:byte(index + 1)
            local third = text:byte(index + 2)
            local fourth = text:byte(index + 3)
            if first == 0xf0 then
                if not second or second < 0x90 or second > 0xbf then return false end
            elseif first == 0xf4 then
                if not second or second < 0x80 or second > 0x8f then return false end
            elseif not is_continuation(second) then
                return false
            end
            if not is_continuation(third) or not is_continuation(fourth) then
                return false
            end
            index = index + 4
        else
            return false
        end
    end

    return true
end

local function read_lua_metadata(path, pkg)
    local file = io.open(path, "rb")
    if not file then return nil end
    local content = file:read("*a")
    file:close()

    if not is_valid_utf8(content) then
        error(MetadataEncodingError.new(pkg, repository_relative_manifest(path)), 0)
    end

    return content
end

-- =========================================================================
-- Helper: find a file by extension in a directory.
-- =========================================================================

local function find_file_by_extension(directory, extension)
    -- Try using lfs if available.
    local lfs_ok, lfs = pcall(require, "lfs")
    if lfs_ok then
        -- lfs.dir() throws if the directory doesn't exist, so wrap in pcall.
        local ok, iter, state = pcall(lfs.dir, directory)
        if not ok then return nil end
        for name in iter, state do
            if name:match("%." .. extension .. "$") then
                return directory .. "/" .. name
            end
        end
        return nil
    end

    -- Fallback: use ls or dir command.
    local sep = package.config:sub(1, 1)
    local cmd
    if sep == "\\" then
        cmd = 'dir /b "' .. directory .. '\\*.' .. extension .. '" 2>nul'
    else
        cmd = 'ls "' .. directory .. '"/*.' .. extension .. ' 2>/dev/null'
    end
    local handle = io.popen(cmd)
    if handle then
        local result = handle:read("*l")
        handle:close()
        if result then
            -- On Unix, ls returns the full path. On Windows, dir returns just the name.
            if sep == "\\" then
                return directory .. "/" .. result:match("^%s*(.-)%s*$")
            else
                return result:match("^%s*(.-)%s*$")
            end
        end
    end
    return nil
end

local function find_files_by_extensions(directory, extensions)
    local files = {}
    local seen = {}

    for _, extension in ipairs(extensions) do
        local filepath = find_file_by_extension(directory, extension)
        if filepath then
            local normalized = normalize_path(filepath)
            if not seen[normalized] then
                seen[normalized] = true
                files[#files + 1] = filepath
            end
        end
    end

    table.sort(files)
    return files
end

local function dependency_scope(language)
    if language == "csharp" or language == "fsharp" or language == "dotnet" then
        return "dotnet"
    elseif language == "wasm" then
        return "rust"
    end
    return language
end

local function in_dependency_scope(package_language, scope)
    if scope == "dotnet" then
        return package_language == "csharp" or package_language == "fsharp" or package_language == "dotnet"
    elseif scope == "rust" then
        return package_language == "rust" or package_language == "wasm"
    end
    return package_language == scope
end

local function read_cargo_package_name(pkg)
    local text = read_file(pkg.path .. "/Cargo.toml")
    if not text then return nil end

    for line in text:gmatch("[^\n]+") do
        local name = line:match('^%s*name%s*=%s*"([^"]+)"')
        if name then
            return name:lower()
        end
    end

    return nil
end

local function set_known(known, key, pkg)
    if not known[key] then
        known[key] = pkg.name
        return
    end

    local normalized = pkg.path:gsub("\\", "/"):lower()
    if not normalized:match("/programs/") then
        known[key] = pkg.name
    end
end

-- =========================================================================
-- Python dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Python pyproject.toml.
--
-- We look for the dependencies = [...] array and extract quoted strings
-- matching our internal naming convention.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from ecosystem name to package name.
-- @return table List of internal dependency package names.
local function parse_python_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/pyproject.toml")
    if not text then return {} end

    local deps = {}
    local in_deps = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if not in_deps then
            if stripped:match("^dependencies") and stripped:match("=") then
                local after_eq = stripped:match("=%s*(.*)$")
                if after_eq and after_eq:match("^%[") then
                    in_deps = true
                    if after_eq:match("%]") then
                        -- Single-line array.
                        for dep_str in after_eq:gmatch('"([^"]+)"') do
                            local name = dep_str:match("^([%w%-]+)"):lower()
                            if known_names[name] then
                                deps[#deps + 1] = known_names[name]
                            end
                        end
                        break
                    end
                end
            end
        else
            if stripped:match("%]") then
                break
            end
            for dep_str in stripped:gmatch('"([^"]+)"') do
                local name = dep_str:match("^([%w%-]+)"):lower()
                if known_names[name] then
                    deps[#deps + 1] = known_names[name]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Ruby dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Ruby .gemspec file.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from gem name to package name.
-- @return table List of internal dependency package names.
local function parse_ruby_deps(pkg, known_names)
    local gemspec_path = find_file_by_extension(pkg.path, "gemspec")
    if not gemspec_path then return {} end

    local text = read_file(gemspec_path)
    if not text then return {} end

    local deps = {}
    for gem_name in text:gmatch('spec%.add_dependency%s+"([^"]+)"') do
        local name = gem_name:lower()
        if known_names[name] then
            deps[#deps + 1] = known_names[name]
        end
    end

    return deps
end

-- =========================================================================
-- Go dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Go go.mod file.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from module path to package name.
-- @return table List of internal dependency package names.
local function parse_go_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/go.mod")
    if not text then return {} end

    local deps = {}
    local in_require = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if stripped == "require (" then
            in_require = true
        elseif stripped == ")" then
            in_require = false
        elseif in_require or stripped:match("^require ") then
            local clean = stripped:gsub("^require%s+", "")
            local module_path = clean:match("^(%S+)")
            if module_path then
                module_path = module_path:lower()
                if known_names[module_path] then
                    deps[#deps + 1] = known_names[module_path]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- TypeScript dependency parsing
-- =========================================================================

--- Extract internal dependencies from a TypeScript package.json.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from npm name to package name.
-- @return table List of internal dependency package names.
local function parse_typescript_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/package.json")
    if not text then return {} end

    local deps = {}
    local in_deps = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if not in_deps then
            if stripped:match('"dependencies"') and stripped:match("{") then
                in_deps = true
            end
        else
            if stripped:match("}") then
                in_deps = false
            else
                for raw_npm_name in stripped:gmatch('"(@coding%-adventures/[^"]+)"') do
                    local npm_name = raw_npm_name:lower()
                    if known_names[npm_name] then
                        deps[#deps + 1] = known_names[npm_name]
                    end
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Rust dependency parsing
-- =========================================================================
--
-- Which Cargo tables are build-graph edges
-- ----------------------------------------
--
-- A graph edge means "rebuild and re-test the dependent when this changes",
-- so it must cover everything Cargo compiles INTO the crate:
--
--   table                                 edge?  why
--   ------------------------------------  -----  ----------------------------
--   [dependencies]                        yes    linked into the library
--   [build-dependencies]                  yes    compiled into build.rs
--   [build_dependencies]                  yes    legacy spelling of the above
--   [target.<cfg>.<one of the above>]     yes    union over every cfg: CI
--                                                builds on several hosts
--   [dev-dependencies] (every form)       no     tests only; Cargo allows a
--                                                dev-dependency cycle
--   [patch], [replace], [workspace], ...  no     not a dependency of the crate
--
-- Both TOML spellings of a path dependency are read:
--
--   [dependencies]
--   logic-gates = { path = "../logic-gates" }          <- inline table
--   gates = { package = "logic-gates", path = "..." }  <- renamed
--
--   [dependencies.logic-gates]                         <- sub-table
--   path = "../logic-gates"
--
-- The line procedure is the Rust paragraph of
-- code/specs/build-tool-conformance.md; every build-tool front implements it
-- and the shared fixture resolution-rust-field-aware pins it. Lua strings
-- are byte strings and every character the procedure compares is ASCII, so
-- scanning bytes is exact.

-- The table names whose path entries link into, or build, the crate.
-- Compared exactly: TOML keys are case-sensitive, so `[Dependencies]` is
-- some other table.
local CARGO_LINK_TABLE_NAMES = {
    ["dependencies"] = true,
    ["build-dependencies"] = true,
    ["build_dependencies"] = true,
}

local QUOTE_DOUBLE, QUOTE_SINGLE, BACKSLASH = 34, 39, 92

--- Trim only spaces and tabs (%s would also eat \f, \v, \r ...).
--
-- An index scan from each end, not gsub("[ \t]+$", ""): an unanchored
-- pattern is retried from every byte of an interior run of spaces and fails
-- at each one, which is quadratic in a long run between two other bytes.
local function trim_spaces_and_tabs(text)
    local first, last = 1, #text
    while first <= last do
        local byte = text:byte(first)
        if byte ~= 32 and byte ~= 9 then break end
        first = first + 1
    end
    while last >= first do
        local byte = text:byte(last)
        if byte ~= 32 and byte ~= 9 then break end
        last = last - 1
    end
    return text:sub(first, last)
end
Resolver.trim_spaces_and_tabs = trim_spaces_and_tabs

--- Lower-case only A-Z (string.lower may follow the C locale).
local function ascii_lower(text)
    return (text:gsub("[A-Z]", function(letter)
        return string.char(letter:byte() + 32)
    end))
end

--- Index just past the string opening at text[start] (a `"` or `'`), or nil
-- when it is not closed on this line (strings never span lines here). In a
-- basic "..." string a backslash makes the next byte part of the string, so
-- `\"` does not end it; a literal '...' string has no escapes.
local function toml_string_end(text, start)
    local quote = text:byte(start)
    local index = start + 1
    while index <= #text do
        local byte = text:byte(index)
        if quote == QUOTE_DOUBLE and byte == BACKSLASH then
            index = index + 2
        elseif byte == quote then
            return index + 1
        else
            index = index + 1
        end
    end
    return nil
end

local function is_quote(byte)
    return byte == QUOTE_DOUBLE or byte == QUOTE_SINGLE
end

--- Drop everything from the first `#` outside a string. An unclosed string
-- runs to the end of the line.
local function strip_toml_line_comment(line)
    local index = 1
    while index <= #line do
        local byte = line:byte(index)
        if is_quote(byte) then
            index = toml_string_end(line, index)
            if not index then return line end
        elseif byte == 35 then -- "#"
            return line:sub(1, index - 1)
        else
            index = index + 1
        end
    end
    return line
end

--- Index of the first `target` byte outside every string, or nil.
local function index_toml_outside_strings(text, target)
    local target_byte = target:byte()
    local index = 1
    while index <= #text do
        local byte = text:byte(index)
        if is_quote(byte) then
            index = toml_string_end(text, index)
            if not index then return nil end
        elseif byte == target_byte then
            return index
        else
            index = index + 1
        end
    end
    return nil
end

--- Count the `{` and `[` outside strings left unclosed, over text fed in
-- pieces. `state` is a table {depth, quote, escape}. Feeding "ab" and then
-- "cd" leaves exactly the state feeding "abcd" would, because the state keeps
-- what a rescan of the joined text would have to rediscover: whether it is
-- inside a string (and which delimiter byte closes it), and whether the
-- previous byte was a backslash escaping the next one in a basic string. So
-- a string still open at the end of one line continues after the joining
-- space, as it does in the joined value:
--
--   feed '{ x = "q\\'   depth 1, inside "...", escape pending
--   feed ' '            the space is escaped: still inside the string
--   feed '", p = 1 }'   the string closes, `}` closes the table: depth 0
--
-- Depth zero (or less) means an inline value is complete:
--
--   { path = "../a", features = [   -> 2
--   { path = "../a" }               -> 0
--
-- Carrying this state is what keeps the continuation join linear: the
-- reader used to rescan the whole growing value for every appended line,
-- which is quadratic in a long entry.
local function toml_bracket_scan(state, text)
    local depth, quote, escape = state.depth, state.quote, state.escape
    for index = 1, #text do
        local byte = text:byte(index)
        if escape then
            escape = false
        elseif quote then
            if quote == QUOTE_DOUBLE and byte == BACKSLASH then
                escape = true
            elseif byte == quote then
                quote = nil
            end
        elseif is_quote(byte) then
            quote = byte
        elseif byte == 123 or byte == 91 then -- { [
            depth = depth + 1
        elseif byte == 125 or byte == 93 then -- } ]
            depth = depth - 1
        end
    end
    state.depth, state.quote, state.escape = depth, quote, escape
end

--- Split at every separator outside strings. With `nested`, a separator
-- inside `[...]` or `{...}` does not split either. An unclosed string runs
-- to the end of the last part.
local function split_toml_outside_strings(text, separator, nested)
    local separator_byte = separator:byte()
    local parts = {}
    local start, depth, index = 1, 0, 1
    while index <= #text do
        local byte = text:byte(index)
        if is_quote(byte) then
            index = toml_string_end(text, index)
            if not index then break end
        else
            if nested and (byte == 91 or byte == 123) then
                depth = depth + 1
            elseif nested and (byte == 93 or byte == 125) then
                depth = depth - 1
            elseif byte == separator_byte and depth == 0 then
                parts[#parts + 1] = text:sub(start, index - 1)
                start = index + 1
            end
            index = index + 1
        end
    end
    parts[#parts + 1] = text:sub(start)
    return parts
end

--- Decode one key or header segment, or nil: a bare key (ASCII letters,
-- digits, `_`, `-`) or exactly one complete string, whose value is the text
-- between its quotes, verbatim (escapes are not decoded).
local function toml_key_segment(raw)
    local segment = trim_spaces_and_tabs(raw)
    if segment == "" then return nil end
    if is_quote(segment:byte(1)) then
        if toml_string_end(segment, 1) ~= #segment + 1 then return nil end
        return segment:sub(2, -2)
    end
    -- [%w_-] would follow the C locale; spell out the ASCII alphabet.
    if segment:find("^[A-Za-z0-9_%-]+$") then return segment end
    return nil
end

--- The content of a value that is exactly one non-empty string: `"1"` is
-- one; `"a" "b"`, `1`, `""` and `["x"]` are not.
local function toml_single_string(raw)
    local value = trim_spaces_and_tabs(raw)
    if value == "" or not is_quote(value:byte(1)) then return nil end
    if toml_string_end(value, 1) ~= #value + 1 then return nil end
    local content = value:sub(2, -2)
    if content == "" then return nil end
    return content
end

--- Decide how to read the lines under a header (already comment-stripped
-- and trimmed, starting with `[` and ending with `]`). Returns kind and, for
-- a sub-table, the dependency key N. The header is split into dotted-key
-- segments at every `.` OUTSIDE a string and each segment is decoded, so a
-- cfg header is one segment however much punctuation it quotes:
--
--   [target.'cfg(any(target_os = "linux", windows))'.dependencies]
--    target  C (one quoted segment)                   D
--
--   [target."thumbv8m.main-none-eabihf".dependencies.pi]  -> sub-table pi
--   [target.x86_64-unknown-linux-gnu.build-dependencies]  -> bare triple
--   ["dependencies"]                                      -> [dependencies]
--
-- Exactly [D], [D.N], [target.C.D] and [target.C.D.N] are read; arrays of
-- tables, malformed headers and every other segment list are ignored.
local function classify_cargo_link_header(line)
    if line:sub(1, 2) == "[[" then
        return "ignored" -- array of tables, e.g. [[bin]]
    end
    local segments = {}
    for _, raw in ipairs(split_toml_outside_strings(line:sub(2, -2), ".", false)) do
        local segment = toml_key_segment(raw)
        if not segment then return "ignored" end
        segments[#segments + 1] = segment
    end
    if #segments >= 3 and segments[1] == "target" then
        -- Drop `target` and the cfg, whatever it is.
        table.remove(segments, 1)
        table.remove(segments, 1)
    end
    if not CARGO_LINK_TABLE_NAMES[segments[1]] then return "ignored" end
    if #segments == 1 then return "inline" end
    if #segments == 2 then return "sub", segments[2] end
    return "ignored"
end

--- The crate a complete inline entry `key = value` names, or nil. Only
-- path dependencies can point at a repository package; the name is the
-- `package` rename if there is one, and the entry key otherwise:
--
--   logic-gates = { path = "../logic-gates" }                    -> logic-gates
--   gates = { package = "logic-gates", path = "../logic-gates" } -> logic-gates
--   serde = "1"                                -> nil (not a table)
--   x = { version = "1", features = ["path"] } -> nil (no path field)
--   foo.path = "../foo"                        -> nil (dotted key)
--
-- Fields are split at commas outside strings AND outside nested `[...]` or
-- `{...}`, so a `features` array cannot leak a fake field and a nested
-- table's `path` is not the entry's. A field key is decoded like an entry
-- key, so `"path"` and `'path'` are the key `path` (TOML, and so Cargo, read
-- them that way); the decoded key must then be exactly `path` or `package`
-- (Cargo ignores `Package`; escapes are not decoded). The first usable field
-- of each wins:
--
--   a = { "path" = "../a" }                 -> a
--   x = { 'package' = "a", path = "../a" }  -> a
--   a = { "Path" = "../a" }                 -> nil (keys are case-sensitive)
--   a = { path.x = "../a" }                 -> nil (dotted field key)
local function cargo_path_dependency_name(key, raw_value)
    local crate_name = toml_key_segment(key)
    if not crate_name then return nil end
    local value = trim_spaces_and_tabs(raw_value)
    if #value < 2 or value:sub(1, 1) ~= "{" or value:sub(-1) ~= "}" then
        return nil
    end
    local path, package_name
    for _, field in ipairs(split_toml_outside_strings(value:sub(2, -2), ",", true)) do
        local equals = index_toml_outside_strings(field, "=")
        if equals then
            local field_value = toml_single_string(field:sub(equals + 1))
            if field_value then
                -- A dotted or malformed field key is no key at all (nil).
                local field_key = toml_key_segment(field:sub(1, equals - 1))
                if field_key == "path" and not path then
                    path = field_value
                elseif field_key == "package" and not package_name then
                    package_name = field_value
                end
            end
        end
    end
    if not path then return nil end
    return ascii_lower(package_name or crate_name)
end

--- Return the lower-case crate name of every path dependency in a
-- manifest's link tables, in manifest order with duplicates kept (the
-- caller collapses them).
--
-- One pass over the lines with a three-state machine:
--
--   "ignored"  not a link table; its lines are skipped
--   "inline"   [D] or [target.C.D]: one entry per line
--   "sub"      [D.N] or [target.C.D.N]: the path/package keys of entry N
--
-- Every header line -- even a malformed one, or an array of tables -- ends
-- the current table, and so does the end of the file; a sub-table's entry
-- is complete only then, because its keys may come in either order.
--
-- One inline entry may cover several lines. TOML 1.0 lets an ARRAY inside
-- an inline table span lines, and the repository writes exactly that:
--
--   coding_adventures_vault_pm_cli = { path = "../vault-pm-cli", features = [
--       "crash-injection",
--   ] }
--
-- So while the value still has an unclosed `{` or `[`, the next lines are
-- joined onto it with one space (and are not looked at as headers or
-- entries). If the file ends first, the entry contributes nothing. The
-- brackets are counted over the joined value as one text -- a string left
-- open at the end of one line continues after the joining space -- but the
-- joined value is never rescanned: toml_bracket_scan carries its state from
-- line to line and the pieces are joined once (table.concat), so the join
-- is linear in the entry's length.
--
-- @param text string The Cargo.toml contents.
-- @return table List of crate names.
function Resolver.cargo_link_dependency_names(text)
    -- Step 1: lines. Ignore a leading UTF-8 BOM, split at LF only (keeping
    -- empty lines), and drop one trailing CR from each line.
    if text:sub(1, 3) == "\239\187\191" then
        text = text:sub(4)
    end
    local lines = {}
    local start = 1
    while true do
        local newline = text:find("\n", start, true)
        if not newline then
            lines[#lines + 1] = text:sub(start)
            break
        end
        lines[#lines + 1] = text:sub(start, newline - 1)
        start = newline + 1
    end
    local function clean(raw)
        if raw:sub(-1) == "\r" then raw = raw:sub(1, -2) end
        return trim_spaces_and_tabs(strip_toml_line_comment(raw))
    end

    local names = {}
    local kind, sub_key, sub_path, sub_package = "ignored", nil, nil, nil
    local function finish_table()
        if kind == "sub" and sub_path then
            names[#names + 1] = ascii_lower(sub_package or sub_key)
        end
    end

    local index = 1
    while index <= #lines do
        local line = clean(lines[index])
        index = index + 1

        if line:sub(1, 1) == "[" and line:sub(-1) == "]" then
            -- Step 2: headers. Every header ends the current table.
            finish_table()
            kind, sub_key = classify_cargo_link_header(line)
            sub_path, sub_package = nil, nil
        else
            local equals = index_toml_outside_strings(line, "=")
            if equals and kind == "inline" then
                -- Step 3: inline entries, continued while a bracket is open.
                -- The pieces are collected and joined once, and the scanner
                -- state carries from piece to piece, so the join is linear.
                local value = trim_spaces_and_tabs(line:sub(equals + 1))
                if value:sub(1, 1) == "{" then
                    local state = {depth = 0, quote = nil, escape = false}
                    toml_bracket_scan(state, value)
                    if state.depth > 0 then
                        local pieces = {value}
                        local closed = false
                        while not closed and index <= #lines do
                            local piece = clean(lines[index])
                            index = index + 1
                            toml_bracket_scan(state, " ")
                            toml_bracket_scan(state, piece)
                            pieces[#pieces + 1] = piece
                            closed = state.depth <= 0
                        end
                        -- If the file ended inside the value, there is no entry.
                        value = closed and table.concat(pieces, " ") or nil
                    end
                end
                local crate_name = value and cargo_path_dependency_name(line:sub(1, equals - 1), value)
                if crate_name then names[#names + 1] = crate_name end
            elseif equals and kind == "sub" then
                -- Step 4: only the keys `path` and `package` matter, bare or
                -- quoted (`"path" = ...` is the same key); first wins.
                -- `version`, `features`, `optional` ... are ignored, and so
                -- is `Path`: TOML keys are case-sensitive.
                local field_value = toml_single_string(line:sub(equals + 1))
                if field_value then
                    local field_key = toml_key_segment(line:sub(1, equals - 1))
                    if field_key == "path" and not sub_path then
                        sub_path = field_value
                    elseif field_key == "package" and not sub_package then
                        sub_package = field_value
                    end
                end
            end
        end
    end
    finish_table()
    return names
end

--- Extract internal dependencies from a Rust Cargo.toml.
--
-- Step 5 of the contract: each candidate name from every link table is
-- looked up among the known Cargo names (directory name and declared
-- [package] name), and duplicates -- the same crate in [dependencies] and
-- [build-dependencies], say -- collapse to one dependency. A `package`
-- rename is authoritative: the entry key of a renamed entry is not looked up.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from crate name to package name.
-- @return table List of internal dependency package names.
local function parse_rust_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/Cargo.toml")
    if not text then return {} end

    local deps, seen = {}, {}
    for _, crate_name in ipairs(Resolver.cargo_link_dependency_names(text)) do
        local dep = known_names[crate_name]
        if dep and not seen[dep] then
            seen[dep] = true
            deps[#deps + 1] = dep
        end
    end
    return deps
end

local function parse_swift_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/Package.swift")
    if not text then return {} end

    local deps = {}
    for dep_path in text:gmatch('%.package%s*%(%s*path%s*:%s*"([^"]+)"') do
        local cleaned = normalize_path(dep_path)
        if not cleaned:match("^[A-Za-z]:/") and cleaned:sub(1, 1) ~= "/" then
            local dep_dir = cleaned:match("([^/]+)$")
            if dep_dir and dep_dir ~= "." and dep_dir ~= ".." then
                dep_dir = dep_dir:lower()
                if known_names[dep_dir] then
                    deps[#deps + 1] = known_names[dep_dir]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Elixir dependency parsing
-- =========================================================================

--- Extract internal dependencies from an Elixir mix.exs.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from app name to package name.
-- @return table List of internal dependency package names.
local function parse_elixir_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/mix.exs")
    if not text then return {} end

    local deps = {}
    for app_name in text:gmatch("{:(coding_adventures_%w+)") do
        local name = app_name:lower()
        if known_names[name] then
            deps[#deps + 1] = known_names[name]
        end
    end

    return deps
end

-- =========================================================================
-- Lua dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Lua .rockspec file.
--
-- LuaRocks rockspec files declare dependencies in a Lua table:
--
--     dependencies = {
--         "lua >= 5.4",
--         "coding-adventures-logic-gates >= 0.1.0",
--     }
--
-- We scan for quoted strings inside the dependencies block that match
-- known internal packages, stripping version specifiers.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from rockspec name to package name.
-- @return table List of internal dependency package names.
local function parse_lua_deps(pkg, known_names)
    local rockspec_path = find_file_by_extension(pkg.path, "rockspec")
    if not rockspec_path then return {} end

    local text = read_lua_metadata(rockspec_path, pkg)
    if not text then return {} end

    local deps = {}
    local in_deps = false

    for line in text:gmatch("[^\n]+") do
        local stripped = line:match("^%s*(.-)%s*$")

        if not in_deps then
            if stripped:match("dependencies") and stripped:match("=") and stripped:match("{") then
                in_deps = true
                if stripped:match("}") then
                    -- Single-line block.
                    for dep_str in stripped:gmatch('"([^"]+)"') do
                        local name = dep_str:match("^([%w%-]+)"):lower()
                        if known_names[name] then
                            deps[#deps + 1] = known_names[name]
                        end
                    end
                    break
                end
            end
        else
            if stripped:match("}") then
                for dep_str in stripped:gmatch('"([^"]+)"') do
                    local name = dep_str:match("^([%w%-]+)"):lower()
                    if known_names[name] then
                        deps[#deps + 1] = known_names[name]
                    end
                end
                break
            end
            for dep_str in stripped:gmatch('"([^"]+)"') do
                local name = dep_str:match("^([%w%-]+)"):lower()
                if known_names[name] then
                    deps[#deps + 1] = known_names[name]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Rosetta Stone: ecosystem name → internal package name
-- =========================================================================
-- Perl dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Perl cpanfile.
--
-- A cpanfile declares dependencies with one `requires` per line:
--
--     requires 'coding-adventures-logic-gates';
--     requires 'coding-adventures-bitset', '>= 0.01';
--
-- We scan for lines matching `requires 'coding-adventures-...'` and map
-- them to internal package names. External deps are silently skipped.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from CPAN dist name to package name.
-- @return table List of internal dependency package names.
local function parse_perl_deps(pkg, known_names)
    local text = read_file(pkg.path .. "/cpanfile")
    if not text then return {} end

    local deps = {}

    for line in text:gmatch("[^\n]+") do
        local trimmed = line:match("^%s*(.-)%s*$")

        -- Skip blank lines and comments.
        if trimmed ~= "" and not trimmed:match("^#") then
            -- Match: requires 'coding-adventures-foo' or requires "coding-adventures-foo"
            local dep_kebab = trimmed:match("requires%s+['\"]coding%-adventures%-([^'\"]+)['\"]")
            if dep_kebab then
                local dep_name = "coding-adventures-" .. dep_kebab:lower()
                if known_names[dep_name] then
                    deps[#deps + 1] = known_names[dep_name]
                end
            end
        end
    end

    return deps
end

-- =========================================================================
-- Haskell dependency parsing
-- =========================================================================

--- Extract internal dependencies from a Haskell .cabal file.
--
-- @param pkg table The package record.
-- @param known_names table Mapping from cabal name to package name.
-- @return table List of internal dependency package names.
local function parse_haskell_deps(pkg, known_names)
    local cabal_path = find_file_by_extension(pkg.path, "cabal")
    if not cabal_path then return {} end

    local text = read_file(cabal_path)
    if not text then return {} end

    local deps = {}

    for dep_name in text:gmatch("(coding%-adventures%-[%a%d%-]+)") do
        local name = dep_name:lower()
        if known_names[name] and known_names[name] ~= pkg.name then
            deps[#deps + 1] = known_names[name]
        end
    end

    return deps
end

local function parse_dotnet_deps(pkg, known_names)
    local project_files = find_files_by_extensions(pkg.path, {"csproj", "fsproj"})
    if #project_files == 0 then return {} end

    local deps = {}
    for _, project_file in ipairs(project_files) do
        local text = read_file(project_file)
        if text then
            for include_path in text:gmatch('<ProjectReference%s+Include%s*=%s*"([^"]+)"') do
                local normalized = normalize_path(pkg.path .. "/" .. include_path)
                if known_names[normalized] then
                    deps[#deps + 1] = known_names[normalized]
                end
            end
        end
    end

    return deps
end

-- =========================================================================

--- Build a mapping from ecosystem-specific dependency names to internal
-- package names.
--
-- This is the "Rosetta Stone" of the build system. Each language ecosystem
-- uses its own naming convention:
--
--   Python:     "coding-adventures-logic-gates"  → "python/logic-gates"
--   Ruby:       "coding_adventures_logic_gates"  → "ruby/logic_gates"
--   Go:         full module path                 → "go/module-name"
--   TypeScript: "@coding-adventures/logic-gates" → "typescript/logic-gates"
--   Rust:       "logic-gates"                    → "rust/logic-gates"
--   Elixir:     "coding_adventures_logic_gates"  → "elixir/logic_gates"
--   Lua:        "coding-adventures-logic-gates"  → "lua/logic_gates"
--
-- @param packages table List of package records.
-- @return table Mapping from ecosystem name to package name.
function Resolver.build_known_names(packages, language)
    local known = {}
    local scope = language or ""

    for _, pkg in ipairs(packages) do
        if scope == "" or in_dependency_scope(pkg.language, scope) then
            local basename = pkg.path:gsub("\\", "/"):match("([^/]+)$"):lower()

            if pkg.language == "python" then
                set_known(known, "coding-adventures-" .. basename, pkg)

            elseif pkg.language == "ruby" then
                set_known(known, "coding_adventures_" .. basename, pkg)

            elseif pkg.language == "go" then
                local text = read_file(pkg.path .. "/go.mod")
                if text then
                    for line in text:gmatch("[^\n]+") do
                        if line:match("^module ") then
                            local module_path = line:gsub("^module%s+", ""):match("^%s*(.-)%s*$"):lower()
                            known[module_path] = pkg.name
                            break
                        end
                    end
                end

            elseif pkg.language == "typescript" then
                set_known(known, "@coding-adventures/" .. basename, pkg)
                set_known(known, basename, pkg)

            elseif pkg.language == "rust" or pkg.language == "wasm" then
                set_known(known, basename, pkg)
                local cargo_name = read_cargo_package_name(pkg)
                if cargo_name then
                    set_known(known, cargo_name, pkg)
                end

            elseif pkg.language == "elixir" then
                set_known(known, "coding_adventures_" .. basename:gsub("%-", "_"), pkg)
                set_known(known, basename:gsub("%-", "_"), pkg)

            elseif pkg.language == "lua" then
                set_known(known, "coding-adventures-" .. basename:gsub("_", "-"), pkg)

            elseif pkg.language == "perl" then
                set_known(known, "coding-adventures-" .. basename, pkg)

            elseif pkg.language == "swift" then
                set_known(known, basename, pkg)

            elseif pkg.language == "haskell" then
                set_known(known, "coding-adventures-" .. basename:gsub("_", "-"), pkg)

            elseif pkg.language == "csharp" or pkg.language == "fsharp" or pkg.language == "dotnet" then
                for _, project_file in ipairs(find_files_by_extensions(pkg.path, {"csproj", "fsproj"})) do
                    known[normalize_path(project_file)] = pkg.name
                end
            end
        end
    end

    return known
end

-- =========================================================================
-- Main dependency resolution
-- =========================================================================

--- Parse package metadata to discover dependencies and build a graph.
--
-- The graph contains all discovered packages as nodes. Edges represent
-- build ordering: an edge from A to B means "A must be built before B"
-- (because B depends on A).
--
-- @param packages table List of package records from discovery.
-- @return DirectedGraph The dependency graph.
function Resolver.resolve_dependencies(packages)
    local graph = DirectedGraph.new()

    -- Add all packages as nodes first.
    for _, pkg in ipairs(packages) do
        graph:add_node(pkg.name)
    end

    local known_names_by_scope = {}
    for _, pkg in ipairs(packages) do
        local scope = dependency_scope(pkg.language)
        if not known_names_by_scope[scope] then
            known_names_by_scope[scope] = Resolver.build_known_names(packages, scope)
        end
    end

    -- Dispatch table for language-specific parsers.
    local parsers = {
        python     = parse_python_deps,
        ruby       = parse_ruby_deps,
        go         = parse_go_deps,
        typescript = parse_typescript_deps,
        rust       = parse_rust_deps,
        wasm       = parse_rust_deps,
        elixir     = parse_elixir_deps,
        lua        = parse_lua_deps,
        perl       = parse_perl_deps,
        swift      = parse_swift_deps,
        haskell    = parse_haskell_deps,
        csharp     = parse_dotnet_deps,
        fsharp     = parse_dotnet_deps,
        dotnet     = parse_dotnet_deps,
    }

    -- Parse dependencies for each package and add edges.
    for _, pkg in ipairs(packages) do
        local parser = parsers[pkg.language]
        if parser then
            local known_names = known_names_by_scope[dependency_scope(pkg.language)] or {}
            local deps = parser(pkg, known_names)
            -- Step 5 of the Rust contract: a Cargo candidate resolving to
            -- the crate itself (by directory alias or [package] name) is
            -- rejected, neither dropped nor drawn.
            if parser == parse_rust_deps then
                for _, dep_name in ipairs(deps) do
                    if dep_name == pkg.name then
                        error(DependencySelfEdgeError.new(
                            pkg, repository_relative_manifest(pkg.path .. "/Cargo.toml")), 0)
                    end
                end
            end
            for _, dep_name in ipairs(deps) do
                -- Edge direction: dep → pkg means "dep must be built before pkg".
                graph:add_edge(dep_name, pkg.name)
            end
        end
    end

    return graph
end

return Resolver
