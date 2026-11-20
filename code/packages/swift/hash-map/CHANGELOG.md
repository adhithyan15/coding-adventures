# Changelog — HashMap (Swift)

All notable changes to this package are documented here.

## Unreleased

- Replay the neutral DT18 v1 corpus through the public Swift map and hash APIs,
  including directed ownership/error cases and both-strategy exhaustive
  persistent-history streams.
- Register the canonical shared corpus as an exact repository source input so
  package fallback hashes change when its cases change.

## 0.1.0 — Initial release

- Implement persistent separate-chaining and linear-probing hash maps.
- Add tombstone-safe deletion, checked resize thresholds, and finite capacity
  and key budgets.
- Use closed, canonical byte/string keys and exact DT17 FNV-1a, Murmur3, and
  SipHash-2-4 strategies from the sibling Swift hash-functions package.
- Define physical-storage iteration order, structural ownership, shallow value
  retention, optional-nil membership, and right-biased construction/merge.
- Add package-native tests, downstream hash-set composition coverage, release
  builds, truthful Unix/macOS and Windows fronts, and an empty capability
  profile.
