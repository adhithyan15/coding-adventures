---
category: Testing & coverage
---

# Native benchmark reports need cross-language replay as well as local round trips

The Ruby microbenchmark driver's own replay accepted its private `machine` and
checksum metadata. Rust and C++ deliberately validate a closed report schema,
so the same report could not pass their readers. TypeScript had a similar drift.

Use the shared `arch` field and keep private checksum state inside collection.
Check the emitted CLI keys and replay actual native reports in the other lanes.
Six producers and six analysis/replay implementations passed all 36 combinations
after alignment. Record which checks exercise full schema validation: the Go
check used its analysis API, so it does not prove that Go rejects extra metadata.
