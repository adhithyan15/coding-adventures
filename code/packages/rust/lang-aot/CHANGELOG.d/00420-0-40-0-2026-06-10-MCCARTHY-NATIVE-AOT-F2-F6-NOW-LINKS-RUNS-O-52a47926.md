## 0.40.0 — 2026-06-10 — McCarthy **native AOT** (F2–F6) now links + runs on macOS arm64 (LANG77 / W14a)

No `lang-aot` source change — the fix is in `code-packager` 0.5.0 (Mach-O external
symbols now carry the leading `_` C decoration), which the native macOS path already
drives. New verify-by-running test `tests/macos_native_lisp.rs` (gated to
`target_os = "macos"`): compiles McCarthy through the native `aarch64-backend`,
links the runtime archive with `ld`, and runs — `(CAR (CONS 7 9))`→7, `(CDR …)`→9,
`(ATOM 7)`→1, `(EQ 7 7)`→1, `(COND …)`→11, `(EQ (QUOTE A) (QUOTE A))`→1. Closes the
macOS runtime-link gap that previously failed native lisp at link time. Lambda (F7)
is still backend-refused — the separate W14b slice.

