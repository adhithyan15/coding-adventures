local Validator = {}

local CI_MANAGED_TOOLCHAIN_LANGUAGES = {
    python = true,
    ruby = true,
    typescript = true,
    rust = true,
    elixir = true,
    lua = true,
    perl = true,
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

return Validator
