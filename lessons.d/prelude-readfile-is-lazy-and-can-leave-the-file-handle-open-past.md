---
category: Haskell
---

# `Prelude.readFile` is lazy and can leave the file handle open past the point the caller thinks it's done

`loadCow`'s `contents <- readFile cowPath; pure (extractHeredocBody contents)` compiled and worked, but a test that immediately `removeDirectoryRecursive`s the temp dir the file lives in intermittently hit `PermissionDenied: ... DeleteFile ...: The process cannot access the file because it is being used by another process` on Windows — the handle hadn't been finalized/closed yet even though the returned `String` looked fully consumed. Fix: use `System.IO.readFile'` (the strict variant, base >=4.15/GHC 9.0+) instead of `readFile` for any file this repo's code reads once and expects to be free of afterward (matches the existing project's minimum bound of `base >=4.14`, so confirm the GHC version actually in use ships strict `readFile'` before relying on it elsewhere).
