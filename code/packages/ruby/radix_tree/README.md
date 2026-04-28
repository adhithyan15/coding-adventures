# coding_adventures_radix_tree

A Ruby compressed trie for string-keyed prefix lookup.

```ruby
tree = CodingAdventures::RadixTree::RadixTree.new
tree.insert("search", 1)
tree.insert("searcher", 2)
tree.words_with_prefix("search") #=> ["search", "searcher"]
```
