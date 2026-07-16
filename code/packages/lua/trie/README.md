# coding-adventures-trie

A dependency-free Lua 5.4 prefix trie for UTF-8 string keys and arbitrary
values. It supports exact lookup, deletion with pruning, lexicographically
sorted enumeration, prefix scans, and longest-prefix matching.

```lua
local Trie = require("coding_adventures.trie").Trie

local trie = Trie.new()
trie:insert("app", 1)
trie:insert("apple", 2)

assert(trie:search("app") == 1)
assert(trie:longest_prefix_match("apples")[1] == "apple")
```
