package.path = table.concat({
    "../src/?.lua",
    "../src/?/init.lua",
    package.path,
}, ";")

local module = require("coding_adventures.trie")
local Trie = module.Trie

local function make_trie(...)
    local trie = Trie.new()
    for _, word in ipairs({ ... }) do
        trie:insert(word)
    end
    return trie
end

describe("Trie", function()
    it("represents an empty trie", function()
        local trie = Trie.new()
        assert.equals(0, trie:size())
        assert.equals(0, #trie)
        assert.is_true(trie:is_empty())
        assert.is_nil(trie:search("anything"))
        assert.is_false(trie:starts_with("a"))
        assert.is_true(trie:is_valid())
    end)

    it("inserts, searches, and updates exact keys", function()
        local trie = Trie.new()
        trie:insert("hello", 42)
        assert.equals(42, trie:search("hello"))
        assert.is_nil(trie:search("hell"))
        assert.is_nil(trie:search("hellos"))

        trie:insert("hello", 99)
        assert.equals(99, trie:search("hello"))
        assert.equals(1, trie:length())
        assert.is_true(trie:contains_key("hello"))
        assert.is_true(trie:key_exists("hello"))

        trie:insert("false", false)
        assert.is_false(trie:search("false"))
        assert.is_true(trie:contains("false"))
    end)

    it("enumerates prefixes and all keys in sorted order", function()
        local trie = make_trie("banana", "app", "apple", "apply", "apt")
        assert.are.same({
            { "app", true },
            { "apple", true },
            { "apply", true },
        }, trie:words_with_prefix("app"))
        assert.are.same({}, trie:words_with_prefix("xyz"))
        assert.are.same({ "app", "apple", "apply", "apt", "banana" }, trie:keys())
        assert.are.same(trie:all_words(), trie:entries())
    end)

    it("deletes leaves and shared prefixes with pruning", function()
        local trie = make_trie("app", "apple", "apt")
        assert.is_true(trie:delete("app"))
        assert.is_false(trie:contains_key("app"))
        assert.is_true(trie:contains_key("apple"))
        assert.is_true(trie:contains_key("apt"))
        assert.equals(2, trie:size())
        assert.is_false(trie:delete("missing"))
        assert.is_false(trie:delete("ap"))
        assert.is_true(trie:delete("apple"))
        assert.is_true(trie:delete("apt"))
        assert.is_true(trie:is_empty())
        assert.is_true(trie:is_valid())
    end)

    it("constructs from entries and finds the longest prefix", function()
        local trie = module.new({
            { "a", 1 },
            { "ab", 2 },
            { "abc", 3 },
            { "abcd", 4 },
        })
        assert.are.same({ "abcd", 4 }, trie:longest_prefix_match("abcde"))
        assert.is_nil(trie:longest_prefix_match("xyz"))
        assert.are.same({ "a", 1 }, trie:longest_prefix_match("a"))
    end)

    it("supports Unicode and empty-string keys", function()
        local trie = Trie.new()
        trie:insert("", "root")
        trie:insert("cafe", "plain")
        trie:insert("cafe\u{0301}", "accent-combining")
        trie:insert("caf\u{00e9}", "accent-single")

        assert.equals("root", trie:search(""))
        assert.is_true(trie:starts_with(""))
        assert.is_true(trie:starts_with("caf"))
        assert.equals("accent-single", trie:search("caf\u{00e9}"))
        assert.are.same(
            { "cafe\u{0301}", "accent-combining" },
            trie:longest_prefix_match("cafe\u{0301}-au-lait")
        )
        assert.is_true(trie:delete(""))
        assert.is_nil(trie:search(""))
        assert.is_truthy(tostring(trie):match("3 keys"))
    end)

    it("validates constructor entries and public string inputs", function()
        assert.has_error(function()
            Trie.new("not-a-table")
        end, "entries must be a table")
        assert.has_error(function()
            Trie.new({ {} })
        end, "entry at index 1 must contain a key")
        assert.has_error(function()
            Trie.new():insert({})
        end, "key must be a string")
        assert.has_error(function()
            Trie.new():starts_with({})
        end, "prefix must be a string")
        assert.has_error(function()
            Trie.new():longest_prefix_match({})
        end, "input must be a string")
    end)
end)
