# CodingAdventures::Conduit (Perl)

A Sinatra/Express-style web framework for Perl, implemented as a thin Perl
layer over the Rust **web-core** HTTP engine (via the WEB08 `conduit` facade).
Routing, lifecycle hooks, and the HTTP I/O loop all run in Rust; your handlers
are ordinary Perl subs. This is the WEB11 port in the cross-language Conduit
family (Python, Ruby, TypeScript, Rust, Java, Kotlin, Lua, Elixir, …).

## How it fits in the stack

```
your Perl handlers
        │  (Perl subs returning [status, headers, body])
CodingAdventures::Conduit            ← this package (Perl + XS)
        │  perl-bridge (zero-dep Perl C-API wrapper)
conduit  (WEB08 Rust facade)
        │
web-core → embeddable-http-server → tcp-runtime   ← the engine
```

The native half is a Rust `cdylib` (`libConduit`) loaded with `DynaLoader`.
It speaks to the interpreter through **perl-bridge**, the repo's zero-dependency
wrapper around Perl's C API — no XS::* CPAN modules, no FFI crates.

## Install / build

This package builds a native library, so it needs a Rust toolchain and the
Perl whose headers it links against:

```sh
cd code/packages/perl/conduit
cargo build --release
# stage the lib where DynaLoader looks for it, then test:
prove -Ilib t/
```

The `BUILD` script does all of this (resolving `perl` via `mise` when present).

## Quick start

```perl
use CodingAdventures::Conduit qw(:all);

my $app = CodingAdventures::Conduit->new;

# Before filter — return a response to short-circuit, or undef to continue.
$app->before(sub {
    my $req = shift;
    return halt(503, 'maintenance') if $req->path eq '/down';
    return undef;
});

$app->get('/', sub { html('<h1>Hello from Conduit!</h1>') });

$app->get('/hello/:name', sub {
    my $req = shift;
    json(sprintf('{"hi":"%s"}', $req->param('name')));
});

$app->post('/echo', sub {
    my $req = shift;
    respond(200, $req->body, { 'content-type' => $req->content_type });
});

$app->not_found(sub { my $req = shift; text('no route: ' . $req->path, 404) });
$app->on_error(sub  { json('{"error":"oops"}', 500) });

my $server = $app->bind('127.0.0.1', 3000);
$server->serve;   # blocks until stopped
```

## Response helpers

Exported with `qw(:all)`. Each returns `[ $status, \%headers, $body ]`:

| Helper | Status | Content-Type |
| ------ | ------ | ------------ |
| `html($body, $status?)`     | 200 | `text/html; charset=utf-8` |
| `json($body, $status?)`     | 200 | `application/json` |
| `text($body, $status?)`     | 200 | `text/plain; charset=utf-8` |
| `respond($status, $body, \%headers?)` | as given | as given |
| `halt($status, $body?)`     | as given | `text/plain; charset=utf-8` |
| `redirect($location, $status?)` | 302 | sets `Location` |

`redirect` dies if the location contains CR or LF (response-splitting guard).
You may also raise a non-local halt anywhere in a handler:

```perl
die CodingAdventures::Conduit::HaltError->new(403, 'forbidden');
```

## The Request object

Handlers receive a `CodingAdventures::Conduit::Request` (read-only):

| Method | Description |
| ------ | ----------- |
| `method`, `path`, `query_string`, `body`, `content_type`, `remote_addr` | request line / metadata |
| `param($name)`        | a `:name` route parameter |
| `query_param($name)`  | a query-string value |
| `header($name)`       | a request header (case-insensitive) |
| `params`, `query_params`, `headers` | the full hashrefs |
| `error`               | inside `on_error`, the error message |

## Application DSL

`new` → `get/post/put/delete/patch($pattern, $sub)` → `before/after($sub)` →
`not_found($sub)` → `on_error($sub)` → `set($k, $v)` / `get_setting($k)` →
`bind($host, $port, $backlog?)`. Every registration call returns `$self`, so
they chain. `bind` returns a `Server`.

## Server

| Method | Description |
| ------ | ----------- |
| `serve`            | run the request loop in the **foreground** (blocks) |
| `serve_background` | spawn a server thread — **requires a MULTIPLICITY/ithreads Perl**; croaks otherwise |
| `stop`             | stop a running server |
| `local_port`       | the bound port (useful after binding to port 0) |
| `running`          | boolean |

### Threading note

The embeddable HTTP engine runs its reactor **inline on the calling thread** —
`serve()` dispatches your handlers on the thread that called it. That is exactly
what a default (single-interpreter) Perl needs, since such an interpreter is
bound to the thread that created it. `serve_background` is the only path that
spawns a thread, so it is gated to threaded Perls. For concurrent testing on a
stock Perl, run `serve()` in the foreground and drive it from a **separate
client process** (this is what `t/04-server.t` does).

## Security posture

- **Response-splitting**: header names/values containing CR or LF are dropped
  during marshaling; `redirect` rejects CR/LF in the location.
- **Status clamping**: status codes are clamped to 100–599 in the native layer.
- **No `strlen` footguns**: all Rust→Perl strings cross with explicit lengths
  (`newSVpvn`), so empty/binary values never read out of bounds.
- **Percent-encoding**: route params, query params, and headers cross the FFI
  boundary percent-encoded, so delimiters in values can't corrupt the framing.

## Tests

`prove -Ilib t/` — 79 assertions across response helpers, the Request decoder,
the Application DSL / HaltError plumbing, and a full end-to-end server run
(routes, params, body echo, query, before-halt, dying-handler→on_error,
custom 404, redirect) with an `alarm()` hang guard.

## License

Part of the coding-adventures learning monorepo.
