# Sha256Native (Swift)

**Native-through-Rust** SHA-256 for Swift. Instead of reimplementing the
algorithm (that's the pure-Swift `sha256` package), this links the Rust
`coding_adventures_sha256` crate at compile time — via the `sha256-c` static
library — and calls it through a C ABI.

This is the first Swift `*-native` package built in the pure-port + native
campaign, and it establishes the Swift native pattern:

```
rust/sha256-c/            ← Rust crate → libsha256_c.a (staticlib) + C header
swift/sha256-native/
    Sources/CSha256/          ← SPM "C target": header + module map only
        include/sha256_c.h
    Sources/Sha256Native/     ← Swift wrapper that `import CSha256`
```

Swift calls the C symbols directly — no runtime bridge, no boxing. The digest is
written into a caller-owned 32-byte buffer, so nothing is allocated across the
boundary on the one-shot path; the streaming `Hasher` owns an opaque native
handle and frees it in `deinit`.

## API

```swift
import Sha256Native

Sha256Native.hexString(Array("abc".utf8))
// ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

let h = Sha256Native.Hasher()
h.update(Array("ab".utf8))
h.update(Array("c".utf8))
h.hexDigest()                 // same digest, computed incrementally
let snapshot = h.copy()       // independent copy
```

`digest(_:) -> [UInt8]`, `hexString(_:) -> String`, and a `Hasher` with
`update` / non-destructive `digest` / `hexDigest` / `copy`.

## Building and testing

The BUILD file compiles the Rust static library, copies it into
`Sources/CSha256/`, and runs the tests:

```
cargo build --manifest-path ../../rust/Cargo.toml -p sha256-c --release
cp ../../rust/target/release/libsha256_c.a Sources/CSha256/
swift test
```
