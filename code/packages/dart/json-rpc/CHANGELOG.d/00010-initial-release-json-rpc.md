## 0.1.0

- Initial Dart lane of the json-rpc identity: JSON-RPC 2.0 messages
  (Request/Response/Notification + parse/serialize), Content-Length
  framing reader and writer over in-memory bytes, and a dispatch
  Server with panic-catching handlers; `ResponseError` carries the
  spec code table and strict struct decoding. JSON values are plain
  `Object?` with serde-shaped deep equality. Fail-closed guards
  where the platform diverges: out-of-int64 integers, lone
  surrogates, and leading BOMs are rejected, and short payloads are
  consumed like `read_exact`.
- Tests mirror the reference suite (5 server tests) and add the
  untested-module matrix, 7 seeded property tests over the in-lane
  `pbt` package, and a 2,000-iteration hostile fuzz campaign over
  the in-lane `fuzz` package; 100% line/branch/function coverage.
