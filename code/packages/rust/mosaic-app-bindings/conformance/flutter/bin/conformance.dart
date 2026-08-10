import 'dart:async';
import 'dart:io';

import 'package:mosaic_flutter_runtime_conformance/mosaic_host.dart';

Never _fail(String assertion) =>
    throw StateError('Failed assertion: $assertion');

void _require(bool condition, String assertion) {
  if (!condition) _fail(assertion);
}

Map<String, Object?> _object(Object? value, String assertion) {
  if (value is! Map) _fail('$assertion was not an object');
  final object = Map<String, Object?>.from(value);
  _require(!object.containsKey('error'), '$assertion returned an error');
  return object;
}

Map<String, Object?> _props(Map<String, Object?> update, String assertion) =>
    _object(update['props'], '$assertion props');

int _integer(Object? value, String assertion) {
  if (value is! num) _fail('$assertion was not numeric');
  return value.toInt();
}

String _expectedPlatform() {
  if (Platform.isMacOS || Platform.isIOS) return 'apple';
  if (Platform.isWindows) return 'windows';
  return 'linux';
}

Future<void> main() async {
  final host = MosaicHost.load();
  if (host == null) _fail('standard Flutter binding did not load the Rust app');

  try {
    final started = _object(await host.props(), 'startup update');
    final startedProps = _props(started, 'startup update');
    _require(
      _integer(started['revision'], 'startup revision') == 1,
      'startup revision',
    );
    _require(
      _integer(startedProps['count'], 'initial count') == 0,
      'initial count',
    );
    _require(
      startedProps['platform'] == _expectedPlatform(),
      'startup platform',
    );
    _require(startedProps['status'] == 'started', 'startup status');

    var notificationCount = 0;
    host.setPropsChangedHandler(() => notificationCount += 1);

    final dispatched = _object(
      await host.handleEvent(<String, Object?>{
        'name': 'increment',
        'payload': <String, Object?>{'amount': 4},
      }),
      'dispatch update',
    );
    final dispatchedProps = _props(dispatched, 'dispatch update');
    _require(
      _integer(dispatched['revision'], 'dispatch revision') == 2,
      'dispatch revision',
    );
    _require(
      _integer(dispatchedProps['count'], 'dispatched count') == 4,
      'dispatched count',
    );
    _require(dispatchedProps['status'] == 'dispatched', 'dispatch status');

    final snapshot = _object(host.snapshot(), 'snapshot');
    _require(
      snapshot['schema'] == 'mosaic-app-conformance/counter',
      'snapshot schema',
    );
    _require(
      _integer(snapshot['version'], 'snapshot version') == 1,
      'snapshot version',
    );
    _require(
      (snapshot['bytes'] as List<Object?>).length == 8,
      'snapshot bytes',
    );

    final restored = _object(host.restore(snapshot), 'restore update');
    final restoredProps = _props(restored, 'restore update');
    _require(
      _integer(restored['revision'], 'restore revision') == 3,
      'restore revision',
    );
    _require(
      _integer(restoredProps['count'], 'restored count') == 4,
      'restored count',
    );
    _require(restoredProps['status'] == 'restored', 'restore status');
    _require(notificationCount == 1, 'restore props-change notification');
  } finally {
    host.dispose();
  }

  stdout.writeln('Mosaic Flutter Rust runtime conformance passed');
}
