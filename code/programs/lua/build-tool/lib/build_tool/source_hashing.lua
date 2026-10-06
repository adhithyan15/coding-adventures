-- source_hashing.lua -- process-free source selection and SHA-256 framing.
--
-- Every candidate and byte arrives from the caller. The only authority this
-- module imports is the generated, reviewed selector table installed beside
-- it; it never enumerates a directory or launches a program.
local packaged_snapshot = require("build_tool.source_registry_data")
local Unicode = require("build_tool.tracked_artifact_unicode17")
local sha256 = require("coding_adventures.sha256")

local function copy(value)
    if type(value) ~= "table" then return value end
    local result = {}
    for key, member in pairs(value) do result[key] = copy(member) end
    return result
end

-- Do not share the production selector table with package.loaded. Even a
-- caller that imports the generated module directly cannot mutate this copy.
local snapshot = copy(packaged_snapshot)
local SourceHashing = {}
local MAX_CANDIDATES = 100000
local MAX_SELECTED = 50000
local MAX_FILE_BYTES = 64 * 1024 * 1024
local MAX_PACKAGE_BYTES = 1024 * 1024 * 1024
local MAX_GLOB_WORK = 50000000

local languages = {}
for _, entry in ipairs(snapshot.data.languages) do
    languages[entry.language] = entry
end

local generated = {}
for _, component in ipairs(snapshot.data.universal_inputs.generated_directory_components) do
    generated[component] = true
end

function SourceHashing.registry()
    -- Return a fresh value: callers cannot mutate the production projection.
    return copy(snapshot.data)
end

function SourceHashing.registry_digest()
    return snapshot.digest
end

local function reject(message)
    error("SOURCE_HASH_INVALID_INPUT: " .. message, 3)
end

local reserved = {
    CON = true, PRN = true, AUX = true, NUL = true,
    COM1 = true, COM2 = true, COM3 = true, COM4 = true, COM5 = true,
    COM6 = true, COM7 = true, COM8 = true, COM9 = true,
    LPT1 = true, LPT2 = true, LPT3 = true, LPT4 = true, LPT5 = true,
    LPT6 = true, LPT7 = true, LPT8 = true, LPT9 = true,
}

local function parts_of(path)
    if type(path) ~= "string" or path == "" or path:sub(1, 1) == "/"
        or path:find("\\", 1, true) or path:find(":", 1, true)
        or path:find('[<>"|?*]')
        or not utf8.len(path) or utf8.len(path) > 4096
        or Unicode.nfc(path) ~= path then
        reject("non-portable relative path")
    end
    for _, scalar in utf8.codes(path) do
        if scalar < 32 or (scalar >= 0x7F and scalar <= 0x9F)
            or (scalar >= 0x200B and scalar <= 0x200F)
            or (scalar >= 0x202A and scalar <= 0x202E)
            or (scalar >= 0x2060 and scalar <= 0x206F)
            or scalar == 0xFEFF then
            reject("control or format path character")
        end
    end
    local parts = {}
    for part in (path .. "/"):gmatch("(.-)/") do
        if part == "" or part == "." or part == ".."
            or part:find("[%z\1-\31\127]") or part:find("[%. ]$") then
            reject("non-portable path component")
        end
        local stem = part:match("^([^.]+)") or part
        if reserved[Unicode.full_uppercase(stem)] then reject("reserved path component") end
        parts[#parts + 1] = part
    end
    return parts
end

local function basename(path)
    return path:match("([^/]+)$")
end

local function suffix_match(name, suffixes)
    for _, suffix in ipairs(suffixes) do
        if #name > #suffix and name:sub(-#suffix) == suffix then return true end
    end
    return false
end

local function exact_match(name, values)
    for _, value in ipairs(values) do
        if name == value then return true end
    end
    return false
end

local function split(path)
    local result = {}
    for part in (path .. "/"):gmatch("(.-)/") do result[#result + 1] = part end
    return result
end

local function scalars(value)
    local result = {}
    for _, scalar in utf8.codes(value) do result[#result + 1] = utf8.char(scalar) end
    return result
end

local function compile_segment(pattern)
    -- Tokens feed a dynamic-programming matcher. This avoids Lua-pattern
    -- backtracking even for hostile runs of '*' and character classes.
    local characters = scalars(pattern)
    local tokens = {}
    local i = 1
    while i <= #characters do
        local char = characters[i]
        if char == "*" then
            tokens[#tokens + 1] = {kind = "star"}
        elseif char == "?" then
            tokens[#tokens + 1] = {kind = "one"}
        elseif char == "[" then
            local first = i + 1
            local negated = characters[first] == "!"
            if negated then first = first + 1 end
            local close = first
            if characters[close] == "]" then close = close + 1 end
            while close <= #characters and characters[close] ~= "]" do
                close = close + 1
            end
            if close > #characters then
                tokens[#tokens + 1] = {kind = "literal", value = "["}
            else
                local content = {}
                for position = first, close - 1 do
                    content[#content + 1] = characters[position]
                end
                if #content == 0 then reject("empty glob class") end
                for position = 1, #content - 1 do
                    local pair = content[position] .. content[position + 1]
                    if pair == "--" or pair == "&&" or pair == "~~" or pair == "||" then
                        reject("ambiguous glob class")
                    end
                end
                local ranges = {}
                local position = 1
                while position <= #content do
                    local low = utf8.codepoint(content[position])
                    if position + 2 <= #content and content[position + 1] == "-" then
                        local high = utf8.codepoint(content[position + 2])
                        if low > high then reject("descending glob range") end
                        ranges[#ranges + 1] = {low, high}
                        position = position + 3
                    else
                        ranges[#ranges + 1] = {low, low}
                        position = position + 1
                    end
                end
                tokens[#tokens + 1] = {kind = "class", ranges = ranges, negated = negated}
                i = close
            end
        else
            tokens[#tokens + 1] = {kind = "literal", value = char}
        end
        i = i + 1
    end
    return tokens
end

local function segment_matches(tokens, value)
    local characters = scalars(value)
    local previous = {[0] = true}
    for _, token in ipairs(tokens) do
        local current = {[0] = token.kind == "star" and previous[0] or false}
        for index = 1, #characters do
            if token.kind == "star" then
                current[index] = previous[index] or current[index - 1] or false
            elseif previous[index - 1] then
                local candidate = characters[index]
                if token.kind == "one" then
                    current[index] = true
                elseif token.kind == "literal" then
                    current[index] = candidate == token.value
                else
                    local scalar = utf8.codepoint(candidate)
                    local inside = false
                    for _, range in ipairs(token.ranges) do
                        if scalar >= range[1] and scalar <= range[2] then
                            inside = true; break
                        end
                    end
                    current[index] = inside ~= token.negated
                end
            end
        end
        previous = current
    end
    return previous[#characters] or false
end

local function compile_globs(patterns)
    if type(patterns) ~= "table" then reject("declared patterns must be an array") end
    local compiled = {}
    for _, pattern in ipairs(patterns) do
        if type(pattern) ~= "string" or pattern == "" or pattern:sub(1, 1) == "/"
            or pattern:find("\\", 1, true)
            or not utf8.len(pattern) or utf8.len(pattern) > 4096
            or Unicode.nfc(pattern) ~= pattern then
            reject("invalid declared pattern")
        end
        local segments = split(pattern)
        local result = {}
        for _, segment in ipairs(segments) do
            if segment == "" or segment == "." or segment == ".." then
                reject("invalid declared pattern segment")
            end
            result[#result + 1] = segment == "**" and "**" or compile_segment(segment)
        end
        compiled[#compiled + 1] = {segments = result, scalar_count = utf8.len(pattern)}
    end
    return compiled
end

local function glob_matches(pattern, path)
    local components = split(path)
    local previous = {[0] = true}
    for _, segment in ipairs(pattern) do
        local current = {[0] = segment == "**" and previous[0] or false}
        for index = 1, #components do
            if segment == "**" then
                current[index] = previous[index] or current[index - 1] or false
            else
                current[index] = previous[index - 1]
                    and segment_matches(segment, components[index]) or false
            end
        end
        previous = current
    end
    return previous[#components] or false
end

local function scoped_match(rule, path, name)
    if rule.scope == "root" then
        if path:find("/", 1, true) then return false end
    elseif rule.scope == "subtree" then
        if path:sub(1, #rule.path_prefix + 1) ~= rule.path_prefix .. "/" then
            return false
        end
    else
        reject("unsupported registry scope")
    end
    return exact_match(name, rule.exact_basenames)
        or suffix_match(name, rule.suffixes)
end

local function chosen_by_registry(entry, root, path, mode, declared_matches)
    local name = basename(path)
    local root_file = not path:find("/", 1, true)
    local universal = snapshot.data.universal_inputs
    if exact_match(name, universal.build_filenames) then return true end
    if root_file and (exact_match(name, universal.root_exact_basenames)
        or exact_match(name, entry.root_exact_basenames)
        or suffix_match(name, entry.root_variable_suffixes)) then return true end
    if exact_match(path, entry.root_exact_relative_paths) then return true end
    for _, exact in ipairs(entry.package_exact_inputs) do
        if root == exact.package_root and exact_match(path, exact.paths) then
            return true
        end
    end
    if mode == "declared_sources" then
        return declared_matches(path)
    end
    if suffix_match(name, entry.recursive_suffixes)
        or exact_match(name, entry.recursive_exact_basenames) then return true end
    for _, rule in ipairs(entry.scoped_inputs) do
        if scoped_match(rule, path, name) then return true end
    end
    return false
end

local function hex_bytes(hex)
    if type(hex) ~= "string" or #hex % 2 ~= 0 or hex:find("[^0-9a-fA-F]") then
        reject("invalid content hex")
    end
    return (hex:gsub("..", function(pair)
        return string.char(tonumber(pair, 16))
    end))
end

local function validate_root(root, language, entry)
    local parts = parts_of(root)
    if #parts == 3 and parts[1] == "code" and parts[2] == "sites" then
        if language ~= "typescript" then reject("site root has wrong language") end
        for _, exact in ipairs(entry.package_exact_inputs) do
            if exact.package_root == root then return end
        end
        reject("unregistered site root")
    end
    if #parts ~= 4 or parts[1] ~= "code"
        or (parts[2] ~= "packages" and parts[2] ~= "programs")
        or parts[3] ~= language then
        reject("package root does not match language")
    end
end

function SourceHashing.collect_source_files(options)
    if type(options) ~= "table" then reject("options must be a table") end
    -- Reject the lane and the registry identity before visiting candidates.
    local entry = languages[options.language]
    if not entry then reject("unknown language") end
    if options.registry_sha256 ~= snapshot.digest then reject("registry digest mismatch") end
    if options.mode ~= "extension" and options.mode ~= "declared_sources" then
        reject("unsupported source collection mode")
    end
    validate_root(options.package_root, options.language, entry)
    local globs = compile_globs(options.declared_srcs)
    local glob_work = 0
    local function declared_matches(path)
        local scalar_count = utf8.len(path)
        for _, glob in ipairs(globs) do
            local cost = (glob.scalar_count + 1) * (scalar_count + 1)
            if cost > MAX_GLOB_WORK - glob_work then
                reject("declared glob work limit")
            end
            glob_work = glob_work + cost
            if glob_matches(glob.segments, path) then return true end
        end
        return false
    end
    if type(options.candidates) ~= "table" or #options.candidates > MAX_CANDIDATES then
        reject("candidate limit or shape")
    end

    local seen, aliases, links, files = {}, {}, {}, {}
    for _, candidate in ipairs(options.candidates) do
        if type(candidate) ~= "table" then reject("candidate shape") end
        local path = candidate.path
        parts_of(path)
        if seen[path] then reject("duplicate candidate path") end
        local alias = Unicode.casefold(path)
        if aliases[alias] and aliases[alias] ~= path then
            reject("candidate platform-identity alias")
        end
        aliases[alias] = path
        seen[path] = candidate.kind
        if candidate.kind == "symlink" or candidate.kind == "reparse_point" then
            links[#links + 1] = path
        elseif candidate.kind ~= "file" then
            reject("unsupported candidate kind")
        end
    end
    for path in pairs(seen) do
        local prefix = path:match("^(.*)/[^/]+$")
        while prefix do
            if seen[prefix] == "file" then reject("file-prefix collision") end
            prefix = prefix:match("^(.*)/[^/]+$")
        end
    end

    local selected_bytes = 0
    for _, candidate in ipairs(options.candidates) do
        local path = candidate.path
        local pruned = false
        local components = parts_of(path)
        for index = 1, #components - 1 do
            if generated[components[index]] then pruned = true; break end
        end
        if not pruned then
            for _, link in ipairs(links) do
                if path == link or path:sub(1, #link + 1) == link .. "/" then
                    pruned = true; break
                end
            end
        end
        if not pruned and candidate.kind == "file"
            and chosen_by_registry(entry, options.package_root, path, options.mode, declared_matches) then
            local bytes = candidate.content_hex and hex_bytes(candidate.content_hex)
                or candidate.content_utf8
            if type(bytes) ~= "string" or #bytes > MAX_FILE_BYTES then
                reject("selected file bytes or limit")
            end
            selected_bytes = selected_bytes + #bytes
            if selected_bytes > MAX_PACKAGE_BYTES or #files >= MAX_SELECTED then
                reject("selected package limit")
            end
            files[#files + 1] = {path = path, digest = sha256.sha256_hex(bytes)}
        end
    end
    table.sort(files, function(left, right) return left.path < right.path end)
    return files
end

function SourceHashing.package_digest(include_paths, contents)
    if type(include_paths) ~= "table" or type(contents) ~= "table"
        or #include_paths > MAX_SELECTED then reject("package digest input shape") end
    local selected, aliases = {}, {}
    for _, path in ipairs(include_paths) do
        parts_of(path)
        local alias = Unicode.casefold(path)
        if aliases[alias] and aliases[alias] ~= path then
            reject("package input platform-identity alias")
        end
        aliases[alias] = path
        if type(contents[path]) ~= "string" or #contents[path] > MAX_FILE_BYTES then
            reject("missing or oversized package input")
        end
        selected[path] = true
    end
    local sorted = {}
    for path in pairs(selected) do sorted[#sorted + 1] = path end
    table.sort(sorted)
    local digest = sha256.new()
    local total = 0
    for _, path in ipairs(sorted) do
        local bytes = contents[path]
        total = total + #bytes
        if total > MAX_PACKAGE_BYTES then reject("package byte limit") end
        digest:update(string.pack(">I8", #path))
        digest:update(path)
        digest:update(string.pack(">I8", #bytes))
        digest:update(bytes)
    end
    return digest:hex_digest()
end

return SourceHashing
