# Changelog — kotlin/treap

## [0.1.0] — 2026-04-25

### Added
- `Node` data class — immutable node with key, priority, left, right
- `Treap` class — purely functional treap
- `Treap.withSeed(Long)` / `Treap.empty()` / `Treap.fromRoot(Node?, Random)` factories
- Companion object helpers: `splitNode`, `splitStrict`, `mergeNodes`, `checkNode`
- `insert(Int)` — random priority via seeded `kotlin.random.Random`
- `insertWithPriority(Int, Double)` — deterministic priority for testing
- `delete(Int)` — via two splits + merge
- `split(Int)` — returns `Pair<Treap, Treap>` (destructurable)
- `mergeTreaps(Treap, Treap)` — top-level function for merging two treaps
- `contains(Int)` — iterative BST search
- `min` / `max` — nullable Int computed properties
- `predecessor(Int)` / `successor(Int)` — nullable Int
- `kthSmallest(Int)` — 1-indexed, throws `IllegalArgumentException` if out of range
- `toSortedList()` — ascending in-order traversal
- `isValidTreap()` — verifies BST and heap properties
- `size`, `height`, `isEmpty` — computed properties
- 44 unit tests mirroring the Java test suite
