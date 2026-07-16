package.path = table.concat({
    "../src/?.lua",
    "../src/?/init.lua",
    package.path,
}, ";")

local module = require("coding_adventures.radix_tree")
local RadixTree = module.RadixTree

local function make_tree(...)
    local tree = RadixTree.new()
    for index, key in ipairs({ ... }) do
        tree:insert(key, index)
    end
    return tree
end

describe("RadixTree", function()
    it("represents an empty tree", function()
        local tree = RadixTree.new()
        assert.equals(0, tree:size())
        assert.equals(0, #tree)
        assert.is_true(tree:is_empty())
        assert.equals(1, tree:node_count())
        assert.is_nil(tree:search("anything"))
        assert.is_false(tree:starts_with(""))
        assert.is_true(tree:is_valid())
    end)

    it("covers all compressed-edge insertion cases and updates", function()
        local tree = RadixTree.new()
        tree:insert("application", 1)
        tree:insert("apple", 2)
        tree:insert("app", 3)
        tree:insert("apt", 4)

        assert.equals(1, tree:search("application"))
        assert.equals(2, tree:search("apple"))
        assert.equals(3, tree:search("app"))
        assert.equals(4, tree:search("apt"))
        assert.is_nil(tree:search("appl"))

        tree:insert("app", 99)
        tree:insert("disabled", false)
        assert.equals(99, tree:search("app"))
        assert.is_false(tree:search("disabled"))
        assert.is_true(tree:contains_key("disabled"))
        assert.equals(5, tree:length())
        assert.is_true(tree:is_valid())
    end)

    it("answers mid-edge prefix queries in sorted order", function()
        local tree = make_tree("search", "searcher", "searching", "banana")
        assert.is_true(tree:starts_with("sear"))
        assert.is_false(tree:starts_with("seek"))
        assert.are.same({ "search", "searcher", "searching" }, tree:words_with_prefix("sear"))
        assert.are.same({ "search", "searcher", "searching" }, tree:words_with_prefix("search"))
        assert.are.same({}, tree:words_with_prefix("xyz"))
        assert.are.same({ "banana", "search", "searcher", "searching" }, tree:keys())
        assert.equals(5, tree:node_count())
    end)

    it("deletes keys and merges compressed edges", function()
        local tree = make_tree("app", "apple", "apt")
        assert.is_true(tree:delete("app"))
        assert.is_false(tree:contains("app"))
        assert.equals(2, tree:search("apple"))
        assert.equals(3, tree:search("apt"))
        assert.is_false(tree:delete("missing"))
        assert.is_false(tree:delete("ap"))
        assert.is_true(tree:delete("apple"))
        assert.equals(2, tree:node_count())
        assert.is_true(tree:is_valid())
    end)

    it("supports longest-prefix matching and empty-string keys", function()
        local tree = module.new({
            { "", "root" },
            { "a", 1 },
            { "ab", 2 },
            { "abc", 3 },
            { "application", 4 },
        })
        assert.equals("abc", tree:longest_prefix_match("abcdef"))
        assert.equals("application", tree:longest_prefix_match("application/json"))
        assert.equals("", tree:longest_prefix_match("xyz"))
        assert.equals("root", tree:search(""))
        assert.is_true(tree:starts_with(""))
        assert.is_true(tree:delete(""))
        assert.is_nil(tree:longest_prefix_match("xyz"))
    end)

    it("keeps UTF-8 labels intact while splitting and merging", function()
        local tree = RadixTree.new()
        tree:insert("caf\u{00e9}", "single")
        tree:insert("cafe\u{0301}", "combining")
        tree:insert("cafeteria", "food")
        tree:insert("\u{732b}", "cat")

        assert.equals("single", tree:search("caf\u{00e9}"))
        assert.are.same({ "cafeteria" }, tree:words_with_prefix("cafet"))
        assert.equals("cafe\u{0301}", tree:longest_prefix_match("cafe\u{0301}-au-lait"))
        assert.is_true(tree:delete("cafe\u{0301}"))
        assert.equals("food", tree:search("cafeteria"))
        assert.is_true(tree:is_valid())
    end)

    it("agrees with a table across deterministic mixed mutations", function()
        local tree = RadixTree.new()
        local expected = {}
        for index = 1, 200 do
            local key = string.format("route/%02d/%03d", index % 17, (index * 37) % 211)
            tree:insert(key, index)
            expected[key] = index
        end

        local expected_keys = {}
        for key, value in pairs(expected) do
            assert.equals(value, tree:search(key))
            expected_keys[#expected_keys + 1] = key
        end
        table.sort(expected_keys)
        assert.are.same(expected_keys, tree:keys())
        assert.equals(#expected_keys, tree:size())

        local to_delete = {}
        for key, value in pairs(expected) do
            if value % 2 == 0 then
                to_delete[#to_delete + 1] = key
            end
        end
        for _, key in ipairs(to_delete) do
            assert.is_true(tree:delete(key))
            expected[key] = nil
        end

        local prefix = "route/03"
        local prefix_keys = {}
        for key in pairs(expected) do
            if key:sub(1, #prefix) == prefix then
                prefix_keys[#prefix_keys + 1] = key
            end
        end
        table.sort(prefix_keys)
        assert.are.same(prefix_keys, tree:words_with_prefix(prefix))
        assert.is_true(tree:is_valid())
    end)

    it("exports values, iterates, and validates public inputs", function()
        local tree = module.new({ { "b", 2 }, { "a", 1 } })
        assert.are.same({ { "a", 1 }, { "b", 2 } }, tree:entries())
        assert.are.same({ a = 1, b = 2 }, tree:to_table())

        local seen = {}
        tree:each(function(key, value)
            seen[#seen + 1] = { key, value }
        end)
        assert.are.same(tree:entries(), seen)
        assert.is_truthy(tostring(tree):match("2 keys"))

        assert.has_error(function()
            RadixTree.new("not-a-table")
        end, "entries must be a table")
        assert.has_error(function()
            RadixTree.new({ {} })
        end, "entry at index 1 must contain a key")
        assert.has_error(function()
            tree:insert({})
        end, "key must be a string")
        assert.has_error(function()
            tree:starts_with({})
        end, "prefix must be a string")
        assert.has_error(function()
            tree:longest_prefix_match({})
        end, "input must be a string")
        assert.has_error(function()
            tree:each("not-a-function")
        end, "callback must be a function")
    end)
end)
