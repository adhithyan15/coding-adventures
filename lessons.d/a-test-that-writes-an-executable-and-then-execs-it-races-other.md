---
category: Testing & coverage
---

# A test that writes an executable and then execs it races other test threads' forks (ETXTBSY)

`chief-of-staff-linux-sandbox`'s
`what_runs_is_the_file_prepared_not_whatever_the_path_names_later` copied the
probe binary with `std::fs::copy`, then exec'd the copy. It passed every run
locally, but failed in CI with `ETXTBSY` ("Text file busy").

The cause is a classic Linux race. While the test thread held the copy's
write descriptor, another test thread forked a child. That child inherits the
descriptor, close-on-exec, and keeps it until its own exec. Exec'ing a file
that any process holds open for writing fails with `ETXTBSY`. The more tests
that spawn processes in parallel, the likelier it gets, so a fast local run
can miss it.

What to do instead:
- Never write a file you are about to exec in a multi-threaded test process.
  Hard-link the existing binary instead. Make the link beside the original
  (`Path::with_file_name`), so it is on the same filesystem: a hard link into
  `/tmp` fails with `EXDEV` when `target/` is on another mount.
- If a copy is unavoidable, write it, close it, and retry the exec on
  `ETXTBSY`. Or do the write in a child process.
