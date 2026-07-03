# coding_adventures_conduit

Sinatra/Express-style web framework for Dart 3, built over the `conduit-capi`
Rust cdylib (WEB12). Part of the coding-adventures monorepo.

## How it fits in the stack

```
Your Dart code
    │
    ▼  dart:ffi (DynamicLibrary.open + NativeCallable.isolateLocal)
coding_adventures_conduit
    │
    ▼  C ABI (conduit_server_bind, conduit_app_add_route, …)
libconduit_capi.so / .dylib  (Rust: WEB12)
    │
    ▼
web-core (WEB00) + embeddable-http-server
```

## Usage

```dart
import 'package:coding_adventures_conduit/conduit.dart';

Future<void> main() async {
  final server = Application()
      .get('/', (req) => Response.html('<h1>Hello, Dart!</h1>'))
      .get('/api/:id', (req) {
        final id = req.param('id') ?? 'unknown';
        return Response.json('{"id":"$id"}');
      })
      .post('/echo', (req) => Response.text(req.bodyString()))
      .before((req) {
        if (req.header('x-api-key') == null) {
          return Response.json('{"error":"unauthorized"}', status: 401);
        }
        return null;
      })
      .after((req, resp) =>
          resp.withHeader('x-served-by', 'my-app/1.0'))
      .notFound((req) =>
          Response.json('{"error":"not found"}', status: 404))
      .bind('127.0.0.1', 3000);

  print('listening on port \${server.localPort}');
  await server.serve(); // blocks until stop() is called
}
```

## Response API

```dart
Response.html(body, {int status = 200})
Response.json(body, {int status = 200})
Response.text(body, {int status = 200})
Response.redirect(location, {int status = 302})
Response.respond(status, body, {List<(String, String)> headers = const []})

resp.withStatus(int status)
resp.withHeader(String name, String value)
```

## Request API

```dart
req.method        // "GET", "POST", …
req.path          // "/api/users/42"
req.queryString   // "q=hello&page=2"
req.contentType   // "application/json"
req.remoteAddr    // "127.0.0.1:54321"
req.param('id')   // named route parameter → String?
req.query('q')    // query string value → String?
req.header('x-api-key')  // request header → String?
req.body()        // Uint8List
req.bodyString()  // String (UTF-8)
```

## Threading model

Dart runs on a single isolate event loop. `NativeCallable.isolateLocal` ensures
that callbacks from Rust/Tokio worker threads are posted to the Dart event loop
and the calling Rust thread blocks until Dart returns. `serve()` is therefore
`async` rather than synchronous — always `await server.serve()` from an
`async main()`.

## Running tests

```sh
sh tools/run-tests.sh
```

The script builds `conduit-capi` in release mode, sets `CONDUIT_CAPI_PATH`, and
runs `dart test`.
