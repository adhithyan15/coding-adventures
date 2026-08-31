-- Tests for the discovery module.
--
-- These tests verify language inference, package name inference, BUILD
-- file platform selection, and file reading utilities.

-- Add lib/ to the module search path.
package.path = "../lib/?.lua;" .. "../lib/?/init.lua;" .. package.path

local json = require("dkjson")
local lfs = require("lfs")
local Discovery = require("build_tool.discovery")

local fixture_dir = "../../../../specs/fixtures/build-tool-v1/cases"
local temporary_roots = {}

local function read_file(path)
    local file = assert(io.open(path, "rb"))
    local contents = file:read("*a")
    file:close()
    return contents
end

local function write_file(path, contents)
    local file = assert(io.open(path, "wb"))
    file:write(contents)
    file:close()
end

local function ensure_directory(path)
    local normalized = path:gsub("\\", "/")
    local current = ""

    if normalized:match("^[A-Za-z]:/") then
        current = normalized:sub(1, 3)
        normalized = normalized:sub(4)
    elseif normalized:sub(1, 1) == "/" then
        current = "/"
        normalized = normalized:sub(2)
    end

    for segment in normalized:gmatch("[^/]+") do
        if current == "" then
            current = segment
        elseif current == "/" or current:match("^[A-Za-z]:/$") then
            current = current .. segment
        else
            current = current .. "/" .. segment
        end
        lfs.mkdir(current)
    end
end

local function remove_tree(path)
    local attributes = lfs.attributes(path)
    if not attributes then return end
    if attributes.mode ~= "directory" then
        os.remove(path)
        return
    end

    for name in lfs.dir(path) do
        if name ~= "." and name ~= ".." then
            remove_tree(path .. "/" .. name)
        end
    end
    lfs.rmdir(path)
end

local function new_temporary_root()
    local root = os.tmpname()
    os.remove(root)
    ensure_directory(root)
    temporary_roots[#temporary_roots + 1] = root
    return root
end

local function load_language_registry_fixture()
    local fixture_text = read_file(fixture_dir .. "/discovery-language-registry.json")
    local fixture, _, decode_error = json.decode(fixture_text)
    assert.is_nil(decode_error)
    return fixture
end

local function relative_path(root, path)
    local normalized_root = root:gsub("\\", "/")
    local normalized_path = path:gsub("\\", "/")
    return assert(normalized_path:match("^" .. normalized_root:gsub("([^%w])", "%%%1") .. "/(.*)$"))
end

local function project_packages(root, packages)
    local projected = {}
    for _, discovered in ipairs(packages) do
        projected[#projected + 1] = {
            language = discovered.language,
            name = discovered.name,
            rel_path = relative_path(root, discovered.path),
        }
    end
    return projected
end

local function detect_os_from_uname(uname)
    local original_config = package.config
    local original_popen = io.popen
    package.config = "/\n;\n?\n!\n-\n"
    io.popen = function(command)
        assert.are.equal("uname -s 2>/dev/null", command)
        return {
            read = function() return uname .. "\n" end,
            close = function() end,
        }
    end

    local ok, result = pcall(Discovery.detect_os)
    package.config = original_config
    io.popen = original_popen
    assert.is_true(ok)
    return result
end

describe("Discovery", function()

    after_each(function()
        for _, root in ipairs(temporary_roots) do
            remove_tree(root)
        end
        temporary_roots = {}
    end)

    describe("infer_language", function()
        it("detects python from path", function()
            assert.are.equal("python", Discovery.infer_language("/repo/code/packages/python/logic-gates"))
        end)

        it("detects ruby from path", function()
            assert.are.equal("ruby", Discovery.infer_language("/repo/code/packages/ruby/logic_gates"))
        end)

        it("detects go from path", function()
            assert.are.equal("go", Discovery.infer_language("/repo/code/packages/go/logic-gates"))
        end)

        it("detects lua from path", function()
            assert.are.equal("lua", Discovery.infer_language("/repo/code/packages/lua/logic_gates"))
        end)

        it("detects rust from path", function()
            assert.are.equal("rust", Discovery.infer_language("/repo/code/packages/rust/logic-gates"))
        end)

        it("detects typescript from path", function()
            assert.are.equal("typescript", Discovery.infer_language("/repo/code/packages/typescript/logic-gates"))
        end)

        it("detects elixir from path", function()
            assert.are.equal("elixir", Discovery.infer_language("/repo/code/packages/elixir/logic_gates"))
        end)

        it("detects swift from path", function()
            assert.are.equal("swift", Discovery.infer_language("/repo/code/packages/swift/graph"))
        end)

        it("detects wasm from path", function()
            assert.are.equal("wasm", Discovery.infer_language("/repo/code/packages/wasm/graph"))
        end)

        it("detects csharp from path", function()
            assert.are.equal("csharp", Discovery.infer_language("/repo/code/packages/csharp/graph"))
        end)

        it("detects fsharp from path", function()
            assert.are.equal("fsharp", Discovery.infer_language("/repo/code/packages/fsharp/graph"))
        end)

        it("returns unknown for unrecognized paths", function()
            assert.are.equal("unknown", Discovery.infer_language("/repo/code/packages/fortran/matrix"))
        end)

        it("handles Windows-style paths", function()
            assert.are.equal("python", Discovery.infer_language("C:\\repo\\code\\packages\\python\\logic-gates"))
        end)
    end)

    describe("infer_package_name", function()
        it("combines language and basename", function()
            assert.are.equal("python/logic-gates",
                Discovery.infer_package_name("/repo/code/packages/python/logic-gates", "python"))
        end)

        it("handles Windows-style paths", function()
            assert.are.equal("go/arithmetic",
                Discovery.infer_package_name("C:\\repo\\code\\packages\\go\\arithmetic", "go"))
        end)
    end)

    describe("read_lines", function()
        it("reads non-blank non-comment lines", function()
            -- Create a temporary file.
            local tmpfile = os.tmpname()
            local f = io.open(tmpfile, "w")
            f:write("# comment\n")
            f:write("\n")
            f:write("line one\n")
            f:write("  line two  \n")
            f:write("# another comment\n")
            f:write("line three\n")
            f:close()

            local lines = Discovery.read_lines(tmpfile)
            os.remove(tmpfile)

            assert.are.equal(3, #lines)
            assert.are.equal("line one", lines[1])
            assert.are.equal("line two", lines[2])
            assert.are.equal("line three", lines[3])
        end)

        it("returns empty table for missing file", function()
            local lines = Discovery.read_lines("/nonexistent/file/path")
            assert.are.equal(0, #lines)
        end)
    end)

    describe("get_build_file", function()
        it("returns nil when no BUILD file exists", function()
            local result = Discovery.get_build_file("/nonexistent/dir")
            assert.is_nil(result)
        end)

        it("returns generic BUILD when it exists", function()
            -- Create a temp dir with a BUILD file.
            local tmpdir = os.tmpname()
            os.remove(tmpdir)
            os.execute('mkdir "' .. tmpdir .. '"')
            local f = io.open(tmpdir .. "/BUILD", "w")
            f:write("echo test\n")
            f:close()

            local result = Discovery.get_build_file(tmpdir)
            assert.are.equal(tmpdir .. "/BUILD", result)

            os.remove(tmpdir .. "/BUILD")
            os.execute('rmdir "' .. tmpdir .. '"')
        end)

        it("applies each platform BUILD precedence", function()
            local root = new_temporary_root()
            for _, filename in ipairs({
                "BUILD", "BUILD_mac", "BUILD_linux", "BUILD_windows",
                "BUILD_mac_and_linux",
            }) do
                write_file(root .. "/" .. filename, filename .. "\n")
            end

            assert.are.equal(root .. "/BUILD_mac", Discovery.get_build_file(root, "darwin"))
            assert.are.equal(root .. "/BUILD_linux", Discovery.get_build_file(root, "linux"))
            assert.are.equal(root .. "/BUILD_windows", Discovery.get_build_file(root, "windows"))

            os.remove(root .. "/BUILD_mac")
            os.remove(root .. "/BUILD_linux")
            assert.are.equal(root .. "/BUILD_mac_and_linux", Discovery.get_build_file(root, "darwin"))
            assert.are.equal(root .. "/BUILD_mac_and_linux", Discovery.get_build_file(root, "linux"))
        end)

        it("checks directories with LuaFileSystem", function()
            local root = new_temporary_root()
            assert.is_true(Discovery.dir_exists(root))
            assert.is_false(Discovery.dir_exists(root .. "/missing"))
        end)

        it("classifies Unix kernels from uname", function()
            assert.are.equal("darwin", detect_os_from_uname("Darwin"))
            assert.are.equal("linux", detect_os_from_uname("Linux"))
            assert.are.equal("unknown", detect_os_from_uname("Plan9"))
        end)
    end)

    describe("shared OCaml and Dune discovery contract", function()
        it("projects the shared OCaml package records", function()
            local fixture = load_language_registry_fixture()
            local root = new_temporary_root()

            for _, entry in ipairs(fixture.workspace.files) do
                if entry.path:match("^code/packages/ocaml/") then
                    local path = root .. "/" .. entry.path
                    ensure_directory(assert(path:match("^(.*)/[^/]+$")))
                    write_file(path, assert(entry.content_utf8))
                end
            end

            local expected = {}
            for _, package_record in ipairs(fixture.expected.result.packages) do
                if package_record.rel_path:match("^code/packages/ocaml/") then
                    expected[#expected + 1] = {
                        language = package_record.language,
                        name = package_record.name,
                        rel_path = package_record.rel_path,
                    }
                end
            end

            local actual = project_packages(root, Discovery.discover_packages(root .. "/code"))
            assert.are.same(expected, actual)
        end)

        it("skips only the exact Dune output component", function()
            local root = new_temporary_root()
            local package_paths = {
                "code/packages/ocaml/generated-a/_build/exact",
                "code/packages/ocaml/generated-b/_Build/case-source",
                "code/packages/ocaml/generated-c/_build-example/near-source",
            }

            for _, package_path in ipairs(package_paths) do
                local path = root .. "/" .. package_path .. "/BUILD"
                ensure_directory(assert(path:match("^(.*)/[^/]+$")))
                write_file(path, "echo ocaml\n")
            end

            local packages = Discovery.discover_packages(root .. "/code")
            local names = {}
            for _, package_record in ipairs(packages) do
                names[#names + 1] = package_record.name
            end

            assert.are.same({"ocaml/case-source", "ocaml/near-source"}, names)
        end)
    end)
end)
