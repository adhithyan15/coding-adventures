# md5-native (Java)

**Native-through-Rust** MD5 for the JVM — companion to the pure-Java `md5`
package. Calls the Rust `coding_adventures_md5` crate through JNI (the
`md5_native_jni` cdylib, built with the zero-dependency `jni-bridge`). Reuses the
JVM native pattern established by `java/sha256-native`.

> **Security:** MD5 is cryptographically broken — checksum use only.

## API (`com.codingadventures.md5native.Md5Native`)

```java
Md5Native.hexString("abc".getBytes(StandardCharsets.UTF_8)); // 900150983cd24fb0d6963f7d28e17f72
try (Md5Native.Digest h = new Md5Native.Digest()) {
    h.update("ab".getBytes());
    h.update("c".getBytes());
    h.hexDigest();
}
```

`sumMd5(byte[]) -> byte[]` (16 bytes), `hexString(byte[]) -> String`, and a
`Digest` (`AutoCloseable`) with `update` / non-destructive `digest` / `hexDigest`
/ `copy`, its native handle freed by `close()` with a `Cleaner` safety net.

## Building

```
cargo build --manifest-path ../../rust/Cargo.toml -p md5-native-jni --release
gradle test   # sets -Djava.library.path to rust/target/release
```
