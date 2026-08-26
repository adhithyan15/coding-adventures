local test_source = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/")
local script_dir = assert(test_source:match("(.*/)"))
package.path = script_dir .. "../lib/?.lua;" .. script_dir .. "../lib/?/init.lua;" .. package.path

local Validator = require("build_tool.validator")
local json = require("dkjson")

local tracked_artifact_cases = {
    "validation-tracked-artifacts-clean.json",
    "validation-tracked-artifacts-forbidden.json",
    "validation-tracked-artifacts-aliases.json",
    "validation-tracked-artifacts-invalid.json",
    "validation-tracked-artifacts-unicode-boundaries.json",
}
local orphan_crate_cases = {
    "validation-orphan-crates-clean.json",
    "validation-orphan-crates-unlisted.json",
    "validation-orphan-exemptions-invalid.json",
    "validation-orphan-exemptions-stale.json",
}
local conformance_cases = "../../../../specs/fixtures/build-tool-v1/cases"

local function read_file(pathname)
    local file = assert(io.open(pathname, "rb"))
    local contents = file:read("*a")
    file:close()
    return contents
end

local function load_fixture(filename)
    local fixture, _, decode_error = json.decode(read_file(conformance_cases .. "/" .. filename))
    assert.is_nil(decode_error)
    return fixture
end

local function utf8_from_scalars(scalars)
    local chunks = {}
    for _, scalar in ipairs(scalars) do
        chunks[#chunks + 1] = utf8.char(scalar)
    end
    return table.concat(chunks)
end

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

    for _, fixture_name in ipairs(tracked_artifact_cases) do
        it("matches shared " .. fixture_name:gsub("%.json$", "") .. " fixture", function()
            local fixture = load_fixture(fixture_name)
            local snapshot = fixture.input.options.tracked_artifact_snapshot
            local actual = Validator.validate_tracked_artifact_snapshot(
                snapshot.entries,
                snapshot.unicode_version
            )

            assert.are.same(fixture.expected.diagnostics, actual)
        end)
    end

    for _, fixture_name in ipairs(orphan_crate_cases) do
        it("matches shared " .. fixture_name:gsub("%.json$", "") .. " fixture", function()
            local fixture = load_fixture(fixture_name)
            local snapshot = fixture.input.options.orphan_snapshot
            local actual = Validator.validate_orphan_crate_snapshot(snapshot)

            assert.are.same(fixture.expected.diagnostics, actual.diagnostics)
            actual.diagnostics = nil
            assert.are.same(fixture.expected.result, actual)
        end)
    end

    it("redacts unsafe orphan exemption paths, including invalid UTF-8", function()
        local unsafe_paths = {
            "",
            string.rep("\240\159\152\128", 513),
            "/absolute/secret-project",
            "C:/host/secret-project",
            "code/packages/rust/bad<name>",
            "code/packages/rust/trailing.",
            "code/packages/rust/CON",
            "code/packages/rust/\255",
        }

        for _, unsafe_path in ipairs(unsafe_paths) do
            local result = Validator.validate_orphan_crate_snapshot({
                directories = {"code/packages/rust/demo"},
                manifests = {{path = "code/packages/rust/demo", kind = "package"}},
                build_files = {},
                exemptions = {{
                    line = 7,
                    kind = "PENDING",
                    path = unsafe_path,
                    reason = "not allowed",
                }},
            })

            local invalid
            for _, diagnostic in ipairs(result.diagnostics) do
                if diagnostic.code == "ORPHAN_EXEMPTION_INVALID" then
                    invalid = diagnostic
                    break
                end
            end
            assert.are.same({
                code = "ORPHAN_EXEMPTION_INVALID",
                severity = "error",
                path = "code/BUILD-EXEMPTIONS",
                details = {line = 7, problem = "PATH_UNSAFE"},
            }, invalid)
            assert.is_nil(invalid.details.path)
        end
    end)

    it("uses the exact Python blank-reason code point set", function()
        local result = Validator.validate_orphan_crate_snapshot({
            directories = {"code/packages/rust/blank", "code/packages/rust/bom"},
            manifests = {
                {path = "code/packages/rust/blank", kind = "package"},
                {path = "code/packages/rust/bom", kind = "package"},
            },
            build_files = {},
            exemptions = {
                {
                    line = 7,
                    kind = "PENDING",
                    path = "code/packages/rust/blank",
                    reason = utf8.char(0x001C),
                },
                {
                    line = 8,
                    kind = "PENDING",
                    path = "code/packages/rust/bom",
                    reason = utf8.char(0xFEFF),
                },
            },
        })

        assert.are.equal(1, result.pending_exemption_count)
        assert.are.same(
            {"ORPHAN_CRATE_UNLISTED", "ORPHAN_EXEMPTION_INVALID"},
            result.diagnostic_codes
        )
        assert.are.equal("REASON_MISSING", result.diagnostics[#result.diagnostics].details.problem)
    end)

    it("chooses the closest empty BUILD with the fixed filename rank", function()
        local result = Validator.validate_orphan_crate_snapshot({
            directories = {"code/packages/rust/demo/child"},
            manifests = {{path = "code/packages/rust/demo/child", kind = "package"}},
            build_files = {
                {path = "code/packages/rust/BUILD", state = "empty"},
                {path = "code/packages/rust/demo/BUILD_linux", state = "empty"},
                {path = "code/packages/rust/demo/BUILD", state = "empty"},
                {path = "code/packages/rust/demo2/BUILD", state = "runnable"},
            },
            exemptions = {},
        })

        assert.are.equal(
            "code/packages/rust/demo/BUILD",
            result.diagnostics[1].details.build_path
        )
    end)

    it("reserves NFC full-fold identities before policy precedence", function()
        local result = Validator.validate_orphan_crate_snapshot({
            directories = {"code/packages/rust/Stra\195\159e"},
            manifests = {{path = "code/packages/rust/Stra\195\159e", kind = "package"}},
            build_files = {},
            exemptions = {
                {
                    line = 7,
                    kind = "UNKNOWN",
                    path = "code/packages/rust/Stra\195\159e",
                    reason = "first",
                },
                {
                    line = 8,
                    kind = "PENDING",
                    path = "CODE/PACKAGES/RUST/STRASSE",
                    reason = "duplicate",
                },
            },
        })

        local invalid_details = {}
        for _, diagnostic in ipairs(result.diagnostics) do
            if diagnostic.code == "ORPHAN_EXEMPTION_INVALID" then
                invalid_details[#invalid_details + 1] = diagnostic.details
            end
        end
        assert.are.same({
            {line = 7, problem = "UNKNOWN_KIND"},
            {line = 8, problem = "DUPLICATE_PATH"},
        }, invalid_details)
    end)

    it("uses ASCII JSON ordering for Unicode diagnostic details", function()
        local accented = "code/packages/rust/\195\169"
        local emoji = "code/packages/rust/\240\159\152\128"
        local result = Validator.validate_orphan_crate_snapshot({
            directories = {},
            manifests = {},
            build_files = {},
            exemptions = {
                {line = 9, kind = "EXCLUDED", path = "code/packages/rust/z", reason = "removed"},
                {line = 8, kind = "EXCLUDED", path = emoji, reason = "removed"},
                {line = 7, kind = "EXCLUDED", path = accented, reason = "removed"},
            },
        })

        local paths = {}
        for _, diagnostic in ipairs(result.diagnostics) do
            paths[#paths + 1] = diagnostic.details.entry_path
        end
        assert.are.same({accented, emoji, "code/packages/rust/z"}, paths)
    end)

    it("rejects Unicode version drift before entries", function()
        assert.are.equal("17.0.0", Validator.TRACKED_ARTIFACT_UNICODE_VERSION)

        local ok, validation_error = pcall(
            Validator.validate_tracked_artifact_snapshot,
            {{ ordinal = 1, path = "/hostile", entry_kind = "regular" }},
            "15.1.0"
        )

        assert.is_false(ok)
        assert.are.equal(
            "tracked artifact Unicode version must be 17.0.0",
            tostring(validation_error):gsub("^.-:%d+: ", "")
        )
    end)

    it("redacts every unsafe tracked path class", function()
        local unsafe_paths = {
            {"", "EMPTY"},
            {string.rep("a", 513), "TOO_LONG"},
            {"code/packages/e\204\129/file.lua", "NON_NFC"},
            {"/absolute/file.lua", "ABSOLUTE"},
            {"C:\\repo\\file.lua", "DRIVE_QUALIFIED"},
            {"code//file.lua", "EMPTY_SEGMENT"},
            {"code/trailing/", "EMPTY_SEGMENT"},
            {"code\\trailing\\", "EMPTY_SEGMENT"},
            {"code/<unsafe>/file.lua", "UNSAFE_CHARACTER"},
            {"code/../file.lua", "DOT_SEGMENT"},
            {"code/trailing./file.lua", "TRAILING_DOT_OR_SPACE"},
            {"code/CON.txt/file.lua", "RESERVED_BASENAME"},
        }

        for _, case in ipairs(unsafe_paths) do
            local diagnostics = Validator.validate_tracked_artifact_snapshot({
                { ordinal = 7, path = case[1], entry_kind = "regular" },
            })
            assert.are.equal(1, #diagnostics)
            assert.are.equal("repository", diagnostics[1].path)
            assert.are.equal(case[2], diagnostics[1].details.problem)
            if case[1] ~= "" then
                assert.is_nil(json.encode(diagnostics):find(case[1], 1, true))
            end
        end
    end)

    it("uses lexical separators and Unicode scalar lengths", function()
        assert.are.same({}, Validator.validate_tracked_artifact_snapshot({
            { ordinal = 1, path = "code\\src\\file.lua", entry_kind = "regular" },
        }))
        assert.are.same({}, Validator.validate_tracked_artifact_snapshot({
            { ordinal = 2, path = string.rep("\240\159\152\128", 512), entry_kind = "regular" },
        }))

        local diagnostics = Validator.validate_tracked_artifact_snapshot({
            { ordinal = 3, path = string.rep("\240\159\152\128", 513), entry_kind = "regular" },
        })
        assert.are.equal("TOO_LONG", diagnostics[1].details.problem)
    end)

    it("uses only pinned Unicode 17 tables", function()
        local unicode = require("build_tool.tracked_artifact_unicode17")
        local todhri_source = utf8_from_scalars({0x105D2, 0x0307})
        local todhri_composed = utf8_from_scalars({0x105C9})
        assert.are.equal(todhri_composed, unicode.nfc(todhri_source))

        local diagnostics = Validator.validate_tracked_artifact_snapshot({
            { ordinal = 1, path = todhri_source, entry_kind = "regular" },
        })
        assert.are.equal("NON_NFC", diagnostics[1].details.problem)

        local outlined_scalars = {}
        for scalar in string.gmatch("NODE_MODULES", ".") do
            local value = string.byte(scalar)
            outlined_scalars[#outlined_scalars + 1] =
                value == 0x5F and value or 0x1CCD6 + value - 0x41
        end
        local outlined = utf8_from_scalars(outlined_scalars)
        assert.are.equal("node_modules", unicode.nfkc_casefold(outlined))
        diagnostics = Validator.validate_tracked_artifact_snapshot({
            { ordinal = 2, path = "code/" .. outlined .. "/file.lua", entry_kind = "regular" },
        })
        assert.are.equal("TRACKED_ARTIFACT_FORBIDDEN", diagnostics[1].code)

        assert.are.equal("CONIN$", unicode.full_uppercase("con\196\177n$"))
        diagnostics = Validator.validate_tracked_artifact_snapshot({
            { ordinal = 3, path = "code/con\196\177n$.txt/file.lua", entry_kind = "regular" },
        })
        assert.are.equal("RESERVED_BASENAME", diagnostics[1].details.problem)

        assert.are.equal("q\204\128", unicode.nfc("q\204\128"))
        assert.are.same({}, Validator.validate_tracked_artifact_snapshot({
            { ordinal = 4, path = "q\204\128/file.lua", entry_kind = "regular" },
        }))
    end)

    it("sorts diagnostics by Unicode scalar value", function()
        local private_use = utf8_from_scalars({0xE000})
        local supplementary = utf8_from_scalars({0x10000})
        local diagnostics = Validator.validate_tracked_artifact_snapshot({
            { ordinal = 1, path = supplementary .. "/node_modules/a", entry_kind = "regular" },
            { ordinal = 2, path = private_use .. "/node_modules/b", entry_kind = "regular" },
        })

        assert.are.same(
            {private_use .. "/node_modules/b", supplementary .. "/node_modules/a"},
            {diagnostics[1].path, diagnostics[2].path}
        )
    end)

    it("treats entry kind as inert metadata", function()
        local diagnostics = Validator.validate_tracked_artifact_snapshot({
            { ordinal = 1, path = "node_modules/a", entry_kind = "regular" },
            { ordinal = 2, path = "node_modules/b", entry_kind = "symlink" },
            { ordinal = 3, path = "node_modules/c", entry_kind = "reparse" },
        })

        assert.are.same(
            {"regular", "symlink", "reparse"},
            {
                diagnostics[1].details.entry_kind,
                diagnostics[2].details.entry_kind,
                diagnostics[3].details.entry_kind,
            }
        )
    end)
end)
