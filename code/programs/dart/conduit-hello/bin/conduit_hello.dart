// conduit_hello.dart — demo server using coding_adventures_conduit (WEB17)
//
// Shows the idiomatic Dart usage:
//   1. Create Application and register routes/filters/hooks via fluent builder.
//   2. Call bind() to obtain a Server.
//   3. Await server.serve() — keeps the event loop alive while the server runs.
//   4. Ctrl-C triggers SIGINT → stop() → serve() resolves → program exits.
//
// Run:  CONDUIT_CAPI_PATH=<path-to-lib> dart run bin/conduit_hello.dart
// Test: sh tools/run-tests.sh

import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_conduit/conduit.dart';

Future<void> main() async {
  final appName = 'conduit-hello';
  final version = '0.1.0';
  final env = Platform.environment['APP_ENV'] ?? 'production';

  // HTML-encode server-controlled values embedded in the template.
  // Defence-in-depth: even operator-supplied env/version values must not be
  // trusted to be free of HTML metacharacters.
  String htmlEncode(String s) => s
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;');

  final safeAppName = htmlEncode(appName);
  final safeVersion = htmlEncode(version);
  final safeEnv = htmlEncode(env);

  // ── Before-filter: simple API-key guard ───────────────────────────────────
  //
  // Return null to pass through; return Response to short-circuit.

  Response? apiKeyFilter(Request req) {
    if (req.path == '/health' || req.path == '/') return null;
    // Skip auth only in explicitly-whitelisted "development" environments.
    // Default env is "production", so omitting APP_ENV is safe.
    if (env != 'development' && req.header('x-api-key') == null) {
      return Response.json(
          '{"error":"missing x-api-key header"}',
          status: 401);
    }
    return null;
  }

  // ── After-hook: stamp every response with server metadata ─────────────────

  Response metaHook(Request req, Response resp) => resp
      .withHeader('x-served-by', '$appName/$version')
      .withHeader('x-env', env)
      .withHeader('x-content-type-options', 'nosniff');

  // ── Routes ────────────────────────────────────────────────────────────────

  final host = Platform.environment['HOST'] ?? '127.0.0.1';
  final portStr = Platform.environment['PORT'] ?? '3000';
  final port = int.tryParse(portStr) ?? 3000;

  final server = Application()
      .before(apiKeyFilter)
      .after(metaHook)
      // Home page — HTML with safe-encoded server values.
      .get('/', (req) => Response.html('''
<!doctype html>
<html>
  <body>
    <h1>$safeAppName</h1>
    <p>Version: $safeVersion | Env: $safeEnv</p>
    <ul>
      <li><a href="/health">/health</a></li>
      <li><a href="/api/greet/World">/api/greet/:name</a></li>
      <li><a href="/api/search?q=conduit">/api/search?q=…</a></li>
    </ul>
  </body>
</html>
'''))
      // Health check.
      .get('/health', (req) => Response.json(jsonEncode({
            'status': 'ok',
            'name': appName,
            'version': version,
            'env': env,
          })))
      // Route parameter — /api/greet/:name
      .get('/api/greet/:name', (req) {
        final name = req.param('name') ?? 'stranger';
        return Response.json(
            jsonEncode({'greeting': 'Hello, $name!', 'from': appName}));
      })
      // Query string — /api/search?q=…&limit=…
      .get('/api/search', (req) {
        final q = req.query('q') ?? '';
        final limitStr = req.query('limit') ?? '10';
        final n = () {
          final v = int.tryParse(limitStr);
          return (v != null && v >= 1 && v <= 100) ? v : 10;
        }();
        return Response.json(
            jsonEncode({'query': q, 'limit': n, 'results': <String>[]}));
      })
      // Echo body — only mirrors safe content types.
      .post('/api/echo', (req) {
        final ct = req.contentType;
        String ct2;
        if (ct.startsWith('application/json')) {
          ct2 = 'application/json';
        } else if (ct.startsWith('text/plain')) {
          ct2 = 'text/plain; charset=utf-8';
        } else {
          ct2 = 'application/octet-stream';
        }
        return Response.respond(200, req.bodyString(),
            headers: [('content-type', ct2)]);
      })
      // Redirect — 3xx demo.
      .get('/old-home', (req) => Response.redirect('/'))
      // Teapot — demonstrates HaltException.
      .get('/tpot', (req) {
        throw HaltException(Response.text("I'm a teapot", status: 418));
      })
      // 404 handler.
      .notFound((req) => Response.json(
          jsonEncode({'error': 'not found', 'path': req.path}),
          status: 404))
      // Error handler — log real error server-side; generic response to client.
      .onError((req) {
        stderr.writeln('[$appName] handler error: ${req.error}');
        return Response.json(
            '{"error":"internal server error"}', status: 500);
      })
      .bind(host, port);

  // Handle SIGINT (Ctrl-C) gracefully.
  ProcessSignal.sigint.watch().first.then((_) {
    stderr.writeln('\n[$appName] SIGINT — shutting down');
    server.stop();
  });

  print('[$appName] listening on port ${server.localPort} (env=$env)');
  await server.serve(); // blocks until stop() is called
}
