-- Tests for the resolver module.
--
-- These tests verify the Rosetta Stone name mapping and dependency parsing
-- for each language ecosystem.

-- Add lib/ to the module search path.
package.path = "../lib/?.lua;" .. "../lib/?/init.lua;" .. package.path

local Resolver = require("build_tool.resolver")

describe("Resolver", function()

    describe("build_known_names", function()
        it("maps Python packages correctly", function()
            local packages = {{
                name = "python/logic-gates",
                path = "/fake/packages/python/logic-gates",
                language = "python",
            }}
            local known = Resolver.build_known_names(packages)
            assert.are.equal("python/logic-gates", known["coding-adventures-logic-gates"])
        end)

        it("maps Ruby packages correctly", function()
            local packages = {{
                name = "ruby/logic_gates",
                path = "/fake/packages/ruby/logic_gates",
                language = "ruby",
            }}
            local known = Resolver.build_known_names(packages)
            assert.are.equal("ruby/logic_gates", known["coding_adventures_logic_gates"])
        end)

        it("maps TypeScript packages correctly", function()
            local packages = {{
                name = "typescript/logic-gates",
                path = "/fake/packages/typescript/logic-gates",
                language = "typescript",
            }}
            local known = Resolver.build_known_names(packages)
            assert.are.equal("typescript/logic-gates", known["@coding-adventures/logic-gates"])
        end)

        it("maps Rust packages correctly", function()
            local packages = {{
                name = "rust/logic-gates",
                path = "/fake/packages/rust/logic-gates",
                language = "rust",
            }}
            local known = Resolver.build_known_names(packages)
            assert.are.equal("rust/logic-gates", known["logic-gates"])
        end)

        it("maps Elixir packages correctly", function()
            local packages = {{
                name = "elixir/logic-gates",
                path = "/fake/packages/elixir/logic-gates",
                language = "elixir",
            }}
            local known = Resolver.build_known_names(packages)
            assert.are.equal("elixir/logic-gates", known["coding_adventures_logic_gates"])
        end)

        it("maps Lua packages correctly (underscores to hyphens)", function()
            local packages = {{
                name = "lua/logic_gates",
                path = "/fake/packages/lua/logic_gates",
                language = "lua",
            }}
            local known = Resolver.build_known_names(packages)
            assert.are.equal("lua/logic_gates", known["coding-adventures-logic-gates"])
        end)

        it("maps Lua packages with multiple underscores", function()
            local packages = {{
                name = "lua/cpu_simulator",
                path = "/fake/packages/lua/cpu_simulator",
                language = "lua",
            }}
            local known = Resolver.build_known_names(packages)
            assert.are.equal("lua/cpu_simulator", known["coding-adventures-cpu-simulator"])
        end)

        it("handles multiple packages across languages", function()
            local packages = {
                {name = "python/arithmetic", path = "/fake/python/arithmetic", language = "python"},
                {name = "lua/arithmetic", path = "/fake/lua/arithmetic", language = "lua"},
            }
            local known = Resolver.build_known_names(packages)
            assert.are.equal("python/arithmetic", known["coding-adventures-arithmetic"])
            -- Lua arithmetic dir has no underscores, so rockspec name is same as Python.
            -- This tests that both can coexist (last one wins for the same key).
        end)
    end)

    describe("resolve_dependencies", function()
        it("creates a graph with all packages as nodes", function()
            local packages = {
                {name = "lua/a", path = "/fake/lua/a", language = "lua", build_commands = {}},
                {name = "lua/b", path = "/fake/lua/b", language = "lua", build_commands = {}},
            }
            local graph = Resolver.resolve_dependencies(packages)
            assert.is_true(graph:has_node("lua/a"))
            assert.is_true(graph:has_node("lua/b"))
        end)

        it("parses Lua rockspec dependencies", function()
            -- Create temp directories with rockspec files.
            local tmpdir_a = os.tmpname()
            os.remove(tmpdir_a)
            os.execute('mkdir -p "' .. tmpdir_a .. '"')

            local tmpdir_b = os.tmpname()
            os.remove(tmpdir_b)
            os.execute('mkdir -p "' .. tmpdir_b .. '"')

            -- Package B has no deps.
            local f = io.open(tmpdir_b .. "/coding-adventures-b-0.1.0-1.rockspec", "w")
            f:write('package = "coding-adventures-b"\n')
            f:write('dependencies = { "lua >= 5.4" }\n')
            f:close()

            -- Package A depends on B.
            f = io.open(tmpdir_a .. "/coding-adventures-a-0.1.0-1.rockspec", "w")
            f:write('package = "coding-adventures-a"\n')
            f:write('dependencies = {\n')
            f:write('    "lua >= 5.4",\n')
            f:write('    "coding-adventures-b >= 0.1.0",\n')
            f:write('}\n')
            f:close()

            local packages = {
                {name = "lua/a", path = tmpdir_a, language = "lua", build_commands = {}},
                {name = "lua/b", path = tmpdir_b, language = "lua", build_commands = {}},
            }

            local graph = Resolver.resolve_dependencies(packages)

            -- B should be a predecessor of A (B must build before A).
            local preds = graph:predecessors("lua/a")
            local found_b = false
            for _, p in ipairs(preds) do
                if p == "lua/b" then found_b = true end
            end
            assert.is_true(found_b, "lua/b should be a predecessor of lua/a")

            -- B should have no predecessors.
            assert.are.equal(0, #graph:predecessors("lua/b"))

            -- Clean up.
            os.remove(tmpdir_a .. "/coding-adventures-a-0.1.0-1.rockspec")
            os.remove(tmpdir_b .. "/coding-adventures-b-0.1.0-1.rockspec")
            os.execute('rmdir "' .. tmpdir_a .. '"')
            os.execute('rmdir "' .. tmpdir_b .. '"')
        end)

        it("handles packages with no metadata files", function()
            local packages = {
                {name = "lua/empty", path = "/nonexistent/path", language = "lua", build_commands = {}},
            }
            local graph = Resolver.resolve_dependencies(packages)
            assert.is_true(graph:has_node("lua/empty"))
            assert.are.equal(0, #graph:predecessors("lua/empty"))
        end)
    end)
end)
