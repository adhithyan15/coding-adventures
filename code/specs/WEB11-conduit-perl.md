# WEB11 — Perl Conduit (XS / perl-bridge port)

## Overview

A Perl port of the Conduit web framework, wrapping the Rust `web-core` engine
(via the WEB08 `conduit` facade) through an XS native library built on the
`perl-bridge` crate. Handlers are plain Perl subs; routing, lifecycle hooks,
and HTTP I/O run in Rust. This closes the original-plan gap (Perl was the last
language from the first port sweep without a Conduit).

```perl
use CodingAdventures::Conduit;

my $app = CodingAdventures::Conduit->new;

$app->before(sub { my $req = shift; $req->path eq '/down' ? halt(503, 'Maintenance') : undef });
$app->get('/',            sub { html('<h1>Hello from Conduit!</h1>') });
$app->get('/hello/:name', sub { my $req = shift; json(qq({"message":"Hello @{[$req->param('name')]}"})) });
$app->post('/echo',       sub { my $req = shift; respond(200, $req->body, { 'content-type' => $req->content_type }) });
$app->not_found(sub { my $req = shift; html('<h1>Not Found: ' . $req->path . '</h1>', 404) });
$app->on_error(sub { json('{"error":"Internal Server Error"}', 500) });

my $server = $app->bind('127.0.0.1', 3000);
$server->serve;   # blocks until stopped
```

## Architecture

```
Perl DSL (CodingAdventures::Conduit, ::Request, ::Response)
    handlers are subs; html/json/text/respond/halt/redirect helpers
    │  Perl coderef ⇄ Rust (perl-bridge: call_coderef; dispatch inline on serve()'s thread)
    ▼
Conduit (Rust XS cdylib, src/lib.rs)  ← boot_CodingAdventures__Conduit
    new_app / add_route / new_server / serve / serve_background / stop ...
    ▼
conduit (WEB08 facade) → web-core → embeddable-http-server → tcp-runtime → kqueue/epoll/IOCP
```

### The threading crux

> **Corrected after implementation.** The original draft of this section assumed
> web-core dispatches requests on background I/O threads, requiring a per-call
> interpreter lock + `PERL_SET_CONTEXT`. That is **not** how the engine actually
> runs, and the first crash was a memory bug, not a threading one. The accurate
> model is below.

The embeddable HTTP engine runs its reactor **inline on the calling thread**.
`HttpServer::bind` builds a single `TcpRuntime` (not the multi-worker
`ShardedTcpRuntime`); `TcpRuntime::serve()` → `StreamReactor::serve()` runs the
event loop and dispatches handlers on whatever thread called `serve()`. So when
Perl calls `$server->serve()`, requests are handled **on the Perl interpreter's
own thread** — exactly what a single-interpreter (non-`MULTIPLICITY`) Perl needs,
since such an interpreter is bound to the thread that created it.

Consequences for this port:

1. **No per-request lock is needed for the foreground path.** Handlers never run
   concurrently — the reactor processes them serially on the calling thread.
   (A `Mutex<()>` still guards the dispatch routine defensively, but it is
   uncontended on the foreground path.)

2. **No context juggling is needed for the foreground path.** Because dispatch
   happens on the original thread, `dTHX`/`PERL_SET_CONTEXT` resolve to the right
   interpreter for free. The captured-context machinery (`get_context`/
   `set_context`) is retained for the threaded path but is a no-op on a
   single-interpreter build.

3. **`serve_background()` is the only thread-spawning path** and is therefore the
   only one that can corrupt a single-interpreter Perl (it calls Perl from a
   freshly spawned OS thread). It is **gated to MULTIPLICITY/ithreads builds** in
   the Perl `Server` class and croaks with guidance on a stock Perl.

The original crash (SIGSEGV on the first dispatch) reproduced **single-threaded**,
which is what unmasked the misdiagnosis. Root cause: `newSVpv(ptr, 0)` treats
`len == 0` as "call `strlen(ptr)`", reading past a non-NUL-terminated empty Rust
`&str` (the first empty field — `QUERY_STRING`). Fix: cross every Rust→Perl
string with explicit length via `newSVpvn`.

For concurrent E2E testing on a stock Perl, run `serve()` in the foreground and
drive it from a **separate client process** — `t/04_server.t` runs the app as its
own OS process and hits it with a raw HTTP/1.0 client.

## Required additions to `perl-bridge`

`perl-bridge` covers SV scalars, arrays (AV), refs, and error croak/warn — but
not calling Perl coderefs, hashes, or thread context. WEB11 adds (Rust
wrappers + C shim, each `dTHX`-wrapped):

| Addition | Purpose |
|----------|---------|
| `perl_bridge_call_coderef(cv, argv, argc, *err_out) -> *SV` | Call a Perl sub with SV args under `G_SCALAR|G_EVAL`; returns the result SV; on `die`, writes a malloc'd error string to `*err_out` and returns NULL. Encapsulates the full PUSHMARK/PUTBACK/call_sv/SPAGAIN stack dance. |
| `perl_bridge_newHV() -> *HV` | Build the env hash. |
| `perl_bridge_hv_store(hv, key, klen, val)` | Populate the env hash. |
| `perl_bridge_newRV_inc(sv) -> *SV` | Make a hashref/arrayref to pass to handlers. |
| `perl_bridge_get_context() -> *PerlInterpreter` | Capture `aTHX` on the main thread (no-op→NULL on non-MULTIPLICITY). |
| `perl_bridge_set_context(interp)` | `PERL_SET_CONTEXT` on a dispatch thread (`#ifdef MULTIPLICITY`). |

All existing perl-bridge tests must still pass (additive change only).

## Marshaling

### Request → Perl

The Rust side builds a flat Perl **env hashref** (CGI/PSGI-style string→string)
and passes it to a thin Perl wrapper sub (installed by `CodingAdventures::Conduit`)
that constructs a `Request` object and calls the user's handler. Nested maps
cross as percent-encoded `k=v&k2=v2` strings (route params, query params,
headers), parsed lazily on the Perl side with `URI::Escape`-free hand decoding
— exactly like the Java port. Keys:

```
REQUEST_METHOD, PATH_INFO, QUERY_STRING, REMOTE_ADDR,
conduit.route_params, conduit.query_params, conduit.headers,
conduit.body, conduit.content_type, conduit.content_length,
conduit.error        # only for the error handler
```

Building a flat hash needs only `newHV`/`hv_store`/`newSVpv` — no nested
HV/AV construction from Rust.

### Response → Rust

A handler returns a Conduit response: an arrayref `[status, body, headers_enc]`
produced by the `html`/`json`/`text`/`respond`/`halt`/`redirect` helpers, where
`headers_enc` is the same percent-encoded `k=v&…` form. Rust reads the AV's
three scalars (`av_fetch` ×3) — no HV iteration. Status is clamped to 100–599;
header names/values with CR/LF or other control chars are dropped (defense
against response splitting), re-checked on the Rust side.

`undef` from a before filter means "continue".

## Halt protocol

`halt(status, body)` and `redirect(location, status)` return a response
arrayref (the common path). For a non-local Sinatra-style halt, a handler may
`die` with a `CodingAdventures::Conduit::HaltError` object; the Rust dispatch
runs under `G_EVAL`, inspects `$@`, and converts a blessed `HaltError` into a
response. Any other `die` routes to the registered `on_error` handler (its
message available via `$req->error`).

## Package layout

```
code/packages/perl/conduit/
├── BUILD                  # cargo build + copy .so to lib/auto/... + prove
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── Makefile.PL
├── Cargo.toml             # cdylib "Conduit"; deps perl-bridge, conduit, web-core
├── build.rs               # -undefined dynamic_lookup on macOS; perl DLL on Windows
├── required_capabilities.json   # ["rust","perl","cargo"]
├── src/lib.rs             # XS boot + native subs (new_app/add_route/new_server/serve/…)
└── lib/CodingAdventures/
    ├── Conduit.pm         # Application + DSL + response helpers + Server + DynaLoader bootstrap
    └── Conduit/Request.pm # Request object over the env hash

# The native XSUBs register directly into the CodingAdventures::Conduit::Native::
# package from the Rust boot function — there is no separate Native.pm.
└── t/
    ├── 01_response.t      # html/json/text/respond/halt/redirect shapes
    ├── 02_request.t       # env → Request, percent decoding, param/query/header
    ├── 03_application.t   # route/filter/handler/setting registration
    └── 04_server.t        # E2E over real TCP via IO::Socket / HTTP::Tiny

code/programs/perl/conduit-hello/   # 8-route demo + tests
```

## Tests (target: 30+)

- Response/Request/Application unit tests (no server).
- `04_server.t` E2E: run the app as a **separate OS process** that binds on
  port 0 and serves in the **foreground** (the reactor dispatches on its own
  main thread — the model a stock single-interpreter Perl needs), publish the
  chosen port, then fire HTTP from the test process via a core `IO::Socket::INET`
  HTTP/1.0 client (no non-core deps). Asserts `/`, `/hello/:name`, POST `/echo`,
  query params, before-filter halt(503), dying-handler→on_error(500),
  not_found(404), redirect(302). A wall-clock guard (`alarm`) bounds any hang and
  the server child is always reaped.
- `conduit-hello`: full demo + `t/smoke.t` integration test (same launch-as-
  process pattern).

**Realized:** 79 assertions across `t/01`–`t/04` (exceeds the 30+ target).

## Out of scope

- Binary response bodies (UTF-8 strings, as in every port).
- Perl `ithreads` user threads — the framework serializes via the interpreter
  lock; it does not run handlers on multiple Perl threads.

## Future

The percent-encoded marshaling + `call_coderef` pattern is reusable for any
future single-interpreter dynamic-language port.
