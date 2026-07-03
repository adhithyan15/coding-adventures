/// Conduit — Sinatra/Express-style web framework for Dart 3 (WEB17).
///
/// Wraps the conduit-capi Rust cdylib (WEB12) via dart:ffi, providing a
/// fluent object-oriented API:
///
/// ```dart
/// import 'package:coding_adventures_conduit/conduit.dart';
///
/// Future<void> main() async {
///   final server = Application()
///     .get('/', (req) => Response.html('<h1>Hello, Dart!</h1>'))
///     .post('/echo', (req) => Response.text(req.bodyString()))
///     .bind('127.0.0.1', 3000);
///   print('listening on port ${server.localPort}');
///   await server.serve();
/// }
/// ```
library conduit;

export 'src/response.dart' show Response, HaltException;
export 'src/request.dart' show Request;
export 'src/application.dart' show Application;
export 'src/server.dart' show Server;
