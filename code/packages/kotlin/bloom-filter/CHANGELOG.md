# Changelog — bloom-filter (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a generic Bloom filter in idiomatic Kotlin.
- `BloomFilter(expectedItems, falsePositiveRate)` — operator `invoke` factory, auto-sizes using optimal m/k formulas.
- `BloomFilter.explicit(bitCount, hashCount)` — explicit parameter factory.
- `add(element)` — sets k bits using double-hashing; O(k) time.
- `contains(element)` — zero false negatives; bounded false positive rate.
- `bitCount`, `hashCount`, `bitsSet`, `size` — read-only properties.
- `fillRatio` — fraction of bits set to 1.
- `estimatedFalsePositiveRate` — current FPR estimate based on fill.
- `isOverCapacity` — true when elements added exceed expectedItems.
- `sizeBytes` — memory usage of the bit array.
- `optimalM(n, p)`, `optimalK(m, n)`, `capacityForMemory(bytes, p)` — companion object utilities.
- Inline FNV-1a 32-bit and DJB2 hash functions with fmix32 decorrelation finalizer.
- Double-hashing scheme matching the Python and Java implementations.
- 40 unit tests covering zero false negatives, FPR bounds, statistics, over-capacity, generics, input validation, and hash function determinism.
