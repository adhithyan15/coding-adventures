---
category: Security boundaries
---

# Windows shard enumeration must query the pinned directory handle

`os.scandir(path)` after `_directory` validates that path is not a safe read
boundary. A concurrent process can rename the validated directory and put a
junction at its old name before `scandir` opens it, so outside JSON filenames
can enter shard-owner selection even when later writes are handle-relative.

Enumerate the already-validated Windows directory handle with
`NtQueryDirectoryFile`. Validate each native record's offset, byte length,
UTF-16 name, and reparse or directory attributes before selecting owners;
reopen selected names relative to that same parent handle for a no-follow kind
check. A deterministic test should swap the directory for a privilege-free
junction immediately after pathname prevalidation and require rejection for
both ordinary shard files and book-generation owner entries.
