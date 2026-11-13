---
category: Perl
---

# A forked Perl test server must end on client EOF, not on sleep and kill

`perl/tcp-client`'s read-timeout test forked a server child that accepted a
connection and then did `sleep(10)`, and the parent stopped it with
`kill('TERM', $pid)` followed by `waitpid`. On Unix that works. On Windows
`fork` is a thread, and a pseudo-process blocked in `accept` or `sleep` cannot be
interrupted by `kill`, so the suite would hang or run 10 seconds per test. The
same file paused with the 4-argument `select(undef, undef, undef, 0.1)`, which is
not a sleep on Windows.

Make the child's lifetime a function of the protocol, not of signals: after
`accept`, loop on `sysread` until it returns 0 (the client closed), then exit;
the parent just closes its connection and `waitpid`s. Pause with
`Time::HiRes::sleep`. Do not assert one specific error class for an unreachable
host (`10.255.255.1`): a firewall drops, a router answers unreachable, and a
sandbox egress proxy answers refused, so accept all three.

Next time: before skipping a Perl socket suite on Windows, check whether each
fork/kill/sleep in it has a protocol-driven equivalent. Loopback TCP, `IO::Select`
on sockets and `waitpid` on pseudo-pids all work on Strawberry Perl.
