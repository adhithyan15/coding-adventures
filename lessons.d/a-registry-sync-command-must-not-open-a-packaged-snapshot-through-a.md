---
category: Security boundaries
---

# A registry sync command must not open a packaged snapshot through a symlink

The Ruby build-tool registry sync initially used `binwrite` on its packaged
snapshot. A checkout-level symlink at that path could redirect the documented
maintenance command to overwrite a writable file outside the checkout; its
`--check` mode also followed the link. Check the source and destination with
`lstat`, reject linked parent directories, write a same-directory temporary
regular file, and rename it over the destination entry. Keep the read-only
check strict too. When adding a generated package-data sync command, review
its write path as a security boundary even if the data itself is inert JSON.
