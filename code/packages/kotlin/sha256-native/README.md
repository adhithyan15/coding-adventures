# sha256-native (Kotlin)

**Native-through-Rust** SHA-256 for Kotlin — companion to the pure-Kotlin
`sha256` package. Calls the Rust `coding_adventures_sha256` crate through JNI,
**reusing the exact same `sha256_native_jni` cdylib as `java/sha256-native`** —
no new Rust crate.

Kotlin's `object Native` `external` functions resolve to the same
`Java_com_codingadventures_sha256native_Native_*` exports, so the one Rust JNI
library serves both the Java and Kotlin bindings.

## API (`com.codingadventures.sha256native.Sha256Native`)

```kotlin
Sha256Native.sha256Hex("abc".toByteArray())
// ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

Sha256Native.Hasher().use { h ->
    h.update("ab".toByteArray())
    h.update("c".toByteArray())
    h.hexDigest()          // same digest, incremental
}
```

`sha256(ByteArray) -> ByteArray`, `sha256Hex(ByteArray) -> String`, and a
`Hasher` (`AutoCloseable`, `Cleaner`-managed handle) with `update` /
non-destructive `digest` / `hexDigest` / `copy`.

## Building

```
cargo build --manifest-path ../../rust/Cargo.toml -p sha256-native-jni --release
gradle test   # sets -Djava.library.path to rust/target/release
```
