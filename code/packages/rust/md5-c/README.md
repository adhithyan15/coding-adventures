# md5-c

C ABI wrapper for the `coding_adventures_md5` crate (staticlib + cdylib), for
Swift/C compile-time interop. Used by `swift/md5-native`. See
`swift/md5-native/Sources/CMd5/include/md5_c.h` for the contract. MD5 is broken —
checksum use only.
