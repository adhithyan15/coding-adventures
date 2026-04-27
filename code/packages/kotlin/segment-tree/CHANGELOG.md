# Changelog — kotlin/segment-tree

## [0.1.0] — 2026-04-25

### Added
- `SegmentTree<T>` class — generic, array-backed, 1-indexed
- Constructor `SegmentTree(Array<T>, (T, T) -> T, T)` — lambda combine
- `query(ql: Int, qr: Int): T` — range aggregate in O(log n)
- `update(index: Int, value: T)` — point update in O(log n)
- `toList(): List<T>` — reconstruct array from leaves in O(n)
- `val size: Int`, `val isEmpty: Boolean` — computed properties
- Companion object factories: `sumTree`, `minTree`, `maxTree`, `gcdTree`
- `require()` for precondition checks (IllegalArgumentException)
- Kotlin idioms: lambda combine, `intArrayOf`, `arrayOfNulls`, `kotlin.math.*`
- 40 unit tests mirroring the Java suite
