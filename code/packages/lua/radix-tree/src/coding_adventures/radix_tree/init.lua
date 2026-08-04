--- A path-compressed radix tree for UTF-8 string keys and arbitrary values.

local RadixTree = {}
RadixTree.__index = RadixTree

local function new_node(terminal, value)
    return {
        children = {},
        terminal = terminal or false,
        value = value,
    }
end

local function assert_string(value, name, level)
    if type(value) ~= "string" then
        error(name .. " must be a string", level or 3)
    end
end

local function characters(value)
    local result = {}
    for _, codepoint in utf8.codes(value) do
        result[#result + 1] = utf8.char(codepoint)
    end
    return result
end

local function slice(chars, first)
    local result = {}
    for index = first, #chars do
        result[#result + 1] = chars[index]
    end
    return result
end

local function concatenate(left, right)
    local result = {}
    for _, character in ipairs(left) do
        result[#result + 1] = character
    end
    for _, character in ipairs(right) do
        result[#result + 1] = character
    end
    return result
end

local function common_prefix_length(left, right)
    local limit = math.min(#left, #right)
    local index = 0
    while index < limit and left[index + 1] == right[index + 1] do
        index = index + 1
    end
    return index
end

local function child_count(node)
    local count = 0
    for _ in pairs(node.children) do
        count = count + 1
    end
    return count
end

local function only_child(node)
    for _, edge in pairs(node.children) do
        return edge
    end
    return nil
end

local function insert_recursive(node, key, value)
    if #key == 0 then
        local added = not node.terminal
        node.terminal = true
        node.value = value
        return added
    end

    local first = key[1]
    local edge = node.children[first]
    if edge == nil then
        node.children[first] = {
            label = key,
            child = new_node(true, value),
        }
        return true
    end

    local common = common_prefix_length(key, edge.label)
    if common == #edge.label then
        return insert_recursive(edge.child, slice(key, common + 1), value)
    end

    local common_label = {}
    for index = 1, common do
        common_label[index] = edge.label[index]
    end
    local old_rest = slice(edge.label, common + 1)
    local key_rest = slice(key, common + 1)
    local split_node = new_node()
    split_node.children[old_rest[1]] = {
        label = old_rest,
        child = edge.child,
    }

    if #key_rest == 0 then
        split_node.terminal = true
        split_node.value = value
    else
        split_node.children[key_rest[1]] = {
            label = key_rest,
            child = new_node(true, value),
        }
    end

    node.children[first] = {
        label = common_label,
        child = split_node,
    }
    return true
end

local function find_node(self, key)
    local chars = characters(key)
    local node = self._root
    local index = 1

    while index <= #chars do
        local edge = node.children[chars[index]]
        if edge == nil then
            return nil
        end
        for offset = 1, #edge.label do
            if chars[index + offset - 1] ~= edge.label[offset] then
                return nil
            end
        end
        index = index + #edge.label
        node = edge.child
    end
    return node
end

local function delete_recursive(node, key)
    if #key == 0 then
        if not node.terminal then
            return false, false
        end
        node.terminal = false
        node.value = nil
        return true, child_count(node) == 1
    end

    local first = key[1]
    local edge = node.children[first]
    if edge == nil then
        return false, false
    end

    local common = common_prefix_length(key, edge.label)
    if common < #edge.label then
        return false, false
    end

    local deleted, child_mergeable = delete_recursive(
        edge.child,
        slice(key, common + 1)
    )
    if not deleted then
        return false, false
    end

    if child_mergeable then
        local grandchild = only_child(edge.child)
        node.children[first] = {
            label = concatenate(edge.label, grandchild.label),
            child = grandchild.child,
        }
    elseif not edge.child.terminal and child_count(edge.child) == 0 then
        node.children[first] = nil
    end

    return true, not node.terminal and child_count(node) == 1
end

local function sorted_child_keys(node)
    local result = {}
    for first in pairs(node.children) do
        result[#result + 1] = first
    end
    table.sort(result)
    return result
end

local function collect_entries(node, current, results)
    if node.terminal then
        results[#results + 1] = { current, node.value }
    end
    for _, first in ipairs(sorted_child_keys(node)) do
        local edge = node.children[first]
        collect_entries(edge.child, current .. table.concat(edge.label), results)
    end
end

local function count_nodes(node)
    local count = 1
    for _, edge in pairs(node.children) do
        count = count + count_nodes(edge.child)
    end
    return count
end

local function validate_node(node, is_root)
    local endpoints = node.terminal and 1 or 0
    local children = 0

    for first, edge in pairs(node.children) do
        children = children + 1
        if type(edge.label) ~= "table"
            or #edge.label == 0
            or edge.label[1] ~= first
            or type(edge.child) ~= "table"
        then
            return false, 0
        end
        local valid, child_endpoints = validate_node(edge.child, false)
        if not valid then
            return false, 0
        end
        endpoints = endpoints + child_endpoints
    end

    if not is_root and not node.terminal and children <= 1 then
        return false, 0
    end
    return true, endpoints
end

function RadixTree.new(entries)
    entries = entries == nil and {} or entries
    if type(entries) ~= "table" then
        error("entries must be a table", 2)
    end

    local self = setmetatable({
        _root = new_node(),
        _size = 0,
    }, RadixTree)

    for index, entry in ipairs(entries) do
        if type(entry) ~= "table" or entry[1] == nil then
            error("entry at index " .. index .. " must contain a key", 2)
        end
        self:insert(entry[1], entry[2])
    end
    return self
end

function RadixTree:insert(key, value)
    assert_string(key, "key", 2)
    if value == nil then
        value = true
    end
    if insert_recursive(self._root, characters(key), value) then
        self._size = self._size + 1
    end
    return self
end

function RadixTree:search(key)
    assert_string(key, "key", 2)
    local node = find_node(self, key)
    if node ~= nil and node.terminal then
        return node.value
    end
    return nil
end

function RadixTree:contains_key(key)
    assert_string(key, "key", 2)
    local node = find_node(self, key)
    return node ~= nil and node.terminal
end

RadixTree.key_exists = RadixTree.contains_key
RadixTree.contains = RadixTree.contains_key

function RadixTree:delete(key)
    assert_string(key, "key", 2)
    local deleted = delete_recursive(self._root, characters(key))
    if deleted then
        self._size = self._size - 1
    end
    return deleted
end

function RadixTree:starts_with(prefix)
    assert_string(prefix, "prefix", 2)
    if prefix == "" then
        return self._size > 0
    end

    local chars = characters(prefix)
    local node = self._root
    local index = 1
    while index <= #chars do
        local edge = node.children[chars[index]]
        if edge == nil then
            return false
        end

        local remaining = #chars - index + 1
        local limit = math.min(remaining, #edge.label)
        for offset = 1, limit do
            if chars[index + offset - 1] ~= edge.label[offset] then
                return false
            end
        end
        if remaining <= #edge.label then
            return true
        end
        index = index + #edge.label
        node = edge.child
    end
    return node.terminal or child_count(node) > 0
end

function RadixTree:entries()
    local results = {}
    collect_entries(self._root, "", results)
    return results
end

RadixTree.all_entries = RadixTree.entries

function RadixTree:keys()
    local result = {}
    for _, entry in ipairs(self:entries()) do
        result[#result + 1] = entry[1]
    end
    return result
end

RadixTree.all_words = RadixTree.keys

function RadixTree:words_with_prefix(prefix)
    assert_string(prefix, "prefix", 2)
    if prefix == "" then
        return self:keys()
    end

    local chars = characters(prefix)
    local node = self._root
    local index = 1
    local path = ""

    while index <= #chars do
        local edge = node.children[chars[index]]
        if edge == nil then
            return {}
        end

        local remaining = #chars - index + 1
        local limit = math.min(remaining, #edge.label)
        for offset = 1, limit do
            if chars[index + offset - 1] ~= edge.label[offset] then
                return {}
            end
        end

        path = path .. table.concat(edge.label)
        if remaining <= #edge.label then
            local entries = {}
            collect_entries(edge.child, path, entries)
            local result = {}
            for _, entry in ipairs(entries) do
                result[#result + 1] = entry[1]
            end
            return result
        end
        index = index + #edge.label
        node = edge.child
    end

    local entries = {}
    collect_entries(node, path, entries)
    local result = {}
    for _, entry in ipairs(entries) do
        result[#result + 1] = entry[1]
    end
    return result
end

function RadixTree:longest_prefix_match(input)
    assert_string(input, "input", 2)
    local chars = characters(input)
    local node = self._root
    local index = 1
    local path = ""
    local best = node.terminal and "" or nil

    while index <= #chars do
        local edge = node.children[chars[index]]
        if edge == nil then
            break
        end
        local matches = true
        for offset = 1, #edge.label do
            if chars[index + offset - 1] ~= edge.label[offset] then
                matches = false
                break
            end
        end
        if not matches then
            break
        end
        path = path .. table.concat(edge.label)
        index = index + #edge.label
        node = edge.child
        if node.terminal then
            best = path
        end
    end
    return best
end

function RadixTree:to_table()
    local result = {}
    for _, entry in ipairs(self:entries()) do
        result[entry[1]] = entry[2]
    end
    return result
end

RadixTree.to_map = RadixTree.to_table

function RadixTree:each(callback)
    if type(callback) ~= "function" then
        error("callback must be a function", 2)
    end
    for _, entry in ipairs(self:entries()) do
        callback(entry[1], entry[2])
    end
    return self
end

function RadixTree:size()
    return self._size
end

RadixTree.length = RadixTree.size

function RadixTree:is_empty()
    return self._size == 0
end

function RadixTree:node_count()
    return count_nodes(self._root)
end

function RadixTree:is_valid()
    local valid, endpoints = validate_node(self._root, true)
    return valid and endpoints == self._size
end

RadixTree.__len = function(self)
    return self._size
end

RadixTree.__tostring = function(self)
    return string.format("RadixTree(%d keys, %d nodes)", self._size, self:node_count())
end

return {
    RadixTree = RadixTree,
    new = RadixTree.new,
}
