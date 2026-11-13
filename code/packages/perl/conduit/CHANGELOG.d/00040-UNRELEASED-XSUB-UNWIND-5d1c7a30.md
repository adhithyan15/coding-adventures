## Unreleased — XSUBs raise Perl errors from `extern "C-unwind"`

- Every XSUB is now `unsafe extern "C-unwind"` and is registered with
  `perl_bridge::newXS_unwind`, so a Perl `die` (an SEH unwind on Windows, a
  `longjmp` elsewhere) may legally leave it. Previously the argument-count and
  bind-failure errors were raised from plain `extern "C"` functions, which abort
  the process on Windows (the follow-up recorded in `perl-bridge`'s changelog).
- The errors are no longer raised from inside the `catch_unwind` closure. The
  closure returns `Some(message)`, drops everything it owns, and the XSUB itself
  then calls `die`, so no Rust frame with live state sits between `die` and Perl.
  A caught panic is now reported as `conduit: internal error` instead of being
  swallowed.
- **Fixed (found by the security review of this change):** `new_server` took
  ownership of (and freed) the native application before the bind could fail, while
  the Perl wrapper's `DESTROY` still disposed of the same handle afterwards. Once the
  error no longer `longjmp`ed past the destructors, that was a double free. The
  application is now only borrowed while the server is built and is released after a
  successful bind, so a failed bind leaves the handle valid and owned by the wrapper.
- New `t/05-xsub-errors.t` drives all eight `die` paths (seven arity checks plus a
  bind to an occupied port) through the native layer, and the failed bind again
  through `CodingAdventures::Conduit->bind` followed by object destruction (it
  segfaults without the fix above). It also proves the interpreter and a fresh
  server still work afterwards. `BUILD_windows` gains a comment naming it so
  a Windows leg runs on the pull request that changes it.
