# Concurrent `cargo test`/`cargo build` across worktrees hangs on a shared registry lock, not a compile

Running `cargo test -p lang-aot` (a large downstream consumer of `wasm-execution`)
while other Claude Code sessions were presumably running their own `cargo` commands in
sibling worktrees appeared to hang indefinitely: the process sat at ~0% CPU for 10+
minutes, no `rustc` child process ever spawned, and `lsof` showed it holding only
`~/.cargo/.global-cache` open with no network sockets. Killing and retrying reproduced
the exact same hang. All worktrees share one `~/.cargo` directory, so cargo's registry
index/global-cache lock is a **machine-wide** mutex across every concurrent `cargo`
invocation from every worktree and every agent session, not a per-worktree one.

**Fix:** add `--offline` when the dependency set hasn't changed (true for a diff that
only edits existing crates' source, as here) -- it skips the registry-index touch
entirely and the same command that hung for 10+ minutes ran to completion in seconds.
**Diagnostic tell:** near-zero CPU, no `rustc`/`ld` child process, and the parent
`cargo` process holding only `.global-cache` open (check with `lsof -p <pid>`) means
it's blocked on a lock, not doing real compilation work -- don't keep waiting or
repeatedly killing/retrying without `--offline`, and don't mistake it for a bug in the
code just compiled.
