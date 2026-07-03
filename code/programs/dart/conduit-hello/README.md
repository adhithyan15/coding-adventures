# conduit-hello (Dart)

Demo Dart 3 server using `coding_adventures_conduit` (WEB17).

## Running

```sh
# Build conduit-capi first
(cd ../../packages/rust/conduit-capi && cargo build --release)
export CONDUIT_CAPI_PATH=../../packages/rust/target/release/libconduit_capi.dylib  # macOS

dart pub get
dart run bin/conduit_hello.dart
```

## Endpoints

| Method | Path | Description |
|---|---|---|
| GET | / | HTML home page |
| GET | /health | JSON health check |
| GET | /api/greet/:name | Personalised greeting |
| GET | /api/search?q=…&limit=… | Search (stub) |
| POST | /api/echo | Echo request body |
| GET | /old-home | 302 redirect to / |
| GET | /tpot | 418 via HaltException |

## Testing

```sh
sh tools/run-tests.sh
```
