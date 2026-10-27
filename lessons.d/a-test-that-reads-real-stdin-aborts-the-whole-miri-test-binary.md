---
category: Testing & coverage
---

# A test that reads real stdin aborts the whole Miri test binary, and isolation-error=warn does not save it

`twig-vm`'s `host_read_byte_returns_value_in_dest` called `std::io::stdin().read()`. Under Miri (isolation on by default) that is an **unsupported operation** — not an `io::Error` the code under test can handle — so Miri aborts the entire test binary. libtest runs tests in one process, so every twig-vm lib test scheduled after it silently never ran under `code/scripts/miri-twig-vm.sh` or the nightly `miri-twig-vm` job: Miri coverage of the crate was effectively zero past that point, and the job "failed" for a reason unrelated to UB.

`MIRIFLAGS=-Zmiri-isolation-error=warn` does **not** fix it, even though Miri's own error message suggests that flag. It only turns *isolation errors* (e.g. `open`, which can return `EACCES`) into errno returns; stdin reads are rejected as *unsupported*, and abort identically with or without the flag (verified with a one-test probe crate on nightly).

Fix: put an injection seam in front of the host stream (`with_host_stdin`, a `cfg(test)` thread-local `Box<dyn BufRead>` override, real `StdinLock` otherwise) and feed tests in-memory input. That also made the test deterministic — reading real stdin only ever saw EOF in CI (`/dev/null`) and blocked on a terminal locally.

Do differently: tests must never touch real stdin, files, sockets or processes in crates that Miri runs. Route host I/O through a reader/writer seam from day one, and when a Miri run "fails", check the test count it reached — an abort hides everything after it.
