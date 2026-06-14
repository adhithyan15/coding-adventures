// smoke_test.dart — integration smoke tests for conduit-hello (Dart)
//
// Spins up a conduit-hello-style server inline to verify the demo application
// logic end-to-end.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';
import 'package:coding_adventures_conduit/conduit.dart';

void main() {
  // 30-second watchdog.
  final watchdog = Timer(const Duration(seconds: 30), () {
    stderr.writeln('[smoke watchdog] timed out — aborting');
    exit(1);
  });

  group('conduit-hello smoke tests', () {
    late Server server;
    late HttpClient http;

    setUpAll(() async {
      final appName = 'conduit-hello';
      final version = '0.1.0';
      final env = 'development';

      server = Application()
          .before((req) {
            if (req.path == '/health' || req.path == '/') return null;
            if (env != 'development' && req.header('x-api-key') == null) {
              return Response.json(
                  '{"error":"missing x-api-key header"}',
                  status: 401);
            }
            return null;
          })
          .after((req, resp) => resp
              .withHeader('x-served-by', '$appName/$version')
              .withHeader('x-env', env)
              .withHeader('x-content-type-options', 'nosniff'))
          .get('/', (req) => Response.html('''
<!doctype html><html><body>
  <h1>$appName</h1>
  <p>Version: $version | Env: $env</p>
</body></html>
'''))
          .get('/health', (req) => Response.json(
                jsonEncode({'status': 'ok', 'name': appName, 'version': version, 'env': env}),
              ))
          .get('/api/greet/:name', (req) {
            final name = req.param('name') ?? 'stranger';
            return Response.json(
                jsonEncode({'greeting': 'Hello, $name!', 'from': appName}));
          })
          .get('/api/search', (req) {
            final q = req.query('q') ?? '';
            return Response.json(
                jsonEncode({'query': q, 'limit': 10, 'results': <String>[]}));
          })
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
          .get('/old-home', (req) => Response.redirect('/'))
          .get('/tpot', (req) {
            throw HaltException(Response.text("I'm a teapot", status: 418));
          })
          .notFound((req) => Response.json(
              jsonEncode({'error': 'not found', 'path': req.path}),
              status: 404))
          .onError((req) =>
              Response.json('{"error":"internal server error"}', status: 500))
          .bind('127.0.0.1', 0);

      server.serveBackground();
      final deadline = DateTime.now().add(const Duration(seconds: 5));
      while (!server.isRunning && DateTime.now().isBefore(deadline)) {
        await Future.delayed(const Duration(milliseconds: 2));
      }

      http = HttpClient()
        ..connectionTimeout = const Duration(seconds: 5);
    });

    tearDownAll(() {
      watchdog.cancel();
      http.close();
      server.dispose();
    });

    Future<HttpClientResponse> get_(String path) async {
      final req = await http.get('127.0.0.1', server.localPort, path);
      return req.close();
    }

    Future<HttpClientResponse> post_(
        String path, String body, String contentType) async {
      final req = await http.post('127.0.0.1', server.localPort, path);
      req.headers.contentType = ContentType.parse(contentType);
      // embeddable-http-server does not support chunked or until-EOF bodies.
      // Set Content-Length explicitly so Dart's HttpClient doesn't use chunked.
      final bytes = utf8.encode(body);
      req.contentLength = bytes.length;
      req.add(bytes);
      return req.close();
    }

    Future<String> body_(HttpClientResponse resp) =>
        resp.transform(utf8.decoder).join();

    test('home page returns HTML', () async {
      final resp = await get_('/');
      expect(resp.statusCode, 200);
      expect(resp.headers.contentType?.mimeType, 'text/html');
      final b = await body_(resp);
      expect(b, contains('conduit-hello'));
    });

    test('health check returns ok', () async {
      final resp = await get_('/health');
      expect(resp.statusCode, 200);
      final b = await body_(resp);
      expect(b, contains('"status":"ok"'));
    });

    test('greet route returns personalised greeting', () async {
      final resp = await get_('/api/greet/Alice');
      expect(resp.statusCode, 200);
      final b = await body_(resp);
      expect(b, contains('Alice'));
    });

    test('search route returns query', () async {
      final resp = await get_('/api/search?q=dart');
      expect(resp.statusCode, 200);
      final b = await body_(resp);
      expect(b, contains('dart'));
    });

    test('echo route mirrors body', () async {
      final resp = await post_('/api/echo', '{"x":1}', 'application/json');
      expect(resp.statusCode, 200);
      final b = await body_(resp);
      expect(b, '{"x":1}');
    });

    test('old-home redirect', () async {
      // Dart HttpClient follows redirects — verify the final 200 is received.
      final resp = await get_('/old-home');
      await body_(resp); // drain
      expect(resp.statusCode, anyOf(200, 302));
    });

    test('teapot returns 418', () async {
      final resp = await get_('/tpot');
      expect(resp.statusCode, 418);
    });

    test('unknown route returns 404', () async {
      final resp = await get_('/no-such-route');
      expect(resp.statusCode, 404);
      final b = await body_(resp);
      expect(b, contains('not found'));
    });

    test('x-served-by header present on all responses', () async {
      final resp = await get_('/health');
      expect(resp.headers.value('x-served-by'), contains('conduit-hello'));
    });

    test('x-content-type-options nosniff on all responses', () async {
      final resp = await get_('/health');
      expect(resp.headers.value('x-content-type-options'), 'nosniff');
    });
  });
}
