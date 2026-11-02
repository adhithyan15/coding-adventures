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

-- One manifest shape per case, pinning each line rule of the Cargo contract
-- in code/specs/build-tool-conformance.md. The table is the Go reference
-- tool's TestCargoLinkDependencyNames, case for case, so a divergence between
-- build-tool fronts shows up as the same named case failing.
describe("Cargo link-table reader", function()
    local cases = {
        {
            "top-level inline",
            "[dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "build-dependencies",
            "[build-dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "legacy build_dependencies",
            "[build_dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "quoted cfg target",
            "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "bare triple target build",
            "[target.x86_64-pc-windows-msvc.build-dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "legacy target build",
            "[target.'cfg(unix)'.build_dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "spaced header segments",
            "[ target . 'cfg(unix)' . dependencies ]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "quoted table segment",
            "[\"dependencies\"]\n" ..
            "a = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "sub-table",
            "[dependencies.a]\n" ..
            "version = \"1\"\n" ..
            "path = \"../a\"\n",
            {"a"},
        },
        {
            "sub-table rename before path",
            "[build-dependencies.x]\n" ..
            "package = \"A\"\n" ..
            "path = '../a'\n",
            {"a"},
        },
        {
            "target sub-table, dotted quoted triple",
            "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\n" ..
            "path = \"../a\"\n",
            {"a"},
        },
        {
            "quoted sub-table key",
            "[dependencies.\"a\"]\n" ..
            "path = \"../a\"\n",
            {"a"},
        },
        {
            "sub-table without path",
            "[dependencies.a]\n" ..
            "version = \"1\"\n",
            {},
        },
        {
            "sub-table first path wins, ended by header",
            "[dependencies.a]\n" ..
            "path = \"../a\"\n" ..
            "path = \"../b\"\n" ..
            "[package]\n" ..
            "name = \"z\"\n",
            {"a"},
        },
        {
            "sub-table non-string path",
            "[dependencies.a]\n" ..
            "path = 1\n",
            {},
        },
        {
            "sub-table empty path",
            "[dependencies.a]\n" ..
            "path = \"\"\n",
            {},
        },
        {
            "sub-table capitalised Path key",
            "[dependencies.a]\n" ..
            "Path = \"../a\"\n",
            {},
        },
        {
            "rename",
            "[dependencies]\n" ..
            "x = { package = \"Logic-Gates\", path = \"../g\" }\n",
            {"logic-gates"},
        },
        {
            "capitalised Package is not a rename",
            "[dependencies]\n" ..
            "x = { Package = \"y\", path = \"../g\" }\n",
            {"x"},
        },
        {
            "capitalised Path is not a path",
            "[dependencies]\n" ..
            "x = { Path = \"../g\" }\n",
            {},
        },
        {
            "version plus path with trailing comment",
            "[dependencies]\n" ..
            "a = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n",
            {"a"},
        },
        {
            "hash inside a string is not a comment",
            "[dependencies]\n" ..
            "a = { path = \"../a#1\" }\n",
            {"a"},
        },
        {
            "quoted entry key",
            "[dependencies]\n" ..
            "\"a\" = { path = \"../a\" }\n",
            {"a"},
        },
        {
            "registry entry with path feature",
            "[dependencies]\n" ..
            "a = { version = \"1\", features = [\"path\", \"x\"] }\n" ..
            "b = \"1\"\n",
            {},
        },
        {
            "nested table path is not the entry's",
            "[dependencies]\n" ..
            "a = { x = { path = \"../a\" } }\n",
            {},
        },
        {
            "features array commas do not split fields",
            "[dependencies]\n" ..
            "a = { features = [\"p\", \"q\"], path = \"../a\" }\n",
            {"a"},
        },
        {
            "empty path",
            "[dependencies]\n" ..
            "a = { path = \"\" }\n",
            {},
        },
        {
            "array continued onto later lines",
            "[dependencies]\n" ..
            "a = { path = \"../a\", features = [\n" ..
            "    \"x\", # [not-a-header]\n" ..
            "] }\n" ..
            "b = { path = \"../b\" }\n",
            {"a", "b"},
        },
        {
            "continuation lines are not headers",
            "[dependencies]\n" ..
            "a = { path = \"../a\", features = [\n" ..
            "[\"x\"]\n" ..
            "] }\n" ..
            "b = { path = \"../b\" }\n",
            {"a", "b"},
        },
        {
            "braces continued onto later lines",
            "[dependencies]\n" ..
            "a = { version = \"1\",\n" ..
            "  path = \"../a\" }\n",
            {"a"},
        },
        {
            "file ends inside the value",
            "[dependencies]\n" ..
            "a = { path = \"../a\", features = [\n" ..
            "  \"x\",\n",
            {},
        },
        {
            "continued value must end with a brace",
            "[dependencies]\n" ..
            "a = { path = \"../a\", features = [\n" ..
            "] } x\n",
            {},
        },
        {
            "dotted key is out of scope",
            "[dependencies]\n" ..
            "a.path = \"../a\"\n",
            {},
        },
        {
            "commented-out entry",
            "[dependencies]\n" ..
            "# a = { path = \"../a\" }\n",
            {},
        },
        {
            "dev-dependencies",
            "[dev-dependencies]\n" ..
            "a = { path = \"../a\" }\n" ..
            "[dev_dependencies]\n" ..
            "b = { path = \"../b\" }\n",
            {},
        },
        {
            "target dev-dependencies",
            "[target.'cfg(unix)'.dev-dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {},
        },
        {
            "dev-dependencies sub-table",
            "[dev-dependencies.a]\n" ..
            "path = \"../a\"\n",
            {},
        },
        {
            "patch, replace, workspace",
            "[patch.crates-io]\n" ..
            "a = { path = \"../a\" }\n" ..
            "[workspace.dependencies]\n" ..
            "b = { path = \"../b\" }\n" ..
            "[replace]\n" ..
            "\"c:1.0.0\" = { path = \"../c\" }\n",
            {},
        },
        {
            "capitalised table name",
            "[Dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {},
        },
        {
            "features sub-table of a dependency",
            "[dependencies.a.features]\n" ..
            "path = \"../a\"\n",
            {},
        },
        {
            "target table itself",
            "[target.'cfg(unix)']\n" ..
            "a = { path = \"../a\" }\n",
            {},
        },
        {
            "array of tables ends the table",
            "[dependencies]\n" ..
            "[[bin]]\n" ..
            "a = { path = \"../a\" }\n",
            {},
        },
        {
            "malformed header ends the table",
            "[dependencies]\n" ..
            "[a b]\n" ..
            "c = { path = \"../c\" }\n",
            {},
        },
        {
            "unterminated quoted header ends the table",
            "[dependencies]\n" ..
            "[target.'cfg(unix).dependencies]\n" ..
            "c = { path = \"../c\" }\n",
            {},
        },
        {
            "CRLF and BOM",
            "\u{FEFF}[dependencies]\r\n" ..
            "a = { path = \"../a\" }\r\n",
            {"a"},
        },
        {
            "union across tables keeps duplicates for the caller",
            "[dependencies]\n" ..
            "a = { path = \"../a\" }\n" ..
            "[build-dependencies]\n" ..
            "a = { path = \"../a\" }\n",
            {"a", "a"},
        },
    }

    for _, case in ipairs(cases) do
        local name, manifest, expected = case[1], case[2], case[3]
        it(name, function()
            assert.are.same(expected, Resolver.cargo_link_dependency_names(manifest))
        end)
    end

    it("ignores the raw UTF-8 byte-order mark bytes", function()
        assert.are.same({"a"}, Resolver.cargo_link_dependency_names(
            "\239\187\191[dependencies]\r\na = { path = \"../a\" }\r\n"))
    end)

    it("collapses a crate named by several link tables", function()
        local tmpdir = os.tmpname()
        os.remove(tmpdir)
        os.execute('mkdir -p "' .. tmpdir .. '/app" "' .. tmpdir .. '/lib"')
        local function write(path, contents)
            local file = assert(io.open(path, "wb"))
            file:write(contents)
            file:close()
        end
        write(tmpdir .. "/app/Cargo.toml",
            "[package]\nname = \"app\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n" ..
            "serde = { path = \"../vendored-serde\" }\n\n[build-dependencies]\nlib = { path = \"../lib\" }\n\n" ..
            "[target.'cfg(windows)'.dependencies.lib]\npath = \"../lib\"\n")
        write(tmpdir .. "/lib/Cargo.toml", "[package]\nname = \"lib\"\n")
        local graph = Resolver.resolve_dependencies({
            {name = "rust/app", path = tmpdir .. "/app", language = "rust"},
            {name = "rust/lib", path = tmpdir .. "/lib", language = "rust"},
        })
        assert.are.same({"rust/app"}, graph:successors("rust/lib"))
        assert.are.same({}, graph:successors("rust/app"))
        os.remove(tmpdir .. "/app/Cargo.toml")
        os.remove(tmpdir .. "/lib/Cargo.toml")
        os.remove(tmpdir .. "/app")
        os.remove(tmpdir .. "/lib")
        os.remove(tmpdir)
    end)
end)
