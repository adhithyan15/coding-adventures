-- Tests for main.lua: the entry-point helpers directly, plus end-to-end CLI
-- invocations spawned as a real subprocess (mirrors how a user actually
-- runs this program, and catches wiring bugs unit tests on cowsay.lua alone
-- cannot -- argv parsing, exit codes, stdout/stderr routing).
--
-- Loading main.lua as a module (require("main")) does NOT call main() --
-- see main.lua's entry-point guard, same pattern as
-- code/programs/lua/parrot/main.lua.

local sep = package.config:sub(1, 1)

package.path = ".." .. sep .. "?.lua;" .. package.path

local Main = require("main")

describe("main.lua helpers", function()
    it("this_dir resolves to a directory (not empty)", function()
        assert.is_true(#Main.this_dir() > 0)
    end)

    it("stdin_is_tty is a callable predicate that never errors", function()
        assert.has_no.errors(function()
            Main.stdin_is_tty()
        end)
    end)
end)

-- ============================================================================
-- End-to-end CLI invocations
-- ============================================================================
--
-- Spawns `lua main.lua <args>` as a real subprocess via io.popen, from the
-- cowsay program's own directory (matching how a developer would actually
-- invoke it). This is the only way to exercise argv parsing, process exit
-- codes, and the help/version/error/list dispatch branches end-to-end.

local LUA = "lua"

-- Tests run with cwd = tests/ (this package's BUILD does `cd tests &&
-- busted ...`, same convention as code/programs/lua/parrot), so main.lua is
-- one level up.
local function run_cowsay(args, stdin_content)
    local cmd = LUA .. " ../main.lua"
    for _, a in ipairs(args) do
        -- Tests only pass fixed, non-attacker-controlled argument strings
        -- (see each test below), so simple quoting is sufficient here.
        cmd = cmd .. " '" .. a:gsub("'", "'\\''") .. "'"
    end

    -- Route stdin through a temp file rather than a heredoc: a heredoc's
    -- closing delimiter must be alone on its own line, which doesn't
    -- compose cleanly with appending " 2>&1; echo EXIT:$?" afterwards on
    -- the same logical command string.
    local stdin_path
    if stdin_content ~= nil then
        stdin_path = os.tmpname()
        local f = io.open(stdin_path, "w")
        f:write(stdin_content)
        f:close()
        cmd = cmd .. ' < "' .. stdin_path .. '"'
    else
        cmd = cmd .. " </dev/null"
    end

    local handle = io.popen(cmd .. " 2>&1; echo EXIT:$?")
    local output = handle:read("a")
    handle:close()

    if stdin_path then
        os.remove(stdin_path)
    end

    local exit_code = tonumber(output:match("EXIT:(%d+)%s*$"))
    local body = output:gsub("EXIT:%d+%s*$", "")
    -- Strip exactly one trailing newline added by `echo`/print, so callers
    -- can compare against the program's own output shape.
    body = body:gsub("\n$", "")
    return body, exit_code
end

describe("cowsay CLI end-to-end", function()
    it("renders a speech bubble for a simple message", function()
        local output, code = run_cowsay({ "Hello, World!" })
        assert.equal(0, code)
        assert.equal(
            table.concat({
                " _______________",
                "< Hello, World! >",
                " ---------------",
                "        \\   ^__^",
                "         \\  (oo)\\_______",
                "            (__)\\       )\\/\\",
                "                ||----w |",
                "                ||     ||",
            }, "\n"),
            output
        )
    end)

    it("renders a thought bubble with the tux cow for --think -f tux", function()
        local output, code = run_cowsay({ "--think", "-f", "tux", "beep boop" })
        assert.equal(0, code)
        assert.is_true(output:find("( beep boop )", 1, true) ~= nil)
    end)

    it("applies mode shortcuts (-b borg)", function()
        local output, code = run_cowsay({ "-b", "resistance is futile" })
        assert.equal(0, code)
        assert.is_true(output:find("(==)", 1, true) ~= nil)
    end)

    it("lists available cow files sorted ordinally", function()
        local output, code = run_cowsay({ "-l" })
        assert.equal(0, code)
        assert.equal("default\ndragon\ntux", output)
    end)

    it("prints version for --version", function()
        local output, code = run_cowsay({ "--version" })
        assert.equal(0, code)
        assert.equal("1.0.0", output)
    end)

    it("prints usage for --help", function()
        local output, code = run_cowsay({ "--help" })
        assert.equal(0, code)
        assert.is_true(output:find("USAGE", 1, true) ~= nil)
    end)

    it("reads the message from stdin when no argument is given", function()
        local output, code = run_cowsay({}, "piped message")
        assert.equal(0, code)
        assert.is_true(output:find("< piped message >", 1, true) ~= nil)
    end)

    it("exits 0 with no output for an empty stdin and no message", function()
        local output, code = run_cowsay({})
        assert.equal(0, code)
        assert.equal("", output)
    end)

    it("falls back to default.cow for a path-traversal cowfile attempt", function()
        local traversal, code1 = run_cowsay({ "-f", "../../../../etc/passwd", "hi" })
        local baseline, code2 = run_cowsay({ "-f", "default", "hi" })
        assert.equal(0, code1)
        assert.equal(0, code2)
        assert.equal(baseline, traversal)
    end)

    it("falls back to default.cow for an absolute-path cowfile attempt", function()
        local rooted, code1 = run_cowsay({ "-f", "/etc/passwd", "hi" })
        local baseline, code2 = run_cowsay({ "-f", "default", "hi" })
        assert.equal(0, code1)
        assert.equal(0, code2)
        assert.equal(baseline, rooted)
    end)

    it("does not wrap when -n/--nowrap is given", function()
        local long_message = "this is a very long line that would normally wrap but should not because of nowrap"
        local output = run_cowsay({ "-n", long_message })
        -- The unwrapped message appears as a single bubble line (padded,
        -- but not broken across "< ... >" boundaries).
        assert.is_true(output:find(long_message, 1, true) ~= nil)
    end)

    it("reports a clean, non-traceback error for an unknown flag", function()
        local output, code = run_cowsay({ "--this-flag-does-not-exist" })
        assert.equal(1, code)
        assert.is_false(output:find("stack traceback", 1, true) ~= nil)
    end)
end)
