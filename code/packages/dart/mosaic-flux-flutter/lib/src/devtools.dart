// devtools.dart — DevTools protocol middleware (v0.1.0 stub).
//
// Per UI33-rewrite §8: every mosaic-flux runtime publishes a uniform
// event stream so the Mosaic DevTools desktop app can attach. v0.1.0
// logs each event to stdout; v0.2.0 will add a TCP socket transport
// on localhost:9229.

import 'action.dart';
import 'middleware.dart';

/// Build a DevTools-protocol middleware. Logs structured events to
/// stdout; v0.2.0 will additionally transmit to localhost:9229.
///
/// `storeName` disambiguates when multiple stores are active.
Middleware<S> devToolsMiddleware<S>({String storeName = 'default'}) {
  return (action, _, __) {
    final timestamp = DateTime.now().toUtc().toIso8601String();
    // ignore: avoid_print
    print(
      '[mosaic-flux-devtools] $timestamp $storeName/${action.runtimeType}',
    );
  };
}
