# A tail expression holding a MutexGuard temporary compiles locally but fails CI (E0597)

`String::from_utf8(buf.lock().unwrap().clone()).expect(...)` as the **final
expression** of a function holds the `MutexGuard` temporary until the end of the
block — i.e. it is dropped *after* the `Arc` it borrows goes out of scope. A
newer local rustc accepted this; the CI toolchain rejected it as E0597
("`buf` does not live long enough ... dropped here while still borrowed"). BA2's
JIT-capture helpers hit this and turned a green local run into a red CI build.
LESSON: the local rustc can be MORE lenient than CI's — a clean local build is
not proof CI compiles. Bind the cloned value to a `let` first
(`let bytes = buf.lock().unwrap().clone();`) so the guard drops at that
statement. When a CI build error can't be reproduced locally, suspect a
toolchain-version difference (temporary-lifetime/edition rules, lint levels)
rather than assuming the local result holds.
