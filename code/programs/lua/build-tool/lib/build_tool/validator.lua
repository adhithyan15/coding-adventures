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
