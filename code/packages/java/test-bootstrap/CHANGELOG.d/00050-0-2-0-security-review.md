## 0.2.0 - security review fixes (VER06f)

- **Security fix.** A throwable whose `getStackTrace()` throws any `Error` (an
  `InternalError`, a custom `Error`) no longer escapes the executor and ends the
  campaign: it is a crash with no frames; an `OutOfMemoryError` there is a counted
  OOM outcome.
- **Security fix.** Budget exhaustion is sticky and shared between `Handle.tick()`
  and `Handle.reserve()`: after the first breach both rethrow it immediately, so a
  target that swallows a breach and calls the other hook is still a timeout or an
  OOM, not a pass.
- **Security fix (CWE-400).** The corpus runner's aggregate `mutate_sequence`
  budget charges the work a call really did (growing buffers included) instead of
  `steps x initial length`: each call gets the remaining budget as its work cap.
- The input is cloned before the guarded region of `runTarget`, so an engine OOM
  on the copy is not counted as a target OOM.
