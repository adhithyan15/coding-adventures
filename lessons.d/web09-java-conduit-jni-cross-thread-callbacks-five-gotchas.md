# WEB09 Java Conduit / JNI cross-thread callbacks — five gotchas

**Date:** 2026-04-27

Porting Conduit to Java over `web-core` via `jni-bridge` surfaced a cluster of JNI-specific traps. All five recur for any future JVM↔Rust callback port (Kotlin, etc.):

1. **`JNIEnv*` is thread-local; the JavaVM is not.** web-core dispatches on Rust I/O threads the JVM never created. You cannot reuse the registration-call `env`. Capture the `JavaVM*` once via `GetJavaVM` (offset 219 on the JNIEnv table) on a JVM thread, then on each I/O thread call `AttachCurrentThreadAsDaemon` (offset 7 on the *JavaVM invocation* table — a different table, `JavaVM = *const *const c_void`). Daemon attach is idempotent, needs no `DetachCurrentThread`, and doesn't block JVM shutdown. Plain `AttachCurrentThread` requires a matching detach before the thread exits or the JVM aborts.

2. **Local refs leak on a self-attached thread.** Local references are normally freed when a native method returns to Java — but an I/O thread that attached itself and loops forever never returns, so every `NewObjectA`/`NewStringUTF`/`CallObjectMethod` local ref accumulates = unbounded leak per request. Bracket each dispatch with `PushLocalFrame(env, n)` / `PopLocalFrame(env, null)` (offsets 19/20) and copy everything you need into owned Rust values before popping.

3. **Handler objects must be promoted to global refs.** A Java lambda passed to `addRoute` is a local ref, dead after the native call returns. `NewGlobalRef` (offset 21) it at registration; `DeleteGlobalRef` (22) on dispose. `jclass` from `FindClass` is also local — pin the classes you cache as globals too, and resolve all method IDs on a JVM thread (FindClass from a native thread uses the system classloader, which can't see app classes).

4. **Disjoint closure capture drops the Send+Sync wrapper.** web-core closures must be `Send + Sync`. Wrapping a raw `jobject` in a `struct Obj(jobject); unsafe impl Send/Sync` is not enough if the closure writes `obj.0` — Rust 2021 captures only the inner `*mut c_void` field (not Send). Add a `fn get(&self) -> jobject { self.0 }` and call `obj.get()` so the whole wrapper is captured. (Same lesson as the Node port's `ThreadSafePtr::get`.)

5. **A Rust panic must never unwind across `extern "C"`.** A poisoned `Mutex` makes `.lock().unwrap()` panic; if that happens inside a JNI entrypoint called from a JVM thread, the unwind crosses the FFI boundary = UB (usually a JVM abort). Use `.lock().unwrap_or_else(|e| e.into_inner())` (poison-tolerant), and null-check every peer `jlong` before `Box::from_raw`/deref so a use-after-close lifecycle bug degrades to a safe no-op instead of dereferencing a dangling pointer. Route handler *errors* as data (an `Outcome` enum), never as panics.

Also: the security sub-agent flagged a `pct_decode` "off-by-one" (`i + 2 < len`) as HIGH — it was a false positive (`i+2 < len` ⟺ `i+2` is a valid index; a trailing `%XX` decodes fine). Always settle a claimed boundary bug with a targeted unit test before "fixing" it; here the fix would have introduced the bug.

---
