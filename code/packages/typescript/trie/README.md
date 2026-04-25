# @coding-adventures/trie

A generic trie for string keys. It stores values on exact keys while sharing
common prefixes, making autocomplete, prefix membership, and longest-prefix
lookups efficient.

## Example

```ts
import { Trie } from "@coding-adventures/trie";

const trie = new Trie<number>();
trie.insert("app", 1);
trie.insert("apple", 2);

trie.search("app"); // 1
trie.wordsWithPrefix("app"); // [["app", 1], ["apple", 2]]
trie.longestPrefixMatch("applesauce"); // ["apple", 2]
```
