# coding-adventures-radix-tree

A dependency-free Lua 5.4 radix tree for UTF-8 string keys and arbitrary
values. Unlike a character-per-node trie, it stores whole substrings on edges
and preserves that compression after deletion.

```lua
local RadixTree = require("coding_adventures.radix_tree").RadixTree

local tree = RadixTree.new()
tree:insert("search", 1)
tree:insert("searcher", 2)

assert(tree:search("search") == 1)
assert(tree:longest_prefix_match("search-party") == "search")
assert(tree:node_count() == 3) -- root plus two compressed key nodes
```

The public API includes exact lookup and membership, sorted key and prefix
enumeration, longest-prefix matching, deletion, map export, node counting, and
structural validation. Empty-string keys and Unicode keys are supported.
