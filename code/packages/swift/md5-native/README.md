# Md5Native (Swift)

**Native-through-Rust** MD5 for Swift — the companion to the pure-Swift `md5`
package. Links the Rust `coding_adventures_md5` crate at compile time (via the
`md5-c` static library) and calls it through a C ABI, exactly like
`swift/sha256-native`.

> **Security:** MD5 is cryptographically broken — checksum use only.

## API

```swift
import Md5Native
Md5Native.hexString(Array("abc".utf8))   // 900150983cd24fb0d6963f7d28e17f72
let h = Md5Native.Hasher()
h.update(Array("ab".utf8)); h.update(Array("c".utf8))
h.hexDigest()                            // same digest, incremental
```

`digest(_:) -> [UInt8]` (16 bytes), `hexString(_:) -> String`, and a `Hasher`
(`update` / non-destructive `digest` / `hexDigest` / `copy`) whose native handle
is freed in `deinit`.

## Building

The BUILD compiles the Rust static library, copies `libmd5_c.a` into
`Sources/CMd5/`, and runs `swift test`:

```
cargo build --manifest-path ../../rust/Cargo.toml -p md5-c --release
cp ../../rust/target/release/libmd5_c.a Sources/CMd5/
swift test
```
