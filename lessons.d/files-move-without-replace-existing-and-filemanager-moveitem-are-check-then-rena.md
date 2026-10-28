---
category: Cross-platform & Windows BUILD_windows
---

# Files.move without REPLACE_EXISTING and FileManager.moveItem are check-then-rename on POSIX, so neither is a no-clobber move

**What went wrong.** The generated Compose and SwiftUI hosts quarantined a rejected state file with `Files.move(src, dst)` (no `REPLACE_EXISTING`) and `FileManager.moveItem(at:to:)`. Both were documented as moves that "refuse an existing target". That holds on Windows, where the move is `MoveFileEx` without `MOVEFILE_REPLACE_EXISTING`. On Linux, macOS and Android, however, the JDK and Foundation check the target and then call rename(2), which atomically *replaces* anything that appeared in between. So an earlier quarantine, possibly the user's only copy, could be overwritten. The same audit found that hosts deciding "slot taken" from the error of a failed exclusive step repeat the Engram Electron bug on Windows. An exclusive create or copy onto a directory reports `EEXIST` on POSIX but access denied (`EPERM`, `AccessDeniedException`, `UnauthorizedAccessException`) on Windows.

**The fix.** Claim the name with a truly exclusive create (`O_CREAT | O_EXCL`, Windows `CREATE_NEW`: `Files.createFile`, `open`, Dart `createSync(exclusive: true)`, Qt `QIODevice::NewOnly`). Then rename atomically onto that empty placeholder (`ATOMIC_MOVE`, rename(2), `std::filesystem::rename`). The rename can only replace the placeholder the host just made. Do not use the JDK's `REPLACE_EXISTING` without `ATOMIC_MOVE`; it is delete-then-rename. When the claim fails, look at the name again without following links, and skip it if anything is there. Any other failure is a real failure.

**Do differently.** A "no-replace" flag is not exclusivity. Check how the runtime implements it on each OS before relying on it to protect data. Never infer "destination exists" from an error code. Look at the name with an lstat-style check that also sees dangling links.
