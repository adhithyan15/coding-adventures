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
                {name = "ruby/arithmetic", path = "/fake/ruby/arithmetic", language = "ruby"},
            }
            local known = Resolver.build_known_names(packages)
            -- Python uses hyphens, Ruby uses underscores — different keys, no collision.
            assert.are.equal("python/arithmetic", known["coding-adventures-arithmetic"])
            assert.are.equal("ruby/arithmetic", known["coding_adventures_arithmetic"])
        end)

        it("maps wasm crates into the rust dependency scope", function()
            local tmpbase = os.tmpname()
            os.remove(tmpbase)
            os.execute('mkdir "' .. tmpbase .. '"')
            os.execute('mkdir "' .. tmpbase .. '/wasm_graph"')

            local cargo = io.open(tmpbase .. "/wasm_graph/Cargo.toml", "w")
            cargo:write('[package]\nname = "graph-wasm"\n')
            cargo:close()

            local packages = {{
                name = "wasm/graph",
                path = tmpbase .. "/wasm_graph",
                language = "wasm",
            }}

            local known = Resolver.build_known_names(packages, "rust")
            assert.are.equal("wasm/graph", known["graph-wasm"])

            os.remove(tmpbase .. "/wasm_graph/Cargo.toml")
            os.execute('rmdir "' .. tmpbase .. '/wasm_graph"')
            os.execute('rmdir "' .. tmpbase .. '"')
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
            -- Create temp directories with predictable basenames so that
            -- build_known_names maps them correctly. The basename of the
            -- directory is used to construct the rockspec name:
            --   dir "pkg_b" → rockspec name "coding-adventures-pkg-b"
            local tmpbase = os.tmpname()
            os.remove(tmpbase)
            os.execute('mkdir -p "' .. tmpbase .. '/pkg_a"')
            os.execute('mkdir -p "' .. tmpbase .. '/pkg_b"')

            local dir_a = tmpbase .. "/pkg_a"
            local dir_b = tmpbase .. "/pkg_b"

            -- Package B has no internal deps.
            local f = io.open(dir_b .. "/coding-adventures-pkg-b-0.1.0-1.rockspec", "w")
            f:write('package = "coding-adventures-pkg-b"\n')
            f:write('dependencies = { "lua >= 5.4" }\n')
            f:close()

            -- Package A depends on B.
            -- build_known_names maps dir "pkg_b" → "coding-adventures-pkg-b"
            f = io.open(dir_a .. "/coding-adventures-pkg-a-0.1.0-1.rockspec", "w")
            f:write('package = "coding-adventures-pkg-a"\n')
            f:write('dependencies = {\n')
            f:write('    "lua >= 5.4",\n')
            f:write('    "coding-adventures-pkg-b >= 0.1.0",\n')
            f:write('}\n')
            f:close()

            local packages = {
                {name = "lua/pkg_a", path = dir_a, language = "lua", build_commands = {}},
                {name = "lua/pkg_b", path = dir_b, language = "lua", build_commands = {}},
            }

            local graph = Resolver.resolve_dependencies(packages)

            -- B should be a predecessor of A (B must build before A).
            local preds = graph:predecessors("lua/pkg_a")
            local found_b = false
            for _, p in ipairs(preds) do
                if p == "lua/pkg_b" then found_b = true end
            end
            assert.is_true(found_b, "lua/pkg_b should be a predecessor of lua/pkg_a")

            -- B should have no predecessors.
            assert.are.equal(0, #graph:predecessors("lua/pkg_b"))

            -- Clean up.
            os.remove(dir_a .. "/coding-adventures-pkg-a-0.1.0-1.rockspec")
            os.remove(dir_b .. "/coding-adventures-pkg-b-0.1.0-1.rockspec")
            os.execute('rm -rf "' .. tmpbase .. '"')
        end)

        it("handles packages with no metadata files", function()
            local packages = {
                {name = "lua/empty", path = "/nonexistent/path", language = "lua", build_commands = {}},
            }
            local graph = Resolver.resolve_dependencies(packages)
            assert.is_true(graph:has_node("lua/empty"))
            assert.are.equal(0, #graph:predecessors("lua/empty"))
        end)

        it("parses .NET project references across C# and F#", function()
            local tmpbase = os.tmpname()
            os.remove(tmpbase)
            os.execute('mkdir "' .. tmpbase .. '"')
            os.execute('mkdir "' .. tmpbase .. '/graph"')
            os.execute('mkdir "' .. tmpbase .. '/graph-tests"')

            local dep = io.open(tmpbase .. "/graph/CodingAdventures.Graph.csproj", "w")
            dep:write('<Project Sdk="Microsoft.NET.Sdk"></Project>\n')
            dep:close()

            local app = io.open(tmpbase .. "/graph-tests/CodingAdventures.Graph.Tests.fsproj", "w")
            app:write('<Project Sdk="Microsoft.NET.Sdk">\n')
            app:write('  <ItemGroup>\n')
            app:write('    <ProjectReference Include="../graph/CodingAdventures.Graph.csproj" />\n')
            app:write('  </ItemGroup>\n')
            app:write('</Project>\n')
            app:close()

            local packages = {
                {name = "csharp/graph", path = tmpbase .. "/graph", language = "csharp", build_commands = {}},
                {name = "fsharp/graph-tests", path = tmpbase .. "/graph-tests", language = "fsharp", build_commands = {}},
            }

            local graph = Resolver.resolve_dependencies(packages)
            local preds = graph:predecessors("fsharp/graph-tests")
            local found = false
            for _, pred in ipairs(preds) do
                if pred == "csharp/graph" then found = true end
            end
            assert.is_true(found, "csharp/graph should be a predecessor of fsharp/graph-tests")

            os.remove(tmpbase .. "/graph/CodingAdventures.Graph.csproj")
            os.remove(tmpbase .. "/graph-tests/CodingAdventures.Graph.Tests.fsproj")
            os.execute('rmdir "' .. tmpbase .. '/graph"')
            os.execute('rmdir "' .. tmpbase .. '/graph-tests"')
            os.execute('rmdir "' .. tmpbase .. '"')
        end)
    end)
end)
