## Unreleased — XSUBs raise Perl errors from `extern "C-unwind"`

- Every XSUB is now `unsafe extern "C-unwind"` and is registered with
  `perl_bridge::newXS_unwind`, so a Perl `die` (an SEH unwind on Windows, a
  `longjmp` elsewhere) may legally leave it. Previously the argument-count and
  bind-failure errors were raised from plain `extern "C"` functions, which abort
  the process on Windows (the follow-up recorded in `perl-bridge`'s changelog).
- The errors are no longer raised from inside the `catch_unwind` closure. The
  closure returns `Some(message)`, drops everything it owns, and the XSUB itself
  then calls `die`, so no Rust frame with live state sits between `die` and Perl.
  A failed bind therefore frees the web application before the error is raised.
- New `t/05-xsub-errors.t` drives all eight `die` paths (seven arity checks plus a
  bind to an occupied port) through the native layer and proves the interpreter and
  a fresh server still work afterwards. `BUILD_windows` gains a comment naming it so
  a Windows leg runs on the pull request that changes it.
