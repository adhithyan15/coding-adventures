# trie — Kotlin

A generic Trie (prefix tree) that maps `String` keys to values. O(len(key))
insert, lookup, and deletion, plus efficient prefix queries for autocomplete
and spell-checking.

## Usage

```kotlin
import com.codingadventures.trie.Trie

val t = Trie<Int>()
t.insert("apple",  1)
t.insert("app",    2)
t.insert("apply",  3)
t.insert("banana", 4)

t["apple"]                   // 1
t.contains("app")            // true
t.startsWith("app")          // true
t.keysWithPrefix("app")      // [app, apple, apply]

t.delete("apple")
t.contains("apple")          // false
t.contains("app")            // true

t.size                       // 3
```

## Running Tests

```bash
gradle test
```

34 tests covering all operations, Unicode, and a 1000-element smoke test.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
