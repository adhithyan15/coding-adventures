local script_dir = debug.getinfo(1, "S").source:sub(2):match("(.*/)")
package.path = script_dir .. "../lib/?.lua;" .. script_dir .. "../lib/?/init.lua;" .. package.path

local Validator = require("build_tool.validator")

local function write_file(pathname, content)
    local file = assert(io.open(pathname, "w"))
    file:write(content)
    file:close()
end

local function make_dir(pathname)
    if package.config:sub(1, 1) == "\\" then
        os.execute('mkdir "' .. pathname .. '" >NUL 2>NUL')
    else
        os.execute('mkdir -p "' .. pathname .. '"')
    end
end

local function remove_dir(pathname)
    if package.config:sub(1, 1) == "\\" then
        os.execute('rmdir /s /q "' .. pathname .. '" >NUL 2>NUL')
    else
        os.execute('rm -rf "' .. pathname .. '"')
    end
end

describe("Validator", function()
    local tmpdir

    before_each(function()
        tmpdir = os.tmpname()
        os.remove(tmpdir)
        make_dir(tmpdir .. "/.github/workflows")
    end)

    after_each(function()
        remove_dir(tmpdir)
    end)

    it("fails without normalized outputs", function()
        write_file(tmpdir .. "/.github/workflows/ci.yml", [[
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.detect.outputs.needs_python }}
      needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
]])

        local error = Validator.validate_ci_full_build_toolchains(tmpdir, {
            { language = "elixir" },
            { language = "python" },
        })

        assert.is_not_nil(error)
        assert.is_truthy(error:find(".github/workflows/ci.yml", 1, true))
        assert.is_truthy(error:find("elixir", 1, true))
        assert.is_truthy(error:find("python", 1, true))
    end)

    it("allows normalized outputs", function()
        write_file(tmpdir .. "/.github/workflows/ci.yml", [[
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.toolchains.outputs.needs_python }}
      needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_python=true' \
            'needs_elixir=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
]])

        assert.is_nil(Validator.validate_ci_full_build_toolchains(tmpdir, {
            { language = "elixir" },
            { language = "python" },
        }))
    end)

    it("allows normalized outputs for jvm toolchains", function()
        write_file(tmpdir .. "/.github/workflows/ci.yml", [[
jobs:
  detect:
    outputs:
      needs_java: ${{ steps.toolchains.outputs.needs_java }}
      needs_kotlin: ${{ steps.toolchains.outputs.needs_kotlin }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_java=true' \
            'needs_kotlin=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
]])

        assert.is_nil(Validator.validate_ci_full_build_toolchains(tmpdir, {
            { language = "java" },
            { language = "kotlin" },
        }))
    end)

    it("flags Lua isolated-build violations", function()
        make_dir(tmpdir .. "/code/packages/lua/problem_pkg")
        write_file(tmpdir .. "/code/packages/lua/problem_pkg/BUILD", [[
luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
]])

        local error = Validator.validate_build_contracts(tmpdir, {
            { language = "lua", path = tmpdir .. "/code/packages/lua/problem_pkg" },
        })

        assert.is_not_nil(error)
        assert.is_truthy(error:find("coding-adventures-branch-predictor", 1, true))
        assert.is_truthy(error:find("state_machine before directed_graph", 1, true))
    end)

    it("flags guarded Lua installs without deps mode", function()
        make_dir(tmpdir .. "/code/packages/lua/guarded_pkg")
        write_file(tmpdir .. "/code/packages/lua/guarded_pkg/BUILD", [[
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
]])

        local error = Validator.validate_build_contracts(tmpdir, {
            { language = "lua", path = tmpdir .. "/code/packages/lua/guarded_pkg" },
        })

        assert.is_not_nil(error)
        assert.is_truthy(error:find("--deps-mode=none or --no-manifest", 1, true))
    end)

    it("flags Windows Lua sibling drift", function()
        make_dir(tmpdir .. "/code/packages/lua/arm1_gatelevel")
        write_file(tmpdir .. "/code/packages/lua/arm1_gatelevel/BUILD", [[
(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
]])
        write_file(tmpdir .. "/code/packages/lua/arm1_gatelevel/BUILD_windows", [[
(cd ..\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
]])

        local error = Validator.validate_build_contracts(tmpdir, {
            { language = "lua", path = tmpdir .. "/code/packages/lua/arm1_gatelevel" },
        })

        assert.is_not_nil(error)
        assert.is_truthy(error:find("BUILD_windows is missing sibling installs present in BUILD", 1, true))
        assert.is_truthy(error:find("../logic_gates", 1, true))
        assert.is_truthy(error:find("../arithmetic", 1, true))
        assert.is_truthy(error:find("--deps-mode=none or --no-manifest", 1, true))
    end)

    it("flags Perl Test2 bootstraps without --notest", function()
        make_dir(tmpdir .. "/code/packages/perl/draw-instructions-svg")
        write_file(tmpdir .. "/code/packages/perl/draw-instructions-svg/BUILD", [[
cpanm --quiet Test2::V0
prove -l -I../draw-instructions/lib -v t/
]])

        local error = Validator.validate_build_contracts(tmpdir, {
            { language = "perl", path = tmpdir .. "/code/packages/perl/draw-instructions-svg" },
        })

        assert.is_not_nil(error)
        assert.is_truthy(error:find("Test2::V0 without --notest", 1, true))
    end)

    it("allows safe Lua isolated-build patterns", function()
        make_dir(tmpdir .. "/code/packages/lua/safe_pkg")
        write_file(tmpdir .. "/code/packages/lua/safe_pkg/BUILD", [[
luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
]])
        write_file(tmpdir .. "/code/packages/lua/safe_pkg/BUILD_windows", [[
luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
]])

        assert.is_nil(Validator.validate_build_contracts(tmpdir, {
            { language = "lua", path = tmpdir .. "/code/packages/lua/safe_pkg" },
        }))
    end)
end)
