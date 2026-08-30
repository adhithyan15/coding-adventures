-- Pure, bounded CI toolchain detection over caller-supplied BUILD snapshots.
--
-- This module deliberately does not read the filesystem, environment, or
-- network and does not launch processes. Callers own every byte of input.

local ToolchainDetection = {}

local MAX_BUILD_BYTES = 65536
local MAX_BUILD_LINES = 4096
local MAX_AGGREGATE_BUILD_BYTES = 1048576
local DECLARATION_PREFIX = "# needs-toolchain:"

local CANONICAL_TOOLCHAINS = {
    "cpp",
    "dart",
    "dotnet",
    "elixir",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "ocaml",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
}

local CANONICAL_TOOLCHAIN_SET = {}
for _, toolchain in ipairs(CANONICAL_TOOLCHAINS) do
    CANONICAL_TOOLCHAIN_SET[toolchain] = true
end

local function copy_array(values)
    local result = {}
    for index, value in ipairs(values) do
        result[index] = value
    end
    return result
end

local function is_ascii_space(byte)
    return byte == 32 or byte == 9
end

local function trim_ascii_space(value)
    local first = 1
    local last = #value
    while first <= last and is_ascii_space(value:byte(first)) do
        first = first + 1
    end
    while last >= first and is_ascii_space(value:byte(last)) do
        last = last - 1
    end
    return value:sub(first, last)
end

local function logical_line_count(content)
    local count = 1
    local index = 1
    while true do
        local newline = content:find("\n", index, true)
        if newline == nil then
            return count
        end
        count = count + 1
        index = newline + 1
    end
end

function ToolchainDetection.canonical_toolchains()
    return copy_array(CANONICAL_TOOLCHAINS)
end

function ToolchainDetection.parse_extra_toolchains(content)
    if #content > MAX_BUILD_BYTES or logical_line_count(content) > MAX_BUILD_LINES then
        return {}
    end

    local declarations = {}
    local seen = {}
    local start_index = 1

    while start_index <= #content + 1 do
        local newline = content:find("\n", start_index, true)
        local line
        if newline == nil then
            line = content:sub(start_index)
        else
            line = content:sub(start_index, newline - 1)
            if line:sub(-1) == "\r" then
                line = line:sub(1, -2)
            end
        end

        line = trim_ascii_space(line)
        if line:sub(1, #DECLARATION_PREFIX) == DECLARATION_PREFIX then
            local suffix = line:sub(#DECLARATION_PREFIX + 1)
            local first = suffix:byte(1)
            if is_ascii_space(first) then
                local name = trim_ascii_space(suffix)
                if CANONICAL_TOOLCHAIN_SET[name] and not seen[name] then
                    declarations[#declarations + 1] = name
                    seen[name] = true
                end
            end
        end

        if newline == nil then
            break
        end
        start_index = newline + 1
    end

    return declarations
end

local function validate_snapshot_limits(packages)
    local aggregate_bytes = 0
    for _, package_snapshot in ipairs(packages) do
        for _, content in pairs(package_snapshot.build_files) do
            local bytes = #content
            if bytes > MAX_BUILD_BYTES or logical_line_count(content) > MAX_BUILD_LINES then
                error("toolchain BUILD snapshot exceeds its per-file resource ceiling", 0)
            end
            aggregate_bytes = aggregate_bytes + bytes
        end
    end
    if aggregate_bytes > MAX_AGGREGATE_BUILD_BYTES then
        error("toolchain BUILD snapshot exceeds its aggregate resource ceiling", 0)
    end
end

local function build_file_candidates(platform)
    if platform == "darwin" then
        return {"BUILD_mac", "BUILD_mac_and_linux", "BUILD"}
    elseif platform == "linux" then
        return {"BUILD_linux", "BUILD_mac_and_linux", "BUILD"}
    elseif platform == "windows" or platform == "win32" then
        return {"BUILD_windows", "BUILD"}
    end
    error("unsupported target platform: " .. tostring(platform), 0)
end

local function selected_front(build_files, platform)
    for _, filename in ipairs(build_file_candidates(platform)) do
        if build_files[filename] ~= nil then
            return build_files[filename]
        end
    end
    return ""
end

local function toolchain_for_language(language)
    if language == "wasm" then
        return "rust"
    elseif language == "c" or language == "cpp" then
        return "cpp"
    elseif language == "csharp" or language == "fsharp" or language == "dotnet" then
        return "dotnet"
    elseif CANONICAL_TOOLCHAIN_SET[language] then
        return language
    end
    return nil
end

local function unsupported(package_name)
    local diagnostic = {code = "TOOLCHAIN_UNSUPPORTED", severity = "error"}
    if package_name ~= nil then
        diagnostic.package = package_name
    end
    return {outcome = "error", toolchains = {}, diagnostics = {diagnostic}}
end

function ToolchainDetection.evaluate_snapshot(
    platform,
    force_full,
    packages,
    scheduled_packages,
    forced_toolchains
)
    validate_snapshot_limits(packages)

    local scheduled = nil
    if scheduled_packages ~= nil then
        scheduled = {}
        for _, package_name in ipairs(scheduled_packages) do
            scheduled[package_name] = true
        end
    end

    local toolchains = {}
    for _, toolchain in ipairs(CANONICAL_TOOLCHAINS) do
        toolchains[toolchain] = force_full
    end

    for _, package_snapshot in ipairs(packages) do
        if scheduled == nil or scheduled[package_snapshot.name] then
            local toolchain = toolchain_for_language(package_snapshot.language)
            if toolchain == nil then
                return unsupported(package_snapshot.name)
            end

            if not force_full then
                toolchains[toolchain] = true
                local content = selected_front(package_snapshot.build_files, platform)
                for _, extra in ipairs(ToolchainDetection.parse_extra_toolchains(content)) do
                    toolchains[extra] = true
                end
            end
        end
    end

    for _, forced in ipairs(forced_toolchains) do
        if not CANONICAL_TOOLCHAIN_SET[forced] then
            return unsupported(nil)
        end
        toolchains[forced] = true
    end

    return {outcome = "ok", toolchains = toolchains, diagnostics = {}}
end

return ToolchainDetection
