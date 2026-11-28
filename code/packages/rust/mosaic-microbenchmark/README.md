# Mosaic VM tree-walk microbenchmark

`mosaic-ir-tree-walk-event-stream-4096` measures the actual Mosaic VM walking a
prepared 4,096-leaf component and emitting renderer callbacks. It compares a
`Vec<String>` event log joined at completion with a directly appended `String`.
The source fixture is packaged under `testdata/`; its bytes are checked against
the deterministic generator. Parsing, analysis, and VM setup happen before
timing. Both renderer paths must return identical event streams.

The executable writes only the portable VER07 core report to stdout and writes
the runtime/timing-boundary envelope to the required `--manifest FILE`. Use
`--replay REPORT` for strict duplicate-aware portable replay or
`--replay-envelope MANIFEST` for envelope validation. Standard runs use 40
pairs, 100,000 ns calibration target, and a maximum batch size of 65,536.

This is one of the guest execution lanes, alongside 18 native library ports;
it does not replace a native implementation or measure physical CPU/cache
behavior. Empty host no-op and empty Mosaic call timing are separately retained
as raw diagnostics and never subtracted from the main observations.

```text
cargo test --manifest-path Cargo.toml
cargo run --release --manifest-path Cargo.toml --bin mosaic-microbenchmark -- --pairs 40 --revision <commit> --manifest mosaic.json
```
