# Changelog — CodingAdventures::Conduit (Perl)

All notable changes to the Perl Conduit port are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/).

## [0.1.0] - 2026-06-13

### Added — WEB11 Perl Conduit port

Initial release: a Sinatra/Express-style web framework for Perl over the Rust
**web-core** engine (WEB08 `conduit` facade), loaded as an XS `cdylib` through
the zero-dependency **perl-bridge** wrapper.

- **Application DSL**: `new`, `get/post/put/delete/patch`, `before`, `after`,
  `not_found`, `on_error`, `set`/`get_setting`, `bind`. All registration calls
  are chainable.
- **Response helpers** (`:all`): `html`, `json`, `text`, `respond`, `halt`,
  `redirect` — each returns `[status, \%headers, body]`. `redirect` rejects
  CR/LF in the location.
- **Non-local halt**: `CodingAdventures::Conduit::HaltError`, caught in the
  handler wrapper and converted to a response.
- **Request object**: `method`/`path`/`query_string`/`body`/`content_type`/
  `remote_addr`/`error` plus `param`/`query_param`/`header` (and the full
  `params`/`query_params`/`headers` hashrefs), decoded lazily.
- **Server**: `serve` (foreground), `serve_background` (gated to MULTIPLICITY/
  ithreads Perls — croaks on a single-interpreter build), `stop`, `local_port`,
  `running`.
- **perl-bridge extensions** (in `code/packages/rust/perl-bridge`): `call_coderef`
  (PUSHMARK/call_sv/G_EVAL stack dance with `$@` capture), `new_hv`/`hv_store`/
  `new_rv_inc`, and `get_context`/`set_context` (no-ops on non-MULTIPLICITY).

### Security

- Header names/values containing CR or LF are dropped during marshaling
  (response-splitting defense); `redirect` rejects CR/LF in the location.
- Status codes clamped to 100–599 in the native layer.
- All Rust→Perl strings cross with explicit lengths (`newSVpvn`), eliminating the
  `newSVpv`/`strlen`-on-empty out-of-bounds read.
- Route params, query params, and headers cross the FFI boundary
  percent-encoded.
- `serve_background` is gated at BOTH the Perl layer (croaks) and the native
  layer (warns and refuses to spawn when the captured interpreter context is
  null, i.e. a single-interpreter build) — defense-in-depth against calling the
  interpreter from a spawned thread.

### Tests

79 assertions: response helpers (`t/01`), the Request decoder (`t/02`), the
Application DSL + HaltError plumbing (`t/03`), and a full end-to-end server run
(`t/04`) — the server runs as its own OS process, driven by a raw HTTP/1.0
client with an `alarm()` hang guard.

### Notes

The embeddable HTTP engine's reactor runs inline on the calling thread, so
foreground `serve()` dispatches handlers on the original interpreter thread —
the dispatch model a single-interpreter Perl requires. No web-core change was
needed; only `serve_background` spawns a thread, hence its gating.
