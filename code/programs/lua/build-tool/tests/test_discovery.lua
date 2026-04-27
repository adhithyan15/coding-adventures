-- Tests for the discovery module.
--
-- These tests verify language inference, package name inference, BUILD
-- file platform selection, and file reading utilities.

-- Add lib/ to the module search path.
package.path = "../lib/?.lua;" .. "../lib/?/init.lua;" .. package.path

local Discovery = require("build_tool.discovery")

describe("Discovery", function()

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
    end)
end)
