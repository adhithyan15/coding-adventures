import 'dart:convert';
import 'dart:ffi' show DynamicLibrary;
import 'dart:io';
import 'dart:isolate';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mosaic_venture_chrome/main.dart';
import 'package:mosaic_venture_chrome/mosaic_host.dart';

Future<void> _servePages(SendPort port) async {
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  port.send(server.port);
  await for (final request in server) {
    print('flutter-live-server-request=${request.uri.path}');
    request.response.headers.contentType = ContentType.html;
    request.response.persistentConnection = false;
    late final String body;
    switch (request.uri.path) {
      case '/start':
        body =
            '''
          <html><head><title>Flutter Start</title></head><body>
          <a href="/link">Open the Flutter link target</a>
          ${List<String>.generate(80, (index) => '<p>scroll row $index</p>').join()}
          </body></html>
        ''';
        break;
      case '/target':
        body =
            '<html><head><title>Flutter Address Target</title></head>'
            '<body>address navigation reached the shared browser</body></html>';
        break;
      case '/link':
        body =
            '<html><head><title>Flutter Link Target</title></head>'
            '<body>native Flutter pointer activation reached Cairo</body></html>';
        break;
      default:
        request.response.statusCode = HttpStatus.notFound;
        body = 'missing page';
    }
    request.response.contentLength = utf8.encode(body).length;
    request.response.write(body);
    await request.response.close();
  }
}

Future<(Isolate, int)> _startPageServer() async {
  final receivePort = ReceivePort();
  final isolate = await Isolate.spawn(_servePages, receivePort.sendPort);
  final port = await receivePort.first as int;
  receivePort.close();
  return (isolate, port);
}

Future<void> _pumpLiveVentureShell(WidgetTester tester, MosaicHost host) async {
  await tester.pumpWidget(MosaicApp(mosaicHost: host));
  await _settleSurface(tester, host);
  expect(find.text('Flutter Start'), findsOneWidget);
  expect(find.byKey(const Key('venture-content-image')), findsOneWidget);
}

Future<void> _settleSurface(WidgetTester tester, MosaicHost host) async {
  await tester.pump();
  await tester.runAsync(
    () => host.surfaceReady.timeout(const Duration(seconds: 10)),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets(
    'package-owned Flutter shell drives the live shared browser and Cairo page',
    (WidgetTester tester) async {
      final libraryPath =
          Platform.environment['VENTURE_BROWSER_FLUTTER_LIBRARY'];
      expect(
        libraryPath,
        isNotNull,
        reason: 'the direct Flutter gate requires the shared Rust bridge',
      );
      await tester.binding.setSurfaceSize(const Size(1400, 960));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      DynamicLibrary.open(libraryPath!);
      final setup = await tester.runAsync(() async {
        final (serverIsolate, port) = await _startPageServer();
        debugPrint('flutter-live-stage=server-ready port=$port');
        final startUrl = 'http://127.0.0.1:$port/start';
        final host = MosaicHost.open(
          libraryPath: libraryPath,
          startUrl: startUrl,
        );
        return (serverIsolate, port, host);
      });
      final (serverIsolate, port, host) = setup!;
      addTearDown(() => serverIsolate.kill(priority: Isolate.immediate));
      addTearDown(host.dispose);
      final targetUrl = 'http://127.0.0.1:$port/target';
      final linkUrl = 'http://127.0.0.1:$port/link';
      debugPrint('flutter-live-stage=host-open');
      await _pumpLiveVentureShell(tester, host);
      debugPrint('flutter-live-stage=shell-pumped');

      final input = find.byType(TextField);
      await tester.enterText(input, targetUrl);
      await tester.testTextInput.receiveAction(TextInputAction.done);
      await tester.pumpAndSettle();
      expect(find.text('Flutter Address Target'), findsOneWidget);
      debugPrint('flutter-live-stage=address-navigation');
      expect(
        tester
            .widget<ElevatedButton>(find.widgetWithText(ElevatedButton, 'Back'))
            .onPressed,
        isNotNull,
      );

      await tester.tap(find.text('Back'));
      await tester.pumpAndSettle();
      debugPrint('flutter-live-stage=history');
      expect(find.text('Flutter Start'), findsOneWidget);
      await tester.tap(find.text('Forward'));
      await tester.pumpAndSettle();
      expect(find.text('Flutter Address Target'), findsOneWidget);
      await tester.tap(find.text('Back'));
      await tester.pumpAndSettle();

      final surface = find.byKey(const Key('venture-content-surface'));
      final center = tester.getCenter(surface);
      final beforeScroll = host.scrollMetrics!;
      expect(beforeScroll['max'], greaterThan(0));
      await tester.sendEventToBinding(
        PointerScrollEvent(position: center, scrollDelta: const Offset(0, 320)),
      );
      await tester.pumpAndSettle();
      expect(host.scrollMetrics!['offset'], greaterThan(0));
      await tester.sendEventToBinding(
        PointerScrollEvent(
          position: center,
          scrollDelta: const Offset(0, -10000),
        ),
      );
      await tester.pumpAndSettle();
      expect(host.scrollMetrics!['offset'], 0);
      debugPrint('flutter-live-stage=scroll');

      final linkPoint = tester.getTopLeft(surface) + const Offset(32, 26);
      await tester.sendEventToBinding(PointerHoverEvent(position: linkPoint));
      await tester.pumpAndSettle();
      expect(find.text(linkUrl), findsOneWidget);
      debugPrint('flutter-live-stage=hover');

      await tester.tapAt(linkPoint);
      await tester.pumpAndSettle();
      expect(find.text('Flutter Link Target'), findsOneWidget);
      expect(tester.widget<TextField>(input).controller!.text, linkUrl);
      expect(find.byKey(const Key('venture-content-image')), findsOneWidget);
      debugPrint('flutter-live-stage=link');
    },
  );
}
