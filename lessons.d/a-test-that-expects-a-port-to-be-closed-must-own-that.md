---
category: Testing & coverage
---

# A test that expects a port to be closed must own that port in both address families

`transport-platform`'s `ipv6_listener_stays_ipv6_only` bound `[::]:0`, read
back port P, and asserted that `connect(127.0.0.1:P)` *fails*. On main's
sharded build (~970 packages testing on one runner) it panicked with
"ipv6 wildcard listener unexpectedly accepted an ipv4 connection", although
the code calls `set_only_v6(true)`.

TCP port numbers are shared between IPv4 and IPv6. The kernel picked P as
free *for IPv6*; nothing stopped another process from listening on
`127.0.0.1:P` at the same moment, and the connect reached that process.
"Connection refused" is an assertion about the whole machine, not about the
socket under test.

**Fix:** claim the IPv4 side first (`std::net::TcpListener::bind("127.0.0.1:0")`
-> P), then bind the platform listener on `[::]:P`. That bind only
succeeds when the socket is V6ONLY, and the connect to `127.0.0.1:P` must
then be accepted by *our* IPv4 listener. Retry a few fresh ports on bind
failure so a stranger grabbing `[::]:P` between the two binds is not a
flake; a dual-stack socket still fails every attempt.

**Rule:** never assert that a port is unreachable unless the test itself
holds every other way to reach it.

Note: this container has no IPv6 at all (`EAFNOSUPPORT`), so the old and
new tests both fail locally; CI's Linux and Windows runners exercise it.
