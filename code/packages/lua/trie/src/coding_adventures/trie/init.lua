--- A prefix trie for UTF-8 string keys and arbitrary values.

local Trie = {}
Trie.__index = Trie

local function new_node()
    return {
        children = {},
        terminal = false,
        value = nil,
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

local function find_node(self, key)
    local node = self._root
    for _, codepoint in utf8.codes(key) do
        node = node.children[utf8.char(codepoint)]
        if node == nil then
            return nil
        end
    end
    return node
end

local function collect(node, current, results)
    if node.terminal then
        results[#results + 1] = { current, node.value }
    end

    local child_keys = {}
    for character in pairs(node.children) do
        child_keys[#child_keys + 1] = character
    end
    table.sort(child_keys)

    for _, character in ipairs(child_keys) do
        collect(node.children[character], current .. character, results)
    end
end

local function delete_recursive(node, chars, depth)
    if depth > #chars then
        node.terminal = false
        node.value = nil
        return next(node.children) == nil
    end

    local character = chars[depth]
    local child = node.children[character]
    if child ~= nil and delete_recursive(child, chars, depth + 1) then
        node.children[character] = nil
    end

    return next(node.children) == nil and not node.terminal
end

local function count_endpoints(node)
    local count = node.terminal and 1 or 0
    for _, child in pairs(node.children) do
        count = count + count_endpoints(child)
    end
    return count
end

function Trie.new(entries)
    entries = entries == nil and {} or entries
    if type(entries) ~= "table" then
        error("entries must be a table", 2)
    end

    local self = setmetatable({
        _root = new_node(),
        _size = 0,
    }, Trie)

    for index, entry in ipairs(entries) do
        if type(entry) ~= "table" or entry[1] == nil then
            error("entry at index " .. index .. " must contain a key", 2)
        end
        self:insert(entry[1], entry[2])
    end
    return self
end

function Trie:insert(key, value)
    assert_string(key, "key", 2)
    if value == nil then
        value = true
    end

    local node = self._root
    for _, codepoint in utf8.codes(key) do
        local character = utf8.char(codepoint)
        local child = node.children[character]
        if child == nil then
            child = new_node()
            node.children[character] = child
        end
        node = child
    end

    if not node.terminal then
        self._size = self._size + 1
    end
    node.terminal = true
    node.value = value
    return self
end

function Trie:search(key)
    assert_string(key, "key", 2)
    local node = find_node(self, key)
    if node ~= nil and node.terminal then
        return node.value
    end
    return nil
end

function Trie:contains_key(key)
    assert_string(key, "key", 2)
    local node = find_node(self, key)
    return node ~= nil and node.terminal
end

Trie.key_exists = Trie.contains_key
Trie.contains = Trie.contains_key

function Trie:delete(key)
    assert_string(key, "key", 2)
    if not self:contains_key(key) then
        return false
    end

    delete_recursive(self._root, characters(key), 1)
    self._size = self._size - 1
    return true
end

function Trie:starts_with(prefix)
    assert_string(prefix, "prefix", 2)
    if prefix == "" then
        return self._size > 0
    end
    return find_node(self, prefix) ~= nil
end

function Trie:words_with_prefix(prefix)
    assert_string(prefix, "prefix", 2)
    local node = find_node(self, prefix)
    if node == nil then
        return {}
    end

    local results = {}
    collect(node, prefix, results)
    return results
end

function Trie:all_words()
    local results = {}
    collect(self._root, "", results)
    return results
end

Trie.entries = Trie.all_words
Trie.to_array = Trie.all_words

function Trie:keys()
    local result = {}
    for _, entry in ipairs(self:all_words()) do
        result[#result + 1] = entry[1]
    end
    return result
end

function Trie:longest_prefix_match(input)
    assert_string(input, "input", 2)
    local node = self._root
    local current = ""
    local best = node.terminal and { "", node.value } or nil

    for _, codepoint in utf8.codes(input) do
        local character = utf8.char(codepoint)
        local child = node.children[character]
        if child == nil then
            break
        end
        current = current .. character
        node = child
        if node.terminal then
            best = { current, node.value }
        end
    end
    return best
end

function Trie:size()
    return self._size
end

Trie.length = Trie.size

function Trie:is_empty()
    return self._size == 0
end

function Trie:is_valid()
    return count_endpoints(self._root) == self._size
end

Trie.__len = function(self)
    return self._size
end

Trie.__tostring = function(self)
    return string.format("Trie(%d keys)", self._size)
end

return {
    Trie = Trie,
    new = Trie.new,
}
