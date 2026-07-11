# Trie (Swift)

A prefix tree (trie) for `String` keys with prefix operations, in pure Swift.

Swift port of the `trie` package that already exists in Rust, Python, and other
languages in the coding-adventures monorepo.

## What it does

| Member | Purpose |
|---|---|
| `insert(_:value:)` | store a key with a value (overwrites) |
| `search(_:)` | value for an exact key, or `nil` |
| `containsKey(_:)` | is this an exact stored key? |
| `delete(_:)` | remove a key, pruning empty nodes |
| `startsWith(_:)` | does any key begin with this prefix? |
| `wordsWithPrefix(_:)` | all keys under a prefix, sorted, with values |
| `allWords()` / `keys()` | every key (with/without values), sorted |
| `longestPrefixMatch(_:)` | most specific stored key that prefixes a string |
| `count` / `isEmpty` / `isValid()` | introspection |

## Unicode

Like the Rust reference (which keys on `char`), this trie keys on
**`Unicode.Scalar`**, not `Character` (grapheme clusters). So a combining
sequence (`"e"` + U+0301) and the precomposed `"é"` (U+00E9) are **distinct**
keys. Children are visited in scalar order, so enumerations come back
lexicographically sorted.

`Trie` is a value type: assigning it makes an independent copy.

## Usage

```swift
import Trie

var trie = Trie<Int>()
trie.insert("apple", value: 1)
trie.insert("app", value: 2)
trie.search("app")                 // 2
trie.startsWith("ap")              // true
trie.wordsWithPrefix("app").map { $0.0 } // ["app", "apple"]
trie.longestPrefixMatch("applet")  // ("apple", 1)
```

## Running the tests

```
swift test
```
