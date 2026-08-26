local Unicode = require("build_tool.tracked_artifact_unicode17")

local Validator = {}

local CI_MANAGED_TOOLCHAIN_LANGUAGES = {
    python = true,
    ruby = true,
    typescript = true,
    rust = true,
    elixir = true,
    lua = true,
    perl = true,
    java = true,
    kotlin = true,
    haskell = true,
}

local TRACKED_ARTIFACT_COMPONENT_IDENTITY = "node_modules"
local TRACKED_ARTIFACT_REDACTED_PATH = "repository"
local ORPHAN_SCAN_ROOT = "code"
local ORPHAN_LEDGER_PATH = "code/BUILD-EXEMPTIONS"
local ORPHAN_BUILD_NAMES = {
    BUILD = 1,
    BUILD_windows = 2,
    BUILD_mac = 3,
    BUILD_linux = 4,
    BUILD_mac_and_linux = 5,
}
local ORPHAN_SKIP_COMPONENTS = {
    [".git"] = true,
    target = true,
    node_modules = true,
    vendor = true,
    [".venv"] = true,
    _build = true,
    deps = true,
    [".build"] = true,
    ["dist-newstyle"] = true,
    [".cargo"] = true,
}
local PYTHON_BLANK_CODEPOINTS = {}
for scalar = 0x0009, 0x000D do PYTHON_BLANK_CODEPOINTS[scalar] = true end
for scalar = 0x001C, 0x0020 do PYTHON_BLANK_CODEPOINTS[scalar] = true end
for _, scalar in ipairs({
    0x0085, 0x00A0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000,
}) do
    PYTHON_BLANK_CODEPOINTS[scalar] = true
end
for scalar = 0x2000, 0x200A do PYTHON_BLANK_CODEPOINTS[scalar] = true end
local WINDOWS_RESERVED_BASENAMES = {
    CON = true,
    PRN = true,
    AUX = true,
    NUL = true,
    ["CONIN$"] = true,
    ["CONOUT$"] = true,
    ["CLOCK$"] = true,
}
for index = 1, 9 do
    WINDOWS_RESERVED_BASENAMES["COM" .. index] = true
    WINDOWS_RESERVED_BASENAMES["LPT" .. index] = true
end
for _, index in ipairs({"¹", "²", "³"}) do
    WINDOWS_RESERVED_BASENAMES["COM" .. index] = true
    WINDOWS_RESERVED_BASENAMES["LPT" .. index] = true
end

Validator.TRACKED_ARTIFACT_UNICODE_VERSION = Unicode.UNICODE_VERSION

local function split_path(path)
    local segments = {}
    local start = 1
    while true do
        local separator = path:find("/", start, true)
        if separator == nil then
            segments[#segments + 1] = path:sub(start)
            return segments
        end
        segments[#segments + 1] = path:sub(start, separator - 1)
        start = separator + 1
    end
end

local function has_unsafe_character(path)
    for _, scalar in utf8.codes(path) do
        if scalar < 32 or scalar == 0x3C or scalar == 0x3E or scalar == 0x3A or
            scalar == 0x22 or scalar == 0x7C or scalar == 0x3F or scalar == 0x2A
        then
            return true
        end
    end
    return false
end

local function normalize_tracked_artifact_path(path)
    local normalized = path:gsub("\\", "/")
    if normalized == "" then return nil, "EMPTY" end

    local scalar_length = utf8.len(normalized)
    if scalar_length == nil or scalar_length > 512 then return nil, "TOO_LONG" end
    if Unicode.nfc(normalized) ~= normalized then return nil, "NON_NFC" end
    if normalized:sub(1, 1) == "/" then return nil, "ABSOLUTE" end
    if normalized:match("^[A-Za-z]:") then return nil, "DRIVE_QUALIFIED" end

    local segments = split_path(normalized)
    for _, segment in ipairs(segments) do
        if segment == "" then return nil, "EMPTY_SEGMENT" end
    end
    if has_unsafe_character(normalized) then return nil, "UNSAFE_CHARACTER" end

    for _, segment in ipairs(segments) do
        if segment == "." or segment == ".." then return nil, "DOT_SEGMENT" end
        local ending = segment:sub(-1)
        if ending == " " or ending == "." then return nil, "TRAILING_DOT_OR_SPACE" end

        local basename = segment:match("^([^.]*)")
        if WINDOWS_RESERVED_BASENAMES[Unicode.full_uppercase(basename)] then
            return nil, "RESERVED_BASENAME"
        end
    end
    return normalized, nil
end

local function canonical_details(details)
    return table.concat({
        details.entry_kind,
        tostring(details.ordinal),
        details.problem or "",
    }, "\0")
end

local function diagnostic_less(left, right)
    if left.code ~= right.code then return left.code < right.code end
    if left.path ~= right.path then return left.path < right.path end
    return canonical_details(left.details) < canonical_details(right.details)
end

-- Lua compares UTF-8 strings by encoded byte, while the contract compares
-- paths by Unicode scalar. Converting these bounded strings to scalar arrays
-- keeps ordering independent of the host locale and string representation.
local function unicode_scalar_less(left, right)
    local left_scalars = {utf8.codepoint(left, 1, -1)}
    local right_scalars = {utf8.codepoint(right, 1, -1)}
    local shared = math.min(#left_scalars, #right_scalars)
    for index = 1, shared do
        if left_scalars[index] ~= right_scalars[index] then
            return left_scalars[index] < right_scalars[index]
        end
    end
    return #left_scalars < #right_scalars
end

local function json_ascii_string(value)
    local chunks = {'"'}
    for _, scalar in utf8.codes(value) do
        if scalar == 0x22 then
            chunks[#chunks + 1] = '\\"'
        elseif scalar == 0x5C then
            chunks[#chunks + 1] = "\\\\"
        elseif scalar == 0x08 then
            chunks[#chunks + 1] = "\\b"
        elseif scalar == 0x09 then
            chunks[#chunks + 1] = "\\t"
        elseif scalar == 0x0A then
            chunks[#chunks + 1] = "\\n"
        elseif scalar == 0x0C then
            chunks[#chunks + 1] = "\\f"
        elseif scalar == 0x0D then
            chunks[#chunks + 1] = "\\r"
        elseif scalar >= 0x20 and scalar <= 0x7E then
            chunks[#chunks + 1] = string.char(scalar)
        elseif scalar <= 0xFFFF then
            chunks[#chunks + 1] = string.format("\\u%04x", scalar)
        else
            local adjusted = scalar - 0x10000
            local high = 0xD800 + (adjusted >> 10)
            local low = 0xDC00 + (adjusted & 0x3FF)
            chunks[#chunks + 1] = string.format("\\u%04x\\u%04x", high, low)
        end
    end
    chunks[#chunks + 1] = '"'
    return table.concat(chunks)
end

-- Match Python's json.dumps(details, sort_keys=True): sorted keys, ASCII-only
-- strings, and the default comma/colon spacing. Details are deliberately flat
-- bounded records, so this small encoder never needs host JSON behavior.
local function canonical_orphan_details(details)
    local keys = {}
    for key in pairs(details) do keys[#keys + 1] = key end
    table.sort(keys)

    local members = {}
    for _, key in ipairs(keys) do
        local value = details[key]
        local encoded
        if type(value) == "string" then
            encoded = json_ascii_string(value)
        elseif type(value) == "boolean" then
            encoded = value and "true" or "false"
        elseif value == nil then
            encoded = "null"
        else
            encoded = tostring(value)
        end
        members[#members + 1] = json_ascii_string(key) .. ": " .. encoded
    end
    return "{" .. table.concat(members, ", ") .. "}"
end

local function orphan_diagnostic_less(left, right)
    if left.code ~= right.code then return unicode_scalar_less(left.code, right.code) end
    if left.path ~= right.path then return unicode_scalar_less(left.path, right.path) end
    return canonical_orphan_details(left.details) < canonical_orphan_details(right.details)
end

local function under_orphan_scan_root(path)
    return path == ORPHAN_SCAN_ROOT or path:sub(1, #ORPHAN_SCAN_ROOT + 1) == "code/"
end

local function orphan_artifact_path(path)
    for _, component in ipairs(split_path(path)) do
        if ORPHAN_SKIP_COMPONENTS[component] then return true end
    end
    return false
end

local function portable_orphan_path(path)
    if type(path) ~= "string" then return false end
    local scalar_length = utf8.len(path)
    if scalar_length == nil or scalar_length == 0 or scalar_length > 512 then return false end
    if Unicode.nfc(path) ~= path then return false end
    if path:sub(1, 1) == "/" or path:find("\\", 1, true) ~= nil then return false end
    if path:find("//", 1, true) ~= nil or path:match("^[A-Za-z]:") then return false end
    if has_unsafe_character(path) then return false end

    for _, component in ipairs(split_path(path)) do
        if component == "" or component == "." or component == ".." then return false end
        local ending = component:sub(-1)
        if ending == " " or ending == "." then return false end
        local basename = component:match("^([^.]*)")
        if WINDOWS_RESERVED_BASENAMES[Unicode.full_uppercase(basename)] then return false end
    end
    return true
end

local function orphan_path_identity(path)
    return Unicode.casefold(Unicode.nfc(path))
end

local function python_blank(value)
    if type(value) ~= "string" or utf8.len(value) == nil then return false end
    for _, scalar in utf8.codes(value) do
        if not PYTHON_BLANK_CODEPOINTS[scalar] then return false end
    end
    return true
end

local function path_depth(path)
    return #split_path(path)
end

local function covering_orphan_build(build_files, manifest_path, wanted_state)
    local best
    local best_parent
    local best_rank
    for _, build_file in ipairs(build_files) do
        if build_file.state == wanted_state then
            local parent, name = build_file.path:match("^(.*)/([^/]*)$")
            local rank = name and ORPHAN_BUILD_NAMES[name]
            local ancestor = parent and
                (manifest_path == parent or manifest_path:sub(1, #parent + 1) == parent .. "/")
            if rank and under_orphan_scan_root(parent) and ancestor then
                local better = best == nil or path_depth(parent) > path_depth(best_parent) or
                    (path_depth(parent) == path_depth(best_parent) and rank < best_rank) or
                    (path_depth(parent) == path_depth(best_parent) and rank == best_rank and
                        unicode_scalar_less(build_file.path, best.path))
                if better then
                    best = build_file
                    best_parent = parent
                    best_rank = rank
                end
            end
        end
    end
    return best
end

-- Validate a closed Cargo/BUILD/ledger snapshot without touching the host.
-- Discovery, checkout enumeration, Git, processes, environment, and network
-- authority stay outside this inert table-in/table-out policy boundary.
function Validator.validate_orphan_crate_snapshot(snapshot)
    local manifests = {}
    local manifest_by_path = {}
    local coverage = {}
    local empty_builds = {}
    for _, manifest in ipairs(snapshot.manifests) do
        if not orphan_artifact_path(manifest.path) then
            manifests[#manifests + 1] = manifest
            manifest_by_path[manifest.path] = manifest
            coverage[manifest.path] =
                covering_orphan_build(snapshot.build_files, manifest.path, "runnable") or false
            empty_builds[manifest.path] =
                covering_orphan_build(snapshot.build_files, manifest.path, "empty") or false
        end
    end

    local directories = {}
    for _, path in ipairs(snapshot.directories) do directories[path] = true end

    local diagnostics = {}
    local seen_exemption_paths = {}
    local valid_exemptions = {}

    -- Reserve identities before field-policy precedence. An invalid first
    -- spelling must not let a later full-fold alias escape duplicate detection.
    for _, exemption in ipairs(snapshot.exemptions) do
        local path = exemption.path
        local identity
        local path_problem
        if portable_orphan_path(path) then
            identity = orphan_path_identity(path)
            if not under_orphan_scan_root(path) then
                path_problem = "PATH_OUTSIDE_SCAN"
            elseif orphan_artifact_path(path) then
                path_problem = "PATH_ARTIFACT"
            end
        else
            path_problem = "PATH_UNSAFE"
        end

        local duplicate = identity ~= nil and seen_exemption_paths[identity] == true
        if identity ~= nil and not duplicate then seen_exemption_paths[identity] = true end

        local problem
        if exemption.kind ~= "EXCLUDED" and exemption.kind ~= "PENDING" then
            problem = "UNKNOWN_KIND"
        elseif python_blank(exemption.reason) then
            problem = "REASON_MISSING"
        elseif duplicate then
            problem = "DUPLICATE_PATH"
        else
            problem = path_problem
        end

        if problem ~= nil then
            diagnostics[#diagnostics + 1] = {
                code = "ORPHAN_EXEMPTION_INVALID",
                severity = "error",
                path = ORPHAN_LEDGER_PATH,
                details = {line = exemption.line, problem = problem},
            }
        else
            valid_exemptions[#valid_exemptions + 1] = exemption
        end
    end

    local active_exemptions = {}
    local pending_exemption_count = 0
    for _, exemption in ipairs(valid_exemptions) do
        local path = exemption.path
        local stale_problem
        if not directories[path] then
            stale_problem = "MISSING_DIRECTORY"
        elseif manifest_by_path[path] == nil then
            stale_problem = "NO_MANIFEST"
        elseif coverage[path] then
            stale_problem = "COVERED"
        end

        if stale_problem ~= nil then
            diagnostics[#diagnostics + 1] = {
                code = "ORPHAN_EXEMPTION_STALE",
                severity = "error",
                path = ORPHAN_LEDGER_PATH,
                details = {
                    entry_path = path,
                    kind = exemption.kind,
                    line = exemption.line,
                    problem = stale_problem,
                },
            }
        else
            active_exemptions[path] = exemption
            if exemption.kind == "PENDING" then
                pending_exemption_count = pending_exemption_count + 1
            end
        end
    end

    for _, manifest in ipairs(manifests) do
        local path = manifest.path
        if not coverage[path] and active_exemptions[path] == nil then
            local empty_build = empty_builds[path]
            if empty_build then
                diagnostics[#diagnostics + 1] = {
                    code = "ORPHAN_CRATE_EMPTY_BUILD",
                    severity = "error",
                    path = path,
                    details = {build_path = empty_build.path, manifest_kind = manifest.kind},
                }
            else
                diagnostics[#diagnostics + 1] = {
                    code = "ORPHAN_CRATE_UNLISTED",
                    severity = "error",
                    path = path,
                    details = {manifest_kind = manifest.kind},
                }
            end
        end
    end

    table.sort(diagnostics, orphan_diagnostic_less)
    local diagnostic_codes = {}
    local seen_codes = {}
    for _, diagnostic in ipairs(diagnostics) do
        if not seen_codes[diagnostic.code] then
            seen_codes[diagnostic.code] = true
            diagnostic_codes[#diagnostic_codes + 1] = diagnostic.code
        end
    end
    table.sort(diagnostic_codes)

    return {
        valid = #diagnostics == 0,
        diagnostic_codes = diagnostic_codes,
        pending_exemption_count = pending_exemption_count,
        diagnostics = diagnostics,
    }
end

-- Validate caller-supplied inert records without reading a checkout, following
-- links, consulting Git, launching a process, or inheriting host Unicode data.
function Validator.validate_tracked_artifact_snapshot(entries, unicode_version)
    unicode_version = unicode_version or Validator.TRACKED_ARTIFACT_UNICODE_VERSION
    if unicode_version ~= Validator.TRACKED_ARTIFACT_UNICODE_VERSION then
        error(
            "tracked artifact Unicode version must be " ..
                Validator.TRACKED_ARTIFACT_UNICODE_VERSION
        )
    end

    local diagnostics = {}
    for _, entry in ipairs(entries) do
        local normalized_path, problem = normalize_tracked_artifact_path(entry.path)
        local details = {
            ordinal = entry.ordinal,
            entry_kind = entry.entry_kind,
        }
        if problem ~= nil then
            details.problem = problem
            diagnostics[#diagnostics + 1] = {
                code = "TRACKED_ARTIFACT_PATH_INVALID",
                severity = "error",
                path = TRACKED_ARTIFACT_REDACTED_PATH,
                details = details,
            }
        else
            local forbidden = false
            for _, component in ipairs(split_path(normalized_path)) do
                if Unicode.nfkc_casefold(component) == TRACKED_ARTIFACT_COMPONENT_IDENTITY then
                    forbidden = true
                    break
                end
            end
            if forbidden then
                diagnostics[#diagnostics + 1] = {
                    code = "TRACKED_ARTIFACT_FORBIDDEN",
                    severity = "error",
                    path = normalized_path,
                    details = details,
                }
            end
        end
    end

    table.sort(diagnostics, diagnostic_less)
    return diagnostics
end

function Validator.validate_ci_full_build_toolchains(root, packages)
    local ci_path = root .. "/.github/workflows/ci.yml"
    local file = io.open(ci_path, "r")
    if not file then
        return nil
    end

    local workflow = file:read("*a")
    file:close()

    if not workflow:find("Full build on main merge", 1, true) then
        return nil
    end

    local compact_workflow = workflow:gsub("%s+", "")
    local missing_output_binding = {}
    local missing_main_force = {}

    for _, lang in ipairs(Validator.languages_needing_ci_toolchains(packages)) do
        local output_binding = "needs_" .. lang .. ":${{steps.toolchains.outputs.needs_" .. lang .. "}}"
        if not compact_workflow:find(output_binding, 1, true) then
            table.insert(missing_output_binding, lang)
        end

        local force_binding = "needs_" .. lang .. "=true"
        if not compact_workflow:find(force_binding, 1, true) then
            table.insert(missing_main_force, lang)
        end
    end

    if #missing_output_binding == 0 and #missing_main_force == 0 then
        return nil
    end

    local parts = {}
    if #missing_output_binding > 0 then
        table.insert(parts,
            "detect outputs for forced main full builds are not normalized through steps.toolchains for: " ..
                table.concat(missing_output_binding, ", "))
    end
    if #missing_main_force > 0 then
        table.insert(parts,
            "forced main full-build path does not explicitly enable toolchains for: " ..
                table.concat(missing_main_force, ", "))
    end

    return ci_path:gsub("\\", "/") .. ": " .. table.concat(parts, "; ")
end

function Validator.validate_build_contracts(root, packages)
    local errors = {}

    local ci_error = Validator.validate_ci_full_build_toolchains(root, packages)
    if ci_error then
        table.insert(errors, ci_error)
    end

    for _, error in ipairs(Validator.validate_lua_isolated_build_files(packages)) do
        table.insert(errors, error)
    end
    for _, error in ipairs(Validator.validate_perl_build_files(packages)) do
        table.insert(errors, error)
    end

    if #errors == 0 then
        return nil
    end

    return table.concat(errors, "\n  - ")
end

function Validator.languages_needing_ci_toolchains(packages)
    local seen = {}
    local langs = {}

    for _, pkg in ipairs(packages) do
        local lang = pkg.language
        if CI_MANAGED_TOOLCHAIN_LANGUAGES[lang] and not seen[lang] then
            seen[lang] = true
            table.insert(langs, lang)
        end
    end

    table.sort(langs)
    return langs
end

function Validator.validate_lua_isolated_build_files(packages)
    local errors = {}

    for _, pkg in ipairs(packages) do
        if pkg.language == "lua" and pkg.path then
            local self_rock = "coding-adventures-" .. pkg.path:match("([^/\\]+)$"):gsub("_", "-")
            local build_lines = {}

            for _, build_path in ipairs(Validator.lua_build_files(pkg.path)) do
                local lines = Validator.read_build_lines(build_path)
                local build_name = build_path:match("([^/\\]+)$")
                build_lines[build_name] = lines
                if #lines > 0 then
                    local foreign_remove = Validator.first_foreign_lua_remove(lines, self_rock)
                    if foreign_remove then
                        table.insert(errors,
                            build_path:gsub("\\", "/") ..
                            ": Lua BUILD removes unrelated rock " .. foreign_remove ..
                            "; isolated package builds should only remove the package they are rebuilding")
                    end

                    local state_machine_index =
                        Validator.first_line_containing(lines, { "../state_machine", "..\\state_machine" })
                    local directed_graph_index =
                        Validator.first_line_containing(lines, { "../directed_graph", "..\\directed_graph" })

                    if state_machine_index and directed_graph_index and state_machine_index < directed_graph_index then
                        table.insert(errors,
                            build_path:gsub("\\", "/") ..
                            ": Lua BUILD installs state_machine before directed_graph; isolated LuaRocks builds require directed_graph first")
                    end

                    if (Validator.guarded_local_lua_install(lines) or
                            (build_name == "BUILD_windows" and Validator.local_lua_sibling_install(lines))) and
                        not Validator.self_install_disables_deps(lines, self_rock)
                    then
                        table.insert(errors,
                            build_path:gsub("\\", "/") ..
                            ": Lua BUILD bootstraps sibling rocks but the final self-install does not pass --deps-mode=none or --no-manifest")
                    end
                end
            end

            local missing_windows_deps =
                Validator.missing_lua_sibling_installs(build_lines.BUILD or {}, build_lines.BUILD_windows or {})
            if #missing_windows_deps > 0 then
                table.insert(errors,
                    (pkg.path .. "/BUILD_windows"):gsub("\\", "/") ..
                    ": Lua BUILD_windows is missing sibling installs present in BUILD: " ..
                    table.concat(missing_windows_deps, ", "))
            end
        end
    end

    return errors
end

function Validator.validate_perl_build_files(packages)
    local errors = {}

    for _, pkg in ipairs(packages) do
        if pkg.language == "perl" and pkg.path then
            for _, build_path in ipairs(Validator.lua_build_files(pkg.path)) do
                local lines = Validator.read_build_lines(build_path)
                for _, line in ipairs(lines) do
                    if line:find("cpanm", 1, true) and
                        line:find("Test2::V0", 1, true) and
                        not line:find("--notest", 1, true)
                    then
                        table.insert(errors,
                            build_path:gsub("\\", "/") ..
                            ": Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself")
                        break
                    end
                end
            end
        end
    end

    return errors
end

function Validator.lua_build_files(pkg_path)
    local files = {}
    local handle

    if package.config:sub(1, 1) == "\\" then
        handle = io.popen('dir /b "' .. pkg_path .. '\\BUILD*" 2>NUL')
    else
        handle = io.popen('find "' .. pkg_path .. '" -maxdepth 1 -type f -name "BUILD*" -exec basename {} \\; 2>/dev/null')
    end

    if not handle then
        return files
    end

    for entry in handle:lines() do
        if entry ~= "" then
            table.insert(files, pkg_path .. "/" .. entry)
        end
    end
    handle:close()

    table.sort(files)
    return files
end

function Validator.read_build_lines(build_path)
    local file = io.open(build_path, "r")
    if not file then
        return {}
    end

    local lines = {}
    for line in file:lines() do
        local trimmed = line:match("^%s*(.-)%s*$")
        if trimmed ~= "" and trimmed:sub(1, 1) ~= "#" then
            table.insert(lines, trimmed)
        end
    end
    file:close()
    return lines
end

function Validator.first_foreign_lua_remove(lines, self_rock)
    for _, line in ipairs(lines) do
        local target = line:match("luarocks remove %-%-force ([^%s]+)")
        if target and target ~= self_rock then
            return target
        end
    end
    return nil
end

function Validator.first_line_containing(lines, needles)
    for index, line in ipairs(lines) do
        for _, needle in ipairs(needles) do
            if line:find(needle, 1, true) then
                return index
            end
        end
    end
    return nil
end

function Validator.guarded_local_lua_install(lines)
    for _, line in ipairs(lines) do
        if line:find("luarocks show ", 1, true) and
            (line:find("../", 1, true) or line:find("..\\", 1, true))
        then
            return true
        end
    end
    return false
end

function Validator.local_lua_sibling_install(lines)
    return #Validator.lua_sibling_install_dirs(lines) > 0
end

function Validator.self_install_disables_deps(lines, self_rock)
    for _, line in ipairs(lines) do
        if line:find("luarocks make", 1, true) and line:find(self_rock, 1, true) and
            (line:find("--deps-mode=none", 1, true) or
                line:find("--deps-mode none", 1, true) or
                line:find("--no-manifest", 1, true))
        then
            return true
        end
    end
    return false
end

function Validator.missing_lua_sibling_installs(unix_lines, windows_lines)
    local windows_deps = {}
    for _, dep in ipairs(Validator.lua_sibling_install_dirs(windows_lines)) do
        windows_deps[dep] = true
    end

    local missing = {}
    for _, dep in ipairs(Validator.lua_sibling_install_dirs(unix_lines)) do
        if not windows_deps[dep] then
            table.insert(missing, dep)
        end
    end
    return missing
end

function Validator.lua_sibling_install_dirs(lines)
    local seen = {}
    local dirs = {}

    for _, line in ipairs(lines) do
        if line:find("luarocks make", 1, true) then
            local dep = line:match("cd%s+([.][.][\\/][^ %(%)\t\r\n&]+)")
            if dep then
                dep = dep:gsub("\\", "/")
                if not seen[dep] then
                    seen[dep] = true
                    table.insert(dirs, dep)
                end
            end
        end
    end

    table.sort(dirs)
    return dirs
end

return Validator
