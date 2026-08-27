-- cowsay — routed through paint-vm-ascii (Lua port)
-- ================================================================
--
-- Ninth and last language in the cowsay-through-paint-vm-ascii rollout (see
-- code/specs/cowsay-paintvm-pipeline.md), after csharp, fsharp, perl,
-- haskell, java, kotlin, dart, and swift. Everything up through composing
-- the bubble+cow text block is ordinary string formatting, ported from the
-- reference implementation at code/programs/go/cowsay/main.go (word
-- wrapping, bubble borders, eyes/tongue mode substitution, .cow template
-- loading) -- cross-checked line-for-line against
-- code/programs/perl/cowsay/lib/CodingAdventures/Cowsay.pm, since this repo
-- requires this port's output be byte-identical to the merged Perl port for
-- the same flags and message.
--
-- The one thing that's different from the Go reference: instead of printing
-- the composed text directly, build_scene converts it into a PaintScene of
-- glyph_run instructions (one glyph placement per non-space character,
-- positioned on an 8x16 character grid), and
-- coding_adventures.paint_vm_ascii.render turns that scene back into the
-- terminal string this module returns.
--
-- This module is pure logic with no CLI parsing and no `os.exit` calls, so
-- every function here is directly unit-testable without spawning a process
-- or driving a real cli_builder Parser -- the same split
-- code/programs/perl/cowsay uses between cowsay.pl (thin CLI glue) and
-- Cowsay.pm (this module's counterpart). code/programs/lua/cowsay/main.lua
-- is the thin CLI glue on top of this module.

local paint = require("coding_adventures.paint_instructions")
local paint_vm_ascii = require("coding_adventures.paint_vm_ascii")

local M = {}

M.VERSION = "0.1.0"

-- paint-vm-ascii's documented default scale factors (P2D02-paint-vm-ascii.md).
local SCALE_X = 8
local SCALE_Y = 16

-- ============================================================================
-- UTF-8 helpers
-- ================================================================
--
-- Perl's Cowsay.pm (`use utf8;`) measures string length, wraps, and pads by
-- CHARACTER count, not byte count -- and this port must match Perl
-- byte-for-byte for the "Output must be verified byte-identical against the
-- merged Perl port" requirement. Lua's `#s` and `string.sub` are always
-- byte-based, so every place Perl's Cowsay.pm calls `length()` or
-- fixed-width `sprintf('%-*s', ...)`, this module goes through
-- `utf8.len`/a small codepoint-aware substring helper instead. (The Go
-- reference implementation actually measures byte length via Go's built-in
-- `len()` on non-ASCII input -- a latent difference between the Go and Perl
-- ports that predates this one. Since the task explicitly calls out
-- byte-identical parity with Perl specifically, this module follows Perl's
-- character-based convention, not Go's byte-based one.)

-- utf8_length(s) -- character count, falling back to byte count for input
-- that isn't valid UTF-8 (utf8.len returns fail rather than raising, so this
-- never errors).
local function utf8_length(s)
    return utf8.len(s) or #s
end

-- utf8_sub(s, first, last) -- a UTF-8-aware, 1-indexed, inclusive substring
-- (mirrors Lua's string.sub indexing convention). Falls back to a plain
-- byte-based string.sub if `s` isn't valid UTF-8 or utf8.offset otherwise
-- raises -- a malformed CLI flag value must never crash the whole program.
local function utf8_sub(s, first, last)
    local ok, result = pcall(function()
        local start_byte = utf8.offset(s, first)
        if start_byte == nil then
            return ""
        end
        local end_byte
        if last == nil then
            end_byte = #s
        else
            local next_start = utf8.offset(s, last + 1)
            end_byte = next_start and (next_start - 1) or #s
        end
        return s:sub(start_byte, end_byte)
    end)
    if ok then
        return result
    end
    return s:sub(first, last or #s)
end

-- pad_right(s, width) -- right-pads `s` with spaces up to `width`
-- characters (a no-op if `s` is already that long or longer), mirroring
-- Perl's `sprintf('%-*s', $width, $s)` under `use utf8`.
local function pad_right(s, width)
    local n = utf8_length(s)
    if n >= width then
        return s
    end
    return s .. string.rep(" ", width - n)
end

-- split_lines_keep_trailing(s) -- splits `s` on "\n", keeping trailing
-- empty fields (so "a\nb\n" becomes {"a", "b", ""}, not {"a", "b"}).
-- Mirrors Perl's `split(/\n/, $s, -1)` (limit -1 means "don't strip
-- trailing empty fields", unlike Lua's usual gmatch-based splitting idioms
-- which drop them).
local function split_lines_keep_trailing(s)
    local lines = {}
    local start = 1
    while true do
        local newline_pos = s:find("\n", start, true)
        if newline_pos == nil then
            lines[#lines + 1] = s:sub(start)
            break
        end
        lines[#lines + 1] = s:sub(start, newline_pos - 1)
        start = newline_pos + 1
    end
    return lines
end

-- codepoints_of(line) -- the list of Unicode codepoints in `line`. Falls
-- back to raw byte values for a line that isn't valid UTF-8, so a single
-- malformed byte from stdin can't crash the whole render (paint_vm_ascii's
-- own glyph safety filter will turn any resulting out-of-range/control
-- value into "?" downstream).
local function codepoints_of(line)
    local points = {}
    local ok = pcall(function()
        for _, code_point in utf8.codes(line) do
            points[#points + 1] = code_point
        end
    end)
    if ok then
        return points
    end

    points = {}
    for i = 1, #line do
        points[#points + 1] = line:byte(i)
    end
    return points
end

-- ============================================================================
-- Rendering core (ported from code/programs/go/cowsay/main.go, cross-checked
-- against code/programs/perl/cowsay/lib/CodingAdventures/Cowsay.pm)
-- ============================================================================

-- split_on_space(text) -- splits on runs of literal ASCII space characters
-- only (not general whitespace), matching Perl's
-- `grep { length($_) > 0 } split(/ /, $text)` exactly -- a lone tab
-- character, for instance, is NOT a word boundary here.
local function split_on_space(text)
    local words = {}
    for word in text:gmatch("[^ ]+") do
        words[#words + 1] = word
    end
    return words
end

-- wrap_text(text, width) -- splits `text` into lines no longer than `width`
-- characters, breaking on word boundaries. A single word longer than
-- `width` is kept whole (never split mid-word).
function M.wrap_text(text, width)
    if utf8_length(text) <= width then
        return { text }
    end

    local words = split_on_space(text)
    if #words == 0 then
        return { "" }
    end

    local lines = {}
    local current = ""
    for _, word in ipairs(words) do
        local current_len = utf8_length(current)
        local word_len = utf8_length(word)
        if current_len + word_len + 1 <= width then
            if current == "" then
                current = word
            else
                current = current .. " " .. word
            end
        else
            if #current > 0 then
                lines[#lines + 1] = current
            end
            current = word
        end
    end
    if #current > 0 then
        lines[#lines + 1] = current
    end

    return lines
end

-- format_bubble(lines, is_think) -- draws the speech/thought bubble around
-- the given lines. A single line gets "< ... >" (or "( ... )" for a thought
-- bubble); multiple lines get "/ ... \", "| ... |", "\ ... /" (or "( ... )"
-- on every line for a thought bubble).
function M.format_bubble(lines, is_think)
    if #lines == 0 then
        return ""
    end

    local max_len = 0
    for _, line in ipairs(lines) do
        local n = utf8_length(line)
        if n > max_len then
            max_len = n
        end
    end

    local border_top = " " .. string.rep("_", max_len + 2)
    local border_bottom = " " .. string.rep("-", max_len + 2)

    local body = {}
    if #lines == 1 then
        local start_ch, end_ch = "<", ">"
        if is_think then
            start_ch, end_ch = "(", ")"
        end
        body[1] = start_ch .. " " .. pad_right(lines[1], max_len) .. " " .. end_ch
    else
        local count = #lines
        for i, line in ipairs(lines) do
            local start_ch, end_ch
            if is_think then
                start_ch, end_ch = "(", ")"
            elseif i == 1 then
                start_ch, end_ch = "/", "\\"
            elseif i == count then
                start_ch, end_ch = "\\", "/"
            else
                start_ch, end_ch = "|", "|"
            end
            body[#body + 1] = start_ch .. " " .. pad_right(line, max_len) .. " " .. end_ch
        end
    end

    local result = { border_top }
    for _, line in ipairs(body) do
        result[#result + 1] = line
    end
    result[#result + 1] = border_bottom

    return table.concat(result, "\n")
end

-- normalize_two_chars(value) -- pads or truncates a mode string
-- (eyes/tongue) to exactly two characters, matching cowsay's convention
-- that eyes/tongue are always a 2-character glyph.
local function normalize_two_chars(value)
    local n = utf8_length(value)
    if n < 2 then
        return value .. string.rep(" ", 2 - n)
    elseif n > 2 then
        return utf8_sub(value, 1, 2)
    end
    return value
end

local MODE_OVERRIDES = {
    borg = { eyes = "==" },
    dead = { eyes = "XX", tongue = "U " },
    greedy = { eyes = "$$" },
    paranoid = { eyes = "@@" },
    stoned = { eyes = "xx", tongue = "U " },
    tired = { eyes = "--" },
    wired = { eyes = "OO" },
    youthful = { eyes = ".." },
}

-- The mood-shortcut flag ids, in the same fixed order every other port uses
-- (matches the "modes" mutually_exclusive_group in code/specs/cowsay.json).
M.MODE_FLAG_IDS = { "borg", "dead", "greedy", "paranoid", "stoned", "tired", "wired", "youthful" }

-- resolve_eyes_and_tongue(base_eyes, base_tongue, active_modes) -- applies
-- mode shortcuts (--borg, --dead, etc.) on top of the base eyes/tongue flag
-- values, then normalizes both to two characters. Modes are mutually
-- exclusive per cowsay.json, but this accepts any set for robustness (same
-- as every other port).
function M.resolve_eyes_and_tongue(base_eyes, base_tongue, active_modes)
    local eyes = base_eyes
    local tongue = base_tongue

    for _, mode in ipairs(active_modes) do
        local override = MODE_OVERRIDES[mode]
        if override then
            eyes = override.eyes
            if override.tongue ~= nil then
                tongue = override.tongue
            end
        end
    end

    return normalize_two_chars(eyes), normalize_two_chars(tongue)
end

-- ============================================================================
-- Cow template loading
-- ============================================================================

-- safe_cow_name(cow_name) -- returns `cow_name` unchanged if it is safe to
-- join onto a directory and use as a bare filename, or nil if it is not.
--
-- $cow_name comes from the user-supplied -f/--file flag, so it is treated
-- as untrusted. Rejecting any path separator (forward OR back slash, since
-- this program may run on Windows) is sufficient on its own to guarantee
-- containment: with no separator characters at all, the resulting
-- "<cows_dir><sep><name>.cow" path structurally cannot climb outside
-- cows_dir via "..", a rooted/absolute override (e.g. "/etc/passwd" or
-- "C:\\Windows\\..."), or a UNC path -- there is no directory component
-- left for any of those to hide in. This is a simpler mechanism than the
-- C#/F#/Perl pilots' "extract basename, verify resolved path stays within
-- root" approach, but arrives at the same guarantee: Lua's standard library
-- has no path-canonicalization function to lean on (no realpath/rel2abs
-- equivalent), so rejecting separators outright avoids needing one at all,
-- rather than attempting (and risking getting wrong) a manual
-- canonicalize-then-verify step.
local function safe_cow_name(cow_name)
    if cow_name == nil or cow_name == "" then
        return nil
    end
    if cow_name:find("/", 1, true) or cow_name:find("\\", 1, true) then
        return nil
    end
    if cow_name == "." or cow_name == ".." then
        return nil
    end
    return cow_name
end

local function read_file(path)
    local f = io.open(path, "r")
    if f == nil then
        return nil
    end
    local content = f:read("a")
    f:close()
    return content
end

-- load_cow(cow_name, cows_dir) -- loads a .cow template's body from
-- cows_dir, falling back to default.cow when the requested file doesn't
-- exist (or `cow_name` failed the safety check above). The template is a
-- heredoc-style block ($the_cow = <<EOC; ... EOC); only the body between the
-- heredoc markers is returned, matching every other port's convention.
function M.load_cow(cow_name, cows_dir)
    local sep = package.config:sub(1, 1)
    local safe_name = safe_cow_name(cow_name)

    local content
    if safe_name then
        content = read_file(cows_dir .. sep .. safe_name .. ".cow")
    end

    if content == nil then
        local default_path = cows_dir .. sep .. "default.cow"
        content = read_file(default_path)
        assert(content, "cowsay: cannot read " .. default_path)
    end

    local body = content:match("<<EOC;\n(.-)EOC")
    if body then
        return body
    end
    return content
end

-- find_repo_root(start_dir) -- walks up from start_dir looking for
-- CLAUDE.md, the repo-root sentinel file. CLAUDE.md (not
-- code/specs/cowsay.json itself) is used deliberately -- it's a more robust
-- marker than reaching for the very file being located, and this exact
-- pattern is called out in code/programs/perl/cowsay's find_repo_root as a
-- lesson from a prior, reverted Lua cowsay port's CI pathing problems (PR
-- #1535). Unlike Perl's version (which walks up from the process's current
-- working directory via Cwd::getcwd), this walks up from the *script's own
-- directory* -- Lua's standard library has no getcwd() equivalent, and
-- anchoring to the script's location (main.lua's `this_dir()`, the same
-- technique code/programs/lua/parrot/main.lua already uses to locate its
-- sibling packages) is actually more robust than a cwd-based walk: it works
-- identically no matter what directory the caller happens to invoke `lua
-- main.lua` from.
function M.find_repo_root(start_dir)
    local sep = package.config:sub(1, 1)
    local dir = start_dir
    for _ = 1, 24 do
        local marker = io.open(dir .. sep .. "CLAUDE.md", "r")
        if marker then
            marker:close()
            return dir
        end
        dir = dir .. sep .. ".."
    end
    return start_dir
end

-- ============================================================================
-- Listing .cow files
-- ============================================================================
--
-- Lua's standard library has no directory-listing function. This mirrors
-- code/programs/lua/build-tool/lib/build_tool/discovery.lua's
-- `list_subdirs`: prefer LuaFileSystem (`lfs`) when it happens to be
-- installed, and fall back to shelling out to `ls`/`dir` when it isn't --
-- this repo's Lua CI toolchain does not install `lfs` (see
-- .github/workflows/ci.yml's Lua setup step), so the fallback path is the
-- one that actually runs in CI today, not a hypothetical. `cows_dir` is
-- never user-controlled (it is derived entirely from find_repo_root's
-- result plus a fixed relative path), so embedding it in the shell command
-- string here carries none of the injection risk load_cow's user-supplied
-- cow_name would.
local lfs_ok, lfs = pcall(require, "lfs")

function M.list_cow_files(cows_dir)
    local names = {}

    if lfs_ok then
        for name in lfs.dir(cows_dir) do
            local cow_name = name:match("^(.*)%.cow$")
            if cow_name then
                names[#names + 1] = cow_name
            end
        end
    else
        local sep = package.config:sub(1, 1)
        local cmd
        if sep == "\\" then
            cmd = 'dir /b "' .. cows_dir .. '" 2>nul'
        else
            cmd = 'ls -1 "' .. cows_dir .. '" 2>/dev/null'
        end
        local handle = io.popen(cmd)
        if handle then
            for raw_name in handle:lines() do
                local name = raw_name:match("^%s*(.-)%s*$")
                local cow_name = name:match("^(.*)%.cow$")
                if cow_name then
                    names[#names + 1] = cow_name
                end
            end
            handle:close()
        end
    end

    table.sort(names)
    return names
end

-- ============================================================================
-- Composition
-- ============================================================================

-- escape_gsub_replacement(s) -- escapes literal "%" characters so `s` is
-- safe to pass as a gsub REPLACEMENT string (Lua's gsub treats "%" as an
-- escape character in replacements, e.g. "%1" is a capture reference). Since
-- eyes/tongue/thoughts ultimately come from user-controlled CLI flags, this
-- prevents a crafted --eyes value from being mis-interpreted as a capture
-- reference instead of literal text.
local function escape_gsub_replacement(s)
    return (s:gsub("%%", "%%%%"))
end

-- compose_content(invocation, cows_dir) -- composes the full bubble+cow text
-- block for one invocation -- everything up to (but not including) the
-- paint-vm-ascii render step. `invocation` is a table with fields: message,
-- eyes, tongue, active_modes (list), nowrap, width, think, cowfile.
function M.compose_content(invocation, cows_dir)
    local eyes, tongue = M.resolve_eyes_and_tongue(invocation.eyes, invocation.tongue, invocation.active_modes)

    local raw_lines = split_lines_keep_trailing(invocation.message)
    local lines = {}
    for _, raw_line in ipairs(raw_lines) do
        if raw_line == "" then
            lines[#lines + 1] = ""
        elseif invocation.nowrap then
            lines[#lines + 1] = raw_line
        else
            for _, wrapped in ipairs(M.wrap_text(raw_line, invocation.width)) do
                lines[#lines + 1] = wrapped
            end
        end
    end

    local thoughts = invocation.think and "o" or "\\"
    local bubble = M.format_bubble(lines, invocation.think)

    local cow_template = M.load_cow(invocation.cowfile, cows_dir)

    local cow = cow_template
    cow = cow:gsub("%$eyes", escape_gsub_replacement(eyes))
    cow = cow:gsub("%$tongue", escape_gsub_replacement(tongue))
    cow = cow:gsub("%$thoughts", escape_gsub_replacement(thoughts))
    -- Final unescape: the .cow template source escapes literal backslashes
    -- as "\\\\" (two characters); collapse each such pair to one backslash.
    cow = cow:gsub("\\\\", "\\")

    return bubble .. "\n" .. cow
end

-- build_scene(text) -- converts a composed text block into a PaintScene: one
-- glyph_run instruction per line, one glyph placement per non-space
-- character. See code/specs/cowsay-paintvm-pipeline.md section 3 for the
-- full contract, including why glyph_id is a literal Unicode code point
-- here (an ASCII-backend-only relaxation of the general PaintGlyphRun
-- contract).
function M.build_scene(text)
    local normalized = (text:gsub("\r\n", "\n"))
    local lines = split_lines_keep_trailing(normalized)

    local max_width = 0
    local instructions = {}

    for row_index, line in ipairs(lines) do
        local row = row_index - 1 -- lines is 1-indexed; scene rows are 0-indexed.
        local points = codepoints_of(line)
        if #points > max_width then
            max_width = #points
        end

        local glyphs = {}
        for col, code_point in ipairs(points) do
            if code_point ~= 0x20 then -- 0x20 == ' ' -- unaddressed cells default to blank.
                glyphs[#glyphs + 1] = paint.paint_glyph_placement(code_point, (col - 1) * SCALE_X, row * SCALE_Y)
            end
        end

        if #glyphs > 0 then
            instructions[#instructions + 1] =
                paint.paint_glyph_run(glyphs, "terminal-mono", SCALE_Y, "#000000")
        end
    end

    local width = (max_width > 0 and max_width or 1) * SCALE_X
    local height = (#lines > 0 and #lines or 1) * SCALE_Y

    return paint.paint_scene(width, height, instructions, "transparent")
end

-- render(invocation, cows_dir) -- end-to-end: compose the bubble+cow text,
-- build a PaintScene from it, and render that scene through paint_vm_ascii.
function M.render(invocation, cows_dir)
    local content = M.compose_content(invocation, cows_dir)
    local scene = M.build_scene(content)
    return paint_vm_ascii.render(scene, { scale_x = SCALE_X, scale_y = SCALE_Y })
end

-- ============================================================================
-- CLI glue helpers
-- ================================================================
--
-- Kept in this module (rather than main.lua) so they're directly
-- unit-testable without spawning a process or driving a real cli_builder
-- Parser -- same reasoning as code/programs/perl/cowsay's split.
-- ============================================================================

function M.is_list_requested(flags)
    return flags.list and true or false
end

-- resolve_message_from_arguments(arguments) -- resolves the message from the
-- parsed "message" positional argument. Returns nil when no message was
-- given on argv -- the caller should fall back to stdin.
function M.resolve_message_from_arguments(arguments)
    local message_arg = arguments and arguments.message
    if type(message_arg) ~= "table" or #message_arg == 0 then
        return nil
    end
    local parts = {}
    for i, value in ipairs(message_arg) do
        parts[i] = tostring(value)
    end
    return table.concat(parts, " ")
end

-- build_invocation(message, flags) -- builds an invocation table from a
-- resolved message and the parsed flags table, applying cowsay.json's
-- documented defaults for any flag that wasn't explicitly set.
function M.build_invocation(message, flags)
    local width = 40
    if flags.width ~= nil then
        width = flags.width
        if width < 1 then
            width = 1
        end
        if width > 2147483647 then -- clamp to a 32-bit int ceiling, like every other port
            width = 2147483647
        end
        width = math.floor(width)
    end

    local active_modes = {}
    for _, id in ipairs(M.MODE_FLAG_IDS) do
        if flags[id] then
            active_modes[#active_modes + 1] = id
        end
    end

    return {
        message = message,
        eyes = flags.eyes ~= nil and flags.eyes or "oo",
        tongue = flags.tongue ~= nil and flags.tongue or "  ",
        active_modes = active_modes,
        nowrap = flags.nowrap and true or false,
        width = width,
        think = flags.think and true or false,
        cowfile = flags.cowfile ~= nil and flags.cowfile or "default",
    }
end

return M
