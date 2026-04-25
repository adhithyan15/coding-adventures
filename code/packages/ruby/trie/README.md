# coding_adventures_trie

A Ruby trie for string keys and arbitrary values. It supports exact lookup,
prefix enumeration, deletion with pruning, sorted keys, and longest-prefix
matching.

```ruby
trie = CodingAdventures::Trie::Trie.new
trie.insert("app", 1)
trie.insert("apple", 2)

trie.search("app") #=> 1
trie.words_with_prefix("app") #=> [["app", 1], ["apple", 2]]
```
