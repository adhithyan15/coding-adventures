# sha256-native (Java)

**Native-through-Rust** SHA-256 for the JVM. Instead of reimplementing the
algorithm (that's the pure-Java `sha256` package), this calls the Rust
`coding_adventures_sha256` crate through JNI — via the `sha256_native_jni`
cdylib and the zero-dependency `jni-bridge` (no `jni` crate, no bindgen).

This is the first JVM `*-native` package for a hash in the campaign, and it
establishes the JVM native pattern:

```
rust/sha256-native-jni/   ← Rust cdylib exporting Java_..._native* (uses jni-bridge)
java/sha256-native/
    src/main/java/.../Native.java        ← System.loadLibrary + native methods
    src/main/java/.../Sha256Native.java  ← public wrapper
```

The JVM finds `libsha256_native_jni.{so,dylib}` via `-Djava.library.path`,
pointed at the Rust workspace's `target/release` by `build.gradle.kts`.

## API (`com.codingadventures.sha256native.Sha256Native`)

```java
Sha256Native.sha256Hex("abc".getBytes(StandardCharsets.UTF_8));
// ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

try (Sha256Native.Hasher h = new Sha256Native.Hasher()) {
    h.update("ab".getBytes());
    h.update("c".getBytes());
    String hex = h.hexDigest();      // same digest, computed incrementally
    Sha256Native.Hasher snap = h.copy(); // independent copy
}
```

- `sha256(byte[]) -> byte[]`, `sha256Hex(byte[]) -> String`.
- `Hasher` (an `AutoCloseable`) — `update` / non-destructive `digest` /
  `hexDigest` / `copy`. It owns a native handle freed by `close()` (idempotent),
  with a `Cleaner` safety net if `close()` is missed.

## Building and testing

The BUILD compiles the Rust cdylib in the workspace, then runs `gradle test`
(which sets `java.library.path` to `rust/target/release`):

```
cargo build --manifest-path ../../rust/Cargo.toml -p sha256-native-jni --release
gradle test
```
