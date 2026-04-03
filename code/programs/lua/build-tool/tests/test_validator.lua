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
end)
