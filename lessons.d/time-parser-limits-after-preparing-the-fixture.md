---
category: Testing & coverage
---

# Time parser limits after preparing the fixture

**What happened.** A Go security regression timed an oversized Cargo
manifest by starting its clock before a helper created a temporary repository
and wrote a 1 MiB file. On a Windows CI runner, fixture I/O pushed the
measurement over the strict one-second parser limit even though local parser
execution took only milliseconds.

**What to do.** Build the same isolated fixture and package list first, start
the timer immediately before the parser or resolver call, and stop it as soon
as that call returns. Keep the original limit and error assertions. For
repository-wide tests, share only a snapshot keyed to the canonical checkout
root; return deep copies for any test that mutates packages or graph data.
Propagate discovery and resolution errors instead of caching a partial
snapshot. Never reuse a temporary fixture across tests.
