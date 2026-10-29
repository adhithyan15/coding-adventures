-- graph_diff.lua -- pure, bounded graph and diff-selection operations.
--
-- Discovery and Git belong outside this module.  The two public functions
-- accept only already-materialized values and return one complete result or
-- one stable error code.  In particular, validation and match-work preflight
-- finish before selection, so cycles and invalid paths cannot leak a partial
-- build plan.
local Unicode = require("build_tool.tracked_artifact_unicode17")
local sha256 = require("coding_adventures.sha256")

local GraphDiff = {}

local MAX_PACKAGES = 4096
local MAX_EDGES = 16384
local MAX_SOURCE_GLOBS = 256
local MAX_PATH_SCALARS = 512
local MAX_NAME_SCALARS = 240
local MAX_MATCH_WORK = 50000000
local MAX_BOUNDARY_DEPTH = 16
local MAX_BOUNDARY_NODES = 65536
local MAX_BOUNDARY_CONTAINERS = 4096
local MAX_BOUNDARY_STRING_BYTES = 16777216
local BOUNDARY_DOMAIN = "coding-adventures/build-tool-repository-source-input-boundary/v1\0"

local BUILD_FRONTS = {
    BUILD = true,
    BUILD_windows = true,
    BUILD_mac = true,
    BUILD_linux = true,
    BUILD_mac_and_linux = true,
}

local BOUNDARY_ORIGINS = {}
for _, origin in ipairs({
    "c", "cpp", "csharp", "dart", "dotnet", "elixir", "fsharp", "go",
    "haskell", "java", "kotlin", "lua", "mosaic", "ocaml", "perl",
    "python", "repository", "ruby", "rust", "starlark", "swift", "twig",
    "typescript", "wasm",
}) do
    BOUNDARY_ORIGINS[origin] = true
end

local RESERVED = {
    CON = true, PRN = true, AUX = true, NUL = true,
    ["CONIN$"] = true, ["CONOUT$"] = true, ["CLOCK$"] = true,
    ["COM¹"] = true, ["COM²"] = true, ["COM³"] = true,
    ["LPT¹"] = true, ["LPT²"] = true, ["LPT³"] = true,
}
for index = 1, 9 do
    RESERVED["COM" .. index] = true
    RESERVED["LPT" .. index] = true
end

local ARRAY_MT = {__graph_diff_array = true}

local function contract(code)
    error({graph_diff_contract = true, code = code}, 0)
end

local function require_contract(condition, code)
    if not condition then contract(code) end
end

local function success(value)
    return {ok = true, value = value}
end

local function failure(code)
    return {ok = false, code = code}
end

local function protect(fallback, operation)
    local ok, value = pcall(operation)
    if ok then return value end
    if type(value) == "table" and rawget(value, "graph_diff_contract") == true then
        return failure(rawget(value, "code"))
    end
    return failure(fallback)
end

local function accepted_metatable(value, expected)
    local mt = getmetatable(value)
    if mt == nil then return true end
    if mt == ARRAY_MT then return expected == "array" end
    if type(mt) ~= "table" then return false end
    local kind = rawget(mt, "__jsontype")
    if kind ~= expected then return false end
    local count = 0
    for key in next, mt do
        count = count + 1
        if key ~= "__jsontype" then return false end
    end
    return count == 1
end

-- Lua has one table type for arrays and objects.  This helper accepts plain
-- dense arrays as well as dkjson's inert array marker, but rejects holes,
-- metamethod-bearing values, and non-integer keys before indexing members.
local function dense_count(value, maximum, shape_code, overflow_code)
    require_contract(type(value) == "table" and accepted_metatable(value, "array"),
        shape_code)
    local count, keys = 0, {}
    for key in next, value do
        count = count + 1
        require_contract(count <= maximum, overflow_code)
        keys[count] = key
    end
    for index = 1, count do
        local key = keys[index]
        require_contract(type(key) == "number" and key % 1 == 0 and key >= 1,
            shape_code)
    end
    for index = 1, count do
        require_contract(rawget(value, index) ~= nil, shape_code)
    end
    return count
end

local function inert_record(value, code)
    require_contract(type(value) == "table" and accepted_metatable(value, "object"), code)
    for key in next, value do require_contract(type(key) == "string", code) end
    return value
end

local function snapshot_array(value, maximum, shape_code, overflow_code, snapshot_member)
    local count = dense_count(value, maximum, shape_code, overflow_code)
    local result = {}
    for index = 1, count do
        result[index] = snapshot_member(rawget(value, index))
    end
    return result
end

local function snapshot_graph_input(input)
    input = inert_record(input, "GRAPH_INPUT_INVALID")
    return {
        packages = snapshot_array(rawget(input, "packages"), MAX_PACKAGES,
            "GRAPH_INPUT_INVALID", "GRAPH_PACKAGE_LIMIT_EXCEEDED",
            function(value) return value end),
        edges = snapshot_array(rawget(input, "edges"), MAX_EDGES,
            "GRAPH_INPUT_INVALID", "GRAPH_EDGE_LIMIT_EXCEEDED",
            function(value)
                return snapshot_array(value, 2, "GRAPH_INPUT_INVALID", "GRAPH_EDGE_UNKNOWN",
                    function(member) return member end)
            end),
    }
end

local function snapshot_diff_input(input)
    input = inert_record(input, "DIFF_INPUT_INVALID")
    local result = {}
    result.packages = snapshot_array(rawget(input, "packages"), MAX_PACKAGES,
        "DIFF_INPUT_INVALID", "DIFF_PACKAGE_INVALID",
        function(value)
            value = inert_record(value, "DIFF_INPUT_INVALID")
            local globs = rawget(value, "source_globs")
            return {
                name = rawget(value, "name"),
                rel_path = rawget(value, "rel_path"),
                source_mode = rawget(value, "source_mode"),
                source_globs = snapshot_array(globs, MAX_SOURCE_GLOBS,
                    "DIFF_INPUT_INVALID", "DIFF_GLOB_INVALID",
                    function(glob) return glob end),
            }
        end)
    result.edges = snapshot_array(rawget(input, "edges"), MAX_EDGES,
        "DIFF_INPUT_INVALID", "GRAPH_EDGE_LIMIT_EXCEEDED",
        function(value)
            return snapshot_array(value, 2, "DIFF_INPUT_INVALID", "GRAPH_EDGE_UNKNOWN",
                function(member) return member end)
        end)
    result.forced_packages = snapshot_array(rawget(input, "forced_packages"),
        MAX_PACKAGES, "DIFF_INPUT_INVALID", "DIFF_FORCED_PACKAGE_INVALID",
        function(value) return value end)
    result.changed_paths = snapshot_array(rawget(input, "changed_paths"),
        MAX_PACKAGES, "DIFF_INPUT_INVALID", "DIFF_PATH_INVALID",
        function(value) return value end)
    result.unknown_path_policy = rawget(input, "unknown_path_policy")
    local digest = rawget(input, "boundary_sha256")
    require_contract(digest == nil or type(digest) == "string", "DIFF_INPUT_INVALID")
    result.boundary_sha256 = digest
    result.boundary = rawget(input, "boundary")
    return result
end

local function scalar_length(value)
    local count = utf8.len(value)
    return count
end

local function valid_utf8(value)
    return type(value) == "string" and utf8.len(value) ~= nil
end

local function valid_package_name(value)
    if not valid_utf8(value) or scalar_length(value) > MAX_NAME_SCALARS then return false end
    local segments = 0
    for segment in (value .. "/"):gmatch("(.-)/") do
        if not segment:match("^[a-z0-9][a-z0-9._-]*$") then return false end
        segments = segments + 1
    end
    return segments >= 2
end

local function reserved(segment)
    local stem = segment:match("^([^.]+)") or segment
    return RESERVED[Unicode.full_uppercase(stem)] == true
end

local function valid_portable_prefix(value, forbidden)
    if not valid_utf8(value) or value == "" or scalar_length(value) > MAX_PATH_SCALARS
        or Unicode.nfc(value) ~= value or value:sub(1, 1) == "/"
        or value:match("^[A-Za-z]:") or value:find("\\", 1, true)
        or value:find("//", 1, true) then
        return false
    end
    for _, scalar in utf8.codes(value) do
        local character = utf8.char(scalar)
        if scalar < 32 or forbidden:find(character, 1, true) then return false end
    end
    return true
end

local function valid_path_segment(segment)
    return segment ~= "" and segment ~= "." and segment ~= ".."
        and not segment:match("[. ]$") and not reserved(segment)
end

local function valid_portable_path(value)
    if not valid_portable_prefix(value, '<>:"|?*') then return false end
    for segment in (value .. "/"):gmatch("(.-)/") do
        if not valid_path_segment(segment) then return false end
    end
    return true
end

local function valid_portable_glob(value)
    if not valid_portable_prefix(value, '<>:"|') then return false end
    for segment in (value .. "/"):gmatch("(.-)/") do
        if segment == "" or segment == "." or segment == ".."
            or segment:match("[. ]$") then return false end
        if not segment:find("[*?%[%]{}]") and reserved(segment) then return false end
    end
    return true
end

local function scalars(value)
    local result = {}
    for _, scalar in utf8.codes(value) do result[#result + 1] = scalar end
    return result
end

local function split_path(value)
    local result = {}
    if value == "" then return result end
    for segment in (value .. "/"):gmatch("(.-)/") do result[#result + 1] = segment end
    return result
end

-- A segment compiles to scalar tokens.  Matching uses a rolling dynamic
-- program, so every (token, scalar) state is visited at most once even for a
-- hostile run of stars.  A whole ** segment receives the analogous path-level
-- dynamic program below.
local function compile_segment(value)
    local characters = scalars(value)
    local tokens = {}
    -- close_at_or_after makes every '[' lookup constant-time.  Without this
    -- reverse index, a run of unmatched opening brackets rescans the remaining
    -- suffix once per scalar and turns compilation quadratic.
    local close_at_or_after = {}
    local next_close
    for position = #characters, 1, -1 do
        if characters[position] == 93 then next_close = position end
        close_at_or_after[position] = next_close
    end
    local index = 1
    while index <= #characters do
        local scalar = characters[index]
        if scalar == 42 then
            tokens[#tokens + 1] = {kind = "star"}
        elseif scalar == 63 then
            tokens[#tokens + 1] = {kind = "one"}
        elseif scalar == 91 then
            local first = index + 1
            local negated = characters[first] == 33
            if negated then first = first + 1 end
            local search_from = first
            if characters[search_from] == 93 then search_from = search_from + 1 end
            local close = close_at_or_after[search_from]
            if close == nil then
                tokens[#tokens + 1] = {kind = "literal", scalar = 91}
            else
                local content = {}
                for position = first, close - 1 do content[#content + 1] = characters[position] end
                require_contract(#content > 0, "DIFF_GLOB_INVALID")
                for position = 1, #content - 1 do
                    local left, right = content[position], content[position + 1]
                    require_contract(not ((left == 45 and right == 45)
                        or (left == 38 and right == 38)
                        or (left == 126 and right == 126)
                        or (left == 124 and right == 124)), "DIFF_GLOB_INVALID")
                end
                local ranges = {}
                local position = 1
                while position <= #content do
                    local low = content[position]
                    if position + 2 <= #content and content[position + 1] == 45 then
                        local high = content[position + 2]
                        require_contract(low <= high, "DIFF_GLOB_INVALID")
                        ranges[#ranges + 1] = {low, high}
                        position = position + 3
                    else
                        ranges[#ranges + 1] = {low, low}
                        position = position + 1
                    end
                end
                tokens[#tokens + 1] = {kind = "class", ranges = ranges, negated = negated}
                index = close
            end
        else
            tokens[#tokens + 1] = {kind = "literal", scalar = scalar}
        end
        index = index + 1
    end
    return tokens
end

local function compile_glob(value)
    local result = {}
    for _, segment in ipairs(split_path(value)) do
        result[#result + 1] = segment == "**" and "**" or compile_segment(segment)
    end
    return result
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
                    current[index] = candidate == token.scalar
                else
                    local inside = false
                    for _, range in ipairs(token.ranges) do
                        if candidate >= range[1] and candidate <= range[2] then
                            inside = true
                            break
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

local function glob_matches(pattern, path)
    local components = split_path(path)
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

local function canonical_edge_less(left, right)
    return left[1] < right[1] or (left[1] == right[1] and left[2] < right[2])
end

local function validate_graph(packages, edges)
    local package_count = dense_count(packages, MAX_PACKAGES,
        "GRAPH_PACKAGE_LIMIT_EXCEEDED", "GRAPH_PACKAGE_LIMIT_EXCEEDED")
    local edge_count = dense_count(edges, MAX_EDGES,
        "GRAPH_EDGE_LIMIT_EXCEEDED", "GRAPH_EDGE_LIMIT_EXCEEDED")

    local names, ordered_names = {}, {}
    for index = 1, package_count do
        local name = rawget(packages, index)
        require_contract(valid_package_name(name), "GRAPH_PACKAGE_INVALID")
        require_contract(not names[name], "GRAPH_PACKAGE_DUPLICATE")
        names[name] = true
        ordered_names[#ordered_names + 1] = name
    end

    local graph = {names = names, ordered_names = ordered_names, edges = {},
        dependents = {}, prerequisites = {}}
    for name in pairs(names) do
        graph.dependents[name], graph.prerequisites[name] = {}, {}
    end
    local seen = {}
    for index = 1, edge_count do
        local edge = rawget(edges, index)
        require_contract(dense_count(edge, 2, "GRAPH_EDGE_UNKNOWN", "GRAPH_EDGE_UNKNOWN") == 2,
            "GRAPH_EDGE_UNKNOWN")
        local prerequisite, dependent = rawget(edge, 1), rawget(edge, 2)
        require_contract(type(prerequisite) == "string" and type(dependent) == "string"
            and names[prerequisite] and names[dependent], "GRAPH_EDGE_UNKNOWN")
        require_contract(prerequisite ~= dependent, "GRAPH_EDGE_SELF")
        local identity = prerequisite .. "\0" .. dependent
        require_contract(not seen[identity], "GRAPH_EDGE_DUPLICATE")
        seen[identity] = true
        graph.edges[#graph.edges + 1] = {prerequisite, dependent}
        graph.dependents[prerequisite][#graph.dependents[prerequisite] + 1] = dependent
        graph.prerequisites[dependent][#graph.prerequisites[dependent] + 1] = prerequisite
    end
    table.sort(graph.edges, canonical_edge_less)
    for name in pairs(names) do
        table.sort(graph.dependents[name])
        table.sort(graph.prerequisites[name])
    end
    return graph
end

local function graph_levels(graph)
    local indegree, ready = {}, {}
    for name in pairs(graph.names) do
        indegree[name] = #graph.prerequisites[name]
        if indegree[name] == 0 then ready[#ready + 1] = name end
    end
    table.sort(ready)
    local levels, visited = {}, 0
    while #ready > 0 do
        local level, following = ready, {}
        levels[#levels + 1] = level
        visited = visited + #level
        for _, name in ipairs(level) do
            for _, dependent in ipairs(graph.dependents[name]) do
                indegree[dependent] = indegree[dependent] - 1
                if indegree[dependent] == 0 then following[#following + 1] = dependent end
            end
        end
        table.sort(following)
        ready = following
    end
    if visited ~= #graph.ordered_names then return nil end
    return levels
end

local function same_set(left, right)
    local left_count, right_count = 0, 0
    for name in pairs(left) do
        left_count = left_count + 1
        if not right[name] then return false end
    end
    for _ in pairs(right) do right_count = right_count + 1 end
    return left_count == right_count
end

local function validate_root_identities(packages)
    local root = {children = {}}
    for _, package in ipairs(packages) do
        local identity = Unicode.casefold(Unicode.nfc(package.rel_path))
        local node = root
        for _, segment in ipairs(split_path(identity)) do
            require_contract(node.terminal ~= true, "DIFF_PATH_INVALID")
            local child = node.children[segment]
            if child == nil then
                child = {children = {}}
                node.children[segment] = child
            end
            node = child
        end
        require_contract(node.terminal ~= true and next(node.children) == nil,
            "DIFF_PATH_INVALID")
        node.terminal = true
    end
end

local function validate_diff_input(input, graph)
    local package_count = dense_count(rawget(input, "packages"), MAX_PACKAGES,
        "DIFF_PACKAGE_INVALID", "DIFF_PACKAGE_INVALID")
    local packages, names = {}, {}
    for index = 1, package_count do
        local raw_package = inert_record(rawget(rawget(input, "packages"), index),
            "DIFF_PACKAGE_INVALID")
        local name = rawget(raw_package, "name")
        local rel_path = rawget(raw_package, "rel_path")
        local source_mode = rawget(raw_package, "source_mode")
        local source_globs = rawget(raw_package, "source_globs")
        require_contract(valid_package_name(name), "DIFF_PACKAGE_INVALID")
        require_contract(valid_portable_path(rel_path), "DIFF_PATH_INVALID")
        require_contract(source_mode == "package_prefix" or source_mode == "strict_globs",
            "DIFF_SOURCE_MODE_INVALID")
        local glob_count = dense_count(source_globs, MAX_SOURCE_GLOBS,
            "DIFF_GLOB_INVALID", "DIFF_GLOB_INVALID")
        local globs, compiled, seen_globs = {}, {}, {}
        for glob_index = 1, glob_count do
            local glob = rawget(source_globs, glob_index)
            require_contract(valid_portable_glob(glob) and not seen_globs[glob],
                "DIFF_GLOB_INVALID")
            seen_globs[glob] = true
            globs[#globs + 1] = glob
            compiled[#compiled + 1] = compile_glob(glob)
        end
        require_contract(source_mode == "strict_globs" or #globs == 0,
            "DIFF_GLOB_INVALID")
        require_contract(not names[name], "DIFF_PACKAGE_DUPLICATE")
        names[name] = true
        packages[#packages + 1] = {name = name, rel_path = rel_path,
            source_mode = source_mode, source_globs = globs, compiled_globs = compiled}
    end
    require_contract(same_set(names, graph.names), "DIFF_PACKAGE_INVALID")
    validate_root_identities(packages)

    local policy = rawget(input, "unknown_path_policy")
    require_contract(policy == "all" or policy == "error", "DIFF_POLICY_INVALID")
    local forced = rawget(input, "forced_packages")
    local forced_count = dense_count(forced, MAX_PACKAGES,
        "DIFF_FORCED_PACKAGE_INVALID", "DIFF_FORCED_PACKAGE_INVALID")
    local seen_forced = {}
    for index = 1, forced_count do
        local name = rawget(forced, index)
        require_contract(valid_package_name(name) and not seen_forced[name],
            "DIFF_FORCED_PACKAGE_INVALID")
        seen_forced[name] = true
        require_contract(names[name], "DIFF_FORCED_PACKAGE_UNKNOWN")
    end
    local changed_paths = rawget(input, "changed_paths")
    local changed_count = dense_count(changed_paths, MAX_PACKAGES,
        "DIFF_PATH_INVALID", "DIFF_PATH_INVALID")
    local seen_paths = {}
    for index = 1, changed_count do
        local path = rawget(changed_paths, index)
        require_contract(valid_portable_path(path) and not seen_paths[path], "DIFF_PATH_INVALID")
        seen_paths[path] = true
    end
    return packages, forced, forced_count, changed_paths, changed_count, policy
end

local function exact_keys(value, expected)
    local actual, wanted = {}, {}
    for key in next, value do actual[key] = true end
    for _, key in ipairs(expected) do wanted[key] = true end
    return same_set(actual, wanted)
end

local function valid_slug(value, maximum)
    if not valid_utf8(value) or scalar_length(value) < 1
        or scalar_length(value) > maximum then return false end
    for segment in (value .. "-"):gmatch("(.-)-") do
        if not segment:match("^[a-z0-9]+$") then return false end
    end
    return true
end

local function valid_scalar_string(value, minimum, maximum)
    return valid_utf8(value) and scalar_length(value) >= minimum
        and scalar_length(value) <= maximum
end

local function valid_generated_component(value)
    if not valid_scalar_string(value, 1, 128) or value:find('[\\/<>:"|?*]') then return false end
    for _, scalar in utf8.codes(value) do if scalar < 32 then return false end end
    return true
end

local function snapshot_boundary(root)
    local seen = {}
    local nodes, containers, string_bytes = 0, 0, 0

    local function visit(value, depth)
        nodes = nodes + 1
        require_contract(nodes <= MAX_BOUNDARY_NODES and depth <= MAX_BOUNDARY_DEPTH,
            "DIFF_BOUNDARY_INVALID")
        if type(value) == "string" then
            require_contract(valid_utf8(value), "DIFF_BOUNDARY_INVALID")
            string_bytes = string_bytes + #value
            require_contract(string_bytes <= MAX_BOUNDARY_STRING_BYTES,
                "DIFF_BOUNDARY_INVALID")
            return value
        end
        if value == nil or type(value) == "boolean" then return value end
        if type(value) == "number" then
            require_contract(value == value and value ~= math.huge and value ~= -math.huge,
                "DIFF_BOUNDARY_INVALID")
            return value
        end
        require_contract(type(value) == "table" and not seen[value], "DIFF_BOUNDARY_INVALID")
        seen[value] = true
        containers = containers + 1
        require_contract(containers <= MAX_BOUNDARY_CONTAINERS, "DIFF_BOUNDARY_INVALID")

        local mt = getmetatable(value)
        local marked_array = mt == ARRAY_MT or (type(mt) == "table"
            and rawget(mt, "__jsontype") == "array")
        local marked_object = mt == nil or (type(mt) == "table"
            and rawget(mt, "__jsontype") == "object")
        if marked_array then
            require_contract(accepted_metatable(value, "array"), "DIFF_BOUNDARY_INVALID")
            local count = dense_count(value, MAX_BOUNDARY_NODES - nodes,
                "DIFF_BOUNDARY_INVALID", "DIFF_BOUNDARY_INVALID")
            require_contract(count <= MAX_BOUNDARY_NODES - nodes, "DIFF_BOUNDARY_INVALID")
            if count > 0 then require_contract(depth < MAX_BOUNDARY_DEPTH,
                "DIFF_BOUNDARY_INVALID") end
            local result = setmetatable({}, ARRAY_MT)
            for index = 1, count do result[index] = visit(rawget(value, index), depth + 1) end
            return result
        end
        require_contract(marked_object and accepted_metatable(value, "object"),
            "DIFF_BOUNDARY_INVALID")
        local key_count = 0
        for key in next, value do
            require_contract(type(key) == "string" and valid_utf8(key), "DIFF_BOUNDARY_INVALID")
            key_count = key_count + 1
            string_bytes = string_bytes + #key
            require_contract(key_count <= MAX_BOUNDARY_NODES - nodes
                and string_bytes <= MAX_BOUNDARY_STRING_BYTES, "DIFF_BOUNDARY_INVALID")
        end
        if key_count > 0 then require_contract(depth < MAX_BOUNDARY_DEPTH,
            "DIFF_BOUNDARY_INVALID") end
        local result = {}
        for key in next, value do result[key] = visit(rawget(value, key), depth + 1) end
        return result
    end

    return visit(root, 0)
end

local JSON_ESCAPES = {['"'] = '\\"', ['\\'] = '\\\\', ['\b'] = '\\b',
    ['\f'] = '\\f', ['\n'] = '\\n', ['\r'] = '\\r', ['\t'] = '\\t'}

local function json_string(value)
    return '"' .. value:gsub('[%z\1-\31\\"]', function(character)
        return JSON_ESCAPES[character] or string.format("\\u%04x", character:byte())
    end) .. '"'
end

local function canonical_json(value)
    local kind = type(value)
    if kind == "string" then return json_string(value) end
    if kind == "boolean" then return value and "true" or "false" end
    if kind == "number" then
        if math.type and math.type(value) == "integer" then return tostring(value) end
        return string.format("%.17g", value)
    end
    if kind == "nil" then return "null" end
    require_contract(kind == "table", "DIFF_BOUNDARY_DIGEST_MISMATCH")
    if getmetatable(value) == ARRAY_MT then
        local elements = {}
        for index = 1, #value do elements[index] = canonical_json(value[index]) end
        return "[" .. table.concat(elements, ",") .. "]"
    end
    local keys = {}
    for key in pairs(value) do keys[#keys + 1] = key end
    table.sort(keys)
    local fields = {}
    for _, key in ipairs(keys) do
        fields[#fields + 1] = json_string(key) .. ":" .. canonical_json(value[key])
    end
    return "{" .. table.concat(fields, ",") .. "}"
end

local function validate_path_array(value, maximum)
    local count = dense_count(value, maximum, "DIFF_BOUNDARY_INVALID",
        "DIFF_BOUNDARY_INVALID")
    local result, seen = {}, {}
    for index = 1, count do
        local path = rawget(value, index)
        require_contract(valid_portable_path(path) and not seen[path], "DIFF_BOUNDARY_INVALID")
        seen[path] = true
        result[#result + 1] = path
    end
    return result
end

local function validate_boundary_input(value)
    value = inert_record(value, "DIFF_BOUNDARY_INVALID")
    local path, role = rawget(value, "path"), rawget(value, "role")
    require_contract(valid_portable_path(path) and (role == "cross_package_exact"
        or role == "generated_pruning_exception" or role == "shared_ancestor"),
        "DIFF_BOUNDARY_INVALID")
    if role == "generated_pruning_exception" then
        require_contract(exact_keys(value, {"path", "role", "generated_component"})
            and valid_generated_component(rawget(value, "generated_component")),
            "DIFF_BOUNDARY_INVALID")
    else
        require_contract(exact_keys(value, {"path", "role"}), "DIFF_BOUNDARY_INVALID")
    end
    return {path = path, role = role, generated_component = rawget(value, "generated_component")}
end

local function validate_applicability(value)
    value = inert_record(value, "DIFF_BOUNDARY_INVALID")
    require_contract(exact_keys(value, {"exact_roots", "descendant_roots", "excluded_roots"}),
        "DIFF_BOUNDARY_INVALID")
    local result = {
        exact_roots = validate_path_array(rawget(value, "exact_roots"), 4096),
        descendant_roots = validate_path_array(rawget(value, "descendant_roots"), 64),
        excluded_roots = validate_path_array(rawget(value, "excluded_roots"), 4096),
    }
    require_contract(#result.exact_roots > 0 or #result.descendant_roots > 0,
        "DIFF_BOUNDARY_INVALID")
    return result
end

local function validate_boundary(value)
    value = inert_record(value, "DIFF_BOUNDARY_INVALID")
    require_contract(exact_keys(value,
        {"schema_version", "language_source_input_registry_sha256", "boundaries"})
        and rawget(value, "schema_version") == 1
        and type(rawget(value, "language_source_input_registry_sha256")) == "string"
        and rawget(value, "language_source_input_registry_sha256"):match("^[0-9a-f]+$")
        and #rawget(value, "language_source_input_registry_sha256") == 64,
        "DIFF_BOUNDARY_INVALID")
    local raw_boundaries = rawget(value, "boundaries")
    local count = dense_count(raw_boundaries, 256, "DIFF_BOUNDARY_INVALID",
        "DIFF_BOUNDARY_INVALID")
    require_contract(count >= 1 and count <= 256, "DIFF_BOUNDARY_INVALID")
    local result, ids = {}, {}
    local scopes, authorizations = 0, 0
    for index = 1, count do
        local rule = inert_record(rawget(raw_boundaries, index), "DIFF_BOUNDARY_INVALID")
        require_contract(exact_keys(rule,
            {"id", "input_origin", "applies_to", "inputs", "reason", "owner"})
            and valid_slug(rawget(rule, "id"), 120)
            and BOUNDARY_ORIGINS[rawget(rule, "input_origin")] == true
            and valid_scalar_string(rawget(rule, "reason"), 1, 512)
            and valid_slug(rawget(rule, "owner"), 120)
            and not ids[rawget(rule, "id")], "DIFF_BOUNDARY_INVALID")
        ids[rawget(rule, "id")] = true
        local applies_to = validate_applicability(rawget(rule, "applies_to"))
        local raw_inputs = rawget(rule, "inputs")
        local input_count = dense_count(raw_inputs, 64, "DIFF_BOUNDARY_INVALID",
            "DIFF_BOUNDARY_INVALID")
        require_contract(input_count >= 1 and input_count <= 64, "DIFF_BOUNDARY_INVALID")
        local inputs = {}
        for input_index = 1, input_count do
            inputs[#inputs + 1] = validate_boundary_input(rawget(raw_inputs, input_index))
        end
        local rule_scopes = #applies_to.exact_roots + #applies_to.descendant_roots
        scopes = scopes + rule_scopes
        authorizations = authorizations + input_count * rule_scopes
        require_contract(scopes <= 8192 and authorizations <= 32768,
            "DIFF_BOUNDARY_INVALID")
        result[#result + 1] = {applies_to = applies_to, inputs = inputs}
    end
    return result
end

local function list_contains(values, target)
    for _, value in ipairs(values) do if value == target then return true end end
    return false
end

local function boundary_applies(value, root)
    if list_contains(value.exact_roots, root) then return true end
    if list_contains(value.excluded_roots, root) then return false end
    for _, parent in ipairs(value.descendant_roots) do
        if root:sub(1, #parent + 1) == parent .. "/" then return true end
    end
    return false
end

local function boundary_reverse_index(input, packages)
    local digest = rawget(input, "boundary_sha256") or ""
    local supplied = rawget(input, "boundary")
    require_contract(type(digest) == "string", "DIFF_BOUNDARY_DIGEST_MISMATCH")
    if digest == "" then
        require_contract(supplied == nil, "DIFF_BOUNDARY_DIGEST_MISMATCH")
        return {}
    end
    require_contract(#digest == 64 and digest:match("^[0-9a-f]+$")
        and type(supplied) == "table", "DIFF_BOUNDARY_DIGEST_MISMATCH")
    local snapshot = snapshot_boundary(supplied)
    local encoded = canonical_json(snapshot)
    local frame = BOUNDARY_DOMAIN .. string.pack(">I8", #encoded) .. encoded
    require_contract(sha256.sha256_hex(frame) == digest, "DIFF_BOUNDARY_DIGEST_MISMATCH")
    local boundaries = validate_boundary(snapshot)
    local consumers = {}
    for _, package in ipairs(packages) do
        for _, rule in ipairs(boundaries) do
            if boundary_applies(rule.applies_to, package.rel_path) then
                for _, source in ipairs(rule.inputs) do
                    consumers[source.path] = consumers[source.path] or {}
                    consumers[source.path][package.name] = true
                end
            end
        end
    end
    return consumers
end

local function inside(path, root)
    return path == root or path:sub(1, #root + 1) == root .. "/"
end

local function relative_path(path, root)
    return path == root and "" or path:sub(#root + 2)
end

local function basename(path)
    return path:match("([^/]+)$") or ""
end

local function preflight_match_work(packages, changed_paths, changed_count)
    local total = 0
    for _, package in ipairs(packages) do
        if package.source_mode == "strict_globs" then
            local pattern_factor = 0
            for _, glob in ipairs(package.source_globs) do
                pattern_factor = pattern_factor + scalar_length(glob) + 1
            end
            for index = 1, changed_count do
                local path = rawget(changed_paths, index)
                if inside(path, package.rel_path) then
                    local relative = relative_path(path, package.rel_path)
                    if not BUILD_FRONTS[basename(relative)] then
                        local path_factor = scalar_length(relative) + 1
                        require_contract(pattern_factor == 0
                            or pattern_factor <= (MAX_MATCH_WORK - total) // path_factor,
                            "DIFF_MATCH_LIMIT_EXCEEDED")
                        total = total + pattern_factor * path_factor
                    end
                end
            end
        end
    end
end

local function closure(seeds, adjacency)
    local result, pending = {}, {}
    for name in pairs(seeds) do result[name] = true; pending[#pending + 1] = name end
    while #pending > 0 do
        local current = pending[#pending]
        pending[#pending] = nil
        for _, name in ipairs(adjacency[current] or {}) do
            if not result[name] then result[name] = true; pending[#pending + 1] = name end
        end
    end
    return result
end

local function sorted_set(values)
    local result = {}
    for value in pairs(values) do result[#result + 1] = value end
    table.sort(result)
    return result
end

local function select_diff(packages, graph, forced, forced_count, changed_paths,
        changed_count, policy, boundary_consumers)
    local changed = {}
    for index = 1, forced_count do changed[rawget(forced, index)] = true end
    local unknown = false
    for index = 1, changed_count do
        local path = rawget(changed_paths, index)
        local consumers = boundary_consumers[path]
        local known = consumers ~= nil and next(consumers) ~= nil
        if consumers then for name in pairs(consumers) do changed[name] = true end end
        for _, package in ipairs(packages) do
            if inside(path, package.rel_path) then
                known = true
                local relative = relative_path(path, package.rel_path)
                local selected = package.source_mode == "package_prefix"
                    or BUILD_FRONTS[basename(relative)] == true
                if not selected then
                    for _, pattern in ipairs(package.compiled_globs) do
                        if glob_matches(pattern, relative) then selected = true; break end
                    end
                end
                if selected then changed[package.name] = true end
            end
        end
        if not known then unknown = true end
    end
    if unknown and policy == "error" then contract("DIFF_UNKNOWN_PATH") end
    if unknown then
        changed = {}
        for name in pairs(graph.names) do changed[name] = true end
    end
    local affected = closure(changed, graph.dependents)
    local prerequisites = closure(affected, graph.prerequisites)
    for name in pairs(affected) do prerequisites[name] = nil end
    return {
        changed_packages = sorted_set(changed),
        affected_packages = sorted_set(affected),
        prerequisite_packages = sorted_set(prerequisites),
    }
end

function GraphDiff.evaluate_graph(input)
    return protect("GRAPH_INPUT_INVALID", function()
        local snapshot = snapshot_graph_input(input)
        local graph = validate_graph(snapshot.packages, snapshot.edges)
        local levels = graph_levels(graph)
        if not levels then return failure("GRAPH_CYCLE") end
        return success({edges = graph.edges, levels = levels})
    end)
end

function GraphDiff.evaluate_diff_selection(input)
    return protect("DIFF_INPUT_INVALID", function()
        input = snapshot_diff_input(input)
        local packages_value = input.packages
        local package_count = dense_count(packages_value, MAX_PACKAGES,
            "DIFF_PACKAGE_INVALID", "DIFF_PACKAGE_INVALID")
        local names = {}
        for index = 1, package_count do
            local package = inert_record(rawget(packages_value, index), "DIFF_PACKAGE_INVALID")
            names[index] = rawget(package, "name")
        end
        local graph = validate_graph(names, rawget(input, "edges"))
        require_contract(graph_levels(graph) ~= nil, "DIFF_EDGE_CYCLE")
        local packages, forced, forced_count, changed_paths, changed_count, policy =
            validate_diff_input(input, graph)
        local boundary_consumers = boundary_reverse_index(input, packages)
        preflight_match_work(packages, changed_paths, changed_count)
        return success(select_diff(packages, graph, forced, forced_count, changed_paths,
            changed_count, policy, boundary_consumers))
    end)
end

return GraphDiff
