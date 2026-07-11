# md5-native (Kotlin)

**Native-through-Rust** MD5 for Kotlin — companion to the pure-Kotlin `md5`
package. Calls the Rust `coding_adventures_md5` crate through JNI, **reusing the
same `md5_native_jni` cdylib as `java/md5-native`** (no new Rust crate). Kotlin's
`object Native` `external` functions resolve to the same
`Java_com_codingadventures_md5native_Native_*` exports.

> **Security:** MD5 is cryptographically broken — checksum use only.

## API (`com.codingadventures.md5native.Md5Native`)

`sumMd5(ByteArray) -> ByteArray` (16 bytes), `hexString(ByteArray) -> String`,
and a `Digest` (`AutoCloseable`, `Cleaner`-managed handle) with `update` /
non-destructive `digest` / `hexDigest` / `copy`.

## Building

```
cargo build --manifest-path ../../rust/Cargo.toml -p md5-native-jni --release
gradle test
```
