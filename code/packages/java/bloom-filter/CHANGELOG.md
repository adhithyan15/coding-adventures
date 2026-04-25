# Changelog — bloom-filter (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a generic Bloom filter.
- `BloomFilter(expectedItems, falsePositiveRate)` — auto-sizes using optimal m/k formulas.
- `BloomFilter(bitCount, hashCount, explicit)` — explicit parameter constructor.
- `add(element)` — sets k bits using double-hashing; O(k) time.
- `contains(element)` — zero false negatives; bounded false positive rate.
- `bitCount()`, `hashCount()`, `bitsSet()`, `size()` — property accessors.
- `fillRatio()` — fraction of bits set to 1.
- `estimatedFalsePositiveRate()` — current FPR estimate based on fill.
- `isOverCapacity()` — true when elements added exceed expectedItems.
- `sizeBytes()` — memory usage of the bit array.
- `optimalM(n, p)`, `optimalK(m, n)`, `capacityForMemory(bytes, p)` — static utilities.
- Inline FNV-1a 32-bit and DJB2 hash functions with fmix32 decorrelation finalizer.
- Double-hashing scheme (k probes from 2 hash values) matching industry practice.
- 34 unit tests covering zero false negatives, FPR bounds, statistics, over-capacity, type generics, and input validation.
