# A test's own `#[cfg(not(target_os = "windows"))]` guard can be stale evidence, not a platform limitation

`embeddable-http-server`'s `sharded_http_server_serves_concurrent_clients_across_shards`
test — the ONLY caller of the `#[cfg(target_os = "windows")]` variant of
`bind_native_sharded_http_server` — was itself gated `#[cfg(not(target_os =
"windows"))]`. Once the build tool started actually building this package on
Windows (see the BUILD-closure lesson above), that made the Windows
`bind_native_sharded_http_server` dead code under `-D warnings`
(`function 'bind_native_sharded_http_server' is never used`).

**Investigation, not just suppression:** `#[allow(dead_code)]` would have silenced
this without checking whether the underlying claim (sharded Windows HTTP serving
doesn't work / isn't tested) was still true. It wasn't: `bind_windows_sharded`
(`ShardedHttpServer<WindowsTransportPlatform>`, IOCP-backed) is a complete,
real implementation sitting right next to the Unix ones, not a stub. Two sibling
tests in the same file (`mailbox_http_server_*`) ARE legitimately Windows-excluded
because `MailboxHttpServer` has no Windows reactor at all — so the sharded test's
exclusion reads, at a glance, like it follows the same precedent, but it doesn't:
different underlying type, different actual platform support.

**Fix:** remove the `#[cfg(not(target_os = "windows"))]` from the sharded test so
it runs everywhere the implementation exists — verified with
`cargo check --target x86_64-pc-windows-gnu --tests` under `RUSTFLAGS="-D warnings"`
(no Windows box needed for a cross-compile check; only actually *running* the test
needs one, which real Windows CI will do on push). **Lesson:** when a `-D warnings`
lint fires on a `#[cfg(target_os = "windows")]` item, check whether its only caller
is excluded from Windows before reaching for `allow` — the cfg may be describing a
gap that was since closed, not a permanent constraint.

**Correction (same PR, next CI round):** this was wrong, and the wrongness was
only reachable by actually running on Windows — `cargo check --target
x86_64-pc-windows-gnu` type-checks but never *executes* `bind_windows_sharded`,
so it happily passed while the runtime path was broken. Real windows-latest CI
panicked: `"SO_REUSEPORT is not supported by the Windows TCP provider"`. Tracing
into `tcp_runtime::bind_sharded_runtime_with_state`, the "default" sharding path
(used by every platform except macOS/BSD) sets `reuse_port = true` for any
`worker_count > 1` — correct for Linux (where `SO_REUSEPORT` load-balances) but
fatal for Windows (no `SO_REUSEPORT` at all). macOS/BSD hit the identical
"`SO_REUSEPORT` exists but doesn't load-balance" problem and already fixed it
with an explicit accept fan-out (`FanoutAcceptor`) — but that fan-out's
`spawn`/`run` are `#[cfg(unix)]`-only, so Windows has no route around the
`reuse_port` panic today. `bind_windows_sharded` is also a **real, currently
broken for `worker_count > 1`** part of the public API — `web-core`'s
`ShardedWebServer` and `conduit` both wrap it — but neither is exercised on
Windows CI yet either, so this was a pre-existing gap this PR's build-plan fix
merely exposed here first, not something this PR introduced or should silently
paper over. Reverted the test back to `#[cfg(not(target_os = "windows"))]`, kept
the Windows-only dispatcher function alive with a `#[allow(dead_code)]` whose
comment states exactly why (a rare case where the suppression IS the honest
answer — see next lesson) and what would need to exist (a Windows fan-out
acceptor in `tcp-runtime`) to remove it.

**Generalized lesson:** "verified with a cross-compile check" only proves the
code *type-checks* for that target — it does not execute a single line of
target-specific runtime logic (socket options, syscalls, ABI-dependent
behavior). Treat a cross-compile check as ruling out compile errors only, never
as confirmation that a previously-excluded-from-Windows test will actually pass
there. When you can't run the real target and the round-trip to find out is
expensive (~80 minutes of CI here), say so plainly in the commit/PR rather than
implying the cross-compile check was sufficient evidence.
