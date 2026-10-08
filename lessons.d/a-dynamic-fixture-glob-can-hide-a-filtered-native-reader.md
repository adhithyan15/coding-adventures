---
category: CI & GitHub Actions
---

# A dynamic fixture glob can hide a filtered native reader

The first source-collection fixture inventory missed Elixir because its test
uses the broad `source-collection-*.json` glob and then filters decoded cases
to the seven package-local registry fixtures. A filename search that only
counts explicit case names or assumes the glob replays all twenty cases gets
both the reader set and the subfamily map wrong. Inspect each native test's
semantic filter and its BUILD front, then record exactly which cases that
front executes. Include dynamic globs and per-language source-reference drift
checks before freezing a fixture-to-native CI selector.
