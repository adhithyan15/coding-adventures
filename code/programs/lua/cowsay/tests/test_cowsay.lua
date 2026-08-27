-- Tests for the cowsay.lua logic module.
--
-- Mirrors code/programs/perl/cowsay/t/cowsay.t's coverage (same function
-- names, same golden fixtures) since this port's contract is to be
-- byte-identical to the merged Perl port.

local sep = package.config:sub(1, 1)

package.path = ".." .. sep .. "?.lua;"
    .. ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
    .. sep .. "packages" .. sep .. "lua" .. sep .. "cli_builder" .. sep .. "src" .. sep .. "?.lua;"
    .. ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
    .. sep .. "packages" .. sep .. "lua" .. sep .. "cli_builder" .. sep .. "src" .. sep .. "?" .. sep .. "init.lua;"
    .. ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
    .. sep .. "packages" .. sep .. "lua" .. sep .. "paint_instructions" .. sep .. "src" .. sep .. "?.lua;"
    .. ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
    .. sep .. "packages" .. sep .. "lua" .. sep .. "paint_instructions" .. sep .. "src" .. sep .. "?" .. sep .. "init.lua;"
    .. ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
    .. sep .. "packages" .. sep .. "lua" .. sep .. "paint_vm_ascii" .. sep .. "src" .. sep .. "?.lua;"
    .. ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
    .. sep .. "packages" .. sep .. "lua" .. sep .. "paint_vm_ascii" .. sep .. "src" .. sep .. "?" .. sep .. "init.lua;"
    .. package.path

local cowsay = require("cowsay")

-- make_tempdir()/remove_tempdir(dir) -- mirrors the temp-directory pattern
-- already established in code/programs/lua/build-tool/tests/test_discovery.lua
-- (os.tmpname() + os.remove + mkdir, since Lua's stdlib has no dedicated
-- temp-directory primitive).
local function make_tempdir()
    local dir = os.tmpname()
    os.remove(dir)
    os.execute('mkdir "' .. dir .. '"')
    return dir
end

local function write_file(path, content)
    local f = io.open(path, "w")
    f:write(content)
    f:close()
end

local function remove_tempdir(dir)
    os.execute('rm -rf "' .. dir .. '" 2>/dev/null || rmdir /s /q "' .. dir .. '" 2>NUL')
end

describe("cowsay", function()
    describe("wrap_text", function()
        it("does not wrap short text", function()
            assert.same({ "hello" }, cowsay.wrap_text("hello", 40))
        end)

        it("wraps long text at word boundaries", function()
            assert.same(
                { "the quick", "brown fox", "jumps over" },
                cowsay.wrap_text("the quick brown fox jumps over", 10)
            )
        end)

        it("returns an empty line for empty text", function()
            assert.same({ "" }, cowsay.wrap_text("", 40))
        end)

        it("keeps a single word longer than the width whole", function()
            assert.same(
                { "supercalifragilisticexpialidocious" },
                cowsay.wrap_text("supercalifragilisticexpialidocious", 5)
            )
        end)

        it("returns an empty line for whitespace-only text", function()
            assert.same({ "" }, cowsay.wrap_text("     ", 3))
        end)
    end)

    describe("format_bubble", function()
        it("returns an empty string for no lines", function()
            assert.equal("", cowsay.format_bubble({}, false))
        end)

        it("formats a single-line speech bubble", function()
            assert.equal(" ____\n< hi >\n ----", cowsay.format_bubble({ "hi" }, false))
        end)

        it("formats a single-line thought bubble", function()
            assert.equal(" ____\n( hi )\n ----", cowsay.format_bubble({ "hi" }, true))
        end)

        it("formats a multi-line speech bubble with slash/pipe/backslash borders", function()
            assert.equal(
                " _______\n/ one   \\\n| two   |\n\\ three /\n -------",
                cowsay.format_bubble({ "one", "two", "three" }, false)
            )
        end)

        it("formats a multi-line thought bubble with parens on every line", function()
            assert.equal(
                " _____\n( one )\n( two )\n -----",
                cowsay.format_bubble({ "one", "two" }, true)
            )
        end)
    end)

    describe("resolve_eyes_and_tongue", function()
        it("keeps base values when no modes are active", function()
            local eyes, tongue = cowsay.resolve_eyes_and_tongue("oo", "  ", {})
            assert.equal("oo", eyes)
            assert.equal("  ", tongue)
        end)

        local expected = {
            borg = { "==", "  " },
            dead = { "XX", "U " },
            greedy = { "$$", "  " },
            paranoid = { "@@", "  " },
            stoned = { "xx", "U " },
            tired = { "--", "  " },
            wired = { "OO", "  " },
            youthful = { "..", "  " },
        }
        for mode, want in pairs(expected) do
            it("mode '" .. mode .. "' overrides eyes and sometimes tongue", function()
                local eyes, tongue = cowsay.resolve_eyes_and_tongue("oo", "  ", { mode })
                assert.equal(want[1], eyes)
                assert.equal(want[2], tongue)
            end)
        end

        it("ignores an unknown mode", function()
            local eyes, tongue = cowsay.resolve_eyes_and_tongue("oo", "  ", { "not-a-real-mode" })
            assert.equal("oo", eyes)
            assert.equal("  ", tongue)
        end)

        it("pads/truncates via normalize_two_chars", function()
            local eyes = cowsay.resolve_eyes_and_tongue("o", "", {})
            assert.equal("o ", eyes)
            local eyes2 = cowsay.resolve_eyes_and_tongue("ooo", "", {})
            assert.equal("oo", eyes2)
        end)
    end)

    describe("load_cow", function()
        local temp_dir

        before_each(function()
            temp_dir = make_tempdir()
        end)

        after_each(function()
            remove_tempdir(temp_dir)
        end)

        it("loads the body between heredoc markers", function()
            write_file(temp_dir .. sep .. "default.cow", "$the_cow = <<EOC;\n  $thoughts   ^__^\n   ($eyes)\nEOC\n")
            assert.equal("  $thoughts   ^__^\n   ($eyes)\n", cowsay.load_cow("default", temp_dir))
        end)

        it("falls back to default.cow when the named cow is missing", function()
            write_file(temp_dir .. sep .. "default.cow", "$the_cow = <<EOC;\nfallback\nEOC\n")
            assert.equal("fallback\n", cowsay.load_cow("does-not-exist", temp_dir))
        end)

        it("falls back to default.cow instead of escaping via traversal", function()
            write_file(temp_dir .. sep .. "default.cow", "$the_cow = <<EOC;\nfallback\nEOC\n")
            local malicious = {
                "../../../../../../etc/passwd",
                "..\\..\\..\\secret",
                "../outside",
                "/etc/passwd",
            }
            for _, name in ipairs(malicious) do
                assert.equal("fallback\n", cowsay.load_cow(name, temp_dir), "traversal attempt: " .. name)
            end
        end)

        it("falls back to default.cow for a rooted path override", function()
            write_file(temp_dir .. sep .. "default.cow", "$the_cow = <<EOC;\nfallback\nEOC\n")
            local outside_dir = make_tempdir()
            write_file(outside_dir .. sep .. "win.cow", "$the_cow = <<EOC;\nSECRET\nEOC\n")
            assert.equal("fallback\n", cowsay.load_cow(outside_dir .. sep .. "win", temp_dir))
            remove_tempdir(outside_dir)
        end)

        it("falls back to default.cow for nil/empty cow name", function()
            write_file(temp_dir .. sep .. "default.cow", "$the_cow = <<EOC;\nfallback\nEOC\n")
            assert.equal("fallback\n", cowsay.load_cow("", temp_dir))
        end)
    end)

    describe("find_repo_root", function()
        it("finds a directory containing CLAUDE.md", function()
            local temp_dir = make_tempdir()
            write_file(temp_dir .. sep .. "CLAUDE.md", "# marker\n")
            assert.equal(temp_dir, cowsay.find_repo_root(temp_dir))
            remove_tempdir(temp_dir)
        end)

        it("walks up through nested subdirectories to find CLAUDE.md", function()
            local temp_dir = make_tempdir()
            write_file(temp_dir .. sep .. "CLAUDE.md", "# marker\n")
            local nested = temp_dir .. sep .. "a" .. sep .. "b" .. sep .. "c"
            os.execute('mkdir -p "' .. nested .. '" 2>/dev/null || mkdir "' .. nested .. '"')
            local found = cowsay.find_repo_root(nested)
            -- The result should resolve to the same file as temp_dir/CLAUDE.md,
            -- even if the string form still carries ".." segments (this
            -- module relies on the OS to resolve those, not string
            -- normalization -- see find_repo_root's doc comment).
            local marker = io.open(found .. sep .. "CLAUDE.md", "r")
            assert.is_not_nil(marker)
            if marker then marker:close() end
            remove_tempdir(temp_dir)
        end)

        it("falls back to start_dir when no CLAUDE.md is found within the bound", function()
            -- /tmp itself (or its ancestors) should not contain a CLAUDE.md,
            -- so this should exhaust the 24-hop bound and return start_dir.
            local temp_dir = make_tempdir()
            local result = cowsay.find_repo_root(temp_dir)
            assert.equal(temp_dir, result)
            remove_tempdir(temp_dir)
        end)
    end)

    describe("list_cow_files", function()
        it("returns cow basenames sorted ordinally", function()
            local temp_dir = make_tempdir()
            for _, name in ipairs({ "tux", "default", "dragon" }) do
                write_file(temp_dir .. sep .. name .. ".cow", "")
            end
            assert.same({ "default", "dragon", "tux" }, cowsay.list_cow_files(temp_dir))
            remove_tempdir(temp_dir)
        end)

        it("ignores non-.cow files", function()
            local temp_dir = make_tempdir()
            write_file(temp_dir .. sep .. "default.cow", "")
            write_file(temp_dir .. sep .. "README.md", "")
            assert.same({ "default" }, cowsay.list_cow_files(temp_dir))
            remove_tempdir(temp_dir)
        end)
    end)

    describe("compose_content", function()
        local temp_dir

        before_each(function()
            temp_dir = make_tempdir()
            write_file(temp_dir .. sep .. "default.cow", "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n")
        end)

        after_each(function()
            remove_tempdir(temp_dir)
        end)

        local function base_invocation()
            return {
                message = "hi",
                eyes = "oo",
                tongue = "  ",
                active_modes = {},
                nowrap = false,
                width = 40,
                think = false,
                cowfile = "default",
            }
        end

        it("composes bubble and cow with substitutions", function()
            assert.equal(
                " ____\n< hi >\n ----\n\\ oo   \n",
                cowsay.compose_content(base_invocation(), temp_dir)
            )
        end)

        it("think mode uses 'o' for thoughts and a paren bubble", function()
            local invocation = base_invocation()
            invocation.think = true
            assert.equal(
                " ____\n( hi )\n ----\no oo   \n",
                cowsay.compose_content(invocation, temp_dir)
            )
        end)

        it("a mode flag overrides eyes (and tongue) in the cow template", function()
            local invocation = base_invocation()
            invocation.active_modes = { "dead" }
            assert.equal(
                " ____\n< hi >\n ----\n\\ XX U \n",
                cowsay.compose_content(invocation, temp_dir)
            )
        end)
    end)

    describe("build_scene", function()
        it("emits one glyph_run per non-blank line", function()
            local scene = cowsay.build_scene("hi\n\nyo")
            local glyph_runs = {}
            for _, inst in ipairs(scene.instructions) do
                if inst.kind == "glyph_run" then
                    glyph_runs[#glyph_runs + 1] = inst
                end
            end
            assert.equal(2, #glyph_runs)

            assert.same({
                { glyph_id = string.byte("h"), x = 0, y = 0 },
                { glyph_id = string.byte("i"), x = 8, y = 0 },
            }, glyph_runs[1].glyphs)

            assert.same({
                { glyph_id = string.byte("y"), x = 0, y = 32 },
                { glyph_id = string.byte("o"), x = 8, y = 32 },
            }, glyph_runs[2].glyphs)
        end)

        it("skips spaces rather than placing them", function()
            local scene = cowsay.build_scene("a b")
            local glyph_runs = {}
            for _, inst in ipairs(scene.instructions) do
                if inst.kind == "glyph_run" then
                    glyph_runs[#glyph_runs + 1] = inst
                end
            end
            assert.equal(1, #glyph_runs)
            assert.equal(2, #glyph_runs[1].glyphs)
        end)

        it("computes scene dimensions from the longest line and line count", function()
            local scene = cowsay.build_scene("abc\nde")
            assert.equal(3 * 8, scene.width)
            assert.equal(2 * 16, scene.height)
        end)

        it("places one glyph per Unicode character, not per byte", function()
            -- "café" is 4 characters but 5 bytes in UTF-8 (é is 2 bytes).
            local scene = cowsay.build_scene("café")
            local glyph_runs = {}
            for _, inst in ipairs(scene.instructions) do
                if inst.kind == "glyph_run" then
                    glyph_runs[#glyph_runs + 1] = inst
                end
            end
            assert.equal(1, #glyph_runs)
            assert.equal(4, #glyph_runs[1].glyphs)
            assert.equal(3 * 8, glyph_runs[1].glyphs[4].x) -- the 4th character, not the 5th byte.
        end)
    end)

    describe("render round-trips through paint_vm_ascii", function()
        local contents = { "hi", "hello\nworld", " ____\n< hi >\n ----\n\\   ^__^\n" }
        for _, content in ipairs(contents) do
            it("round-trips '" .. content:gsub("\n", "\\n") .. "'", function()
                local paint_vm_ascii = require("coding_adventures.paint_vm_ascii")
                local scene = cowsay.build_scene(content)
                local output = paint_vm_ascii.render(scene, { scale_x = 8, scale_y = 16 })

                local lines = {}
                local start = 1
                while true do
                    local pos = content:find("\n", start, true)
                    if pos == nil then
                        lines[#lines + 1] = content:sub(start)
                        break
                    end
                    lines[#lines + 1] = content:sub(start, pos - 1)
                    start = pos + 1
                end
                for i, line in ipairs(lines) do
                    lines[i] = (line:gsub("%s+$", ""))
                end
                local expected = table.concat(lines, "\n")
                expected = expected:gsub("[%s\n]+$", "")

                assert.equal(expected, output)
            end)
        end
    end)

    describe("is_list_requested", function()
        it("is true when the list flag is truthy", function()
            assert.is_true(cowsay.is_list_requested({ list = true }))
        end)

        it("is false when the list flag is absent or false", function()
            assert.is_false(cowsay.is_list_requested({}))
            assert.is_false(cowsay.is_list_requested({ list = false }))
        end)
    end)

    describe("resolve_message_from_arguments", function()
        it("joins message tokens with a space", function()
            assert.equal("hello there", cowsay.resolve_message_from_arguments({ message = { "hello", "there" } }))
        end)

        it("returns nil when there are no arguments", function()
            assert.is_nil(cowsay.resolve_message_from_arguments({}))
        end)

        it("returns nil for an empty message list", function()
            assert.is_nil(cowsay.resolve_message_from_arguments({ message = {} }))
        end)
    end)

    describe("build_invocation", function()
        it("applies documented defaults", function()
            local invocation = cowsay.build_invocation("hi", {})
            assert.equal("hi", invocation.message)
            assert.equal("oo", invocation.eyes)
            assert.equal("  ", invocation.tongue)
            assert.equal("default", invocation.cowfile)
            assert.is_false(invocation.nowrap)
            assert.is_false(invocation.think)
            assert.equal(40, invocation.width)
            assert.same({}, invocation.active_modes)
        end)

        it("honors explicit flags", function()
            local invocation = cowsay.build_invocation("hi", {
                eyes = "^^", tongue = "vv", cowfile = "dragon",
                nowrap = true, think = true, width = 20, borg = true,
            })
            assert.equal("^^", invocation.eyes)
            assert.equal("vv", invocation.tongue)
            assert.equal("dragon", invocation.cowfile)
            assert.is_true(invocation.nowrap)
            assert.is_true(invocation.think)
            assert.equal(20, invocation.width)
            assert.same({ "borg" }, invocation.active_modes)
        end)

        it("clamps width to a 32-bit ceiling and a floor of 1", function()
            assert.equal(2147483647, cowsay.build_invocation("hi", { width = 99999999999 }).width)
            assert.equal(1, cowsay.build_invocation("hi", { width = -5 }).width)
        end)
    end)

    describe("end-to-end golden output", function()
        local cows_dir = ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
            .. sep .. "specs" .. sep .. "cows"

        it("renders the default cow speaking Hello, World!", function()
            local invocation = {
                message = "Hello, World!", eyes = "oo", tongue = "  ", active_modes = {},
                nowrap = false, width = 40, think = false, cowfile = "default",
            }
            local expected = table.concat({
                " _______________",
                "< Hello, World! >",
                " ---------------",
                "        \\   ^__^",
                "         \\  (oo)\\_______",
                "            (__)\\       )\\/\\",
                "                ||----w |",
                "                ||     ||",
            }, "\n")
            assert.equal(expected, cowsay.render(invocation, cows_dir))
        end)

        it("renders borg mode thinking with the default cow", function()
            local invocation = {
                message = "beep", eyes = "oo", tongue = "  ", active_modes = { "borg" },
                nowrap = false, width = 40, think = true, cowfile = "default",
            }
            local expected = table.concat({
                " ______",
                "( beep )",
                " ------",
                "        o   ^__^",
                "         o  (==)\\_______",
                "            (__)\\       )\\/\\",
                "                ||----w |",
                "                ||     ||",
            }, "\n")
            assert.equal(expected, cowsay.render(invocation, cows_dir))
        end)
    end)
end)
