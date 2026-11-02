---
category: Cross-platform & Windows BUILD_windows
---

# copyFileSync with COPYFILE_EXCL onto a directory throws EPERM on Windows, not EEXIST, so an EEXIST-only slot-taken check misbehaves there

**What went wrong.** Engram's Electron host (`code/programs/mosaic/engram-app/host/electron/host.js`, from #18315) quarantines an unreadable snapshot by copying it to the first free name among `.corrupt`, `.corrupt.1` … `.corrupt.9`. It uses `copyFileSync(src, dst, constants.COPYFILE_EXCL)` and treats only `error.code === "EEXIST"` as "slot taken, try the next one". Any other error switches persistence off.

The test `snapshot_quarantine::electron_never_replaces_an_earlier_quarantine` puts a *directory* in slot 1. On macOS and Linux, copying onto a directory with `COPYFILE_EXCL` reports `EEXIST`, so the host moved on and the test passed. On Windows the same call reports `EPERM` (or `EISDIR`), so the host disabled persistence. Slot 2 was never written, and the test failed with `NotFound` in `build (windows-2025)`. It surfaced on #18477, the first PR in a while whose diff made the Windows lane run engram-app's tests.

**The fix.** Treat the slot as taken when the copy fails *and* something already exists at that name (`error.code === "EEXIST" || existsSync(dst)`). Any other failure still disables persistence.

**Do differently.** Never decide "the destination already exists" from an errno alone when the destination might be a directory. Errno for exclusive create/copy onto a directory differs between POSIX and Windows. Check existence explicitly, or use `lstat`. A quarantine or exclusive-write test that passes on macOS proves nothing about Windows; make sure the Windows lane actually runs it.
