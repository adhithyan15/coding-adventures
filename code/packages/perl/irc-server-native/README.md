# CodingAdventures::IrcServerNative (Perl)

A **high-performance IRC server for Perl** — every line of IRC and TCP logic runs
in Rust (the [`irc-net-reactor`](../../rust/irc-net-reactor) engine on the
home-grown `kqueue`/`epoll` reactor). Perl only launches and controls the server,
through an XS native extension built on the zero-dependency `perl-bridge`.

## Why

This is the Perl member of a family: one Rust IRC engine, embedded natively in
every language. Because all logic lives in Rust, the binding is a pure lifecycle
control surface — **create, serve, stop** — with no callback into Perl.

## Usage

```perl
use CodingAdventures::IrcServerNative;

my $server = CodingAdventures::IrcServerNative->new(port => 6667);
$server->serve_background;     # runs the loop on a Rust thread
# ... connect IRC clients to $server->local_host : $server->local_port ...
$server->stop;
```

`serve` runs the loop in the calling Perl process and blocks until `stop`;
`serve_background` runs it on a Rust OS thread and returns immediately.

> Unlike the Perl `conduit` binding, `serve_background` is **safe on any Perl**
> (including single-interpreter / non-ithreads builds): the spawned thread runs
> pure Rust and never enters the Perl interpreter, because the IRC server has no
> per-request callback into Perl.

## API

`CodingAdventures::IrcServerNative->new(%opts)` — `host`, `port`, `server_name`,
`motd` (arrayref), `oper_password`, `max_connections`. Returns a server object:

| method            | description                                              |
|-------------------|----------------------------------------------------------|
| `serve`           | run the loop in this process, blocking                   |
| `serve_background`| run the loop on a Rust thread, returns immediately       |
| `stop`            | signal the loop to stop and join the thread              |
| `running`         | whether the loop is running                              |
| `local_host` / `local_port` / `local_addr` | the bound address          |
| `close`           | free the native peer (also runs on `DESTROY`)            |

## How it's built

`BUILD` runs `cargo build --release`, copies the cdylib into
`lib/auto/CodingAdventures/IrcServerNative/` under Perl's `$Config{dlext}`, then
loads the module and runs the `t/` tests with `prove`. The XS is loaded via
`DynaLoader`.

## Safety

Every XSUB body is wrapped in `catch_unwind` (a Rust panic must never unwind into
the interpreter), all Rust→Perl strings use `newSVpvn` (explicit length — never
`strlen`), and `serve`/`serve_background` run an **owned clone** of the engine so
the peer can't dangle.

## Layer position

```
CodingAdventures::IrcServerNative   (this package — Perl facade)
        ↓ IrcServerNative XS extension (perl-bridge; foreground or background serve)
irc-net-reactor   (Rust IRC engine, in-process broadcast)
        ↓
tcp-runtime → transport-platform → kqueue / epoll / IOCP
```
