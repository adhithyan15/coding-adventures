---
category: Rust
---

# A private staging directory does not make rename into its backup path no-clobber

The independent CV02 review populated the private staging directory with an unknown `old` file before the Install hook returned. `fs::rename(destination, old)` overwrote it and cleanup removed the replacement, returning success. The native temp-only regression reproduced this before repair. Create the backup with an exclusive no-clobber hard link, track whether that link was created, then remove only the observed original destination. Cleanup must never claim an old-path occupant solely because it has the same object ID as the original. Test both different-file and same-inode occupants, and failure after backup creation but before original unlink.
