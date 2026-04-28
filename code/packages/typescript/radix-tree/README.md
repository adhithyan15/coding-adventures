# @coding-adventures/radix-tree

A compressed trie for string-keyed prefix lookup.

```ts
import { RadixTree } from "@coding-adventures/radix-tree";

const tree = new RadixTree<number>();
tree.insert("search", 1);
tree.insert("searcher", 2);

tree.search("search"); // 1
tree.wordsWithPrefix("search"); // ["search", "searcher"]
```
