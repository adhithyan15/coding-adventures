# trie — Java

A generic Trie (prefix tree) that maps `String` keys to values. Supports
O(len(key)) insert, lookup, and deletion, plus efficient prefix-based key
enumeration — ideal for autocomplete, spell-checking, and prefix routing.

## Usage

```java
import com.codingadventures.trie.Trie;

Trie<Integer> t = new Trie<>();
t.insert("apple",  1);
t.insert("app",    2);
t.insert("apply",  3);
t.insert("banana", 4);

t.get("apple");              // Optional[1]
t.contains("app");           // true
t.startsWith("app");         // true
t.keysWithPrefix("app");     // [app, apple, apply]

t.delete("apple");
t.contains("apple");         // false
t.contains("app");           // true  (shared prefix survives)

t.size();                    // 3
```

## How it works

```
(root)
├── a
│   └── p
│       ├── p (*)   ← "app"
│       │   └── l
│       │       ├── e (*)  ← "apple"
│       │       └── y (*)  ← "apply"
└── b
    └── a → n → a → n → a (*)  ← "banana"
```

Each node holds a `HashMap<Character, Node>` for children. This generalises
to any Unicode character set. A boolean `isEnd` flag marks complete keys.

## Running Tests

```bash
gradle test
```

34 tests covering insert, get, contains, startsWith, delete, prefix queries,
size, empty key, Unicode keys, and a 1000-element smoke test.

## Part of the Coding Adventures series

Java counterpart to the Python, Rust, Go, TypeScript, and Kotlin implementations.
